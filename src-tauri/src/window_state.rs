use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, sync::OnceLock, time::Duration};
use tauri::{PhysicalPosition, PhysicalSize, Window};
use tokio::sync::mpsc;

const SETTINGS_FILE: &str = "settings.toml";
const MIN_WIDTH: u32 = 800;
const MIN_HEIGHT: u32 = 600;
const MIN_VISIBLE_EDGE: i64 = 64;
const SAVE_DEBOUNCE: Duration = Duration::from_millis(300);

static SAVE_SENDER: OnceLock<mpsc::UnboundedSender<WindowGeometry>> = OnceLock::new();

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
struct WindowGeometry {
	x: i32,
	y: i32,
	width: u32,
	height: u32,
}

#[derive(Debug, Deserialize, Serialize)]
struct SettingsFile {
	window: WindowGeometry,
}

pub fn initialize(window: &Window) {
	start_save_worker();
	match load() {
		Ok(Some(geometry)) => restore(window, geometry),
		Ok(None) => {}
		Err(error) => log::warn!("Failed to load {}: {error:#}", settings_path().display()),
	}
	save_now(window);
}

pub fn queue_save(window: &Window) {
	let Some(geometry) = capture(window) else {
		return;
	};
	if let Some(sender) = SAVE_SENDER.get() {
		let _ = sender.send(geometry);
	}
}

pub fn save_now(window: &Window) {
	if let Some(geometry) = capture(window)
		&& let Err(error) = save(geometry)
	{
		log::warn!("Failed to save {}: {error:#}", settings_path().display());
	}
}

fn start_save_worker() {
	let (sender, mut receiver) = mpsc::unbounded_channel();
	if SAVE_SENDER.set(sender).is_err() {
		return;
	}
	tauri::async_runtime::spawn(async move {
		while let Some(mut pending) = receiver.recv().await {
			while let Ok(Some(newer)) = tokio::time::timeout(SAVE_DEBOUNCE, receiver.recv()).await {
				pending = newer;
			}
			if let Err(error) = save(pending) {
				log::warn!("Failed to save {}: {error:#}", settings_path().display());
			}
		}
	});
}

fn capture(window: &Window) -> Option<WindowGeometry> {
	let position = window.outer_position().ok()?;
	let size = window.inner_size().ok()?;
	if size.width < MIN_WIDTH || size.height < MIN_HEIGHT {
		return None;
	}
	let geometry = WindowGeometry {
		x: position.x,
		y: position.y,
		width: size.width,
		height: size.height,
	};
	geometry_is_visible(window, geometry).then_some(geometry)
}

fn restore(window: &Window, geometry: WindowGeometry) {
	let size = PhysicalSize::new(geometry.width.max(MIN_WIDTH), geometry.height.max(MIN_HEIGHT));
	if let Err(error) = window.set_size(size) {
		log::warn!("Failed to restore main window size: {error}");
	}
	if geometry_is_visible(window, geometry)
		&& let Err(error) = window.set_position(PhysicalPosition::new(geometry.x, geometry.y))
	{
		log::warn!("Failed to restore main window position: {error}");
	}
}

fn geometry_is_visible(window: &Window, geometry: WindowGeometry) -> bool {
	window.available_monitors().unwrap_or_default().iter().any(|monitor| {
		let area = monitor.work_area();
		rectangles_overlap(
			(i64::from(geometry.x), i64::from(geometry.y), i64::from(geometry.width), i64::from(geometry.height)),
			(i64::from(area.position.x), i64::from(area.position.y), i64::from(area.size.width), i64::from(area.size.height)),
			MIN_VISIBLE_EDGE,
		)
	})
}

fn rectangles_overlap(a: (i64, i64, i64, i64), b: (i64, i64, i64, i64), minimum: i64) -> bool {
	let horizontal = (a.0 + a.2).min(b.0 + b.2) - a.0.max(b.0);
	let vertical = (a.1 + a.3).min(b.1 + b.3) - a.1.max(b.1);
	horizontal >= minimum && vertical >= minimum
}

fn settings_path() -> PathBuf {
	std::env::current_exe()
		.ok()
		.and_then(|path| path.parent().map(|parent| parent.join(SETTINGS_FILE)))
		.unwrap_or_else(|| PathBuf::from(SETTINGS_FILE))
}

fn load() -> anyhow::Result<Option<WindowGeometry>> {
	let path = settings_path();
	if !path.exists() {
		return Ok(None);
	}
	let settings: SettingsFile = toml::from_str(&fs::read_to_string(path)?)?;
	Ok(Some(settings.window))
}

fn save(geometry: WindowGeometry) -> anyhow::Result<()> {
	let settings = SettingsFile { window: geometry };
	fs::write(settings_path(), toml::to_string_pretty(&settings)?)?;
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::{SettingsFile, WindowGeometry, rectangles_overlap};

	#[test]
	fn settings_toml_round_trips_window_geometry() {
		let settings = SettingsFile {
			window: WindowGeometry {
				x: -120,
				y: 80,
				width: 1280,
				height: 720,
			},
		};
		let serialized = toml::to_string_pretty(&settings).unwrap();
		let restored: SettingsFile = toml::from_str(&serialized).unwrap();

		assert_eq!(restored.window, settings.window);
		assert!(serialized.contains("[window]"));
	}

	#[test]
	fn visible_overlap_requires_a_usable_edge() {
		let monitor = (0, 0, 1920, 1080);
		assert!(rectangles_overlap((100, 100, 800, 600), monitor, 64));
		assert!(rectangles_overlap((-736, 100, 800, 600), monitor, 64));
		assert!(!rectangles_overlap((-737, 100, 800, 600), monitor, 64));
		assert!(!rectangles_overlap((2000, 100, 800, 600), monitor, 64));
	}
}
