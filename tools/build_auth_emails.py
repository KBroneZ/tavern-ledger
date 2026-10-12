"""Builds the Supabase Auth email templates in English and Spanish (T-542, D-114).

One layout, one copy table: every template in supabase/templates/ is written from here, so the five
emails stay alike and a copy change is made once. Each file holds the whole email twice, Spanish and
English, picked by a Go template condition on the account's `lang` metadata (written at sign-up from
the site's language); anything but "es", a missing value included, gets English. Subjects use the same
condition. The links are Supabase's own `{{ .ConfirmationURL }}`, untouched (D-048, deploy.md 10.3).

Writes:
- supabase/templates/<name>.html   the body of each email (pasted into the dashboard, read by the
                                   local stack through supabase/config.toml)

Python standard library only.

Usage:
    python tools/build_auth_emails.py            write every template
    python tools/build_auth_emails.py --check    exit 1 if a template or a config.toml subject is out of date
"""

from __future__ import annotations

import argparse
import html
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TEMPLATES = ROOT / "supabase" / "templates"
CONFIG = ROOT / "supabase" / "config.toml"

# The logo is a plain PNG on the site (made by tools/build_icons.py), shown at half its size.
LOGO_URL = "https://tavernledger.net/email/logo.png"
LOGO_SIZE = 56
SITE_URL = "https://tavernledger.net/"
LINK = "{{ .ConfirmationURL }}"
# `print` turns any value (missing, null, a number) into text, so `eq` never fails on a strange one.
IS_SPANISH = '{{ if eq (print .Data.lang) "es" }}'
NOTICE_EN = "Unofficial fan project. Not affiliated with or endorsed by Blizzard Entertainment."

# Colours: the site's ink and amber (D-094) around a parchment card (D-105), dark by default so
# clients that darken emails change little; the card keeps dark text if a client drops the ground.
INK = "#120d0b"
AMBER = "#ffb24a"
FOAM = "#ffd08a"
PARCHMENT = "#f3e4c4"
EDGE = "#8a5a2b"
TEXT = "#2b1d13"
LINK_COLOUR = "#7d3a0c"
FOOT = "#cdb995"
SERIF = "Georgia, 'Times New Roman', serif"
SANS = "-apple-system, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif"

# Every email Supabase Auth sends for this project. Reauthentication is left out: nothing asks for it
# (the site never calls reauthenticate()), and it carries a code, not a link.
COPY: dict[str, dict[str, dict[str, object]]] = {
    "confirmation": {
        "en": {
            "subject": "Confirm your Tavern Ledger account",
            "heading": "Confirm your email",
            "body": ["Thanks for signing up to Tavern Ledger. Confirm this email address to finish creating your account."],
            "button": "Confirm my email",
            "ignore": "Didn't sign up? Ignore this email: without the confirmation, nobody can sign in with this address.",
        },
        "es": {
            "subject": "Confirma tu cuenta de Tavern Ledger",
            "heading": "Confirma tu correo",
            "body": ["Gracias por registrarte en Tavern Ledger. Confirma esta dirección de correo para terminar de crear tu cuenta."],
            "button": "Confirmar mi correo",
            "ignore": "¿No te has registrado tú? Ignora este correo: sin la confirmación, nadie puede iniciar sesión con esta dirección.",
        },
    },
    "magic_link": {
        "en": {
            "subject": "Your Tavern Ledger sign-in link",
            "heading": "Sign in to the app",
            "body": [
                "Open this link on the PC where you asked for it to sign in to the Tavern Ledger app.",
                "It works once and expires in 1 hour.",
            ],
            "button": "Sign in",
            "ignore": "Didn't ask for it? Ignore this email: nobody can sign in without this link.",
        },
        "es": {
            "subject": "Tu enlace para iniciar sesión en Tavern Ledger",
            "heading": "Inicia sesión en la aplicación",
            "body": [
                "Abre este enlace en el PC donde lo pediste para iniciar sesión en la aplicación de Tavern Ledger.",
                "Solo sirve una vez y caduca en 1 hora.",
            ],
            "button": "Iniciar sesión",
            "ignore": "¿No lo has pedido tú? Ignora este correo: sin este enlace, nadie puede entrar en tu cuenta.",
        },
    },
    "recovery": {
        "en": {
            "subject": "Reset your Tavern Ledger password",
            "heading": "Choose a new password",
            "body": [
                "Someone asked to reset the password of the Tavern Ledger account with this email.",
                "Open the link in the browser where you asked for it. It works once and expires in 1 hour.",
            ],
            "button": "Choose a new password",
            "ignore": "Didn't ask for it? Ignore this email: your password stays as it is.",
        },
        "es": {
            "subject": "Cambia tu contraseña de Tavern Ledger",
            "heading": "Elige una contraseña nueva",
            "body": [
                "Alguien ha pedido cambiar la contraseña de la cuenta de Tavern Ledger con este correo.",
                "Abre el enlace en el navegador donde lo pediste. Solo sirve una vez y caduca en 1 hora.",
            ],
            "button": "Elegir una contraseña nueva",
            "ignore": "¿No lo has pedido tú? Ignora este correo: tu contraseña no cambia.",
        },
    },
    "email_change": {
        "en": {
            "subject": "Confirm the email change on Tavern Ledger",
            "heading": "Confirm the email change",
            "body": ["Someone asked to change the email of a Tavern Ledger account from {{ .Email }} to {{ .NewEmail }}. Confirm to make the change."],
            "button": "Confirm the change",
            "ignore": "Didn't ask for it? Don't open the link, and change your password: someone else may be signed in to your account.",
        },
        "es": {
            "subject": "Confirma el cambio de correo en Tavern Ledger",
            "heading": "Confirma el cambio de correo",
            "body": ["Alguien ha pedido cambiar el correo de una cuenta de Tavern Ledger de {{ .Email }} a {{ .NewEmail }}. Confírmalo para hacer el cambio."],
            "button": "Confirmar el cambio",
            "ignore": "¿No lo has pedido tú? No abras el enlace y cambia tu contraseña: puede que otra persona tenga tu sesión abierta.",
        },
    },
    "invite": {
        "en": {
            "subject": "You're invited to Tavern Ledger",
            "heading": "You're invited",
            "body": [
                "You've been invited to make a Tavern Ledger account with this email.",
                "Accept to confirm it, then use “Forgot password?” on the sign-in page to choose a password.",
            ],
            "button": "Accept the invite",
            "ignore": "Weren't expecting it? Ignore this email: no account is made without your answer.",
        },
        "es": {
            "subject": "Te han invitado a Tavern Ledger",
            "heading": "Te han invitado",
            "body": [
                "Te han invitado a crear una cuenta de Tavern Ledger con este correo.",
                "Acepta para confirmarla y luego usa «¿Has olvidado la contraseña?» en la página de inicio de sesión para elegir una contraseña.",
            ],
            "button": "Aceptar la invitación",
            "ignore": "¿No te lo esperabas? Ignora este correo: sin tu respuesta no se crea ninguna cuenta.",
        },
    },
}

