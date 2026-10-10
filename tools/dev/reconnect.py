"""Dev tool, never shipped: drop Hearthstone's TCP connections on a hotkey.

T-D01, D-022, D-043. When collecting test games, dropping the game's
connection makes the client reconnect and skip ahead past combat animations.
Each press of the hotkey resets the open IPv4 TCP connections owned by
Hearthstone.exe (found by process id) with `SetTcpEntry(MIB_TCP_STATE_DELETE_TCB)`
from Windows' IP Helper API. Nothing is installed, no rule or setting is
changed, so nothing is left behind when the tool stops or crashes; the game
opens new connections by itself. No packet capture, no memory reading, no
injection, no change to game files, no other program's connections.

It needs an elevated terminal ("Run as administrator"); Windows refuses the
reset otherwise. Using it can carry risk for your Battle.net account: run it
only by hand, on your own account, to collect test games. Every press that
resets a connection is recorded (UTC time only) in
%APPDATA%\\TavernLedger\\dev-reconnects.jsonl so the tracker marks that game
and leaves it out of stats, uploads and fixtures.

Usage (from an elevated terminal; -I keeps user-writable Python paths out
of an elevated process, and the tool refuses to run elevated without it):
    python -I tools/dev/reconnect.py [--hotkey ctrl+alt+f9] [--cooldown 10]
        [--remote-port N ...] [--data-dir DIR]
    python tools/dev/reconnect.py --list           (no admin needed, drops nothing)
    python -I tools/dev/reconnect.py --self-test   (drops only a local test connection)

Stop it with Ctrl+C or by closing the window.
"""

from __future__ import annotations

import argparse
import ctypes
import ipaddress
import math
import os
import socket
import struct
import sys
import time
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Callable, Iterable, Protocol

# Run elevated from a user-writable folder: write no .pyc there, and find the
# sibling module explicitly (python -I does not add the script's folder).
sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))

import reconnect_marks  # noqa: E402

PROCESS_NAME = "Hearthstone.exe"
DEFAULT_HOTKEY = "ctrl+alt+f9"
DEFAULT_COOLDOWN = 10.0
MIN_COOLDOWN = 2.0
POLL_SECONDS = 0.05

# tcpmib.h, iprtrmib.h, winerror.h, winuser.h (Microsoft Learn; see
# docs/research/dev-reconnect.md).
MIB_TCP_STATE_ESTAB = 5
MIB_TCP_STATE_DELETE_TCB = 12
AF_INET = 2
AF_INET6 = 23
TCP_TABLE_OWNER_PID_CONNECTIONS = 4
NO_ERROR = 0
ERROR_INSUFFICIENT_BUFFER = 122
MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_NOREPEAT = 0x0001, 0x0002, 0x0004, 0x4000
WM_HOTKEY = 0x0312
PM_REMOVE = 0x0001
TH32CS_SNAPPROCESS = 0x00000002
LOAD_LIBRARY_SEARCH_SYSTEM32 = 0x00000800
HOTKEY_ID = 0x0031

BANNER = """\
================================================================
 Tavern Ledger DEV TOOL: reconnect (never part of the app)
 Drops Hearthstone's own TCP connections when you press the hotkey.
 It can carry RISK FOR YOUR BATTLE.NET ACCOUNT. Use it only by hand,
 on your own account, to collect test games. Games played with it
 are marked and never count in stats, uploads or fixtures.
================================================================"""

MODIFIERS = {"ctrl": MOD_CONTROL, "alt": MOD_ALT, "shift": MOD_SHIFT}


class UsageError(Exception):
    pass


def parse_hotkey(text: str) -> tuple[int, int]:
    """'ctrl+alt+f9' -> (modifiers, virtual-key code). Needs Ctrl or Alt so
    normal typing never fires it; F12 is reserved by Windows; no Win key."""
    parts = [p.strip().lower() for p in text.split("+") if p.strip()]
    if len(parts) < 2:
        raise UsageError("a hotkey is modifiers plus one key, e.g. ctrl+alt+f9")
    *mods, key = parts
    flags = 0
    for mod in mods:
        if mod not in MODIFIERS:
            raise UsageError(f"unknown modifier {mod!r} (use ctrl, alt, shift)")
        flags |= MODIFIERS[mod]
    if not flags & (MOD_CONTROL | MOD_ALT):
        raise UsageError("the hotkey needs ctrl or alt")
    return flags, virtual_key(key)


