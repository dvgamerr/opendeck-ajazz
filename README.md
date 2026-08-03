<div align="center">

<img src="assets/logo.svg" alt="Pixeldeck Ajazz" width="190" />

# Pixeldeck Ajazz

### Make every key, dial, and touchscreen zone yours.

Open-source desktop control software built specifically for Ajazz stream controllers.

[![Latest release](https://img.shields.io/github/v/release/dvgamerr/pixeldeck-ajazz?style=flat-square&color=4f8cff)](https://github.com/dvgamerr/pixeldeck-ajazz/releases/latest)
[![Lint](https://img.shields.io/github/actions/workflow/status/dvgamerr/pixeldeck-ajazz/lint-app.yml?branch=main&style=flat-square&label=checks)](https://github.com/dvgamerr/pixeldeck-ajazz/actions/workflows/lint-app.yml)
[![License](https://img.shields.io/badge/license-GPL--3.0--or--later-6d5dfc?style=flat-square)](LICENSE.md)
[![Windows and Linux](https://img.shields.io/badge/platform-Windows%20%7C%20Linux-1f9d76?style=flat-square)](#installation)

[Download](https://github.com/dvgamerr/pixeldeck-ajazz/releases/latest) · [Supported devices](#supported-devices) · [Build from source](#development) · [Report a bug](https://github.com/dvgamerr/pixeldeck-ajazz/issues)

</div>

---

Pixeldeck Ajazz turns an Ajazz control deck into a flexible desktop workspace. Assign actions to display keys, use rotary encoders and touchscreen zones, organize layouts into profiles, and switch context without reaching for the mouse.

It is a hard fork of [OpenDeck](https://github.com/nekename/OpenDeck), with a bundled Ajazz driver and device-specific layouts maintained in this repository.

## Highlights

- **Ajazz-native hardware support** — USB/HID discovery, correct key geometry, brightness, boot images, and device-specific image conversion.
- **Keys, dials, and touch zones** — configure keypad and encoder actions from one visual editor.
- **Fast profile workflows** — create folders, rename profiles, map profiles to applications, or page through an ordered set with a dial.
- **Readable dial feedback** — Profile Pagination shows the active profile and page indicators on supported LCD zones.
- **Plugin ecosystem** — bundled everyday actions plus support for OpenDeck/Stream Deck-compatible plugins; Wine can run many Windows-only plugins on Linux.
- **Local-first configuration** — profiles and settings stay on your computer.

## Supported devices

| Device             | Display keys |                 Encoders / touch |
| ------------------ | -----------: | -------------------------------: |
| Ajazz AKP153       |           18 |                                — |
| Ajazz AKP153E      |           18 |                                — |
| Ajazz AKP153R      |           18 |                                — |
| Ajazz AKP815       |           15 |                                — |
| Ajazz AKP03        |            6 |                       3 encoders |
| Ajazz AKP03E       |            6 |                       3 encoders |
| Ajazz AKP03R       |            6 |                       3 encoders |
| Ajazz AKP03R rev 2 |            6 |                       3 encoders |
| Ajazz AKP05E_552A  |           10 | 4 encoders + 4 touchscreen zones |

> [!IMPORTANT]
> AKP05E_552A is the exact supported model name. Its touchscreen is one physical strip exposed as four configurable action zones.

## Installation

Download the newest package from [GitHub Releases](https://github.com/dvgamerr/pixeldeck-ajazz/releases/latest).

### Windows

1. Download the `.exe` (NSIS) or `.msi` installer.
2. Run the installer, connect the device, and launch Pixeldeck Ajazz.
3. If Windows warns about an unsigned community build, verify that it came from this repository before continuing.

### Linux

1. Prefer the `.deb` or `.rpm` package for your distribution. These packages install the udev rule automatically.
2. For another package format, install [`40-ajazz.rules`](src-tauri/bundle/40-ajazz.rules) manually:

   ```bash
   sudo cp src-tauri/bundle/40-ajazz.rules /etc/udev/rules.d/
   sudo udevadm control --reload-rules
   sudo udevadm trigger
   ```

3. Reconnect the device, then start Pixeldeck Ajazz.

AppImage builds are not recommended because USB permissions and plugin execution can vary between distributions. Install [Wine](https://www.winehq.org/) (and Wine Mono when required) if you use plugins distributed only as Windows executables.

## Profile Pagination in 30 seconds

1. Drop **Profile Pagination** onto an encoder/touchscreen zone.
2. Add profiles under **Available profiles**.
3. Use the arrow buttons to set the page order.
4. Place the action on the same dial position in every selected profile.
5. Turn the dial to move between profiles; the order wraps at both ends.

The property inspector saves changes immediately. Renamed and deleted profiles are also updated in pagination, application mappings, and Switch Profile actions.

## Development

### Prerequisites

- [Bun 1.3+](https://bun.sh/)
- A stable [Rust toolchain](https://rustup.rs/)
- [Tauri 2 system prerequisites](https://tauri.app/start/prerequisites/)
- Linux only: the `libudev` development package for your distribution

### Run locally

```bash
git clone https://github.com/dvgamerr/pixeldeck-ajazz.git
cd pixeldeck-ajazz
bun install --frozen-lockfile
bun run tauri dev
```

Do not start multiple Tauri development sessions at once; both sessions may compete for the same HID device and local plugin ports.

### Useful commands

| Command                                                                                | Purpose                                                      |
| -------------------------------------------------------------------------------------- | ------------------------------------------------------------ |
| `bun run dev`                                                                          | Start the frontend only                                      |
| `bun run tauri dev`                                                                    | Start the complete desktop application                       |
| `bun run verify`                                                                       | Format-check, type-check, lint, test, and build the frontend |
| `cargo test --manifest-path src-tauri/Cargo.toml`                                      | Test the Tauri backend                                       |
| `cargo test --manifest-path src-tauri/lib/ajazz-sdk/Cargo.toml`                        | Test the Ajazz driver                                        |
| `cargo test --manifest-path plugins/com.amansprojects.starterpack.sdPlugin/Cargo.toml` | Test bundled actions                                         |
| `./scripts/lint-app.sh`                                                                | Run the complete repository validation gate                  |

### Project map

```text
src/                                             Svelte desktop UI
src-tauri/src/                                   Tauri backend and event bridge
src-tauri/lib/ajazz-sdk/                         Ajazz HID driver and protocol
plugins/com.amansprojects.starterpack.sdPlugin/  Bundled actions and inspectors
tests/                                           Frontend behavior tests
```

The Tauri build stages the bundled plugin automatically. Frontend package management uses Bun; keep `bun.lock` in sync with `package.json`.

## Contributing

Bug reports, device traces, documentation improvements, and focused pull requests are welcome. Before opening a pull request:

1. Keep device-specific geometry and protocol behavior covered by tests.
2. Run `bun install --frozen-lockfile` from a clean checkout.
3. Run `./scripts/lint-app.sh` (or the equivalent commands from the table above on Windows).
4. Describe the device model and operating system used for hardware testing.

## Troubleshooting

- **Device not detected on Linux:** confirm the udev rule is installed, reload the rules, and reconnect the USB cable.
- **Plugin does not start on Linux:** check whether it is Windows-only and requires Wine or Wine Mono.
- **Profile dial disappears after switching:** add Profile Pagination to the same encoder slot in every profile in its page order.
- **Two app windows behave unpredictably:** close all instances and start a single Pixeldeck Ajazz session.

## Credits and license

Pixeldeck Ajazz builds on [OpenDeck](https://github.com/nekename/OpenDeck) and the work of its contributors. Thank you to everyone testing Ajazz hardware and sharing protocol findings.

Licensed under the [GNU General Public License v3.0 or later](LICENSE.md).
