use openaction::{EventHandlerResult, OutboundEventManager, SettingsValue};
use plotters::prelude::{ChartBuilder, IntoDrawingArea, LineSeries, RGBColor, SVGBackend};
use std::{
	collections::{HashMap, VecDeque},
	sync::{
		Arc, LazyLock, Mutex,
		atomic::{AtomicBool, Ordering},
	},
	time::Duration,
};
use tokio::sync::mpsc;

use crate::{
	live::{self, LiveAction},
	pixel::{data_uri, text_path},
};

pub const ACTION: &str = "com.amansprojects.starterpack.systemmonitor";
const SAMPLE_INTERVAL: Duration = Duration::from_secs(1);
const HISTORY_LENGTH: usize = 30;

#[derive(Clone, Debug, PartialEq)]
struct MonitorSnapshot {
	cpu: u8,
	gpu: Option<u8>,
	memory: u8,
	memory_total_mib: u64,
	memory_available_mib: u64,
	pagefile_total_mib: u64,
	pagefile_available_mib: u64,
}

impl MonitorSnapshot {
	fn memory_used_mib(&self) -> u64 {
		self.memory_total_mib
			.saturating_sub(self.memory_available_mib)
	}

	fn pagefile_used_mib(&self) -> u64 {
		self.pagefile_total_mib
			.saturating_sub(self.pagefile_available_mib)
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum DisplayMode {
	Compact,
	#[default]
	Normal,
	Full,
}

impl DisplayMode {
	fn from_settings(settings: &SettingsValue) -> Self {
		match settings
			.as_object()
			.and_then(|settings| settings.get("mode"))
			.and_then(SettingsValue::as_str)
		{
			Some("compact" | "mini") => Self::Compact,
			Some("full") => Self::Full,
			_ => Self::Normal,
		}
	}
}

#[derive(Clone, Copy, Debug)]
struct HistoryPoint {
	cpu: u8,
	gpu: Option<u8>,
	memory: u8,
}

impl From<&MonitorSnapshot> for HistoryPoint {
	fn from(snapshot: &MonitorSnapshot) -> Self {
		Self {
			cpu: snapshot.cpu,
			gpu: snapshot.gpu,
			memory: snapshot.memory,
		}
	}
}

#[derive(Clone)]
struct RenderState {
	snapshot: MonitorSnapshot,
	history: VecDeque<HistoryPoint>,
}

static LIVE: LazyLock<Mutex<LiveAction>> = LazyLock::new(Default::default);
static MODES: LazyLock<Mutex<HashMap<String, DisplayMode>>> = LazyLock::new(Default::default);
static LAST_STATE: LazyLock<Mutex<Option<RenderState>>> = LazyLock::new(Default::default);

fn meter(label: &str, value: Option<u8>, color: &str, y: u8) -> String {
	const SEGMENTS: usize = 12;
	let active = value
		.map(|value| (usize::from(value) * SEGMENTS).div_ceil(100))
		.unwrap_or_default();
	let value = value
		.map(|value| format!("{value}%"))
		.unwrap_or_else(|| "--".to_owned());
	let label = text_path(label, 20.0, y + 12, 9, color);
	let value = text_path(&value, 47.0, y + 26, 15, "#f8fafc");
	let mut bars = String::with_capacity(SEGMENTS * 70);
	for index in 0..SEGMENTS {
		let x = 66 + index * 9;
		let fill = if index < active { color } else { "#171c24" };
		bars.push_str(&format!(
			r##"<rect x="{x}" y="{}" width="7" height="16" fill="{fill}"/>"##,
			y + 8
		));
	}
	format!(
		r##"<rect x="2" y="{y}" width="172" height="34" fill="#080b10"/>
<rect x="2" y="{y}" width="3" height="34" fill="{color}"/>
{label}{value}{bars}"##
	)
}

fn medium_image(snapshot: &MonitorSnapshot) -> String {
	let cpu = meter("CPU", Some(snapshot.cpu), "#20e3ff", 3);
	let gpu = meter("GPU", snapshot.gpu, "#ff4fd8", 39);
	let memory = meter("RAM", Some(snapshot.memory), "#facc15", 75);
	let svg = format!(
		r##"<svg xmlns="http://www.w3.org/2000/svg" width="176" height="112" viewBox="0 0 176 112" shape-rendering="crispEdges">
<rect width="176" height="112" fill="#000"/>
{cpu}{gpu}{memory}
</svg>"##
	);
	data_uri(&svg)
}

fn compact_image(snapshot: &MonitorSnapshot) -> String {
	let labels = [
		text_path("CPU", 29.0, 44, 12, "#20e3ff"),
		text_path("GPU", 88.0, 44, 12, "#ff4fd8"),
		text_path("MEM", 147.0, 44, 12, "#facc15"),
	];
	let values = [
		text_path(&format!("{}%", snapshot.cpu), 29.0, 72, 16, "#f8fafc"),
		text_path(
			&snapshot
				.gpu
				.map(|value| format!("{value}%"))
				.unwrap_or_else(|| "--".to_owned()),
			88.0,
			72,
			16,
			"#f8fafc",
		),
		text_path(
			&format!("{}GB", (snapshot.memory_used_mib() + 512) / 1024),
			147.0,
			72,
			16,
			"#f8fafc",
		),
	];
	data_uri(&format!(
		r##"<svg xmlns="http://www.w3.org/2000/svg" width="176" height="112" viewBox="0 0 176 112" shape-rendering="crispEdges">
<rect width="176" height="112" fill="#000"/>
<path d="M58 27v58M117 27v58" stroke="#171c24"/>
{}{}
</svg>"##,
		labels.concat(),
		values.concat()
	))
}

fn gibibytes(mib: u64) -> String {
	format!("{:.1}", mib as f64 / 1024.0)
}

fn chart_fragment(history: &VecDeque<HistoryPoint>) -> Option<String> {
	let mut svg = String::new();
	{
		let root = SVGBackend::with_string(&mut svg, (172, 43)).into_drawing_area();
		root.fill(&RGBColor(4, 8, 14)).ok()?;
		let mut chart = ChartBuilder::on(&root)
			.margin(1)
			.build_cartesian_2d(0i32..(HISTORY_LENGTH as i32 - 1), 0i32..100i32)
			.ok()?;
		let offset = HISTORY_LENGTH.saturating_sub(history.len());
		let points = |value: fn(&HistoryPoint) -> Option<u8>| {
			history
				.iter()
				.enumerate()
				.filter_map(move |(index, point)| {
					value(point).map(|value| ((offset + index) as i32, i32::from(value)))
				})
		};
		chart
			.draw_series(LineSeries::new(
				points(|point| Some(point.cpu)),
				RGBColor(32, 227, 255),
			))
			.ok()?;
		chart
			.draw_series(LineSeries::new(
				points(|point| point.gpu),
				RGBColor(255, 79, 216),
			))
			.ok()?;
		chart
			.draw_series(LineSeries::new(
				points(|point| Some(point.memory)),
				RGBColor(250, 204, 21),
			))
			.ok()?;
		root.present().ok()?;
	}
	let start = svg.find('>')? + 1;
	let end = svg.rfind("</svg>")?;
	Some(svg[start..end].to_owned())
}

fn full_image(snapshot: &MonitorSnapshot, history: &VecDeque<HistoryPoint>) -> String {
	let headings = [
		text_path("CPU", 29.0, 9, 7, "#20e3ff"),
		text_path("GPU", 88.0, 9, 7, "#ff4fd8"),
		text_path("RAM", 147.0, 9, 7, "#facc15"),
	];
	let values = [
		text_path(&format!("{}%", snapshot.cpu), 29.0, 22, 10, "#f8fafc"),
		text_path(
			&snapshot
				.gpu
				.map(|value| format!("{value}%"))
				.unwrap_or_else(|| "--".to_owned()),
			88.0,
			22,
			10,
			"#f8fafc",
		),
		text_path(&format!("{}%", snapshot.memory), 147.0, 22, 10, "#f8fafc"),
	];
	let ram = text_path(
		&format!(
			"RAM {} / {} GB",
			gibibytes(snapshot.memory_used_mib()),
			gibibytes(snapshot.memory_total_mib)
		),
		88.0,
		83,
		7,
		"#dbeafe",
	);
	let free = text_path(
		&format!(
			"FREE {} GB  LOAD {}%",
			gibibytes(snapshot.memory_available_mib),
			snapshot.memory
		),
		88.0,
		94,
		7,
		"#cbd5e1",
	);
	let page = text_path(
		&format!(
			"PAGE {} / {} GB",
			gibibytes(snapshot.pagefile_used_mib()),
			gibibytes(snapshot.pagefile_total_mib)
		),
		88.0,
		105,
		7,
		"#94a3b8",
	);
	let chart = chart_fragment(history).unwrap_or_default();
	data_uri(&format!(
		r##"<svg xmlns="http://www.w3.org/2000/svg" width="176" height="112" viewBox="0 0 176 112">
<rect width="176" height="112" fill="#000"/>
{}{}
<svg x="2" y="27" width="172" height="43" viewBox="0 0 172 43">{chart}</svg>
<rect x="2" y="27" width="172" height="43" fill="none" stroke="#1e293b"/>
{ram}{free}{page}
</svg>"##,
		headings.concat(),
		values.concat(),
	))
}

fn monitor_image(
	snapshot: &MonitorSnapshot,
	mode: DisplayMode,
	history: &VecDeque<HistoryPoint>,
) -> String {
	match mode {
		DisplayMode::Compact => compact_image(snapshot),
		DisplayMode::Normal => medium_image(snapshot),
		DisplayMode::Full => full_image(snapshot, history),
	}
}

fn loading_image() -> String {
	let title = text_path("SYSTEM", 88.0, 49, 17, "#20e3ff");
	let status = text_path("LOADING...", 88.0, 72, 10, "#a7b0c0");
	data_uri(&format!(
		r##"<svg xmlns="http://www.w3.org/2000/svg" width="176" height="112" viewBox="0 0 176 112" shape-rendering="crispEdges">
<rect width="176" height="112" fill="#000"/>
<path d="M32 28h8v-8h96v8h8v56h-8v8H40v-8h-8z" fill="#080b10" stroke="#20e3ff" stroke-width="2"/>
{title}{status}
</svg>"##
	))
}

async fn render(mut receiver: mpsc::Receiver<MonitorSnapshot>) {
	let mut history = VecDeque::with_capacity(HISTORY_LENGTH);
	while let Some(snapshot) = receiver.recv().await {
		if history.len() == HISTORY_LENGTH {
			history.pop_front();
		}
		history.push_back(HistoryPoint::from(&snapshot));
		*LAST_STATE.lock().unwrap() = Some(RenderState {
			snapshot: snapshot.clone(),
			history: history.clone(),
		});
		let modes = MODES.lock().unwrap().clone();
		if let Err(error) = live::broadcast_mapped(&LIVE, |context| {
			monitor_image(
				&snapshot,
				modes.get(context).copied().unwrap_or_default(),
				&history,
			)
		})
		.await
		{
			log::warn!("System monitor image update failed: {error}");
			break;
		}
	}
}

pub async fn appear(
	context: String,
	settings: SettingsValue,
	outbound: &mut OutboundEventManager,
) -> EventHandlerResult {
	MODES
		.lock()
		.unwrap()
		.insert(context.clone(), DisplayMode::from_settings(&settings));
	outbound
		.set_image(context.clone(), Some(loading_image()), None)
		.await?;

	let mut live = LIVE.lock().unwrap();
	if !live.subscribe(context.clone()) {
		return Ok(());
	}
	let cancel = Arc::new(AtomicBool::new(false));
	let sampler_cancel = cancel.clone();
	let (sender, receiver) = mpsc::channel(2);
	if let Err(error) = std::thread::Builder::new()
		.name("system-monitor-sampler".to_owned())
		.spawn(move || {
			let mut sampler = platform::Sampler::new();
			std::thread::sleep(Duration::from_millis(250));
			while !sampler_cancel.load(Ordering::Acquire) {
				if sender.blocking_send(sampler.snapshot()).is_err() {
					break;
				}
				for _ in 0..10 {
					if sampler_cancel.load(Ordering::Acquire) {
						return;
					}
					std::thread::sleep(SAMPLE_INTERVAL / 10);
				}
			}
		}) {
		live.unsubscribe(&context);
		return Err(error.into());
	}
	live.start(tokio::spawn(render(receiver)), cancel);
	Ok(())
}

pub async fn refresh(
	context: String,
	settings: SettingsValue,
	outbound: &mut OutboundEventManager,
) -> EventHandlerResult {
	let mode = DisplayMode::from_settings(&settings);
	MODES.lock().unwrap().insert(context.clone(), mode);
	let state = LAST_STATE.lock().unwrap().clone();
	if let Some(state) = state {
		outbound
			.set_image(
				context,
				Some(monitor_image(&state.snapshot, mode, &state.history)),
				None,
			)
			.await?;
	}
	Ok(())
}

pub fn disappear(context: &str) {
	MODES.lock().unwrap().remove(context);
	LIVE.lock().unwrap().unsubscribe(context);
}

#[cfg(windows)]
mod platform {
	use super::MonitorSnapshot;
	use std::{collections::HashMap, mem::MaybeUninit};
	use windows::{
		Win32::{
			Foundation::FILETIME,
			System::{
				Performance::{
					PDH_CSTATUS_NEW_DATA, PDH_CSTATUS_VALID_DATA, PDH_FMT_COUNTERVALUE_ITEM_W,
					PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY, PDH_MORE_DATA, PdhAddEnglishCounterW,
					PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW,
					PdhOpenQueryW,
				},
				SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX},
				Threading::GetSystemTimes,
			},
		},
		core::{PCWSTR, w},
	};

	#[derive(Clone, Copy, Default)]
	struct CpuTimes {
		idle: u64,
		kernel: u64,
		user: u64,
	}

	impl CpuTimes {
		fn read() -> Option<Self> {
			let mut idle = FILETIME::default();
			let mut kernel = FILETIME::default();
			let mut user = FILETIME::default();
			// SAFETY: All three FILETIME output pointers are valid for this call.
			unsafe {
				GetSystemTimes(Some(&mut idle), Some(&mut kernel), Some(&mut user)).ok()?;
			}
			Some(Self {
				idle: filetime(idle),
				kernel: filetime(kernel),
				user: filetime(user),
			})
		}
	}

	fn filetime(value: FILETIME) -> u64 {
		(u64::from(value.dwHighDateTime) << 32) | u64::from(value.dwLowDateTime)
	}

	struct GpuQuery {
		query: PDH_HQUERY,
		counter: PDH_HCOUNTER,
	}

	impl GpuQuery {
		fn new() -> Option<Self> {
			let mut query = PDH_HQUERY::default();
			// SAFETY: The output handle is valid and a null data source selects realtime data.
			if unsafe { PdhOpenQueryW(PCWSTR::null(), 0, &mut query) } != 0 {
				return None;
			}
			let mut counter = PDH_HCOUNTER::default();
			// SAFETY: The query is open and the wildcard English counter path is static.
			if unsafe {
				PdhAddEnglishCounterW(
					query,
					w!(r"\GPU Engine(*)\Utilization Percentage"),
					0,
					&mut counter,
				)
			} != 0
			{
				// SAFETY: query was successfully opened above.
				unsafe {
					PdhCloseQuery(query);
				}
				return None;
			}
			// Prime rate counters; the next collection produces a formatted value.
			// SAFETY: query and counter remain valid for the lifetime of this object.
			unsafe {
				PdhCollectQueryData(query);
			}
			Some(Self { query, counter })
		}

		fn sample(&mut self) -> Option<u8> {
			// SAFETY: The query remains valid until Drop.
			if unsafe { PdhCollectQueryData(self.query) } != 0 {
				return None;
			}
			let mut bytes = 0;
			let mut item_count = 0;
			// SAFETY: A null item buffer is the documented size-query call.
			let status = unsafe {
				PdhGetFormattedCounterArrayW(
					self.counter,
					PDH_FMT_DOUBLE,
					&mut bytes,
					&mut item_count,
					None,
				)
			};
			if status != PDH_MORE_DATA || bytes == 0 || item_count == 0 {
				return None;
			}

			let item_size = size_of::<PDH_FMT_COUNTERVALUE_ITEM_W>();
			let slots = (bytes as usize).div_ceil(item_size);
			let mut buffer = vec![MaybeUninit::<PDH_FMT_COUNTERVALUE_ITEM_W>::uninit(); slots];
			// SAFETY: The aligned buffer has at least the byte size requested by PDH.
			if unsafe {
				PdhGetFormattedCounterArrayW(
					self.counter,
					PDH_FMT_DOUBLE,
					&mut bytes,
					&mut item_count,
					Some(buffer.as_mut_ptr().cast()),
				)
			} != 0
			{
				return None;
			}

			// SAFETY: PDH initialized item_count entries in the aligned buffer.
			let items = unsafe {
				std::slice::from_raw_parts(
					buffer.as_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>(),
					item_count as usize,
				)
			};
			let mut engines: HashMap<String, f64> = HashMap::new();
			for item in items {
				if !matches!(
					item.FmtValue.CStatus,
					PDH_CSTATUS_VALID_DATA | PDH_CSTATUS_NEW_DATA
				) {
					continue;
				}
				// SAFETY: PDH returns a null-terminated instance name in this buffer.
				let name = unsafe { item.szName.to_string() }.unwrap_or_default();
				let engine = name
					.find("_luid_")
					.map(|position| &name[position..])
					.unwrap_or(&name);
				// SAFETY: PDH_FMT_DOUBLE selects the doubleValue union member.
				let value = unsafe { item.FmtValue.Anonymous.doubleValue };
				*engines.entry(engine.to_owned()).or_default() += value.max(0.0);
			}
			engines
				.into_values()
				.reduce(f64::max)
				.map(|value| value.clamp(0.0, 100.0).round() as u8)
		}
	}

	impl Drop for GpuQuery {
		fn drop(&mut self) {
			// SAFETY: This handle was opened once and is closed exactly once here.
			unsafe {
				PdhCloseQuery(self.query);
			}
		}
	}

	pub struct Sampler {
		previous_cpu: CpuTimes,
		gpu: Option<GpuQuery>,
	}

	impl Sampler {
		pub fn new() -> Self {
			Self {
				previous_cpu: CpuTimes::read().unwrap_or_default(),
				gpu: GpuQuery::new(),
			}
		}

		pub fn snapshot(&mut self) -> MonitorSnapshot {
			let current = CpuTimes::read().unwrap_or(self.previous_cpu);
			let idle = current.idle.saturating_sub(self.previous_cpu.idle);
			let kernel = current.kernel.saturating_sub(self.previous_cpu.kernel);
			let user = current.user.saturating_sub(self.previous_cpu.user);
			let total = kernel.saturating_add(user);
			let cpu = total
				.saturating_sub(idle)
				.saturating_mul(100)
				.checked_div(total)
				.unwrap_or_default()
				.min(100) as u8;
			self.previous_cpu = current;

			let mut memory_status = MEMORYSTATUSEX {
				dwLength: size_of::<MEMORYSTATUSEX>() as u32,
				..Default::default()
			};
			// SAFETY: memory_status has the required size field and is valid for output.
			let memory_available = unsafe { GlobalMemoryStatusEx(&mut memory_status) }.is_ok();
			let (
				memory,
				memory_total_mib,
				memory_available_mib,
				pagefile_total_mib,
				pagefile_available_mib,
			) = if memory_available {
				(
					memory_status.dwMemoryLoad.min(100) as u8,
					memory_status.ullTotalPhys / (1024 * 1024),
					memory_status.ullAvailPhys / (1024 * 1024),
					memory_status.ullTotalPageFile / (1024 * 1024),
					memory_status.ullAvailPageFile / (1024 * 1024),
				)
			} else {
				(0, 0, 0, 0, 0)
			};
			let gpu = self.gpu.as_mut().and_then(GpuQuery::sample);

			MonitorSnapshot {
				cpu,
				gpu,
				memory,
				memory_total_mib,
				memory_available_mib,
				pagefile_total_mib,
				pagefile_available_mib,
			}
		}
	}
}

#[cfg(not(windows))]
mod platform {
	use super::MonitorSnapshot;