UI = {
    "en": {
        "fallback": "Button not working? Copy this link into your browser:",
        "why": "You got this email because this address was used on",
        "notice": NOTICE_EN,
    },
    "es": {
        "fallback": "¿No funciona el botón? Copia este enlace en tu navegador:",
        "why": "Te llega este correo porque se usó esta dirección en",
        "notice": "Proyecto no oficial de fans. Sin afiliación con Blizzard Entertainment ni respaldo de esta.",
    },
}

# Go template actions pass through the escaping untouched; everything else is HTML-escaped.
ACTION = re.compile(r"(\{\{.*?\}\})")


def text(value: str) -> str:
    parts = ACTION.split(value)
    return "".join(p if ACTION.fullmatch(p) else html.escape(p, quote=False) for p in parts)


STYLE = """
    :root { color-scheme: light dark; supported-color-schemes: light dark; }
    body { margin: 0; padding: 0; }
    a.plain { color: %(link)s; }
    @media (max-width: 600px) {
      .card { padding: 24px 18px !important; }
    }
    @media (prefers-color-scheme: dark) {
      .card { background: #24180f !important; border-color: #5c3d22 !important; }
      .card h1, .card p { color: #f1e3c6 !important; }
      .card a.plain { color: %(amber)s !important; }
    }
""" % {"link": LINK_COLOUR, "amber": AMBER}


def paragraphs(lines: list[str]) -> str:
    style = f"margin:0 0 16px;font-family:{SANS};font-size:16px;line-height:24px;color:{TEXT};"
    return "".join(f'<p style="{style}">{text(line)}</p>' for line in lines)


