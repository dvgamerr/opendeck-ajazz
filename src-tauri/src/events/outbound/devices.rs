use super::{send_to_all_plugins, send_to_plugin};

use crate::plugins::{DEVICE_NAMESPACES, info_param::DeviceInfo};

use serde::Serialize;

#[derive(Serialize)]
#[allow(non_snake_case)]
struct DeviceDidConnectEvent {
	event: &'static str,
	device: String,
	deviceInfo: DeviceInfo,
}

pub async fn device_did_connect(id: &str, info: DeviceInfo) -> Result<(), anyhow::Error> {
	send_to_all_plugins(&DeviceDidConnectEvent {
		event: "deviceDidConnect",
		device: id.to_owned(),
		deviceInfo: info,
	})
	.await
}

#[derive(Serialize)]
struct DeviceDidDisconnectEvent {
	event: &'static str,
	device: String,
}

pub async fn device_did_disconnect(id: &str) -> Result<(), anyhow::Error> {
	send_to_all_plugins(&DeviceDidDisconnectEvent {
		event: "deviceDidDisconnect",
		device: id.to_owned(),
	})
	.await
}

#[derive(Serialize)]
struct SetImageEvent {
	event: &'static str,
	device: String,
	controller: Option<String>,
	position: Option<u8>,
	image: Option<String>,
}

pub async fn update_image(context: crate::shared::Context, image: Option<String>) -> Result<(), anyhow::Error> {
	update_images(vec![(context, image)]).await
}

pub async fn update_images(updates: Vec<(crate::shared::Context, Option<String>)>) -> Result<(), anyhow::Error> {
	let Some(device) = updates.first().map(|(context, _)| context.device.clone()) else {
		return Ok(());
	};
	if updates.iter().any(|(context, _)| context.device != device) {
		return Err(anyhow::anyhow!("An image batch cannot target multiple devices"));
	}

	if let Some(plugin) = DEVICE_NAMESPACES.read().await.get(&device[..2]) {
		for (context, image) in updates {
			send_to_plugin(
				plugin,
				&SetImageEvent {
					event: "setImage",
					device: context.device,
					controller: Some(context.controller),
					position: Some(context.position),
					image,
				},
			)
			.await?;
		}
	} else if device.starts_with("sd-") {
		crate::ajazz::update_images(updates).await?;
	}

	Ok(())
}

pub async fn clear_screen(device: String) -> Result<(), anyhow::Error> {
	if let Some(plugin) = DEVICE_NAMESPACES.read().await.get(&device[..2]) {
		send_to_plugin(
			plugin,
			&SetImageEvent {
				event: "setImage",
				device,
				controller: None,
				position: None,
				image: None,
			},
		)
		.await?;
	} else if device.starts_with("sd-") {
		crate::ajazz::clear_screen(&device).await?;
	}

	Ok(())
}

pub async fn clear_keypad(device: String, key_count: u8) -> Result<(), anyhow::Error> {
	let plugin = DEVICE_NAMESPACES.read().await.get(&device[..2]).cloned();
	if let Some(plugin) = plugin {
		for position in 0..key_count {
			send_to_plugin(
				&plugin,
				&SetImageEvent {
					event: "setImage",
					device: device.clone(),
					controller: Some("Keypad".to_owned()),
					position: Some(position),
					image: None,
				},
			)
			.await?;
		}
	} else if device.starts_with("sd-") {
		crate::ajazz::clear_keypad(&device).await?;
	}

	Ok(())
}

#[derive(Serialize)]
struct SetBrightnessEvent {
	event: &'static str,
	device: String,
	brightness: u8,
}

pub async fn set_brightness(brightness: u8) -> Result<(), anyhow::Error> {
	let namespaces = DEVICE_NAMESPACES.read().await;
	for device in crate::shared::DEVICES.iter() {
		if let Some(plugin) = namespaces.get(&device.id[..2]) {
			send_to_plugin(
				plugin,
				&SetBrightnessEvent {
					event: "setBrightness",
					device: device.id.clone(),
					brightness,
				},
			)
			.await?;
		}
	}
	crate::ajazz::set_brightness(brightness).await;

	Ok(())
}