def virtual_key(key: str) -> int:
    if len(key) == 1 and ("a" <= key <= "z" or "0" <= key <= "9"):
        return ord(key.upper())
    number = key[1:]
    if key.startswith("f") and number.isascii() and number.isdigit() and 1 <= int(number) <= 11:
        return 0x70 + int(key[1:]) - 1  # VK_F1 = 0x70
    raise UsageError(f"key {key!r} not supported (use a-z, 0-9 or f1-f11; f12 is reserved)")


@dataclass(frozen=True)
class Row:
    """One IPv4 TCP connection; ports in host order, raw fields kept for
    SetTcpEntry."""

    state: int
    local: ipaddress.IPv4Address
    local_port: int
    remote: ipaddress.IPv4Address
    remote_port: int
    pid: int
    raw: tuple[int, int, int, int]  # dwLocalAddr, dwLocalPort, dwRemoteAddr, dwRemotePort


def row_from_raw(state: int, local: int, lport: int, remote: int, rport: int, pid: int) -> Row:
    def addr(v: int) -> ipaddress.IPv4Address:
        # Stored like in_addr: the DWORD's bytes in memory are the address.
        return ipaddress.IPv4Address(struct.pack("<I", v))

    def port(v: int) -> int:
        # Network byte order in the low 16 bits; the high bits may be junk.
        return socket.ntohs(v & 0xFFFF)

    return Row(state, addr(local), port(lport), addr(remote), port(rport), pid,
               (local, lport, remote, rport))


def game_targets(rows: Iterable[Row], pids: set[int], remote_ports: set[int] | None) -> list[Row]:
    """Open connections of the game's processes to another computer."""
    return [
        r for r in rows
        if r.pid in pids
        and r.state == MIB_TCP_STATE_ESTAB
        and not r.remote.is_loopback
        and not r.remote.is_unspecified
        and (not remote_ports or r.remote_port in remote_ports)
    ]


class Api(Protocol):
    def find_pids(self, name: str) -> set[int]: ...
    def tcp_rows(self) -> list[Row]: ...
    def count_ipv6(self, pids: set[int]) -> int: ...
    def delete(self, row: Row) -> int: ...


@dataclass
class DropResult:
    running: bool
    dropped: int = 0
    failed: list[int] | None = None  # error codes
    ipv6_left: int = 0
    recorded: Path | None = None

    def message(self) -> str:
        if not self.running:
            return f"{PROCESS_NAME} is not running; nothing dropped, nothing recorded."
        if self.recorded is None:
            return "No open connection to drop; nothing recorded."
        text = f"Dropped {self.dropped} connection(s)."
        if self.failed:
            text += f" {len(self.failed)} could not be dropped (error {', '.join(map(str, self.failed))})."
        if self.ipv6_left:
            text += f" {self.ipv6_left} IPv6 connection(s) left alone (Windows has no reset for them)."
        return text


def drop_once(api: Api, data_dir: Path, remote_ports: set[int] | None,
              now: Callable[[], datetime] = lambda: datetime.now(timezone.utc)) -> DropResult:
    """Resets the game's connections once. The use is recorded before the
    reset, so even a crash in between leaves the game marked."""
    pids = api.find_pids(PROCESS_NAME)
    if not pids:
        return DropResult(running=False)
    targets = game_targets(api.tcp_rows(), pids, remote_ports)
    ipv6 = api.count_ipv6(pids)
    if not targets:
        return DropResult(running=True, ipv6_left=ipv6)
    recorded = reconnect_marks.append_use(data_dir, now())
    codes = [api.delete(row) for row in targets]
    failed = [c for c in codes if c != NO_ERROR]
    return DropResult(True, len(codes) - len(failed), failed, ipv6, recorded)


