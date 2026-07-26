# opendeck-ajazz Project Knowledge

## Scope and terminology

- This project is a Tauri desktop application for **stream controller / Stream Deck-style hardware**. It is not related to Valve's Steam Deck.
- It is a hard fork of OpenDeck with a local Rust driver in `src-tauri/lib/ajazz-sdk`.
- The frontend is Svelte 4 + SvelteKit + Tailwind CSS 4. The backend is Rust + Tauri 2.
- JavaScript tooling is **Bun only**. Do not add Deno commands or Deno-specific configuration back into the project.

## Primary device: Ajazz AKP05E_552A

The exact model name matters. Use these names consistently:

- User-facing name: `Ajazz AKP05E_552A`
- Rust enum: `Kind::Akp05E552A`
- PID constant: `PID_AJAZZ_AKP05E_552A`
- USB VID/PID: `0x0300:0x3004`

Do not shorten the model to `AKP05`, `AKP05E`, or treat it as a generic four-column device.

### Physical layout

- 10 display buttons arranged as **2 rows × 5 columns**.
- One continuous physical touchscreen strip below the buttons.
- The touchscreen exposes **4 action zones** to the protocol/OpenDeck model.
- 4 rotary encoders/knobs below the touchscreen.
- OpenDeck device type `7` selects the special AKP05E_552A frontend layout.

The touchscreen must look like one continuous screen in the UI. Do not draw four separate physical screens. Internally, it must remain four `Encoder` action slots because that is how the device protocol and OpenDeck controller model expose the four zones.

### Display and image formats

- Button image: JPEG, `100×100`, rotated 180°, no mirroring.
- Full LCD strip metadata: `800×100`, JPEG, rotated 180°.
- Each touchscreen action-zone upload: JPEG, `176×112`, rotated 180°.
- The UI renders the four `176×112` zone canvases as four adjacent `160×100` elements so their combined visible width matches the five-button grid.
- Image writes are cached and require `flush()` before they appear on the hardware.
- `clear_all_button_images()` uses the `0xff` clear-all sentinel; `opendeck_to_device_key()` must continue accepting this sentinel.

Although the hardware is one long screen, do not merge the four zone uploads into one frontend action. The Mirabox N4 protocol exposes four second-screen keys/zones.

### Button and input mapping

- OpenDeck display-key positions `0..9` map to native positions:
  `[10, 11, 12, 13, 14, 5, 6, 7, 8, 9]`.
- Input button codes `0x01..=0x0a` map to OpenDeck button positions `0..9`.
- Encoder rotation codes:
  - encoder 0: `0xa0` / `0xa1`
  - encoder 1: `0x50` / `0x51`
  - encoder 2: `0x90` / `0x91`
  - encoder 3: `0x70` / `0x71`
- Press/touch codes are accepted in pairs:
  - encoder/zone 0: `0x37` or `0x40`
  - encoder/zone 1: `0x35` or `0x41`
  - encoder/zone 2: `0x33` or `0x42`
  - encoder/zone 3: `0x36` or `0x43`
- Swipe codes `0x38` and `0x39` are intentionally ignored because the current OpenDeck controller model has no swipe action.
- Unknown packets return `AjazzError::BadData`; the device loop logs them at debug level and continues.

Primary protocol reference:

- Mirabox StreamDock N4 SDK:
  <https://github.com/MiraboxSpace/StreamDock-Device-SDK/blob/31d887551de556bd0776bf4982233999d58e49d1/CPP-SDK/src/HotspotDevice/StreamDockN4/streamdockN4.cpp>

## HID discovery and device lifecycle

- AKP05E_552A should use HID usage page `0xffa0` and usage `0x0001`.
- On platforms where usage information is unavailable as `(0, 0)`, interface `0` is the supported fallback.
- Filter the interface before connecting. The device can expose multiple HID interfaces.
- Device IDs are generated as `sd-{serial}`. Never hard-code the observed serial number into application logic or profile data.
- `MANAGED_DEVICES` prevents multiple connection tasks for the same physical device.
- `AJAZZ_DEVICES` contains only successfully initialized/registered devices.
- Initialization order is:
  1. connect;
  2. clear images;
  3. apply brightness;
  4. flush;
  5. register with OpenDeck;
  6. insert into `AJAZZ_DEVICES`;
  7. start the input/keep-alive loop.
- The loop uses a 50 ms input timeout and sends keep-alive every 10 seconds. Preserve this bounded read; do not replace it with a busy loop or add another polling process.
- On read or keep-alive failure, remove the device from both maps and emit deregistration.
- All known `Event` variants are matched explicitly. Do not add an unreachable wildcard arm.

## Frontend device/profile lifecycle

