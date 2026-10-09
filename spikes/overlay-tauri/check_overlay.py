"""Check the overlay spike on Windows: styles, hit testing and a real click.

Starts the spike, puts a Tk window under it, clicks on the overlay's panel and
checks that the click reaches the Tk window. Restores the mouse position.
Only reads window state and clicks on its own windows; never touches the game.

    python check_overlay.py [--exe PATH] [--screenshot PATH]
"""
import argparse
import ctypes
import ctypes.wintypes as wt
import pathlib
import subprocess
import sys
import time
import tkinter as tk

TITLE = "Tavern Ledger overlay spike"
GWL_EXSTYLE = -20
WS_EX_TOPMOST = 0x00000008
WS_EX_TRANSPARENT = 0x00000020
WS_EX_LAYERED = 0x00080000
MOUSEEVENTF_LEFTDOWN = 0x0002
MOUSEEVENTF_LEFTUP = 0x0004
GA_ROOT = 2
# Fraction of the overlay window that falls inside the HTML panel.
PANEL_FRACTION = 0.15
WAIT_SECONDS = 20

ctypes.windll.shcore.SetProcessDpiAwareness(2)  # physical pixels everywhere
user32 = ctypes.windll.user32
user32.FindWindowW.restype = wt.HWND
user32.WindowFromPoint.argtypes = [wt.POINT]
user32.WindowFromPoint.restype = wt.HWND
user32.GetAncestor.restype = wt.HWND
user32.GetWindowLongW.restype = ctypes.c_long


def find_overlay():
    deadline = time.monotonic() + WAIT_SECONDS
    while time.monotonic() < deadline:
        hwnd = user32.FindWindowW(None, TITLE)
        if hwnd:
            return hwnd
        time.sleep(0.2)
    return None


def root_at(x, y):
    hwnd = user32.WindowFromPoint(wt.POINT(x, y))
    return user32.GetAncestor(hwnd, GA_ROOT) if hwnd else None


def click(x, y):
    old = wt.POINT()
    user32.GetCursorPos(ctypes.byref(old))
    user32.SetCursorPos(x, y)
    user32.mouse_event(MOUSEEVENTF_LEFTDOWN, 0, 0, 0, 0)
    user32.mouse_event(MOUSEEVENTF_LEFTUP, 0, 0, 0, 0)
    time.sleep(0.1)
    user32.SetCursorPos(old.x, old.y)


def window_rect(hwnd):
    rect = wt.RECT()
    user32.GetWindowRect(hwnd, ctypes.byref(rect))
    return rect


def screenshot(path, rect):
    script = (
        "Add-Type -AssemblyName System.Drawing;"
        f"$b=New-Object System.Drawing.Bitmap {rect.right - rect.left + 80},{rect.bottom - rect.top + 80};"
        "$g=[System.Drawing.Graphics]::FromImage($b);"
        f"$g.CopyFromScreen({rect.left - 40},{rect.top - 40},0,0,$b.Size);"
        f"$b.Save('{path}');"
    )
    subprocess.run(["powershell", "-NoProfile", "-Command", script], check=True, timeout=30)


def main():
    parser = argparse.ArgumentParser()
    here = pathlib.Path(__file__).resolve().parent
    parser.add_argument("--exe", default=str(here / "target" / "debug" / "overlay-spike.exe"))
    parser.add_argument("--screenshot")
    args = parser.parse_args()

    # Tk window below the overlay, bright green so transparency is visible.
    root = tk.Tk()
    root.title("overlay-check-below")
    root.geometry("1000x700+0+0")
    root.configure(bg="#2e8b57")
    clicks = []
    root.bind("<Button-1>", lambda e: clicks.append((e.x_root, e.y_root)))
    root.update()

    proc = subprocess.Popen([str(pathlib.Path(args.exe).resolve())])
    try:
        overlay = find_overlay()
        if not overlay:
            print("FAIL: overlay window not found")
            return 1
        time.sleep(2)  # let WebView2 paint
        root.update()

        ex = user32.GetWindowLongW(overlay, GWL_EXSTYLE)
        results = {
            "topmost": bool(ex & WS_EX_TOPMOST),
            "layered": bool(ex & WS_EX_LAYERED),
            "transparent_to_input": bool(ex & WS_EX_TRANSPARENT),
        }
        rect = window_rect(overlay)
        point = (
            rect.left + int((rect.right - rect.left) * PANEL_FRACTION),
            rect.top + int((rect.bottom - rect.top) * PANEL_FRACTION),
        )
        below = root_at(*point)
        results["hit_test_skips_overlay"] = bool(below) and below != overlay

        if args.screenshot:
            screenshot(args.screenshot, rect)

        click(*point)
        for _ in range(20):
            root.update()
            time.sleep(0.05)
        results["click_reached_window_below"] = bool(clicks)

        for name, ok in results.items():
            print(f"{'OK  ' if ok else 'FAIL'} {name}")
        return 0 if all(results.values()) else 1
    finally:
        proc.terminate()
        root.destroy()


if __name__ == "__main__":
    sys.exit(main())
