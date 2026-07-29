mod audio;
mod device_brightness;
mod input_simulation;
mod live;
mod pixel;
mod run_command;
mod switch_profile;
mod system_monitor;

use openaction::*;

trait ActionEvent {
	fn context(&self) -> &String;
	fn settings(&self) -> &SettingsValue;
}
impl ActionEvent for KeyEvent {
	fn context(&self) -> &String {
		&self.context
	}
	fn settings(&self) -> &SettingsValue {
		&self.payload.settings
	}
}
impl ActionEvent for DialPressEvent {
	fn context(&self) -> &String {
		&self.context
	}
	fn settings(&self) -> &SettingsValue {
		&self.payload.settings
	}
}
impl ActionEvent for DialRotateEvent {
	fn context(&self) -> &String {
		&self.context
	}
	fn settings(&self) -> &SettingsValue {
		&self.payload.settings
	}
}

struct GlobalEventHandler {}
impl openaction::GlobalEventHandler for GlobalEventHandler {}

struct ActionEventHandler {}
impl openaction::ActionEventHandler for ActionEventHandler {
	async fn key_down(
		&self,
		event: KeyEvent,
		_outbound: &mut openaction::OutboundEventManager,
	) -> EventHandlerResult {
		match &event.action[..] {
			"com.amansprojects.starterpack.runcommand" => run_command::down_up("down", event),
			"com.amansprojects.starterpack.inputsimulation" => {
				input_simulation::down_up("down", event).await
			}
			_ => Ok(()),
		}
	}

	async fn key_up(
		&self,
		event: KeyEvent,
		outbound: &mut openaction::OutboundEventManager,
	) -> EventHandlerResult {
		match &event.action[..] {
			"com.amansprojects.starterpack.runcommand" => run_command::down_up("up", event),
			"com.amansprojects.starterpack.inputsimulation" => {
				input_simulation::down_up("up", event).await
			}
			"com.amansprojects.starterpack.switchprofile" => {
				switch_profile::key_up(event, outbound).await
			}
			"com.amansprojects.starterpack.devicebrightness" => {
				device_brightness::up(event, outbound).await
			}
			_ => Ok(()),
		}
	}

	async fn dial_down(
		&self,
		event: DialPressEvent,
		_outbound: &mut openaction::OutboundEventManager,
	) -> EventHandlerResult {
		match &event.action[..] {
			"com.amansprojects.starterpack.runcommand" => run_command::down_up("down", event),
			"com.amansprojects.starterpack.inputsimulation" => {
				input_simulation::down_up("down", event).await
			}
			_ => Ok(()),
		}
	}

	async fn dial_up(
		&self,
		event: DialPressEvent,
		outbound: &mut openaction::OutboundEventManager,
	) -> EventHandlerResult {
		match &event.action[..] {
			"com.amansprojects.starterpack.runcommand" => run_command::down_up("up", event),
			"com.amansprojects.starterpack.inputsimulation" => {
				input_simulation::down_up("up", event).await
			}
			"com.amansprojects.starterpack.devicebrightness" => {
				device_brightness::up(event, outbound).await
			}
			"com.amansprojects.starterpack.systemvolume" => {
				audio::press_device(event, outbound).await
			}
			_ => Ok(()),
		}
	}

	async fn dial_rotate(
		&self,
		event: DialRotateEvent,
		outbound: &mut openaction::OutboundEventManager,
	) -> EventHandlerResult {
		match &event.action[..] {
			"com.amansprojects.starterpack.runcommand" => run_command::rotate(event),
			"com.amansprojects.starterpack.inputsimulation" => {
				input_simulation::rotate(event).await
			}
			"com.amansprojects.starterpack.devicebrightness" => {
				device_brightness::rotate(event, outbound).await
			}
			"com.amansprojects.starterpack.systemvolume" => {
				audio::rotate_volume(event, outbound).await
			}
			_ => Ok(()),
		}
	}

	async fn will_appear(
		&self,
		event: AppearEvent,
		outbound: &mut openaction::OutboundEventManager,
	) -> EventHandlerResult {
		match event.action.as_str() {
			audio::ACTION => audio::appear(event.context, outbound).await,
			system_monitor::ACTION => system_monitor::appear(event.context, outbound).await,
			_ => Ok(()),
		}
	}

	async fn will_disappear(
		&self,
		event: AppearEvent,
		_outbound: &mut openaction::OutboundEventManager,
	) -> EventHandlerResult {
		match event.action.as_str() {
			audio::ACTION => audio::disappear(&event.context),
			system_monitor::ACTION => system_monitor::disappear(&event.context),
			_ => {}
		}
		Ok(())
	}

	async fn did_receive_settings(
		&self,
		event: DidReceiveSettingsEvent,
		outbound: &mut openaction::OutboundEventManager,
	) -> EventHandlerResult {
		if event.action == audio::ACTION {
			audio::refresh(event.context, outbound).await
		} else {
			Ok(())
		}
	}
}

#[tokio::main]
async fn main() {
	simplelog::TermLogger::init(
		simplelog::LevelFilter::Debug,
		simplelog::Config::default(),
		simplelog::TerminalMode::Stdout,
		simplelog::ColorChoice::Never,
	)
	.unwrap();

	if let Err(error) = init_plugin(GlobalEventHandler {}, ActionEventHandler {}).await {
		log::error!("Failed to initialise plugin: {}", error);
	}
}
