# SpotiFrame

A lightweight, native desktop lyrics display for Spotify built in Rust with `egui`/`eframe`. SpotiFrame pairs a blurred album-art backdrop with synced, clickable lyrics via [LRCLIB](https://lrclib.net/) and built-in playback controls—designed to run cleanly on a secondary monitor or in borderless fullscreen.

![SpotiFrame Preview](assets/preview.png)

## Features

- **Synced Lyrics (LRCLIB):** Automatically fetches time-synced lyrics with active-line highlighting, smooth auto-scrolling, and click-to-seek on any lyric line. Falls back cleanly to plain lyrics or an instrumental indicator.
- **Dynamic Visuals:** Full-bleed blurred album-art backdrop, shadowed cover art, and responsive layout that adapts automatically to long track titles and multi-artist credits.
- **Built-in Playback Controls:** Interactive seekbar, Previous, Play/Pause, Next, Repeat (off / context / track), and Fullscreen toggle directly below the album art.
- **Borderless Custom UI:** Transparent, draggable custom title bar that blends into the backdrop and hides automatically in fullscreen.
- **Idle Animation:** Displays a custom looping GIF (`assets/idle.gif`) on a pure black background when nothing is playing on Spotify.
- **Multi-Monitor Memory:** Remembers the last window position across monitors (`window_pos.txt`) and reopens on the same display.
- **Zero-Console Release Binary:** Runs as a native Windows GUI app with custom `.exe` and taskbar iconography.

## Requirements

- A **Spotify Premium** account (required by the Spotify Web API for playback control).
- A free **Spotify Developer App** Client ID (takes approximately 2 minutes to set up).
- The **Rust** toolchain (`cargo`) if building from source.

## Spotify Setup

SpotiFrame authenticates via Spotify OAuth 2.0 PKCE directly in your browser—it never sees your password.

1. Open the [Spotify Developer Dashboard](https://developer.spotify.com/dashboard).
2. Click **Create app** (any name and description will do).
3. Under **Redirect URIs**, add:

   ```text
   http://127.0.0.1:8888/callback
   ```

   Make sure this exactly matches the `REDIRECT_URI` configured in `src/spotify.rs`.
4. Tick **Web API**, accept the terms, and click **Save**.
5. Copy your **Client ID**, paste it into SpotiFrame's login screen, and click **Sign in with Spotify**.

## Building from Source

1. Clone the repository:

   ```bash
   git clone https://github.com/SAPNXTDOOR/SpotiFrame.git
   cd SpotiFrame
   ```

2. Ensure the `assets/` folder contains the required files:

   ```text
   assets/
   ├── Inter-Regular.ttf
   ├── Inter-Bold.ttf
   ├── NotoEmoji.ttf       # optional fallback
   ├── idle.gif            # animation shown when nothing is playing
   ├── icon.ico            # Windows executable icon
   └── icon.png            # taskbar / window icon (256x256)
   ```

3. Build and run the release binary:

   ```bash
   cargo build --release
   ```

The compiled executable will be at `target/release/SpotiFrame.exe`. Keep the `assets/` folder alongside the `.exe` when moving or creating shortcuts.

## Command-Line Arguments & Shortcuts

### Launch Flags

Launch directly into fullscreen (useful for desktop shortcuts or startup scripts):

```bash
SpotiFrame.exe --fullscreen
```

Supported aliases: `-fullscreen`, `--fullscreen`, `-f`.

### Keyboard Shortcuts

| Key | Action |
|---|---|
| `F` or `F11` | Toggle fullscreen |
| `Esc` | Exit fullscreen |

## Tech Stack

- **GUI:** [eframe](https://crates.io/crates/eframe) / [egui](https://github.com/emilk/egui)
- **Networking:** [reqwest](https://crates.io/crates/reqwest) (Spotify Web API + LRCLIB)
- **Image & GIF Processing:** [image](https://crates.io/crates/image)
- **Windows Resources:** [winres](https://crates.io/crates/winres)

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.
'''

path = Path("/mnt/data/README.md")
path.write_text(readme, encoding="utf-8")
print(f"Created: {path}")
