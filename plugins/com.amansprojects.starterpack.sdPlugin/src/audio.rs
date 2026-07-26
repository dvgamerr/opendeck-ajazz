use openaction::*;

const SYSTEM_VOLUME_ACTION: &str = "com.amansprojects.starterpack.systemvolume";
const SWITCH_SOUND_DEVICE_ACTION: &str = "com.amansprojects.starterpack.switchsounddevice";

#[derive(Clone, Debug)]
struct AudioSnapshot {
	device_name: String,
	volume: u8,
	muted: bool,
}

fn setting_u8(settings: &SettingsValue, key: &str, fallback: u8) -> u8 {
	settings
		.as_object()
		.and_then(|settings| settings.get(key))
		.and_then(|value| value.as_u64())
		.map(|value| value.min(u8::MAX as u64) as u8)
		.unwrap_or(fallback)
}

fn setting_direction(settings: &SettingsValue) -> i32 {
	if settings
		.as_object()
		.and_then(|settings| settings.get("direction"))
		.and_then(|value| value.as_str())
		== Some("previous")
	{
		-1
	} else {
		1
	}
}

fn escape_xml(value: &str) -> String {
	value
		.replace('&', "&amp;")
		.replace('<', "&lt;")
		.replace('>', "&gt;")
		.replace('"', "&quot;")
		.replace('\'', "&apos;")
}

fn truncate_label(value: &str, max_chars: usize) -> String {
	let mut chars = value.chars();
	let mut result: String = chars.by_ref().take(max_chars).collect();
	if chars.next().is_some() {
		result.pop();
		result.push('…');
	}
	result
}

fn percent_encode(value: &str) -> String {
	use std::fmt::Write;

	let mut encoded = String::with_capacity(value.len() * 2);
	for byte in value.bytes() {
		if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
			encoded.push(byte as char);
		} else {
			let _ = write!(encoded, "%{byte:02X}");
		}
	}
	encoded
}

fn snapshot_image(snapshot: &AudioSnapshot, device_changed: bool) -> String {
	let volume = snapshot.volume.min(100);
	let bar_width = u16::from(volume) * 128 / 100;
	let accent = if snapshot.muted {
		"#fb7185"
	} else if device_changed {
		"#38bdf8"
	} else {
		"#a78bfa"
	};
	let status = if snapshot.muted {
		"MUTED".to_owned()
	} else {
		format!("{volume}%")
	};
	let device_name = escape_xml(&truncate_label(&snapshot.device_name, 25));
	let speaker_waves = if snapshot.muted {
		r##"<path d="M31 42l16 16m0-16L31 58" stroke="#fb7185" stroke-width="4" stroke-linecap="round"/>"##
	} else {
		r##"<path d="M34 44c4 3 4 9 0 12M39 39c8 7 8 18 0 24" fill="none" stroke="#f8fafc" stroke-width="3" stroke-linecap="round"/>"##
	};
	let switch_badge = if device_changed {
		r##"<path d="M145 24h18l-5-5m5 5-5 5M163 34h-18l5-5m-5 5 5 5" fill="none" stroke="#38bdf8" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"/>"##
	} else {
		""
	};

	let svg = format!(
		r##"<svg xmlns="http://www.w3.org/2000/svg" width="176" height="112" viewBox="0 0 176 112">
<defs><linearGradient id="bg" x1="0" y1="0" x2="1" y2="1"><stop stop-color="#111827"/><stop offset="1" stop-color="#020617"/></linearGradient></defs>
<rect width="176" height="112" rx="10" fill="url(#bg)"/>
<rect x="1" y="1" width="174" height="110" rx="9" fill="none" stroke="#334155"/>
<path d="M14 47h9l12-10v30L23 57h-9z" fill="#f8fafc"/>
{speaker_waves}
<text x="65" y="62" fill="{accent}" font-family="Arial, sans-serif" font-size="28" font-weight="700">{status}</text>
{switch_badge}
<rect x="24" y="77" width="128" height="8" rx="4" fill="#334155"/>
<rect x="24" y="77" width="{bar_width}" height="8" rx="4" fill="{accent}"/>
<text x="88" y="101" text-anchor="middle" fill="#cbd5e1" font-family="Arial, sans-serif" font-size="11">{device_name}</text>
</svg>"##
	);

	format!("data:image/svg+xml,{}", percent_encode(&svg))
}

async fn render_snapshot(
	context: String,
	snapshot: AudioSnapshot,
	device_changed: bool,
	outbound: &mut OutboundEventManager,
) -> EventHandlerResult {
	outbound
		.set_image(
			context,
			Some(snapshot_image(&snapshot, device_changed)),
			None,
		)
		.await?;
	Ok(())
}

