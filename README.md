<div align="center">

![Pixeldeck Ajazz Logo](assets/logo.svg)

# Pixeldeck Ajazz

[![Made with love](assets/badge-made-with-love.svg)](https://github.com/dvgamerr/pixeldeck-ajazz/graphs/contributors)
[![GitHub release (latest by date)](https://img.shields.io/github/v/release/dvgamerr/pixeldeck-ajazz?style=for-the-badge)](https://github.com/dvgamerr/pixeldeck-ajazz/releases/latest)
[![Development status)](assets/badge-development-status.svg)](https://github.com/dvgamerr/pixeldeck-ajazz/issues)
[![Discord](assets/badge-discord.svg)](https://mistweaverco.com/discord)

<p></p>

Linux software for your Ajazz stream controller devices.

Hard-fork of [OpenDeck](https://github.com/nekename/OpenDeck) with
support for Ajazz devices.

<p></p>

</div>

> [!NOTE]
> The [OG OpenDeck](https://github.com/nekename/OpenDeck) supports Ajazz devices via plugins.

## Supported devices

- Ajazz AKP153
- Ajazz AKP815
- Ajazz AKP153E
- Ajazz AKP153R
- Ajazz AKP03
- Ajazz AKP03E
- Ajazz AKP03R
- Ajazz AKP03R rev 2
- Ajazz AKP05

## Installation

### Linux

- Download the latest release from [GitHub Releases](https://github.com/dvgamerr/pixeldeck-ajazz/releases/latest).
	- You should avoid AppImage releases of Pixeldeck Ajazz as they tend to
    have problems (you should also just avoid AppImages in general).
- Install the appropriate udev subsystem rules from
  [here](src-tauri/bundle/40-ajazz.rules)
	- If you're using a `.deb` or `.rpm` release artifact,
    this file should be installed automatically.
	- Otherwise, download and copy it to the correct location with
    `sudo cp src-tauri/bundle/40-ajazz.rules /etc/udev/rules.d/`.
	- In both cases, you will need to reload your udev subsystem rules with:
    `sudo udevadm control --reload-rules && sudo udevadm trigger`.
- If you intend to use plugins that are not compiled for Linux (which are the majority of plugins),
  you will need to have [Wine](https://www.winehq.org/) installed
  on your system.
  Some plugins may also depend on Wine Mono
  (which is sometimes, but not always included,
  in your distro's packaging of Wine).

## Contributing

You'll need to ensure that all of the
[prerequisites for building a
Tauri application](https://tauri.app/start/prerequisites)
are satisfied to build Pixeldeck Ajazz from source.

You'll also need `libudev` installed for your distribution.

After running `bun install --frozen-lockfile`,
you can use `bun run tauri dev` and
`NO_STRIP=true bun run tauri build` to work with Pixeldeck Ajazz.

Before each commit, please ensure that all of the following are completed:

1. Rust code has been linted using `cargo clippy` and it discovers no violations
2. Rust code has been formatted using `cargo fmt`
3. TypeScript and Svelte code has been checked using `bun run check`
4. Frontend code has been linted using `bun run lint`
5. Frontend code has been formatted using `bun run format`

## License

Pixeldeck Ajazz is based of OpenDeck and
therefore licensed under the GNU General Public License version 3.0 or later.

For more details, see the [LICENSE.md](LICENSE.md) file.

