"""End-to-end checks against the local Supabase stack (T-104a).

Skipped unless TAVERN_SUPABASE_LOCAL=1 and `npx supabase start` is running.
The stack's URL and keys are read from `npx supabase status` into memory and
never printed. The users are made up and removed at the end.

    $env:TAVERN_SUPABASE_LOCAL = "1"
    python -m unittest tests.test_supabase_local -v
"""

import gzip
import json
import os
import secrets
import shutil
import subprocess
import unittest
import urllib.error
import urllib.request
import uuid

ENABLED = os.environ.get("TAVERN_SUPABASE_LOCAL") == "1"
SESSION = "Hearthstone_2026_10_09_18_30_00"
TIMEOUT = 15
DB_CONTAINER = "supabase_db_tavern-ledger"


def _stack():
    npx = shutil.which("npx")
    out = subprocess.run(
        [npx, "supabase", "status", "-o", "json"],
        capture_output=True, text=True, timeout=120, check=True,
    ).stdout
    status = json.loads(out[out.index("{"):])
    return {
        "url": status["API_URL"],
        "anon": status["ANON_KEY"],
        "service": status["SERVICE_ROLE_KEY"],
    }


def _psql(sql):
    out = subprocess.run(
        ["docker", "exec", DB_CONTAINER, "psql", "-U", "postgres", "-tAc", sql],
        capture_output=True, text=True, timeout=60, check=True,
    ).stdout
    return out.strip()


class Api:
    def __init__(self, stack):
        self.stack = stack

    def request(self, method, path, *, token=None, key=None, body=None,
                raw=None, content_type="application/json", headers=None):
        key = key or self.stack["anon"]
        hdrs = {"apikey": key, "Authorization": f"Bearer {token or key}"}
        hdrs.update(headers or {})
        data = raw
        if body is not None:
            data = json.dumps(body).encode()
        if data is not None:
            hdrs["Content-Type"] = content_type
        req = urllib.request.Request(
            self.stack["url"] + path, data=data, method=method, headers=hdrs)
        try:
            with urllib.request.urlopen(req, timeout=TIMEOUT) as res:
                payload = res.read()
                return res.status, json.loads(payload) if payload else None
        except urllib.error.HTTPError as e:
            payload = e.read()
            try:
                return e.code, json.loads(payload) if payload else None
            except ValueError:
                return e.code, None

    def download(self, path, token):
        req = urllib.request.Request(self.stack["url"] + path, headers={
            "apikey": self.stack["anon"], "Authorization": f"Bearer {token}"})
        with urllib.request.urlopen(req, timeout=TIMEOUT) as res:
            return res.read()

    def service(self, method, path, **kw):
        return self.request(method, path, key=self.stack["service"], **kw)


