# Desktop app stack (T-005, P-002)

Date: 2026-10-09. Report to decide P-002. **Decided: Tauri 2 (D-014).**

## What the app has to do

It comes from the F1-F3 tasks in the plan and the F0 prototypes:

1. **Follow `Power.log` while it grows** and rebuild each game (T-101). The logic fits in under 500 lines of Python (`tools/parse_bg.py`) and is rewritten from [`parser-hslog.md`](parser-hslog.md), without `hslog` or other trackers' code.
2. **Store games locally** (SQLite or files) and show stats (T-102).
3. **Query the leaderboard** with cache and limits (D-012, D-013); the prototype is `tools/leaderboard.py`.
4. **Upload games** to the web once there are accounts (T-104).
5. **Installer with auto-update and signing** (T-105, P-005: SignPath Foundation first).
6. **Minimal overlay** (T-301): transparent, always-on-top window that lets clicks through where there is nothing.
7. Windows only. Maintainable by one person with Claude Code.

## Options

| | Rust + Tauri 2 | C# / .NET (WPF or Avalonia) | Electron |
|---|---|---|---|
| Installer size | A few MB: WebView2 ships with Windows 10 (April 2018+) and 11; the default installer only downloads the runtime if it is missing (0 MB extra) [1] | Avalonia with Native AOT: ~18 MB according to a community template, no official measurement [3] | 100-150 MB for a minimal app; almost all of it is Chromium [4] |
| Overlay | `transparent` when creating the window, `set_always_on_top` and `set_ignore_cursor_events` in the Tauri 2.12 API [2]. Untested: transparent WebView2 has had painting bugs | WPF: the most proven on Windows. Avalonia: full transparency on Windows only; to let clicks through you must set `Background="{x:Null}"` [3] | `setIgnoreMouseEvents` and transparent windows, widely used |
| UI | HTML/TS: can be shared with the web (P-003) | XAML; nothing shared with the web | HTML/TS: shared with the web |
| Logic (parser, leaderboard) | Rust: strict types, native binary, tests with `cargo test` | C#: strict types, `dotnet test` | TS/Node |
| Auto-update | Official `updater` plugin, with its own signing of updates | Velopack or other (external dependency) | `electron-updater` |
| Licenses for SignPath | MIT/Apache-2.0; WebView2 is a system library | .NET and Avalonia MIT | Chromium and Electron BSD/MIT |
| Tools on the machine | `cargo` 1.96 and Node 24 installed | No .NET SDK | Node 24 installed |

SignPath Foundation requires all components to have an OSI license, allows system libraries, requires an automated build verifiable from the repo (CI) and, for executables, "some verifiable reputation" of the project [5]. All three options meet the license requirement; reputation weighs the same on all.

## Recommendation

**Tauri 2 (logic in Rust, UI in TypeScript).**

- Small installer and official auto-update with signing, which fits T-105.
- The TS UI can be reused on the web (P-003) for the game viewer and recaps (F2), instead of building it twice.
- The parser and the leaderboard client are pure logic, easy to port to Rust with tests from the prototypes and their synthetic tests.
- Rust and Node are already installed; .NET is not.

**Main risk: the overlay.** Tested on 2026-10-09 with a minimal window, first over another window and then on top of Hearthstone in full screen (below): it works. Plan B if something fails later: WPF only for the overlay or for the whole app.

## Overlay test

Code in [`spikes/overlay-tauri/`](../../spikes/overlay-tauri/): Tauri 2.12.2, no npm (static HTML), a borderless window, `transparent`, `alwaysOnTop`, outside the taskbar and with `set_ignore_cursor_events(true)`. It does not touch the game or its files.

[`check_overlay.py`](../../spikes/overlay-tauri/check_overlay.py) opens a Tk window underneath, reads the overlay's Win32 styles, does hit testing and a real click on the panel (returns the mouse to its place):

| Check | Debug | Release |
|-------|-------|---------|
| `WS_EX_TOPMOST` (always on top) | OK | OK |
| `WS_EX_LAYERED` + `WS_EX_TRANSPARENT` (receives no mouse) | OK | OK |
| `WindowFromPoint` over the panel returns the window underneath | OK | OK |
| The click on the panel reaches the window underneath | OK | OK |

The screenshot shows the background of the window underneath around the panel and through its semi-transparent background: WebView2 paints transparency correctly on Windows 11.

- **Size:** the release `.exe` takes 8.5 MB with no installer or compression.
- **Build:** 1 min (debug) and 1 min 16 s (release) from scratch; 407 crates in `Cargo.lock`, all with OSI licenses (MIT, Apache-2.0, Zlib, BSD, ISC, Unicode-3.0 and 5 with MPL-2.0, used unmodified).
- **Locking and unlocking panels** (the user's idea for the future, T-302): `set_ignore_cursor_events` is a runtime switch, so "unlocked" can receive the mouse for dragging and "locked" can let clicks through. Not tested yet.
### On top of Hearthstone

With the user's OK, Claude opened the game from Battle.net (`--exec="launch WTCG"`) and left it at the main menu. It sent no clicks or keys (D-004); it only brought it to the foreground with `SetForegroundWindow`, which is not an action inside the game. The game opened with its usual setup: **borderless** full screen (a frameless window filling the monitor's 3840×2160).

| Check | Result |
|-------|--------|
| Hearthstone in the foreground and full screen | OK |
| The overlay stays always on top and receives no mouse | OK |
| `WindowFromPoint` over the panel returns the game window | OK |
| Screenshot: the panel shows on top of the game menu | OK |

- **Real click on the game:** not tested so as not to send input to the game; the hit testing already shows the click would go to the game.
- **Windowed mode:** untested (changing it means touching the game menu). It is the easy case: the overlay goes on top of any normal window, as in the first test.
- **Scale:** at 4K the 420×260 logical px panel looks small. The real overlay will have to scale with the game's resolution.

Electron is ruled out unless Tauri fails: it does the same with an installer 20-50 times larger.

## Proposed next steps

1. The user decides P-002. If Tauri wins: T-101 with the parser ported to Rust and the synthetic tests in `tests/` as reference.
3. New dependencies (Tauri crates, npm packages) are pinned with a lockfile and recorded in `PROVENANCE.md` (R2: `/security-review`).

## Sources

1. Tauri, [Windows Installer](https://v2.tauri.app/distribute/windows-installer/) (WebView2 modes and the size they add).
2. Tauri, [`Window` on docs.rs](https://docs.rs/tauri/latest/tauri/window/struct.Window.html) (version 2.12.2).
3. Avalonia, [How to: Work with Windows](https://docs.avaloniaui.net/docs/how-to/window-how-to) and [Native AOT](https://docs.avaloniaui.net/docs/deployment/native-aot); template [AvaloniaAOT](https://github.com/lixinyang123/AvaloniaAOT) (~18 MB, community figure).
4. [Electron Packager](https://packages.electronjs.org/packager) (the prebuilt binary sets the minimum) and third-party measurements of 115-151 MB; no official figure.
5. SignPath Foundation, [terms](https://signpath.org/terms).
