# Dev tools

Never shipped: not in the app, the installer, releases or user docs. For collecting test games and debugging on the developer's own PC.

| Tool | What it does |
|------|--------------|
| `replay_log.py` | Replays a saved `Power.log` into a temporary folder so the app can be tested without playing (T-D02). |
| `reconnect.py` | Drops Hearthstone's own TCP connections on a hotkey, to skip combat animations while collecting test games (T-D01, D-022, D-043). |
| `reconnect_marks.py` | Reads the record of `reconnect.py` uses; fixture tools call it to refuse logs with games played with the tool. |

## Reconnect (`reconnect.py`)

**Risk.** Dropping the game's connection changes how the client behaves on the network. It can carry risk for your Battle.net account. Use it only by hand, on your own account, to collect test games, knowing that. It is never automatic: no timers, nothing triggered by the log.

**What it does.** While it runs, each press of the hotkey (default `Ctrl+Alt+F9`, at most once every 10 seconds) resets every open IPv4 TCP connection owned by `Hearthstone.exe`, found by process id, with Windows' `SetTcpEntry`. The client notices and reconnects. Nothing else is touched: no other program's connections, no firewall rule, no setting, no game file, no memory, no packet capture. Method and Microsoft sources: [docs/research/dev-reconnect.md](../../docs/research/dev-reconnect.md).

**Admin.** Windows only lets an elevated process reset connections. Open the terminal with "Run as administrator"; without it the tool says so and does nothing.

```
python -I tools/dev/reconnect.py                       # wait for Ctrl+Alt+F9
python -I tools/dev/reconnect.py --hotkey ctrl+shift+r --cooldown 20
python tools/dev/reconnect.py --list                   # no admin: show the game's connections, drop nothing
python -I tools/dev/reconnect.py --remote-port 3724    # only connections to that remote port
python -I tools/dev/reconnect.py --self-test           # admin: drop a local test connection only
```

`-I` (isolated mode) is required when elevated: it keeps user-writable Python paths out of an admin process. Without `--data-dir`, the tool refuses a data folder that has no Tavern Ledger history (for example an elevated terminal of another account, whose uses your tracker would never see). A hotkey needs Ctrl or Alt plus a letter, a digit or F1-F11 (F12 is reserved by Windows). IPv6 connections cannot be reset this way; the tool counts them and leaves them alone.

**Marks.** Every press that drops something first appends one line to `%APPDATA%\TavernLedger\dev-reconnects.jsonl`: `{"utc": "2026-10-10T19:15:03Z"}`, the UTC time and nothing else. The tracker marks every game whose time (from the log's own clock) holds one of those lines, give or take 30 seconds, and each game also owns the gaps before and after it (a use between two games marks both): the history shows "Dev reconnect" on it, and the stats, hero stats and upload leave it out; while the file cannot be read at all, nothing is uploaded. `reconnect_marks.py` makes fixture tools refuse such logs:

```
python tools/dev/reconnect_marks.py "<Logs>\Hearthstone_<date>\Power_old.log"
```

Keep that file: deleting it removes the marks.

**How to undo.** There is nothing to undo: the tool changes no setting, so stopping it (Ctrl+C or closing the window), or a crash, leaves nothing behind. The game opens new connections by itself. If the game stays disconnected, restart it.