	pub struct Sampler;

	impl Sampler {
		pub fn new() -> Self {
			Self
		}

		pub fn snapshot(&mut self) -> MonitorSnapshot {
			MonitorSnapshot {
				cpu: 0,
				gpu: None,
				memory: 0,
				memory_total_mib: 0,
				memory_available_mib: 0,
				pagefile_total_mib: 0,
				pagefile_available_mib: 0,
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::{DisplayMode, HistoryPoint, MonitorSnapshot, monitor_image};
	use std::collections::VecDeque;

	fn snapshot() -> MonitorSnapshot {
		MonitorSnapshot {
			cpu: 23,
			gpu: Some(67),
			memory: 81,
			memory_total_mib: 32 * 1024,
			memory_available_mib: 6 * 1024,
			pagefile_total_mib: 40 * 1024,
			pagefile_available_mib: 18 * 1024,
		}
	}

	fn history(snapshot: &MonitorSnapshot) -> VecDeque<HistoryPoint> {
		VecDeque::from([HistoryPoint::from(snapshot)])
	}

	#[test]
	fn image_is_native_pixel_art_with_all_three_metrics() {
		let snapshot = snapshot();
		let image = monitor_image(&snapshot, DisplayMode::Normal, &history(&snapshot));

		assert!(image.starts_with("data:image/svg+xml,"));
		assert!(image.contains("width%3D%22176%22"));
		assert!(image.contains("height%3D%22112%22"));
		assert!(image.contains("shape-rendering%3D%22crispEdges%22"));
		assert!(image.contains("%2320e3ff"));
		assert!(image.contains("%23ff4fd8"));
		assert!(image.contains("%23facc15"));
		assert!(!image.contains("%3Ctext"));
	}

	#[test]
	fn unavailable_gpu_is_rendered_without_panicking() {
		let snapshot = MonitorSnapshot {
			cpu: 1,
			gpu: None,
			memory: 2,
			memory_total_mib: 16 * 1024,
			memory_available_mib: 12 * 1024,
			pagefile_total_mib: 20 * 1024,
			pagefile_available_mib: 10 * 1024,
		};
		let image = monitor_image(&snapshot, DisplayMode::Normal, &history(&snapshot));
		assert!(image.len() < 50_000);
	}

	#[test]
	fn compact_mode_stays_compact_and_full_mode_contains_plotters_lines() {
		let snapshot = snapshot();
		let history = history(&snapshot);
		let compact = monitor_image(&snapshot, DisplayMode::Compact, &history);
		let full = monitor_image(&snapshot, DisplayMode::Full, &history);

		assert!(compact.len() < 30_000);
		assert!(full.contains("%3Cpolyline") || full.contains("%3Cpath"));
		assert!(full.contains("%23ff4fd8"));
	}

	#[test]
	fn display_mode_settings_accept_current_and_legacy_names() {
		assert_eq!(
			DisplayMode::from_settings(&serde_json::json!({ "mode": "compact" })),
			DisplayMode::Compact
		);
		assert_eq!(
			DisplayMode::from_settings(&serde_json::json!({ "mode": "normal" })),
			DisplayMode::Normal
		);
		assert_eq!(
			DisplayMode::from_settings(&serde_json::json!({ "mode": "full" })),
			DisplayMode::Full
		);
		assert_eq!(
			DisplayMode::from_settings(&serde_json::json!({ "mode": "mini" })),
			DisplayMode::Compact
		);
		assert_eq!(
			DisplayMode::from_settings(&serde_json::json!({ "mode": "medium" })),
			DisplayMode::Normal
		);
	}

	#[cfg(windows)]
	#[test]
	fn windows_system_snapshot_is_readable() {
		let mut sampler = super::platform::Sampler::new();
		std::thread::sleep(std::time::Duration::from_millis(250));
		let snapshot = sampler.snapshot();

		assert!(snapshot.cpu <= 100);
		assert!((1..=100).contains(&snapshot.memory));
		assert!(snapshot.gpu.is_none_or(|value| value <= 100));
	}
}