async fn report_error(
	context: String,
	error: anyhow::Error,
	outbound: &mut OutboundEventManager,
) -> EventHandlerResult {
	log::warn!("Audio action failed for {context}: {error:#}");
	outbound.show_alert(context).await?;
	Err(error)
}

pub async fn rotate_volume(
	event: DialRotateEvent,
	outbound: &mut OutboundEventManager,
) -> EventHandlerResult {
	let context = event.context.clone();
	let step = setting_u8(&event.payload.settings, "step", 2).clamp(1, 20);
	let delta = i32::from(event.payload.ticks) * i32::from(step);
	match platform::change_volume(delta) {
		Ok(snapshot) => render_snapshot(context, snapshot, false, outbound).await,
		Err(error) => report_error(context, error, outbound).await,
	}
}

pub async fn toggle_mute(
	event: DialPressEvent,
	outbound: &mut OutboundEventManager,
) -> EventHandlerResult {
	let context = event.context.clone();
	match platform::toggle_mute() {
		Ok(snapshot) => render_snapshot(context, snapshot, false, outbound).await,
		Err(error) => report_error(context, error, outbound).await,
	}
}

pub async fn switch_on_press(
	event: DialPressEvent,
	outbound: &mut OutboundEventManager,
) -> EventHandlerResult {
	let context = event.context.clone();
	let direction = setting_direction(&event.payload.settings);
	match platform::switch_device(direction) {
		Ok(snapshot) => render_snapshot(context, snapshot, true, outbound).await,
		Err(error) => report_error(context, error, outbound).await,
	}
}

pub async fn switch_on_rotate(
	event: DialRotateEvent,
	outbound: &mut OutboundEventManager,
) -> EventHandlerResult {
	let context = event.context.clone();
	let direction = i32::from(event.payload.ticks.signum());
	if direction == 0 {
		return Ok(());
	}
	match platform::switch_device(direction) {
		Ok(snapshot) => render_snapshot(context, snapshot, true, outbound).await,
		Err(error) => report_error(context, error, outbound).await,
	}
}

pub async fn refresh(
	action: &str,
	context: String,
	outbound: &mut OutboundEventManager,
) -> EventHandlerResult {
	if !matches!(action, SYSTEM_VOLUME_ACTION | SWITCH_SOUND_DEVICE_ACTION) {
		return Ok(());
	}
	match platform::snapshot() {
		Ok(snapshot) => {
			render_snapshot(
				context,
				snapshot,
				action == SWITCH_SOUND_DEVICE_ACTION,
				outbound,
			)
			.await
		}
		Err(error) => report_error(context, error, outbound).await,
	}
}

#[cfg(windows)]
mod platform {
	use super::AudioSnapshot;

	use anyhow::{Context, Result, bail};
	use com_policy_config::{IPolicyConfig, PolicyConfigClient};
	use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
	use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
	use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
	use windows::Win32::Media::Audio::{
		DEVICE_STATE_ACTIVE, IMMDevice, IMMDeviceEnumerator, MMDeviceEnumerator, eCommunications,
		eConsole, eMultimedia, eRender,
	};
	use windows::Win32::System::Com::StructuredStorage::{PropVariantClear, PropVariantToString};
	use windows::Win32::System::Com::{
		CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree,
		CoUninitialize, STGM_READ,
	};
	use windows::core::{HSTRING, PCWSTR, PWSTR};

	struct ComApartment {
		uninitialize: bool,
	}

	impl ComApartment {
		fn enter() -> Result<Self> {
			// SAFETY: Every call in this module runs synchronously on the current thread.
			let result = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
			match result {
				status if status.is_ok() => Ok(Self { uninitialize: true }),
				RPC_E_CHANGED_MODE => Ok(Self {
					uninitialize: false,
				}),
				status => Err(windows::core::Error::from_hresult(status))
					.context("failed to initialise COM for audio control"),
			}
		}
	}

	impl Drop for ComApartment {
		fn drop(&mut self) {
			if self.uninitialize {
				// SAFETY: This balances the successful CoInitializeEx call on this thread.
				unsafe { CoUninitialize() };
			}
		}
	}

	struct Endpoint {
		id: String,
		name: String,
		device: IMMDevice,
	}

	fn take_pwstr(value: PWSTR) -> Result<String> {
		// SAFETY: GetId returns a null-terminated string allocated with CoTaskMemAlloc.
		let result = unsafe { value.to_string() };
		// SAFETY: This frees the buffer returned by IMMDevice::GetId exactly once.
		unsafe { CoTaskMemFree(Some(value.0.cast())) };
		result.context("audio endpoint ID was not valid UTF-16")
	}