- Device events and profile requests can race during USB disconnect/reconnect.
- `DeviceSelector.svelte` uses a generation counter and a `registered` set. Late profile responses must be ignored when their device/generation is stale.
- Remove profiles for devices no longer present.
- Tauri event listeners created in `onMount` must always keep and call their `UnlistenFn` callbacks during teardown.
- Do not repeatedly mount/unmount `PropertyInspectorView` when merely selecting a key. Keep the component mounted and toggle its visibility so its window listener/iframes are not recreated.
- Avoid adding timers or polling loops when an existing Tauri event can update the state.

## Action model and interaction behavior

- OpenDeck controller names are exactly `Keypad` and `Encoder`.
- Touchscreen zones and rotary encoder actions share the `Encoder` profile slots.
- The Keys/Dials tabs in `ActionList.svelte` are functional filters based on `action.controllers`; they are not decorative tabs.
- Dragging a library action uses MIME type `application/x-opendeck-action` and retains the legacy `"action"` payload for compatibility.
- Before replacing an occupied slot, verify that `action.controllers` includes the destination controller. Then remove the existing instance and create the replacement.
- Moving an existing configured slot is allowed only within the same controller type; the Rust backend rejects cross-controller moves.
- Drop handlers must call `preventDefault()` and `stopPropagation()` before asynchronous work.
- The native browser/WebView context menu is disabled at page level.
- The custom key context menu uses `clientX/clientY` with `position: fixed`. Do not use viewport coordinates with an absolutely positioned device-relative menu.
- Clamp the custom menu to the viewport and close it when dragging begins.

## UI layout invariants

- The current visual direction is a dark desktop workspace inspired by Stream Deck software:
  - application/device toolbar at the top;
  - centered physical-device canvas;
  - action library sidebar on the right;
  - device/profile selectors at the top of the sidebar.
- Preserve the real AKP05E_552A geometry even if a design reference shows a different button count.
- The five-button row and four-zone touchscreen must remain visually aligned.
- The current minimum-window calculation reserves room for the `21rem` action sidebar:
  `max(columns, encoders) * 132 + 392`.
- Shared design styles live in `src/app.css`; avoid duplicating large class groups across components.

## Important source locations

- Device metadata and layouts: `src-tauri/lib/ajazz-sdk/src/info.rs`
- HID interface filtering: `src-tauri/lib/ajazz-sdk/src/hid.rs`
- Input parsing: `src-tauri/lib/ajazz-sdk/src/protocol/parser.rs`
- Image upload/clear operations: `src-tauri/lib/ajazz-sdk/src/device.rs`
- Application device loop: `src-tauri/src/ajazz.rs`
- Tauri setup: `src-tauri/src/main.rs`
- Main UI shell: `src/routes/+page.svelte`
- Physical device UI: `src/components/DeviceView.svelte`
- Key canvas and context menu: `src/components/Key.svelte`
- Action library and drag payload: `src/components/ActionList.svelte`
- Device/profile race handling: `src/components/DeviceSelector.svelte`

## Tauri and Windows notes

- `tauri-plugin-single-instance` is pinned to `2.2.2`.
- Initialize the single-instance plugin first in the Tauri builder chain. Changing its initialization order previously caused a Windows null-pointer abort.
- `dragDropEnabled` is `false` in `tauri.conf.json` so frontend HTML drag-and-drop remains in control.
- Do not launch duplicate Tauri dev sessions. Check existing `bun`, `cargo`, and `opendeck-ajazz.exe` processes first.

## Development commands

Install dependencies:

```powershell
bun install --frozen-lockfile
```

Run the application:

```powershell
bun run tauri dev
```

Frontend validation:

```powershell
bun run check
bun run lint
bun run format:check
bun run build
```

Rust validation:

```powershell
cargo fmt --check --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/lib/ajazz-sdk/Cargo.toml
```

When running build/dev commands from this repository, use a visible PowerShell 7 session with `Start-Transcript`, and read the same transcript after completion.

## Validation expectations

For frontend-only changes, run at least:

1. Prettier on touched files.
2. `bun run check`.
3. `bun run lint`.
4. `bun run build`.

For driver/protocol changes, also run the Rust checks and the `ajazz-sdk` tests. The AKP05E_552A metadata test must continue asserting:

- product name `Ajazz AKP05E_552A`;
- key layout `(2, 5)`;
- 4 touchscreen points;
- 4 encoders;
- LCD strip size `(800, 100)`.

Hardware acceptance is stronger than compilation. A successful session should log:

```text
Registered Ajazz AKP05E_552A as sd-<serial>
```

and should not repeatedly log `device ... not found`, keep-alive failures, or reader failures afterward.

## Known non-blocking diagnostic

`svelte-check` currently reports one existing warning for the exported `params` property in `src/routes/+page.svelte`. It has not prevented checking, linting, or production builds. Do not confuse this warning with a driver or device-registration failure.
