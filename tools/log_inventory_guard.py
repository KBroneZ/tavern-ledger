"""Privacy guard of tools/log_inventory.py (T-111).

Keeps in memory, never prints, the player names and account numbers met in the
logs, and withholds or refuses any output value that matches them or looks
personal (BattleTag, IP address, e-mail, path with a user name, account id,
deck code). Standard library only.
"""

from __future__ import annotations

import os
import re
from collections import Counter
from dataclasses import dataclass, field

WITHHELD = "<withheld>"
# Names the game itself uses for a player it does not name ("Player"): they
# say nothing about anyone and would otherwise hide every key called "player".
GENERIC_NAMES = {"player"}
INT = re.compile(r"^-?\d+$")


class PrivacyRefusal(Exception):
    """The result holds something outside the allow-list; nothing is printed."""


DENY_PATTERNS = [
    ("battletag", re.compile(r"[^\s#]#\d{3,}")),
    ("ipv4", re.compile(r"\b\d{1,3}(?:\.\d{1,3}){3}\b")),
    ("ipv6", re.compile(r"(?i)\b[0-9a-f]{1,4}(?::[0-9a-f]{0,4}){3,}\b|[0-9a-f]{1,4}::[0-9a-f]{0,4}")),
    ("email", re.compile(r"[^\s@]+@[^\s@]+\.[A-Za-z]{2,}")),
    ("user path", re.compile(r"(?i)[a-z]:[\\/]+users[\\/]|/home/|/Users/")),
    ("long number", re.compile(r"\d{8,}")),
    ("account id", re.compile(r"(?i)\b(?:hi|lo)=\d")),
    ("deck code", re.compile(r"AAE[A-Za-z0-9+/]{8,}")),
]


@dataclass
class Guard:
    """Personal values read from the logs. They are never printed."""

    names: set[str] = field(default_factory=set)
    numbers: set[int] = field(default_factory=set)
    withheld: Counter = field(default_factory=Counter)  # reason -> count

    def add_name(self, name: str) -> None:
        name = name.strip()
        if len(name) >= 2 and name.casefold() not in GENERIC_NAMES:
            self.names.add(name.casefold())
            # A BattleTag has no spaces; the part before `#` is the name shown.
            # Names with spaces (Bob's) are matched whole, so a common word in
            # them does not hide every key that uses it.
            base = name.split("#", 1)[0]
            if len(base) >= 2 and base.casefold() not in GENERIC_NAMES:
                self.names.add(base.casefold())

    def add_number(self, text: str) -> None:
        if INT.match(text) and int(text) > 1000:
            self.numbers.add(int(text))

    def scrub(self, obj):
        """A copy of `obj` with every unsafe key, text or number withheld."""
        if isinstance(obj, dict):
            out = {}
            for key, value in obj.items():
                reason = self.reason(key) or (str(key).startswith(WITHHELD) and "placeholder")
                if reason:
                    self.withheld[reason] += 1
                    key = f"{WITHHELD}{len(out)}"
                out[key] = self.scrub(value)
            return out
        if isinstance(obj, (list, tuple)):
            return [self.scrub(item) for item in obj]
        reason = self.reason(obj)
        if reason:
            self.withheld[reason] += 1
            return WITHHELD
        return obj

    def check(self, obj) -> None:
        """Raise PrivacyRefusal if anything in `obj` is unsafe; the message says why, never what or where."""
        if isinstance(obj, dict):
            for key, value in obj.items():
                reason = self.reason(key)
                if reason:
                    raise PrivacyRefusal(f"{reason} in a key")
                self.check(value)
        elif isinstance(obj, (list, tuple)):
            for item in obj:
                self.check(item)
        else:
            reason = self.reason(obj)
            if reason:
                raise PrivacyRefusal(reason)

    def reason(self, value) -> str | None:
        """Why a key, text or number may not be printed, or None."""
        if isinstance(value, bool) or value is None or isinstance(value, float):
            return None
        if isinstance(value, int):
            return "account number" if value in self.numbers else None
        text = str(value)
        for label, pattern in DENY_PATTERNS:
            if pattern.search(text):
                return label
        words = {w.casefold() for w in re.split(r"[^\w#]+", text) if w}
        if words & self.names or text.casefold() in self.names:
            return "player name"
        if any(int(n) in self.numbers for n in re.findall(r"\d+", text)):
            return "account number"
        user = os.environ.get("USERNAME") or os.environ.get("USER")
        if user and len(user) >= 3 and user.casefold() in words:
            return "user name"
        return None