	fn friendly_name(device: &IMMDevice) -> Result<String> {
		// SAFETY: The endpoint is active for the lifetime of the returned property store.
		let store = unsafe { device.OpenPropertyStore(STGM_READ) }
			.context("failed to open audio endpoint properties")?;
		// SAFETY: PKEY_Device_FriendlyName is a valid property key.
		let mut value = unsafe { store.GetValue(&PKEY_Device_FriendlyName) }
			.context("failed to read audio endpoint name")?;
		let mut buffer = [0u16; 512];
		// SAFETY: The output slice is writable and the PROPVARIANT remains alive for the call.
		let converted = unsafe { PropVariantToString(&value, &mut buffer) };
		// SAFETY: This clears the PROPVARIANT returned by IPropertyStore::GetValue.
		let cleared = unsafe { PropVariantClear(&mut value) };
		converted.context("failed to convert audio endpoint name")?;
		cleared.context("failed to clear audio endpoint property")?;
		let end = buffer
			.iter()
			.position(|character| *character == 0)
			.unwrap_or(buffer.len());
		Ok(String::from_utf16_lossy(&buffer[..end]))
	}

	fn enumerator() -> Result<IMMDeviceEnumerator> {
		// SAFETY: COM has been initialised on the current thread by the caller.
		unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) }
			.context("failed to create the Windows audio device enumerator")
	}

	fn endpoint_snapshot(device: &IMMDevice, name: String) -> Result<AudioSnapshot> {
		// SAFETY: IAudioEndpointVolume is supported by active render endpoints.
		let volume: IAudioEndpointVolume = unsafe { device.Activate(CLSCTX_ALL, None) }
			.context("failed to open the endpoint volume control")?;
		// SAFETY: The COM interface remains alive for both calls.
		let scalar = unsafe { volume.GetMasterVolumeLevelScalar() }
			.context("failed to read the system volume")?;
		// SAFETY: The COM interface remains alive for the call.
		let muted = unsafe { volume.GetMute() }
			.context("failed to read the system mute state")?
			.as_bool();
		Ok(AudioSnapshot {
			device_name: name,
			volume: (scalar.clamp(0.0, 1.0) * 100.0).round() as u8,
			muted,
		})
	}

	fn default_endpoint(enumerator: &IMMDeviceEnumerator) -> Result<Endpoint> {
		// SAFETY: The enumerator is valid and eRender/eMultimedia identify the default output.
		let device = unsafe { enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia) }
			.context("no default sound output device is available")?;
		// SAFETY: IMMDevice::GetId returns a CoTaskMem-allocated string.
		let id = take_pwstr(unsafe { device.GetId() }?)?;
		let name = friendly_name(&device)?;
		Ok(Endpoint { id, name, device })
	}

	fn active_endpoints(enumerator: &IMMDeviceEnumerator) -> Result<Vec<Endpoint>> {
		// SAFETY: The enumerator is valid and the requested state/dataflow are supported.
		let collection = unsafe { enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE) }
			.context("failed to list active sound output devices")?;
		// SAFETY: The collection remains alive while it is enumerated.
		let count = unsafe { collection.GetCount() }?;
		let mut endpoints = Vec::with_capacity(count as usize);
		for index in 0..count {
			// SAFETY: index is within the count returned by this collection.
			let device = unsafe { collection.Item(index) }?;
			// SAFETY: IMMDevice::GetId returns a CoTaskMem-allocated string.
			let id = take_pwstr(unsafe { device.GetId() }?)?;
			let name = friendly_name(&device)?;
			endpoints.push(Endpoint { id, name, device });
		}
		endpoints.sort_by_key(|endpoint| endpoint.name.to_lowercase());
		Ok(endpoints)
	}

	pub fn snapshot() -> Result<AudioSnapshot> {
		let _apartment = ComApartment::enter()?;
		let endpoint = default_endpoint(&enumerator()?)?;
		endpoint_snapshot(&endpoint.device, endpoint.name)
	}

	pub fn change_volume(delta: i32) -> Result<AudioSnapshot> {
		let _apartment = ComApartment::enter()?;
		let endpoint = default_endpoint(&enumerator()?)?;
		// SAFETY: IAudioEndpointVolume is supported by active render endpoints.
		let volume: IAudioEndpointVolume = unsafe { endpoint.device.Activate(CLSCTX_ALL, None) }
			.context("failed to open the endpoint volume control")?;
		// SAFETY: The COM interface remains alive for all calls.
		let current = unsafe { volume.GetMasterVolumeLevelScalar() }?;
		let next = (current + delta as f32 / 100.0).clamp(0.0, 1.0);
		// SAFETY: A null event context is explicitly supported by the Core Audio API.
		unsafe {
			volume.SetMasterVolumeLevelScalar(next, std::ptr::null())?;
			if delta != 0 {
				volume.SetMute(false, std::ptr::null())?;
			}
		}
		endpoint_snapshot(&endpoint.device, endpoint.name)
	}

	pub fn toggle_mute() -> Result<AudioSnapshot> {
		let _apartment = ComApartment::enter()?;
		let endpoint = default_endpoint(&enumerator()?)?;
		// SAFETY: IAudioEndpointVolume is supported by active render endpoints.
		let volume: IAudioEndpointVolume = unsafe { endpoint.device.Activate(CLSCTX_ALL, None) }
			.context("failed to open the endpoint volume control")?;
		// SAFETY: The COM interface remains alive and a null event context is supported.
		unsafe {
			let muted = volume.GetMute()?.as_bool();
			volume.SetMute(!muted, std::ptr::null())?;
		}
		endpoint_snapshot(&endpoint.device, endpoint.name)
	}

	pub fn switch_device(direction: i32) -> Result<AudioSnapshot> {
		let _apartment = ComApartment::enter()?;
		let enumerator = enumerator()?;
		let current = default_endpoint(&enumerator)?;
		let endpoints = active_endpoints(&enumerator)?;
		if endpoints.len() < 2 {
			bail!("at least two active sound output devices are required");
		}
		let current_index = endpoints
			.iter()
			.position(|endpoint| endpoint.id == current.id)
			.unwrap_or(0);
		let target_index =
			(current_index as i32 + direction.signum()).rem_euclid(endpoints.len() as i32) as usize;
		let target = &endpoints[target_index];
		let target_id = HSTRING::from(&target.id);
		let target_id = PCWSTR(target_id.as_ptr());
		// SAFETY: COM is initialised, PolicyConfigClient is the registered policy COM class,
		// and the endpoint ID stays alive for all three role updates.
		let policy: IPolicyConfig =
			unsafe { CoCreateInstance(&PolicyConfigClient, None, CLSCTX_ALL) }
				.context("failed to open the Windows sound device policy")?;
		// Set every role so applications do not keep using a stale communications endpoint.
		unsafe {
			policy.SetDefaultEndpoint(target_id, eConsole)?;
			policy.SetDefaultEndpoint(target_id, eMultimedia)?;
			policy.SetDefaultEndpoint(target_id, eCommunications)?;
		}
		endpoint_snapshot(&target.device, target.name.clone())
	}
}

