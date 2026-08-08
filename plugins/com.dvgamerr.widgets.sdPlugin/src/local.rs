use openaction::{EventHandlerResult, OUTBOUND_EVENT_MANAGER, OutboundEventManager, SettingsValue};

use crate::{
	model::{ActionKind, setting_string},
	platform, render,
};

pub async fn appear(
	context: String,
	kind: ActionKind,
	settings: &SettingsValue,
	outbound: &mut OutboundEventManager,
) -> EventHandlerResult {
	if kind == ActionKind::PowerShell {
		outbound
			.set_image(context, Some(render::powershell(settings, "idle")), None)
			.await?;
	}
	Ok(())
}

pub async fn settings_changed(
	context: String,
	kind: ActionKind,
	settings: &SettingsValue,
	outbound: &mut OutboundEventManager,
) -> EventHandlerResult {
	appear(context, kind, settings, outbound).await
}

pub async fn press(
	context: String,
	kind: ActionKind,
	settings: SettingsValue,
	outbound: &mut OutboundEventManager,
) -> EventHandlerResult {
	if kind != ActionKind::PowerShell {
		return Ok(());
	}

	let script = setting_string(&settings, "script", "");
	outbound
		.set_image(
			context.clone(),
			Some(render::powershell(&settings, "running")),
			None,
		)
		.await?;
	tokio::spawn(async move {
		let result = platform::run_script(&script).await;
		let image = render::powershell(&settings, if result.is_ok() { "ok" } else { "error" });
		if let Err(error) = &result {
			log::warn!("PowerShell action failed for {context}: {error:#}");
		}
		let mut manager = OUTBOUND_EVENT_MANAGER.lock().await;
		if let Some(outbound) = manager.as_mut() {
			let _ = outbound.set_image(context.clone(), Some(image), None).await;
			if result.is_ok() {
				let _ = outbound.show_ok(context).await;
			} else {
				let _ = outbound.show_alert(context).await;
			}
		}
	});
	Ok(())
}