class Cooldown:
    def __init__(self, seconds: float, clock: Callable[[], float] = time.monotonic):
        self.seconds = seconds
        self.clock = clock
        self.last: float | None = None

    def left(self) -> float:
        """Seconds until the next press counts; 0 when ready."""
        if self.last is None:
            return 0.0
        return max(0.0, self.last + self.seconds - self.clock())

    def use(self) -> None:
        self.last = self.clock()


# --- Windows calls (ctypes, declared by hand; Microsoft Learn pages in
# docs/research/dev-reconnect.md) ---

class Win32:
    def __init__(self) -> None:
        from ctypes import wintypes

        self.wt = wintypes

        def system_dll(name: str) -> ctypes.WinDLL:
            # System32 only: never a same-named DLL planted next to Python.
            return ctypes.WinDLL(name, use_last_error=True, winmode=LOAD_LIBRARY_SEARCH_SYSTEM32)

        self.iphlpapi = system_dll("iphlpapi")
        self.kernel32 = system_dll("kernel32")
        self.advapi32 = system_dll("advapi32")
        self.user32 = system_dll("user32")
        self._declare()

    def _declare(self) -> None:
        wt = self.wt
        self.iphlpapi.GetExtendedTcpTable.argtypes = [
            ctypes.c_void_p, ctypes.POINTER(wt.DWORD), wt.BOOL, wt.ULONG, ctypes.c_int, wt.ULONG]
        self.iphlpapi.GetExtendedTcpTable.restype = wt.DWORD
        self.iphlpapi.SetTcpEntry.argtypes = [ctypes.POINTER(MibTcpRow)]
        self.iphlpapi.SetTcpEntry.restype = wt.DWORD
        self.kernel32.CreateToolhelp32Snapshot.argtypes = [wt.DWORD, wt.DWORD]
        self.kernel32.CreateToolhelp32Snapshot.restype = ctypes.c_void_p
        for name in ("Process32FirstW", "Process32NextW"):
            fn = getattr(self.kernel32, name)
            fn.argtypes = [ctypes.c_void_p, ctypes.POINTER(ProcessEntry32W)]
            fn.restype = wt.BOOL
        self.kernel32.CloseHandle.argtypes = [ctypes.c_void_p]
        self.kernel32.CloseHandle.restype = wt.BOOL
        self.advapi32.AllocateAndInitializeSid.argtypes = [
            ctypes.POINTER(SidIdentifierAuthority), ctypes.c_ubyte] + [wt.DWORD] * 8 + [
            ctypes.POINTER(ctypes.c_void_p)]
        self.advapi32.AllocateAndInitializeSid.restype = wt.BOOL
        self.advapi32.CheckTokenMembership.argtypes = [
            ctypes.c_void_p, ctypes.c_void_p, ctypes.POINTER(wt.BOOL)]
        self.advapi32.CheckTokenMembership.restype = wt.BOOL
        self.advapi32.FreeSid.argtypes = [ctypes.c_void_p]
        self.advapi32.FreeSid.restype = ctypes.c_void_p
        self.user32.RegisterHotKey.argtypes = [wt.HWND, ctypes.c_int, wt.UINT, wt.UINT]
        self.user32.RegisterHotKey.restype = wt.BOOL
        self.user32.UnregisterHotKey.argtypes = [wt.HWND, ctypes.c_int]
        self.user32.UnregisterHotKey.restype = wt.BOOL
        self.user32.PeekMessageW.argtypes = [
            ctypes.POINTER(wt.MSG), wt.HWND, wt.UINT, wt.UINT, wt.UINT]
        self.user32.PeekMessageW.restype = wt.BOOL

    def is_admin(self) -> bool:
        """Member of Administrators with the group enabled (an elevated
        token), as in Microsoft's CheckTokenMembership example."""
        nt_authority = SidIdentifierAuthority((0, 0, 0, 0, 0, 5))
        sid = ctypes.c_void_p()
        if not self.advapi32.AllocateAndInitializeSid(
                ctypes.byref(nt_authority), 2, 0x20, 0x220, 0, 0, 0, 0, 0, 0, ctypes.byref(sid)):
            return False
        try:
            member = self.wt.BOOL(False)
            ok = self.advapi32.CheckTokenMembership(None, sid, ctypes.byref(member))
            return bool(ok and member.value)
        finally:
            self.advapi32.FreeSid(sid)

    def find_pids(self, name: str) -> set[int]:
        """Process ids by executable name; reads the process list only."""
        snapshot = self.kernel32.CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)
        if snapshot is None or snapshot == ctypes.c_void_p(-1).value:
            raise OSError(ctypes.get_last_error(), "CreateToolhelp32Snapshot failed")
        try:
            entry = ProcessEntry32W()
            entry.dwSize = ctypes.sizeof(ProcessEntry32W)
            pids = set()
            ok = self.kernel32.Process32FirstW(snapshot, ctypes.byref(entry))
            while ok:
                if entry.szExeFile.lower() == name.lower():
                    pids.add(entry.th32ProcessID)
                ok = self.kernel32.Process32NextW(snapshot, ctypes.byref(entry))
            return pids
        finally:
            self.kernel32.CloseHandle(snapshot)

    def _table(self, family: int) -> ctypes.Array:
        size = self.wt.DWORD(0)
        for _ in range(5):  # the table can grow between the two calls
            buf = ctypes.create_string_buffer(max(size.value, 4))
            size = self.wt.DWORD(len(buf))
            err = self.iphlpapi.GetExtendedTcpTable(
                buf, ctypes.byref(size), False, family, TCP_TABLE_OWNER_PID_CONNECTIONS, 0)
            if err == NO_ERROR:
                return buf
            if err != ERROR_INSUFFICIENT_BUFFER:
                raise OSError(err, "GetExtendedTcpTable failed")
        raise OSError(ERROR_INSUFFICIENT_BUFFER, "GetExtendedTcpTable kept growing")

    def tcp_rows(self) -> list[Row]:
        buf = self._table(AF_INET)
        count = ctypes.c_uint32.from_buffer(buf).value
        rows = (MibTcpRowOwnerPid * count).from_buffer(buf, 4)
        return [row_from_raw(r.dwState, r.dwLocalAddr, r.dwLocalPort, r.dwRemoteAddr,
                             r.dwRemotePort, r.dwOwningPid) for r in rows]

    def count_ipv6(self, pids: set[int]) -> int:
        buf = self._table(AF_INET6)
        count = ctypes.c_uint32.from_buffer(buf).value
        rows = (MibTcp6RowOwnerPid * count).from_buffer(buf, 4)
        return sum(1 for r in rows if r.dwOwningPid in pids and r.dwState == MIB_TCP_STATE_ESTAB)

    def delete(self, row: Row) -> int:
        local, lport, remote, rport = row.raw
        target = MibTcpRow(MIB_TCP_STATE_DELETE_TCB, local, lport, remote, rport)
        return self.iphlpapi.SetTcpEntry(ctypes.byref(target))

    def register_hotkey(self, mods: int, vk: int) -> None:
        if not self.user32.RegisterHotKey(None, HOTKEY_ID, mods | MOD_NOREPEAT, vk):
            raise OSError(ctypes.get_last_error(), "RegisterHotKey failed (is the key taken?)")

    def unregister_hotkey(self) -> None:
        self.user32.UnregisterHotKey(None, HOTKEY_ID)

    def hotkey_pressed(self) -> bool:
        msg = self.wt.MSG()
        found = False
        while self.user32.PeekMessageW(ctypes.byref(msg), None, WM_HOTKEY, WM_HOTKEY, PM_REMOVE):
            found = found or msg.wParam == HOTKEY_ID
        return found


