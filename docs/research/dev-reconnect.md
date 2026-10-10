# Reconnect dev tool: method (T-D01)

Session 031, 2026-10-10. How `tools/dev/reconnect.py` drops Hearthstone's connection, designed from Microsoft's documentation only. No other tracker's code was read (clean-room, absolute rule 1): the GitHub survey ([github-landscape.md](github-landscape.md)) only told us such tools exist and need admin rights.

## Requirements (D-022, D-035)

- Affect only `Hearthstone.exe`'s connections, by process id; never another program's.
- Leave nothing behind: no rule or setting to restore, also after a crash or Ctrl+C.
- No packet capture, no memory reading, no injection, no change to game files (D-004, D-006).
- Standard library and `ctypes` only.

## Options looked at

| Method | Only the game? | Left behind on a crash | Verdict |
|--------|----------------|------------------------|---------|
| `SetTcpEntry` with `MIB_TCP_STATE_DELETE_TCB` (IP Helper) | Yes: the rows come with their owning process id (`GetExtendedTcpTable`), and only those rows are reset | Nothing: the connection is gone and the game opens a new one; no state was changed | **Chosen** |
| Windows Firewall rule blocking the game's program for a moment (`netsh advfirewall` or the firewall COM API) | Yes (rule per program path) | The rule, if the tool dies before removing it: the game stays offline until someone deletes it | Rejected: needs clean-up to be safe |
| Disabling the network adapter | No: every program loses the network | The adapter stays off on a crash | Rejected |

## How it works

1. **Admin check.** `CheckTokenMembership` with the Administrators SID (built with `AllocateAndInitializeSid`, as in Microsoft's example). Under UAC a non-elevated token has the group for deny only, so the check is false unless the terminal was "Run as administrator", which is what `SetTcpEntry` needs.
2. **Find the game.** `CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS)` and `Process32FirstW`/`Process32NextW`: the process list only (executable name and id). No handle is opened on the game's process.
3. **Find its connections.** `GetExtendedTcpTable(AF_INET, TCP_TABLE_OWNER_PID_CONNECTIONS)` gives `MIB_TCPROW_OWNER_PID` rows (state, addresses, ports in network byte order in the low 16 bits, owning pid). Targets: rows of the game's pids, state `ESTAB` (5), remote address not loopback or `0.0.0.0`, optionally only some remote ports (`--remote-port`).
4. **Drop.** For each target, `SetTcpEntry` with a `MIB_TCPROW` holding the row's addresses and ports and state `MIB_TCP_STATE_DELETE_TCB` (12), "currently the only state to which a TCP connection can be set". Returns 0, `ERROR_ACCESS_DENIED` (5) or 317 when not elevated.
5. **IPv6.** There is no `SetTcpEntry` for IPv6. The tool counts the game's open IPv6 connections (`AF_INET6`, `MIB_TCP6ROW_OWNER_PID`) and says it left them alone.
6. **Hotkey.** `RegisterHotKey(NULL, id, MOD_CONTROL | MOD_ALT | MOD_NOREPEAT, VK_F9)` posts `WM_HOTKEY` to the tool's own thread, read with `PeekMessageW` in a short sleep loop so Ctrl+C still stops it; `UnregisterHotKey` on the way out (Windows also frees it when the process ends). F12 is reserved by Windows and refused; a hotkey needs Ctrl or Alt.

Which connection is the game server? The documentation cannot say, and reading the game's traffic is ruled out. So a press drops every open IPv4 connection of the game (the client reconnects them), and `--list` (no admin) shows the remote ports so `--remote-port` can narrow it after a test.

## Marking games (D-043)

Each press that drops something appends `{"utc": "..."}` to `%APPDATA%\TavernLedger\dev-reconnects.jsonl` **before** the reset. The tracker gives every game a span from the log's own clock (session folder name for the date, each line's time of day, midnight roll-over), turned into UTC with `TzSpecificLocalTimeToSystemTime` (null time zone = the active one, daylight saving included), and marks a game whose span, plus 30 seconds each side, holds a use. Known limit from the same page: a local time in the hour repeated when daylight saving ends is ambiguous, and Windows treats it as daylight time; a game there could be checked one hour off.

## Sources (Microsoft Learn, read 2026-10-10)

- SetTcpEntry: https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-settcpentry
- MIB_TCPROW: https://learn.microsoft.com/en-us/windows/win32/api/tcpmib/ns-tcpmib-mib_tcprow_lh
- GetExtendedTcpTable: https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getextendedtcptable
- MIB_TCPROW_OWNER_PID: https://learn.microsoft.com/en-us/windows/win32/api/tcpmib/ns-tcpmib-mib_tcprow_owner_pid
- MIB_TCP6ROW_OWNER_PID: https://learn.microsoft.com/en-us/windows/win32/api/tcpmib/ns-tcpmib-mib_tcp6row_owner_pid
- CreateToolhelp32Snapshot: https://learn.microsoft.com/en-us/windows/win32/api/tlhelp32/nf-tlhelp32-createtoolhelp32snapshot
- PROCESSENTRY32W: https://learn.microsoft.com/en-us/windows/win32/api/tlhelp32/ns-tlhelp32-processentry32w
- CheckTokenMembership: https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-checktokenmembership
- RegisterHotKey: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-registerhotkey
- TzSpecificLocalTimeToSystemTime: https://learn.microsoft.com/en-us/windows/win32/api/timezoneapi/nf-timezoneapi-tzspecificlocaltimetosystemtime

The calendar arithmetic in `crates/tracker/src/game_clock.rs` (`days_from_civil`, `civil_from_days`) follows Howard Hinnant's public-domain date algorithms (http://howardhinnant.github.io/date_algorithms.html).
