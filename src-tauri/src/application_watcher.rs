use crate::store::{NotProfile, Store};

use std::collections::HashMap;
#[cfg(target_os = "windows")]
use std::ffi::OsStr;

use active_win_pos_rs::get_active_window;
use once_cell::sync::Lazy;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};
use tauri::{Emitter, Manager};
use tokio::sync::RwLock;

pub type ApplicationProfiles = HashMap<String, HashMap<String, String>>;
impl NotProfile for ApplicationProfiles {}

pub static APPLICATIONS: RwLock<Vec<String>> = RwLock::const_new(Vec::new());
pub static APPLICATION_PROFILES: Lazy<RwLock<Store<ApplicationProfiles>>> = Lazy::new(|| RwLock::new(Store::new("applications", &crate::shared::config_dir(), HashMap::new()).unwrap()));

pub static APPLICATION_PROCESSES: Lazy<RwLock<HashMap<String, Vec<u32>>>> = Lazy::new(|| RwLock::new(HashMap::new()));
pub static APPLICATION_PLUGINS: Lazy<RwLock<HashMap<String, Vec<String>>>> = Lazy::new(|| RwLock::new(HashMap::new()));
pub static PREVIOUS_PROFILES: Lazy<RwLock<HashMap<String, String>>> = Lazy::new(|| RwLock::new(HashMap::new()));

#[cfg(target_os = "windows")]
fn is_microsoft_teams_process(name: &OsStr) -> bool {
	matches!(name.to_string_lossy().to_ascii_lowercase().as_str(), "ms-teams.exe" | "msteams.exe" | "teams.exe")
}

#[derive(Clone, serde::Serialize)]
pub struct SwitchProfileEvent {
	device: String,
	profile: String,
}

pub fn init_application_watcher() {
	tokio::spawn(async move {
		let mut previous = String::new();
		let app_handle = crate::APP_HANDLE.get().unwrap();
		loop {
			let app_name = if let Ok(win) = get_active_window() {
				let mut applications = APPLICATIONS.write().await;
				if !applications.contains(&win.app_name) && !win.app_name.to_lowercase().starts_with("opendeck") && !win.app_name.trim().is_empty() {
					applications.push(win.app_name.clone());
					let _ = app_handle.get_webview_window("main").unwrap().emit("applications", applications.clone());
				}
				win.app_name
			} else {
				String::new()
			};

			if app_name != previous {
				let application_profiles = APPLICATION_PROFILES.read().await.value.clone();
				let application = application_profiles.get(&app_name);
				let default = application_profiles.get("opendeck_default");
				let devices: Vec<String> = crate::shared::DEVICES.iter().map(|value| value.key().clone()).collect();
				for device in devices {
					let Ok(current_profile) = crate::store::profiles::DEVICE_STORES.write().await.get_selected_profile(&device) else {
						continue;
					};

					let application_profile = application.and_then(|profiles| profiles.get(&device)).cloned();
					let previous_profile = PREVIOUS_PROFILES.read().await.get(&device).cloned();
					let (profile, remember_current, restore_previous) = if let Some(profile) = application_profile {
						(profile, previous_profile.is_none(), false)
					} else if let Some(profile) = previous_profile {
						(profile, false, true)
					} else if let Some(profile) = default.and_then(|profiles| profiles.get(&device)).cloned() {
						(profile, false, false)
					} else {
						continue;
					};

					if current_profile != profile {
						match crate::events::frontend::profiles::set_selected_profile(device.clone(), profile.clone()).await {
							Ok(()) => {
								let _ = app_handle.get_webview_window("main").unwrap().emit(
									"switch_profile",
									SwitchProfileEvent {
										device: device.clone(),
										profile: profile.clone(),
									},
								);
							}
							Err(error) => {
								log::error!("Failed to switch device {device} to profile {profile} for application {app_name}: {error}");
								continue;
							}
						}
					}

					if remember_current {
						PREVIOUS_PROFILES.write().await.insert(device.clone(), current_profile);
					} else if restore_previous {
						PREVIOUS_PROFILES.write().await.remove(&device);
					}
				}
				previous = app_name;
			}

			tokio::time::sleep(std::time::Duration::from_millis(250)).await;
		}
	});

	tokio::spawn(async move {
		let mut system = System::new_with_specifics(RefreshKind::nothing().with_processes(ProcessRefreshKind::nothing().without_tasks()));
		#[cfg(target_os = "windows")]
		let mut microsoft_teams_was_running = None;

		loop {
			system.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing().without_tasks());
			#[cfg(target_os = "windows")]
			let microsoft_teams_running = system.processes().values().any(|process| is_microsoft_teams_process(process.name()));

			for (application, processes) in APPLICATION_PROCESSES.write().await.iter_mut() {
				let mut alive_processes = Vec::with_capacity(processes.len());
				for pid in processes.iter() {
					if system.process(Pid::from_u32(*pid)).is_some() {
						alive_processes.push(*pid);
					} else {
						for plugin in APPLICATION_PLUGINS.read().await.get(application).into_iter().flatten() {
							let _ = crate::events::outbound::applications::application_did_terminate(plugin, application.clone()).await;
						}
					}
				}
				*processes = alive_processes;
			}

			let application_plugins = APPLICATION_PLUGINS.read().await;
			for (application, plugins) in application_plugins.iter() {
				for process in system.processes_by_exact_name(application.as_ref()) {
					let pid = process.pid().as_u32();
					let mut application_processes = APPLICATION_PROCESSES.write().await;
					let pids = application_processes.entry(application.clone()).or_default();
					if !pids.contains(&pid) {
						pids.push(pid);
						for plugin in plugins {
							let _ = crate::events::outbound::applications::application_did_launch(plugin, application.clone()).await;
						}
					}
				}
			}
			drop(application_plugins);

			#[cfg(target_os = "windows")]
			{
				// Keep checking the stopped state so a concurrently initialising or
				// manually reloaded Teams plugin is shut down on the next pass.
				if (!microsoft_teams_running || microsoft_teams_was_running == Some(false))
					&& let Err(error) = crate::plugins::set_microsoft_teams_running(microsoft_teams_running).await
				{
					log::warn!("Failed to update Microsoft Teams plugin lifecycle: {error}");
				}
				microsoft_teams_was_running = Some(microsoft_teams_running);
			}

			tokio::time::sleep(std::time::Duration::from_millis(250)).await;
		}
	});
}

pub async fn start_monitoring(plugin: &str, applications: &Vec<String>) {
	let mut application_plugins = APPLICATION_PLUGINS.write().await;

	for application in applications {
		application_plugins.entry(application.to_owned()).or_default().push(plugin.to_owned());

		let application_processes = APPLICATION_PROCESSES.read().await;
		if let Some(pids) = application_processes.get(application) {
			for _ in pids {
				let _ = crate::events::outbound::applications::application_did_launch(plugin, application.to_owned()).await;
			}
		}
	}
}

pub async fn stop_monitoring(plugin: &str) {
	let mut application_plugins = APPLICATION_PLUGINS.write().await;
	for plugins in application_plugins.values_mut() {
		plugins.retain(|p| p != plugin);
	}
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
	use super::is_microsoft_teams_process;
	use std::ffi::OsStr;

	#[test]
	fn detects_new_classic_and_store_teams_processes_case_insensitively() {
		for name in ["ms-teams.exe", "MS-TEAMS.EXE", "Teams.exe", "msteams.exe"] {
			assert!(is_microsoft_teams_process(OsStr::new(name)));
		}
		assert!(!is_microsoft_teams_process(OsStr::new("TeamsUpdater.exe")));
	}
}
