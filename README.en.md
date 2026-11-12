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

> It does what the Android app does, adapted to the desktop.

## What it does

- **Download** from YouTube, TikTok, X, Instagram and Facebook (and many more sites) with
  yt-dlp, as H.264 + AAC so the video plays and shares anywhere. Pick the quality for each
  video: best, 1080p, 720p, 480p or audio only in **M4A** (not re-encoded) or **MP3** (VBR V0,
  about 245 kbps). Audio files carry the cover, title and artist and, in playlists, the album
  (the playlist name) and the track number. On TikTok only formats with picture and
  sound together are used, and if a video arrives silent Klippo asks for it again in another
  format.
- YouTube **playlists** (a `youtube.com/playlist?list=…` link, up to 100 items) and Instagram
  and X **carousels** (up to 20): the bar shows the overall progress ("40 % · 4 of 10") and the
  playlist takes a single card with **Save all** (`Downloads\Klippo\<playlist>\`, each file
  named after its title), **Share**, **Copy** and **WhatsApp** for all of them at once; "Show
  one by one" expands it. If a video is private or deleted, the rest are still downloaded.
- **From Spotify**: paste (or drag, or send with the bookmark) a Spotify playlist, album or
  song link. Klippo reads its songs without an account, searches each one on YouTube and tells
  you whether the match is **identical** (same title, artist and length), **similar** or not
  found; click a row to open the video and check it. "Download all" gets the audio (M4A or MP3,
  in *Settings*) with Spotify's title, artist, album, track number and cover inside, named
  `Artist - Title` (in a playlist or album, with the number first: `04 - Artist - Title`). Two download at a time (the rest are "Queued"); if a video fails, another of
  the matches is tried. With "Save when finished" each song goes to
  `Downloads\Klippo\<playlist>\` on its own, and "Show in File Explorer" opens that folder.
  Without an account Spotify only shows the first 100 songs of a playlist: if it has more,
  Klippo tells you. If a song is not found or you do not like the chosen video, the magnifier on
  its row lets you search YouTube yourself or paste the link of the video you want.
- **Optional login** to Instagram, X and Facebook for videos that require it
  (*Settings → Accounts*, or the button on the failed download). The official website opens in
  a window; Klippo never sees your password and only keeps the cookies, on your computer.
- Light, dark or Windows theme. If the engine is more than a month old, Klippo suggests
  updating it.
- Progress with time left, cancel, retry and, if a platform changes, **Update engine and
  retry** right from the card. List or grid with thumbnails.

- **From the browser to Klippo without extensions**, in Chrome, Edge, Opera, Firefox or any other:
  - **Copy the link.** If Klippo is in front, it appears in the field; if it's behind or in the
    tray, a Windows notification offers to download it. It never downloads on its own.
  - **“Send to Klippo” bookmark.** A button on the bookmarks bar that sends the tab's video to
    Klippo in one click (*Settings → From the browser → Install*).
  - Or drag the tab onto the Klippo window, or press Ctrl+V in it.
  - With *Start with Windows*, Klippo waits in the tray without opening the window.
- **Share** from each video: WhatsApp (copies the video and opens WhatsApp: pick the chat and
  press Ctrl+V), the Windows Share menu, Copy (to paste it anywhere) or drag the thumbnail to
  another app or a folder.
- **Save** creates `Downloads\Klippo\<video title>.mp4` (or `.m4a`, `.mp3`), with
  `klippo_<date>` only when there's no title; if it already exists, it adds " (1)", " (2)"… If
  you don't click it, the video never leaves Klippo's temporary folder.
- A Windows notification when each download finishes, if Klippo isn't in front, with WhatsApp,
  Copy and Save.

- **Edit** (click the thumbnail or the pencil):
  - Trim by dragging the handles or typing the start and end to the tenth of a second, with a
    looping preview of the selection.
  - Crop to 9:16, 1:1, 4:5 or free: move and resize the frame over the video.
  - Text with outline or background, and emojis: drag them and resize with the mouse wheel.
  - Pixelate fixed areas (faces, number plates…) and remove the sound.
  - Everything is applied at once with FFmpeg into an H.264 + AAC video, which you share or
    save from the same window.


## Installation

1. Download `Klippo_…_x64-setup.exe` from the [latest release](https://github.com/rboluda/KlippoDesktop/releases/latest).
2. Open it. It installs for your user only, without asking for administrator rights.

The installer is signed with a personal certificate (rboluda), not a paid one from a public
authority, so the first time Windows may show *"Windows protected your PC"*: click **More info →
Run anyway**.

Requirements: 64-bit Windows 10 or 11.

## Updates

Once a day, when it opens, Klippo checks this repository for a new version and shows it at the
top of the window. Click **Update** and it installs itself. You can also check from
*Settings → Updates* or turn the check off. Updates are signed: Klippo only installs the ones
published here.

## Privacy

No ads, no analytics, no accounts, no welcome screens. Klippo only connects to download the
video you ask for, to update its engine when you ask, and once a day to check for a new
version (can be turned off). It only connects to Spotify when you paste or share a Spotify
link, to read that playlist and its cover.

## Building

You need [Node.js](https://nodejs.org), [Rust](https://rustup.rs) and the Visual Studio C++
tools (*Desktop development with C++* workload).

```bash
npm install
pwsh scripts/preparar-motor.ps1   # downloads yt-dlp, Deno and FFmpeg (pinned versions and hashes)
npm run tauri dev                 # to try it
npm run tauri build               # installer in src-tauri/target/release/bundle/nsis
```

Icons are generated from `docs/icono/klipo_icon.png` with `python scripts/generar_iconos.py`
(needs Pillow). Versions are published with `scripts/publicar.ps1`.

Built with [Tauri 2](https://tauri.app) (Rust) and [Svelte 5](https://svelte.dev) with
TypeScript, on WebView2.

## Credits

| Project | License |
| --- | --- |
| [yt-dlp](https://github.com/yt-dlp/yt-dlp) | Unlicense |
| [FFmpeg](https://ffmpeg.org) ([gyan.dev build](https://www.gyan.dev/ffmpeg/builds/), with libx264) | GPL-3.0 |
| [Deno](https://deno.com) | MIT |
| [Tauri](https://tauri.app) | MIT / Apache 2.0 |
| [Svelte](https://svelte.dev) | MIT |

The FFmpeg source code for each version is at [ffmpeg.org/releases](https://ffmpeg.org/releases/)
(the exact version is shown in *Settings → Download engine*), and the source of each bundled
library at [gyan.dev](https://www.gyan.dev/ffmpeg/builds/).

## License

Klippo is free software under the [GNU General Public License v3.0 or later](LICENSE)
(GPL-3.0-or-later), like the Android version: you can use, study, modify and redistribute it,
as long as the versions you distribute stay free with their source code available.

---

<p align="center">Made by <a href="https://github.com/rboluda">rboluda</a></p>