DWORD = ctypes.c_uint32


class MibTcpRow(ctypes.Structure):
    _fields_ = [("dwState", DWORD), ("dwLocalAddr", DWORD), ("dwLocalPort", DWORD),
                ("dwRemoteAddr", DWORD), ("dwRemotePort", DWORD)]


class MibTcpRowOwnerPid(ctypes.Structure):
    _fields_ = MibTcpRow._fields_ + [("dwOwningPid", DWORD)]


class MibTcp6RowOwnerPid(ctypes.Structure):
    _fields_ = [("ucLocalAddr", ctypes.c_ubyte * 16), ("dwLocalScopeId", DWORD),
                ("dwLocalPort", DWORD), ("ucRemoteAddr", ctypes.c_ubyte * 16),
                ("dwRemoteScopeId", DWORD), ("dwRemotePort", DWORD), ("dwState", DWORD),
                ("dwOwningPid", DWORD)]


class SidIdentifierAuthority(ctypes.Structure):
    _fields_ = [("Value", ctypes.c_ubyte * 6)]


class ProcessEntry32W(ctypes.Structure):
    _fields_ = [("dwSize", DWORD), ("cntUsage", DWORD), ("th32ProcessID", DWORD),
                ("th32DefaultHeapID", ctypes.c_size_t), ("th32ModuleID", DWORD),
                ("cntThreads", DWORD), ("th32ParentProcessID", DWORD),
                ("pcPriClassBase", ctypes.c_long), ("dwFlags", DWORD),
                ("szExeFile", ctypes.c_wchar * 260)]


