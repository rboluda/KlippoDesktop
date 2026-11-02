<p align="center">
  <img src="docs/icono/klippo_redondeado.png" width="128" alt="Klippo icon">
</p>

<h1 align="center">Klippo for Windows</h1>

<p align="center"><a href="README.md">Español</a> · <b>English</b></p>

<p align="center">
  <b>Download, trim and share videos from YouTube, TikTok, X, Instagram and Facebook.</b><br>
  No ads. No accounts. Nothing left behind on your disk.<br>
  Videos delete themselves: Klippo empties its temporary folder every hour.
</p>

<p align="center">
  <a href="https://github.com/rboluda/KlippoDesktop/releases/latest"><b>⬇️ Download the installer</b></a>
</p>

---

This is the desktop version of [Klippo for Android](https://github.com/rboluda/Klippo): the
same idea, the same colors and everything learned there about each platform.

> **Work in progress.** This first version installs Klippo, keeps it in the system tray and
> updates itself. Downloads, the editor and the browser hand-off arrive in the next versions.

## What it will do

- **Download** from YouTube, TikTok, X, Instagram and Facebook with yt-dlp, as H.264 + AAC so
  the video plays and shares anywhere. Best quality, 1080p, 720p, 480p or audio only.
- **From the browser to Klippo without extensions**, in Chrome, Opera, Firefox or any other:
  copy the link and Klippo offers to download it, or click the *Send to Klippo* bookmark.
- **Edit**: trim to the exact time, crop to 9:16, 1:1, 4:5 or free, add text and emojis,
  pixelate areas and mute. Everything is applied at once with FFmpeg.
- **Share**: copy the video to paste it into WhatsApp, Telegram or anywhere, drag it, or save
  it to *Downloads\Klippo*. If you don't save it, it's gone after an hour.

## Installation

1. Download `Klippo_…_x64-setup.exe` from the [latest release](https://github.com/rboluda/KlippoDesktop/releases/latest).
2. Open it. It installs for your user only, without asking for administrator rights.

The installer isn't signed with a paid certificate, so the first time Windows may show
*"Windows protected your PC"*: click **More info → Run anyway**.

Requirements: 64-bit Windows 10 or 11.

## Updates

Once a day, when it opens, Klippo checks this repository for a new version and shows it at the
top of the window. Click **Update** and it installs itself. You can also check from
*Settings → Updates* or turn the check off. Updates are signed: Klippo only installs the ones
published here.

## Privacy

No ads, no analytics, no accounts, no welcome screens. Klippo only connects to download the
video you ask for, to update its engine when you ask, and once a day to check for a new
version (can be turned off).

## Building

You need [Node.js](https://nodejs.org), [Rust](https://rustup.rs) and the Visual Studio C++
tools (*Desktop development with C++* workload).

```bash
npm install
npm run tauri dev      # to try it
npm run tauri build    # installer in src-tauri/target/release/bundle/nsis
```

Icons are generated from `docs/icono/klipo_icon.png` with `python scripts/generar_iconos.py`
(needs Pillow). Versions are published with `scripts/publicar.ps1`.

Built with [Tauri 2](https://tauri.app) (Rust) and [Svelte 5](https://svelte.dev) with
TypeScript, on WebView2.

## Credits

| Project | License |
| --- | --- |
| [Tauri](https://tauri.app) | MIT / Apache 2.0 |
| [Svelte](https://svelte.dev) | MIT |

## License

Klippo is free software under the [GNU General Public License v3.0 or later](LICENSE)
(GPL-3.0-or-later), like the Android version: you can use, study, modify and redistribute it,
as long as the versions you distribute stay free with their source code available.

---

<p align="center">Made by <a href="https://github.com/rboluda">rboluda</a></p>