#[cfg(not(windows))]
mod platform {
	use super::AudioSnapshot;
	use anyhow::{Result, bail};

	fn unsupported<T>() -> Result<T> {
		bail!("system audio actions are currently available on Windows")
	}

	pub fn snapshot() -> Result<AudioSnapshot> {
		unsupported()
	}

	pub fn change_volume(_delta: i32) -> Result<AudioSnapshot> {
		unsupported()
	}

	pub fn toggle_mute() -> Result<AudioSnapshot> {
		unsupported()
	}

	pub fn switch_device(_direction: i32) -> Result<AudioSnapshot> {
		unsupported()
	}
}

#[cfg(test)]
mod tests {
	use super::{AudioSnapshot, snapshot_image};

	#[test]
	fn lcd_image_uses_native_zone_dimensions_and_escapes_device_name() {
		let image = snapshot_image(
			&AudioSnapshot {
				device_name: "Speakers & Headphones".to_owned(),
				volume: 67,
				muted: false,
			},
			false,
		);

		assert!(image.starts_with("data:image/svg+xml,"));
		assert!(image.contains("width%3D%22176%22"));
		assert!(image.contains("height%3D%22112%22"));
		assert!(image.contains("Speakers%20%26amp%3B%20Headphones"));
		assert!(image.contains("67%25"));
	}

	#[test]
	fn lcd_image_marks_muted_and_device_switch_states() {
		let muted = snapshot_image(
			&AudioSnapshot {
				device_name: "Output".to_owned(),
				volume: 50,
				muted: true,
			},
			false,
		);
		let switched = snapshot_image(
			&AudioSnapshot {
				device_name: "Output".to_owned(),
				volume: 50,
				muted: false,
			},
			true,
		);

		assert!(muted.contains("MUTED"));
		assert!(muted.contains("%23fb7185"));
		assert!(switched.contains("%2338bdf8"));
	}

	#[cfg(windows)]
	#[test]
	fn windows_default_output_snapshot_is_readable() {
		let snapshot =
			super::platform::snapshot().expect("default Windows output should be readable");

		assert!(!snapshot.device_name.trim().is_empty());
		assert!(snapshot.volume <= 100);
	}
}