# --- commands ---

def list_connections(api: Api) -> int:
    pids = api.find_pids(PROCESS_NAME)
    if not pids:
        print(f"{PROCESS_NAME} is not running.")
        return 0
    rows = [r for r in api.tcp_rows() if r.pid in pids]
    print(f"{PROCESS_NAME}: {len(rows)} IPv4 TCP connection(s), "
          f"{api.count_ipv6(pids)} open IPv6 (never touched)")
    for r in rows:
        kind = "open" if r.state == MIB_TCP_STATE_ESTAB else f"state {r.state}"
        where = "this PC" if r.remote.is_loopback else "remote"
        print(f"  {kind:>9}  {where:<7} port {r.remote_port}")
    targets = game_targets(rows, pids, None)
    print(f"A press would drop {len(targets)} of them (narrow it with --remote-port).")
    return 0


def self_test(api: Win32) -> int:
    """Opens two local connections in this process and drops one of them:
    proves the reset works and touches nothing else. Records nothing."""
    with socket.socket() as server:
        server.bind(("127.0.0.1", 0))
        server.listen(2)
        port = server.getsockname()[1]
        a, b = socket.create_connection(("127.0.0.1", port)), socket.create_connection(("127.0.0.1", port))
        a_peer, b_peer = server.accept()[0], server.accept()[0]
        sockets = [a, b, a_peer, b_peer]
        try:
            for s in sockets:
                s.settimeout(2)
            a_port = a.getsockname()[1]
            mine = [r for r in api.tcp_rows()
                    if r.pid == os.getpid() and r.local_port == a_port and r.remote_port == port]
            if len(mine) != 1:
                print(f"Self-test failed: found {len(mine)} rows for the test connection.")
                return 1
            code = api.delete(mine[0])
            if code != NO_ERROR:
                print(f"Self-test failed: SetTcpEntry returned {code}.")
                return 1
            a_dropped = _is_dropped(a)
            b.sendall(b"ok")
            b_alive = b_peer.recv(2) == b"ok"
            left = [r for r in api.tcp_rows() if r.pid == os.getpid() and r.local_port == a_port
                    and r.state == MIB_TCP_STATE_ESTAB]
        finally:
            for s in sockets:
                s.close()
    print(f"Dropped connection reset: {a_dropped}; other connection still works: {b_alive}; "
          f"dropped connection still open: {bool(left)}")
    return 0 if a_dropped and b_alive and not left else 1


def _is_dropped(sock: socket.socket) -> bool:
    try:
        sock.sendall(b"x")
        return sock.recv(1) == b""
    except (ConnectionResetError, ConnectionAbortedError):
        return True
    except OSError:
        return False


