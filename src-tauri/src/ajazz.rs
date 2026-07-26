use crate::events::outbound::{encoder, keypad};

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use ajazz_sdk::{asynchronous::AsyncAjazz, AjazzError, Event, Kind};
use base64::Engine as _;
use once_cell::sync::Lazy;
use tokio::sync::RwLock;
use tokio::time::Instant;

static AJAZZ_DEVICES: Lazy<RwLock<HashMap<String, AsyncAjazz>>> = Lazy::new(|| RwLock::new(HashMap::new()));
static MANAGED_DEVICES: Lazy<RwLock<HashSet<String>>> = Lazy::new(|| RwLock::new(HashSet::new()));
const INPUT_READ_TIMEOUT: Duration = Duration::from_millis(50);
const KEEP_ALIVE_INTERVAL: Duration = Duration::from_secs(10);

pub async fn update_image(context: &crate::shared::Context, image: Option<&str>) -> Result<(), anyhow::Error> {
	if let Some(device) = AJAZZ_DEVICES.read().await.get(&context.device) {
		if let Some(image) = image {
			let data = image.split_once(',').unwrap().1;
			let bytes = base64::engine::general_purpose::STANDARD.decode(data)?;
			if context.controller == "Encoder" {
				device.set_touch_zone_image(context.position, image::load_from_memory(&bytes)?).await?;
			} else {
				device.set_button_image(context.position, image::load_from_memory(&bytes)?).await?;
			}
		} else if context.controller == "Encoder" {
			device.clear_touch_zone_image(context.position).await?;
		} else {
			device.clear_button_image(context.position).await?;
		}
		device.flush().await?;
	}
	Ok(())
}

pub async fn clear_screen(id: &str) -> Result<(), anyhow::Error> {
	if let Some(device) = AJAZZ_DEVICES.read().await.get(id) {
		device.clear_all_button_images().await?;
		device.flush().await?;
	}
	Ok(())
}

pub async fn set_brightness(brightness: u8) {
	for (_id, device) in AJAZZ_DEVICES.read().await.iter() {
		let _ = device.set_brightness(brightness.clamp(0, 100)).await;
		let _ = device.flush().await;
	}
}

pub async fn reset_devices() {
	for (_id, device) in AJAZZ_DEVICES.read().await.iter() {
		let _ = device.reset().await;
		let _ = device.flush().await;
	}
}

async fn init(device: AsyncAjazz, device_id: String) {
	if AJAZZ_DEVICES.read().await.contains_key(&device_id) {
		MANAGED_DEVICES.write().await.remove(&device_id);
		return;
	}

	let kind = device.kind();
	let device_type = match kind {
		Kind::Akp153 | Kind::Akp153E | Kind::Akp153R => 2,
		Kind::Akp815 => 2,
		Kind::Akp03 | Kind::Akp03E | Kind::Akp03R => 2,
		Kind::Akp03RRev2 => 2,
		Kind::Akp05E552A => 7,
	};
	if let Err(error) = device.clear_all_button_images().await {
		log::warn!("Failed to initialise {device_id}: {error}");
		MANAGED_DEVICES.write().await.remove(&device_id);
		return;
	}
	if let Ok(settings) = crate::store::get_settings() {
		if let Err(error) = device.set_brightness(settings.value.brightness).await {
			log::warn!("Failed to set brightness for {device_id}: {error}");
		}
	}
	if let Err(error) = device.flush().await {
		log::warn!("Failed to flush initial state for {device_id}: {error}");
		MANAGED_DEVICES.write().await.remove(&device_id);
		return;
	}
	if let Err(error) = crate::events::inbound::devices::register_device(
		"",
		crate::events::inbound::PayloadEvent {
			payload: crate::shared::DeviceInfo {
				id: device_id.clone(),
				plugin: String::new(),
				name: device.product_name.to_owned(),
				rows: kind.row_count(),
				columns: kind.column_count(),
				encoders: kind.encoder_count(),
				r#type: device_type,
			},
		},
	)
	.await
	{
		log::warn!("Failed to register {device_id}: {error}");
		MANAGED_DEVICES.write().await.remove(&device_id);
		return;
	}

	let reader = device.get_reader();
	AJAZZ_DEVICES.write().await.insert(device_id.clone(), device.clone());
	log::info!("Registered {} as {}", device.product_name, device_id);
	let mut next_keep_alive = Instant::now();
	loop {
		if Instant::now() >= next_keep_alive {
			if let Err(error) = device.keep_alive().await {
				log::warn!("Keep-alive failed for {device_id}: {error}");
				break;
			}
			next_keep_alive = Instant::now() + KEEP_ALIVE_INTERVAL;
		}

		let updates = match reader.read_timeout(INPUT_READ_TIMEOUT).await {
			Ok(updates) => updates,
			Err(AjazzError::BadData) => {
				log::debug!("Ignored unsupported input packet from {device_id}");
				continue;
			}
			Err(error) => {
				log::warn!("Device reader stopped for {device_id}: {error}");
				break;
			}
		};
		for update in updates {
			match match update {
				Event::ButtonDown(key) => keypad::key_down(&device_id, key).await,
				Event::ButtonUp(key) => keypad::key_up(&device_id, key).await,
				Event::EncoderTwist(dial, ticks) => encoder::dial_rotate(&device_id, dial, ticks.into()).await,
				Event::EncoderDown(dial) => encoder::dial_press(&device_id, "dialDown", dial).await,
				Event::EncoderUp(dial) => encoder::dial_press(&device_id, "dialUp", dial).await,
			} {
				Ok(_) => (),
				Err(error) => log::warn!("Failed to process device event {update:?}: {error}"),
			}
		}
	}

	AJAZZ_DEVICES.write().await.remove(&device_id);
	MANAGED_DEVICES.write().await.remove(&device_id);
	if let Err(error) = crate::events::inbound::devices::deregister_device("", crate::events::inbound::PayloadEvent { payload: device_id.clone() }).await {
		log::warn!("Failed to deregister {device_id}: {error}");
	}
}

/// Attempt to initialise all connected devices.
pub async fn initialise_devices() {
	if let Ok(settings) = crate::store::get_settings() {
		if settings.value.disabledevices {
			crate::plugins::DEVICE_NAMESPACES
				.write()
				.await
				.insert("sd".to_owned(), "opendeck_alternative_ajazz_implementation".to_owned());
			return;
		} else {
			crate::plugins::DEVICE_NAMESPACES.write().await.remove("sd");
		}
	}

	// Iterate through detected Ajazz devices and attempt to register them.
	match ajazz_sdk::new_hidapi() {
		Ok(hid) => {
			for (kind, serial) in ajazz_sdk::list_devices(&hid) {
				let device_id = format!("sd-{serial}");
				if !MANAGED_DEVICES.write().await.insert(device_id.clone()) {
					continue;
				}
				match AsyncAjazz::connect(&hid, kind, &serial) {
					Ok(device) => {
						tokio::spawn(init(device, device_id));
					}
					Err(error) => {
						MANAGED_DEVICES.write().await.remove(&device_id);
						log::warn!("Failed to connect to Ajazz device: {error}");
					}
				}
			}
		}
		Err(error) => log::warn!("Failed to initialise hidapi: {error}"),
	}
}
