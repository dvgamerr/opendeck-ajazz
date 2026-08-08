use openaction::{EventHandlerResult, OUTBOUND_EVENT_MANAGER, OutboundEventManager, SettingsValue};
use std::{
	collections::HashMap,
	sync::{LazyLock, Mutex},
	time::{Duration, Instant},
};
use tokio::task::JoinHandle;

use crate::{
	fetch,
	model::{ActionKind, WidgetData, setting_string},
	render,
};

struct Entry {
	kind: ActionKind,
	settings: SettingsValue,
	generation: u64,
	next_due: Instant,
	busy: bool,
	data: Option<WidgetData>,
	last_image: Option<String>,
}

#[derive(Default)]
struct Runtime {
	entries: HashMap<String, Entry>,
	worker: Option<JoinHandle<()>>,
}

struct JobGroup {
	kind: ActionKind,
	settings: SettingsValue,
	targets: Vec<(String, u64)>,
}

static RUNTIME: LazyLock<Mutex<Runtime>> = LazyLock::new(Default::default);

fn ensure_worker(runtime: &mut Runtime) {
	if runtime
		.worker
		.as_ref()
		.is_some_and(|worker| !worker.is_finished())
	{
		return;
	}
	runtime.worker = Some(tokio::spawn(scheduler()));
}

pub async fn appear(
	context: String,
	kind: ActionKind,
	settings: SettingsValue,
	outbound: &mut OutboundEventManager,
) -> EventHandlerResult {
	let image = render::loading(kind);
	{
		let mut runtime = RUNTIME.lock().unwrap();
		runtime.entries.insert(
			context.clone(),
			Entry {
				kind,
				settings,
				generation: 0,
				next_due: Instant::now(),
				busy: false,
				data: None,
				last_image: Some(image.clone()),
			},
		);
		ensure_worker(&mut runtime);
	}
	outbound.set_image(context, Some(image), None).await?;
	Ok(())
}

pub fn disappear(context: &str) {
	let mut runtime = RUNTIME.lock().unwrap();
	runtime.entries.remove(context);
	if runtime.entries.is_empty()
		&& let Some(worker) = runtime.worker.take()
	{
		worker.abort();
	}
}

pub fn update(context: &str, settings: SettingsValue) {
	let mut runtime = RUNTIME.lock().unwrap();
	if let Some(entry) = runtime.entries.get_mut(context) {
		entry.settings = settings;
		entry.generation = entry.generation.wrapping_add(1);
		entry.next_due = Instant::now();
		entry.data = None;
	}
}

pub fn refresh(context: &str) {
	if let Some(entry) = RUNTIME.lock().unwrap().entries.get_mut(context) {
		entry.next_due = Instant::now();
	}
}

fn collect_jobs() -> Vec<JobGroup> {
	let now = Instant::now();
	let mut runtime = RUNTIME.lock().unwrap();
	let mut groups: HashMap<(ActionKind, String), JobGroup> = HashMap::new();
	for (context, entry) in &mut runtime.entries {
		if entry.busy || entry.next_due > now {
			continue;
		}
		entry.busy = true;
		entry.next_due = now + entry.kind.refresh_interval(&entry.settings);
		let key = (entry.kind, request_key(entry.kind, &entry.settings));
		groups
			.entry(key)
			.or_insert_with(|| JobGroup {
				kind: entry.kind,
				settings: entry.settings.clone(),
				targets: Vec::new(),
			})
			.targets
			.push((context.clone(), entry.generation));
	}
	groups.into_values().collect()
}

fn request_key(kind: ActionKind, settings: &SettingsValue) -> String {
	match kind {
		ActionKind::Gold => "gold".to_owned(),
		ActionKind::Currency => format!(
			"{}:{}",
			setting_string(settings, "from", "USD").to_uppercase(),
			setting_string(settings, "to", "THB").to_uppercase()
		),
		ActionKind::AirQuality => setting_string(
			settings,
			"url",
			"https://www.iqair.com/th-en/thailand/bangkok/nong-khaem",
		),
		ActionKind::Weather => format!(
			"{}:{}",
			setting_string(settings, "lat", "13.72"),
			setting_string(settings, "lon", "100.41")
		),
		ActionKind::WorkHours => "work-hours".to_owned(),
		_ => settings.to_string(),
	}
}

async fn scheduler() {
	let mut interval = tokio::time::interval(Duration::from_secs(1));
	interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
	loop {
		interval.tick().await;
		for group in collect_jobs() {
			tokio::spawn(async move {
				let result = fetch::widget(group.kind, &group.settings)
					.await
					.map_err(|error| format!("{error:#}"));
				for (context, generation) in group.targets {
					finish(context, generation, result.clone()).await;
				}
			});
		}
	}
}

async fn finish(context: String, generation: u64, result: Result<WidgetData, String>) {
	let image = {
		let mut runtime = RUNTIME.lock().unwrap();
		let Some(entry) = runtime.entries.get_mut(&context) else {
			return;
		};
		entry.busy = false;
		if entry.generation != generation {
			entry.next_due = Instant::now();
			return;
		}
		let image = match result {
			Ok(data) => {
				let image = render::widget(entry.kind, &data, &entry.settings);
				entry.data = Some(data);
				image
			}
			Err(error) => {
				log::warn!("{:?} refresh failed for {context}: {error}", entry.kind);
				render::error(&error)
			}
		};
		if entry.last_image.as_ref() == Some(&image) {
			return;
		}
		entry.last_image = Some(image.clone());
		image
	};

	let mut manager = OUTBOUND_EVENT_MANAGER.lock().await;
	if let Some(outbound) = manager.as_mut()
		&& let Err(error) = outbound.set_image(context, Some(image), None).await
	{
		log::warn!("widget image update failed: {error}");
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;

	#[test]
	fn identical_requests_share_one_scheduler_group() {
		let mut runtime = RUNTIME.lock().unwrap();
		runtime.entries.clear();
		for (context, interval) in [("one", 5_000), ("two", 60_000)] {
			runtime.entries.insert(
				context.to_owned(),
				Entry {
					kind: ActionKind::Currency,
					settings: json!({ "from": "USD", "to": "THB", "interval": interval }),
					generation: 0,
					next_due: Instant::now(),
					busy: false,
					data: None,
					last_image: None,
				},
			);
		}
		drop(runtime);
		let jobs = collect_jobs();
		assert_eq!(jobs.len(), 1);
		assert_eq!(jobs[0].targets.len(), 2);
		RUNTIME.lock().unwrap().entries.clear();
	}
}