@unittest.skipUnless(ENABLED, "set TAVERN_SUPABASE_LOCAL=1 with the local stack running")
class LocalStackTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.api = Api(_stack())

    def setUp(self):
        self.users = []

    def tearDown(self):
        # Deleting an auth user does not remove their files, so files first.
        for uid in self.users:
            _, files = self.api.service(
                "POST", "/storage/v1/object/list/games", body={"prefix": f"{uid}/"})
            names = [f"{uid}/{f['name']}" for f in files or []]
            if names:
                self.api.service("DELETE", "/storage/v1/object/games",
                                 body={"prefixes": names})
            self.api.service("DELETE", f"/auth/v1/admin/users/{uid}")

    def make_user(self):
        email = f"test-{uuid.uuid4().hex[:12]}@example.test"
        password = secrets.token_urlsafe(24)
        status, user = self.api.service("POST", "/auth/v1/admin/users", body={
            "email": email, "password": password, "email_confirm": True})
        self.assertEqual(status, 200, "create user")
        self.users.append(user["id"])
        status, tok = self.api.request(
            "POST", "/auth/v1/token?grant_type=password",
            body={"email": email, "password": password})
        self.assertEqual(status, 200, "sign in")
        return user["id"], email, tok["access_token"]

    def add_game(self, uid, index=1):
        """What the upload Edge Function will do (T-104d), with the service role."""
        record = json.dumps({"session": SESSION, "index": index, "report": {}}).encode()
        blob = gzip.compress(record)
        path = f"{uid}/{SESSION}-{index}.json.gz"
        status, _ = self.api.service(
            "POST", f"/storage/v1/object/games/{path}", raw=blob,
            content_type="application/gzip")
        self.assertEqual(status, 200, "service uploads a file")
        status, _ = self.api.service("POST", "/rest/v1/games", body={
            "user_id": uid, "session": SESSION, "game_index": index,
            "game_type": "GT_BATTLEGROUNDS", "status": "ok",
            "hero_card_id": "BG20_HERO_202", "final_place": 3, "build": 253216,
            "played_on": "2026-10-09", "tribes_offered": {"BEAST": 4},
            "saved_at": 1791000000, "content_sha256": "0" * 64,
            "size_bytes": len(blob)})
        self.assertEqual(status, 201, "service writes a game row")
        return path

    def test_clients_cannot_write_games_or_files(self):
        uid, _, token = self.make_user()
        blob = gzip.compress(b"{}")
        status, _ = self.api.request(
            "POST", f"/storage/v1/object/games/{uid}/{SESSION}-9.json.gz",
            token=token, raw=blob, content_type="application/gzip")
        self.assertIn(status, (400, 401, 403), "user upload to the bucket")
        status, _ = self.api.request("POST", "/rest/v1/games", token=token, body={
            "user_id": uid, "session": SESSION, "game_index": 9,
            "game_type": "GT_BATTLEGROUNDS", "status": "ok", "played_on": "2026-10-09",
            "saved_at": 1, "content_sha256": "0" * 64, "size_bytes": 1})
        self.assertIn(status, (401, 403), "user insert into games")
        path = self.add_game(uid)
        self.api.request(
            "DELETE", "/storage/v1/object/games", token=token, body={"prefixes": [path]})
        _, files = self.api.service(
            "POST", "/storage/v1/object/list/games", body={"prefix": f"{uid}/"})
        self.assertEqual([f["name"] for f in files], [path.split("/", 1)[1]],
                         "a user's own delete request leaves the file in place")

    def test_another_user_cannot_read_private_data(self):
        a, _, _ = self.make_user()
        path = self.add_game(a)
        _, _, token_b = self.make_user()
        _, profiles = self.api.request("GET", f"/rest/v1/profiles?user_id=eq.{a}", token=token_b)
        _, games = self.api.request("GET", f"/rest/v1/games?user_id=eq.{a}", token=token_b)
        status, _ = self.api.request(
            "GET", f"/storage/v1/object/authenticated/games/{path}", token=token_b)
        self.assertEqual(profiles, [])
        self.assertEqual(games, [])
        self.assertIn(status, (400, 403, 404))

    def test_export_then_delete_leaves_nothing(self):
        uid, email, token = self.make_user()
        path = self.add_game(uid)

        status, doc = self.api.request("POST", "/rest/v1/rpc/export_my_data",
                                       token=token, body={})
        self.assertEqual(status, 200)
        self.assertEqual(doc["account"]["email"], email)
        self.assertEqual([g["storage_path"] for g in doc["games"]], [path])
        self.assertEqual([f["name"] for f in doc["files"]], [path])
        self.assertTrue(doc["sessions"], "export lists sign-in sessions")
        self.assertEqual(doc["profile"]["user_id"], uid)
        self.assertNotIn("encrypted_password", json.dumps(doc))

        status, result = self.api.request(
            "DELETE", "/functions/v1/delete-account", token=token)
        self.assertEqual(status, 200, result)
        self.assertEqual(result["deleted"]["files"], 1)
        self.assertEqual(result["deleted"]["games"], 1)

        status, _ = self.api.service("GET", f"/auth/v1/admin/users/{uid}")
        self.assertEqual(status, 404, "auth user is gone")
        status, _ = self.api.service("GET", f"/storage/v1/object/games/{path}")
        self.assertIn(status, (400, 404), "file bytes are gone")
        left = _psql(
            "select (select count(*) from public.games where user_id = '{u}')"
            " + (select count(*) from public.profiles where user_id = '{u}')"
            " + (select count(*) from storage.objects where name like '{u}/%')"
            " + (select count(*) from auth.users where id = '{u}')"
            " + (select count(*) from auth.identities where user_id = '{u}')"
            " + (select count(*) from auth.sessions where user_id = '{u}')"
            " + (select count(*) from auth.refresh_tokens where user_id = '{u}')"
            " + (select count(*) from auth.audit_log_entries"
            "    where payload::text like '%{u}%' or payload::text like '%{e}%')"
            .format(u=uid, e=email))
        self.assertEqual(left, "0", "nothing of the user remains")
        self.users.remove(uid)

        status, _ = self.api.request("DELETE", "/functions/v1/delete-account", token=token)
        self.assertEqual(status, 401, "the old token no longer works")

    def test_delete_needs_a_valid_token(self):
        status, _ = self.api.request("DELETE", "/functions/v1/delete-account")
        self.assertEqual(status, 401)

    # ------------------------------------------------------------ upload (T-104d)

    def upload(self, token, record, index=1):
        return self.api.request(
            "PUT", f"/functions/v1/upload-game/v1/games/{SESSION}/{index}",
            token=token, raw=gzip.compress(_compact(record)),
            content_type="application/gzip")

    def test_upload_stores_a_checked_game_once(self):
        uid, _, token = self.make_user()
        record = _record()
        status, body = self.upload(token, record)
        self.assertEqual(status, 201, body)
        self.assertEqual(body["result"], "created")
        status, body = self.upload(token, record)
        self.assertEqual((status, body["result"]), (200, "unchanged"), "same bytes again")

        _, rows = self.api.request("GET", f"/rest/v1/games?user_id=eq.{uid}", token=token)
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]["parser_version"], "0.1.0+r1")
        self.assertEqual(rows[0]["final_place"], record["report"]["final_place"])
        stored = self.api.download(f"/storage/v1/object/authenticated/games/"
                                   f"{rows[0]['storage_path']}", token=token)
        self.assertEqual(json.loads(gzip.decompress(stored)), record,
                         "the owner reads back the record that was sent")

        newer = _record(saved_at=record["saved_at"] + 10, final_place=2)
        status, body = self.upload(token, newer)
        self.assertEqual((status, body["result"]), (200, "replaced"))
        status, body = self.upload(token, _record(saved_at=record["saved_at"] - 10))
        self.assertEqual((status, body["code"]), (409, "older_revision"))

        forged = _record()
        forged["report"]["card_names"] = {forged["report"]["hero"]: "Someone#1234"}
        status, body = self.upload(token, forged, index=2)
        self.assertEqual((status, body["code"]), (400, "player_name"))
        padded = json.dumps(_record(), indent=2).encode()
        status, body = self.api.request(
            "PUT", f"/functions/v1/upload-game/v1/games/{SESSION}/1", token=token,
            raw=gzip.compress(padded), content_type="application/gzip")
        self.assertEqual((status, body["code"]), (400, "invalid_json"), "only compact JSON")
        forged = _record()
        forged["report"]["lobby"][0]["battle_tag"] = "Someone"
        status, body = self.upload(token, forged)
        self.assertEqual((status, body["code"]), (400, "invalid_report"))
        _, rows = self.api.request("GET", f"/rest/v1/games?user_id=eq.{uid}", token=token)
        self.assertEqual([r["game_index"] for r in rows], [1], "nothing forged was stored")

    def test_upload_limits(self):
        uid, _, token = self.make_user()
        _psql("insert into private.upload_events (user_id, at) select '{u}', now()"
              " from generate_series(1, 600)".format(u=uid))
        status, body = self.upload(token, _record())
        self.assertEqual((status, body["code"]), (429, "rate_limited"))
        status, body = self.upload(None, _record())
        self.assertEqual(status, 401, "no user token")
        status, body = self.api.request(
            "PUT", f"/functions/v1/upload-game/v1/games/{SESSION}/1",
            token=token, raw=b"x" * 70000, content_type="application/gzip")
        self.assertEqual(status, 413)

    def test_sweep_removes_orphan_files(self):
        uid, _, token = self.make_user()
        status, _ = self.upload(token, _record())
        self.assertEqual(status, 201)
        orphan = self.add_file_only(uid, index=7)
        # Replica mode skips the trigger that would set updated_at back to now().
        _psql("set session_replication_role = replica;"
              " update storage.objects set created_at = now() - interval '2 hours',"
              " updated_at = now() - interval '2 hours' where name = '{n}'".format(n=orphan))
        status, body = self.api.service("POST", "/functions/v1/sweep", body={})
        self.assertEqual(status, 200, body)
        self.assertGreaterEqual(body["files"], 1)
        left = _psql("select string_agg(name, ',' order by name) from storage.objects"
                     " where name like '{u}/%'".format(u=uid))
        self.assertEqual(left, f"{uid}/{SESSION}-1.json.gz", "the game with a row stays")
        status, _ = self.api.request("POST", "/functions/v1/sweep", token=token, body={})
        self.assertEqual(status, 401, "a user cannot run the sweep")

    def add_file_only(self, uid, index):
        path = f"{uid}/{SESSION}-{index}.json.gz"
        status, _ = self.api.service(
            "POST", f"/storage/v1/object/games/{path}", raw=gzip.compress(b"{}"),
            content_type="application/gzip")
        self.assertEqual(status, 200)
        return path


def _compact(record):
    """The bytes the desktop app sends: compact JSON (serde_json's output)."""
    return json.dumps(record, separators=(",", ":"), ensure_ascii=False).encode()


def _record(saved_at=1791000000, final_place=None):
    """A record as the desktop app keeps it, around a real parser report."""
    with open(os.path.join(os.path.dirname(__file__), "..", "crates", "bg-parser", "tests",
                           "data", "duo_game.json"), encoding="utf-8") as f:
        report = json.load(f)[0]
    if final_place is not None:
        report["final_place"] = final_place
    return {"session": SESSION, "index": 1, "saved_at": saved_at,
            "parser": {"version": "0.1.0", "revision": 1}, "report": report}


if __name__ == "__main__":
    unittest.main()