def run_hotkey(api: Win32, mods: int, vk: int, cooldown: Cooldown, data_dir: Path,
               remote_ports: set[int] | None, hotkey: str) -> int:
    api.register_hotkey(mods, vk)
    print(f"Ready. Press {hotkey} to drop Hearthstone's connections once "
          f"(cooldown {cooldown.seconds:g} s). Ctrl+C here to stop.")
    try:
        while True:
            if api.hotkey_pressed():
                on_press(api, cooldown, data_dir, remote_ports)
            time.sleep(POLL_SECONDS)
    except KeyboardInterrupt:
        print("Stopped. Nothing to undo: the tool changed no setting.")
        return 0
    finally:
        api.unregister_hotkey()


def on_press(api: Api, cooldown: Cooldown, data_dir: Path, remote_ports: set[int] | None) -> None:
    """One hotkey press. The cooldown starts only when something was dropped;
    an error is reported and the tool keeps waiting."""
    left = cooldown.left()
    if left > 0:
        print(f"Cooldown: {left:.0f} s left; ignored.")
        return
    try:
        result = drop_once(api, data_dir, remote_ports)
    except OSError as e:
        print(f"{datetime.now():%H:%M:%S} Nothing dropped: {e}")
        return
    if result.recorded is not None:
        cooldown.use()
    print(f"{datetime.now():%H:%M:%S} {result.message()}")


def is_isolated() -> bool:
    return bool(sys.flags.isolated)


def history_missing(data_dir: Path) -> bool:
    """No tracker history in the default data folder: likely an elevated
    terminal of another account, whose %APPDATA% the tracker never reads."""
    return not (data_dir / "games.jsonl").is_file()


def parse_args(argv: list[str] | None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--list", action="store_true", help="show the game's connections, drop nothing")
    mode.add_argument("--self-test", action="store_true", help="drop a local test connection only")
    parser.add_argument("--hotkey", default=DEFAULT_HOTKEY)
    parser.add_argument("--cooldown", type=float, default=DEFAULT_COOLDOWN, help="seconds between drops")
    parser.add_argument("--remote-port", type=int, action="append",
                        help="only drop connections to this remote port (repeatable)")
    parser.add_argument("--data-dir", type=Path, help="default: %%APPDATA%%\\TavernLedger")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    if sys.platform != "win32":
        print("This tool only runs on Windows.", file=sys.stderr)
        return 2
    try:
        mods, vk = parse_hotkey(args.hotkey)
        if not (math.isfinite(args.cooldown) and args.cooldown >= MIN_COOLDOWN):
            raise UsageError(f"--cooldown must be a number of seconds, at least {MIN_COOLDOWN:g}")
        if args.remote_port and not all(0 < p < 65536 for p in args.remote_port):
            raise UsageError("--remote-port must be 1-65535")
    except UsageError as e:
        print(f"Error: {e}", file=sys.stderr)
        return 2
    try:
        return run(args, mods, vk)
    except OSError as e:
        print(f"Error: {e}", file=sys.stderr)
        return 2


def run(args: argparse.Namespace, mods: int, vk: int) -> int:
    if not args.list and not is_isolated():
        print("Run it as 'python -I tools/dev/reconnect.py ...': an elevated Python must not "
              "load code from user-writable paths. Nothing was done.", file=sys.stderr)
        return 2
    api = Win32()
    if args.list:
        return list_connections(api)
    if not api.is_admin():
        print("This tool needs an elevated terminal: open the terminal with "
              "'Run as administrator' and start it again. Nothing was done.", file=sys.stderr)
        return 2
    if args.self_test:
        return self_test(api)
    data_dir = args.data_dir or reconnect_marks.default_data_dir()
    if args.data_dir is None and history_missing(data_dir):
        print(f"No Tavern Ledger history in {data_dir}. If this elevated terminal belongs to another "
              "account, its uses would never reach your tracker: pass --data-dir with your own "
              "%APPDATA%\\TavernLedger. Nothing was done.", file=sys.stderr)
        return 2
    print(BANNER)
    print(f"Uses are recorded in {data_dir / reconnect_marks.FILE_NAME} (UTC time only).")
    remote_ports = set(args.remote_port) if args.remote_port else None
    return run_hotkey(api, mods, vk, Cooldown(args.cooldown), data_dir, remote_ports, args.hotkey)


if __name__ == "__main__":
    sys.exit(main())