def email(name: str, lang: str) -> str:
    """One whole email in one language."""
    copy = COPY[name][lang]
    ui = UI[lang]
    notice_en = "" if lang == "en" else f' <span lang="en">({html.escape(NOTICE_EN)})</span>'
    small = f"margin:0 0 12px;font-family:{SANS};font-size:13px;line-height:20px;color:{TEXT};"
    return f"""<!DOCTYPE html>
<html lang="{lang}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="color-scheme" content="light dark">
<meta name="supported-color-schemes" content="light dark">
<title>{text(str(copy["subject"]))}</title>
<style>{STYLE}</style>
</head>
<body style="margin:0;padding:0;background:{INK};" bgcolor="{INK}">
<div style="display:none;max-height:0;overflow:hidden;mso-hide:all;">{text(str(list(copy["body"])[0]))}</div>
<table role="presentation" width="100%" cellpadding="0" cellspacing="0" border="0" bgcolor="{INK}" style="background:{INK};">
<tr><td align="center" style="padding:32px 16px;">
<!--[if mso]><table role="presentation" width="560" cellpadding="0" cellspacing="0" border="0"><tr><td><![endif]-->
<table role="presentation" width="100%" cellpadding="0" cellspacing="0" border="0" style="max-width:560px;">
<tr><td align="center" style="padding:0 0 20px;">
<a href="{SITE_URL}" style="text-decoration:none;"><img src="{LOGO_URL}" width="{LOGO_SIZE}" height="{LOGO_SIZE}" alt="Tavern Ledger" style="display:block;border:0;width:{LOGO_SIZE}px;height:{LOGO_SIZE}px;"></a>
<p style="margin:10px 0 0;font-family:{SERIF};font-size:20px;line-height:26px;font-weight:bold;color:{FOAM};">Tavern Ledger</p>
</td></tr>
<tr><td class="card" bgcolor="{PARCHMENT}" style="background:{PARCHMENT};border:1px solid {EDGE};border-radius:10px;padding:32px 28px;">
<h1 style="margin:0 0 16px;font-family:{SERIF};font-size:26px;line-height:32px;font-weight:bold;color:{TEXT};">{text(str(copy["heading"]))}</h1>
{paragraphs(list(copy["body"]))}
<table role="presentation" cellpadding="0" cellspacing="0" border="0" style="margin:8px 0 24px;">
<tr><td align="center" bgcolor="{AMBER}" style="background:{AMBER};border:1px solid #c47f22;border-radius:8px;">
<a href="{LINK}" style="display:inline-block;padding:14px 28px;font-family:{SANS};font-size:16px;line-height:20px;font-weight:bold;color:{INK};text-decoration:none;border-radius:8px;">{text(str(copy["button"]))}</a>
</td></tr>
</table>
<p style="{small}">{text(ui["fallback"])}<br><a class="plain" href="{LINK}" style="color:{LINK_COLOUR};word-break:break-all;">{LINK}</a></p>
<p style="{small}margin:0;">{text(str(copy["ignore"]))}</p>
</td></tr>
<tr><td align="center" bgcolor="{INK}" style="background:{INK};padding:20px 8px 0;">
<p style="margin:0 0 8px;font-family:{SANS};font-size:12px;line-height:18px;color:{FOOT};">{text(ui["why"])} <a href="{SITE_URL}" style="color:{FOOT};">tavernledger.net</a>.</p>
<p style="margin:0;font-family:{SANS};font-size:12px;line-height:18px;color:{FOOT};">{text(ui["notice"])}{notice_en}</p>
</td></tr>
</table>
<!--[if mso]></td></tr></table><![endif]-->
</td></tr>
</table>
</body>
</html>
"""


def template(name: str) -> str:
    """The file for the dashboard: Spanish when the account's language is "es", else English."""
    return f"{IS_SPANISH}\n{email(name, 'es')}{{{{ else }}}}\n{email(name, 'en')}{{{{ end }}}}\n"


def subject(name: str) -> str:
    return f'{IS_SPANISH}{COPY[name]["es"]["subject"]}{{{{ else }}}}{COPY[name]["en"]["subject"]}{{{{ end }}}}'


def toml_string(value: str) -> str:
    return '"' + value.replace("\\", "\\\\").replace('"', '\\"') + '"'


def config_block(name: str) -> str:
    return (f"[auth.email.template.{name}]\n"
            f"subject = {toml_string(subject(name))}\n"
            f'content_path = "./supabase/templates/{name}.html"\n')


def stale() -> list[str]:
    out = []
    for name in COPY:
        path = TEMPLATES / f"{name}.html"
        if not path.is_file() or path.read_text(encoding="utf-8") != template(name):
            out.append(path.relative_to(ROOT).as_posix())
    config = CONFIG.read_text(encoding="utf-8")
    out += [f"supabase/config.toml [auth.email.template.{n}]" for n in COPY if config_block(n) not in config]
    return out


def build() -> None:
    TEMPLATES.mkdir(parents=True, exist_ok=True)
    for name in COPY:
        path = TEMPLATES / f"{name}.html"
        path.write_bytes(template(name).encode("utf-8"))
        print(f"wrote {path.relative_to(ROOT).as_posix()}")
    print("config.toml blocks (paste if changed):\n")
    for name in COPY:
        print(config_block(name))


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true", help="only check that the files are up to date")
    args = parser.parse_args(argv)
    if args.check:
        out = stale()
        for item in out:
            print(f"out of date: {item}", file=sys.stderr)
        return 1 if out else 0
    build()
    return 0


if __name__ == "__main__":
    sys.exit(main())
