# MMR sources (T-004)

Date: 2026-10-09. User logs (6 sessions, 23 games, all `GT_BATTLEGROUNDS_DUO`, build 253216; none of Solo) and 7 manual requests to the public leaderboard. Cited lines are approximate and refer to session `Hearthstone_2026_10_08_00_05_26` unless stated otherwise. No player name comes from the logs or the responses in this report.

## Summary

- **No local log carries the MMR value**, in Solo or Duos. Neither before nor after the game.
- There is a **signal**: `Net.log` records the arrival of `NetCacheBaconRatingInfo` twice at login and, normally, twice 4 to 9 s after each game ends. It says *when* the rating changes, not *by how much*.
- With the `[Net]` section on `Verbose=true` the value **does not** appear either: tested with a Solo game on 2026-10-09 (see [Experiment: `[Net]` verbose](#experiment-net-verbose)).
- The **public leaderboard** works as described in [`feasibility.md`](feasibility.md): 25 rows per page, only the visible name (BattleTag without number) and rating, cut-off at 8000. No search by name.
- **New risk:** the terms of use of Blizzard's websites grant a "personal use only" license and exclude commercial use and downloading parts of the site except page caching. This affects how the leaderboard is used (D-005). Closed with D-012: only the user's own row, from the user's app.
- **Decision (D-011, closes P-007):** leaderboard as guide. Inside it, its rating; outside, "below the cut-off" (< 8000), never a number. **Do not infer deltas.**
- First Solo game (2026-10-09): `GT_BATTLEGROUNDS`, same build. No MMR in any log either.

## Table: data → source

| Data | Source | How | Evidence | Limitations |
|------|--------|-----|----------|-------------|
| Current MMR (Solo or Duos) | **None local** | — | Search for `mmr`, `rating`, `leaderboard`, `rank`, `medal`, `elo`, `season` in all logs of the 6 sessions: only class names (below), no value | — |
| MMR if in the top | Public leaderboard | `rating` of the row whose `accountid` matches the local name, in the region and mode of the game | 7 requests on 2026-10-09 (below) | Only covers ≥ 8000. Repeated names. Pages must be walked. Only the user's own row (D-012) |
| "The rating has changed" | `Net.log` | Line `OnNetCacheObjReceived SAVE --> NetCacheBaconRatingInfo` | Lines 33 and 54 (login); 57–58, 62–63… (one pair after each game, 4–9 s after `Reason: EndGameScreen` in `GameNetLogger.log`, lines 14, 28, 77…). Also after the Solo game (session `Hearthstone_2026_10_09_00_26_56`, lines 57–58) | No value, not even with `Verbose=true`. The pair arrives the same in Solo and Duos: it does not tell the mode |
| End of game | `GameNetLogger.log` | `Network.DisconnectFromGameServer() - Reason: EndGameScreen` | 8 in the 8-Oct session, same as the 8 games in `Power.log` | `Power.log` already gives the end of game (`STATE=COMPLETE`) |
| Account region | `Hearthstone.log` | Line `Region: EU` right after login | Line 48 in all 6 sessions | Undocumented format |
| Local player name | `Power.log` | Name of the local `Player` in `CREATE_GAME`. `tools/parse_bg.py` identifies the local player but does not output the name (privacy) | See [`parser-hslog.md`](parser-hslog.md) | It is a BattleTag with number: the leaderboard gives it without number |
| Mode (Solo or Duos) | `Power.log` | `GameType=` (`GT_BATTLEGROUNDS` / `GT_BATTLEGROUNDS_DUO`) | See [`parser-hslog.md`](parser-hslog.md) | Solo untested with real logs |
| MMR change per game | Only if two values are available | Difference between the rating before and after | — | Outside the leaderboard there is no base value; inside it, the update frequency is unknown |

## Local logs

### `log.config`

`%LOCALAPPDATA%\Blizzard\Hearthstone\log.config` has 7 sections, all with `LogLevel=1` and `FilePrinting=true`:

| Section | `Verbose` | File | Content useful for MMR |
|---------|-----------|------|------------------------|
| `[Power]` | true | `Power_old.log` (120–590 MB) | Nothing (T-003) |
| `[Net]` | false (true since 2026-10-09, for the experiment) | `Net.log` (4–8 KB) | `NetCacheBaconRatingInfo` signal, no value |
| `[Achievements]` | true | `Achievements_old.log` | Nothing (only `MERCENARIES_SEASON_ROLL`) |
| `[LoadingScreen]` | false | `LoadingScreen_old.log` | Nothing (scene changes) |
| `[Decks]` | false | `Decks.log` | Nothing |
| `[FullScreenFX]` | true | `FullScreenFX.log` | Nothing |
| `[Arena]` | false | (not created: the user has not played Arena) | — |

### Files of each session

The game writes these files even if they have no section in `log.config`. Sizes from the 8-Oct session.

| File | Size | Contents | MMR? |
|------|------|----------|------|
| `Hearthstone.log` | 5.7 MB | General client log: startup, region, tavern UI (`TB_BaconShop_*`; `PlayerLeaderboardMainCardActor` is the in-game health scoreboard, not the ranking) | No |
| `GameNetLogger.log` | 23 KB | Queue, connection and disconnection from the game server | No |
| `Net.log` | 8 KB | `NetCache` objects arriving from the server, coin balance | Only the signal |
| `Gameplay.log`, `Spells.log`, `Asset.log` | ≤ 15 KB | Visual effects | No |
| `Login.log`, `LuckyDraw.log`, `Store.log`, `Downloader.log`, `ExceptionReporter.log`, `All.log`, `BattleNet.log` | ≤ 150 KB | Login, events, store, errors | No |

`options.txt` (next to `log.config`) has no Battlegrounds or rating key.

### Sensitive data in other logs

`Net.log` includes an **authentication token** in the Battle.net connection line. `Hearthstone.log` and `Power.log` carry BattleTags. Consequences for the product:

- The app never uploads, attaches or copies whole `Net.log` or `Hearthstone.log` files (not even in error reports).
- If it reads anything from them, it does so line by line with a closed pattern (e.g. only `Region: ([A-Z]+)`) and discards the rest.
- Nothing from these files goes into fixtures without anonymizing.

### Experiment: `[Net]` verbose

`NetCacheBaconRatingInfo` is an object of the `[Net]` section, and that section already dumps contents in other cases (`Caching currency state: { … }` with the balances). We tested whether `Verbose=true` also wrote the rating.

- Change made by the user on 2026-10-09: line `Verbose=false` → `Verbose=true` in `[Net]`, saved 8 s before starting the game. To undo it, go back to `Verbose=false`.
- One Solo game (`GT_BATTLEGROUNDS`, build 253216) in session `Hearthstone_2026_10_09_00_26_56`.
- Result: **negative**. `Net.log` has the same size (≈ 4.3 KB) and the same lines as with `Verbose=false`; the four `NetCacheBaconRatingInfo` lines (33, 54, 57, 58) still have no content. No other log of the session mentions rating, MMR or leaderboard.

## Public leaderboard

### Endpoint

Used by the site itself (`hearthstone.blizzard.com/<locale>/community/leaderboards`). It is not documented: the page script builds the URL like this, and there is no other public leaderboard API (the official Hearthstone developer API covers cards, decks and metadata; the `partner-*.api.blizzard.net` links that appear in the response belong to a partner API, not a public one).

```
GET https://hearthstone.blizzard.com/en-gb/api/community/leaderboardsData?region=EU&leaderboardId=battlegrounds&page=1
```

| Parameter | Values | Notes |
|-----------|--------|-------|
| `region` | `EU`, `US`, `AP` | The response carries the `regions` map (`US`=1, `EU`=2, `AP`=3) |
| `leaderboardId` | `battlegrounds` (Solo), `battlegroundsduo` (Duos) | Also `standard`, `wild`, `arena`… |
| `page` | 1… | 25 rows per page. The site does not send it if it is 1 |
| `seasonId` | optional | Without it, current season. BG Solo and Duos: season 19 on 2026-10-09 |

Example request, with an identifiable User-Agent and timeout (rule 4):

```
curl -sS -m 15 -A "TavernLedger-research/0.1 (+https://github.com/KBroneZ/tavern-ledger)" \
  "https://hearthstone.blizzard.com/en-gb/api/community/leaderboardsData?region=EU&leaderboardId=battlegroundsduo&page=1"
```

### Response

JSON of about 300 KB per page, mostly presentation metadata (`displayMetaData`, `seasonMetaData` with the seasons of each mode and region). The useful part (invented names):

```json
{
  "seasonId": 19,
  "region": "EU",
  "leaderboard": {
    "columns": ["rank", "accountid", "rating"],
    "rows": [
      {"rank": 1, "accountid": "ExamplePlayer", "rating": 19118},
      {"rank": 2, "accountid": "AnotherName", "rating": 18688}
    ],
    "pagination": {"totalPages": 173, "totalSize": 4305}
  }
}
```

- `accountid` is the visible name: BattleTag **without** `#number` (0 of 25 with `#` on page 1). It may contain non-ASCII characters.
- There is no stable account id or update date. The site script reads a `metadata.last_updated_time` that is not in the response today.
- No real response is stored in the repo: the site's terms do not allow downloading or redistributing parts of the site (see below). For T-006, synthetic responses with this shape are enough.

### Coverage (2026-10-09, season 19)

| Leaderboard | Rows | Pages | First | Last |
|-------------|------|-------|-------|------|
| BG Solo EU | 4305 | 173 | 19118 | 8000 |
| BG Duos EU | 1482 | 60 | 22498 | 8000 |
| BG Solo EU, season 18 (closed) | 11450 | 458 | 19020 | — |

The 8000 cut-off is confirmed in Solo and Duos. Season 18 has many more rows; its cut-off was not checked.

### Behavior and limits

| Finding | Implication |
|---------|-------------|
| Out-of-range page (`page=999`): HTTP 200 with `rows: []` and `totalPages: 0` | An empty page does **not** mean "the player is not there"; the client must compare against `totalPages` from page 1 |
| No search by name | Finding a name requires walking pages: up to 173 (≈ 52 MB; ~3 MB with gzip, T-006) in Solo EU |
| Responses in 0.7–1.8 s; no rate-limit headers or `Cache-Control`; weak `ETag` | Real limit unknown. Own cache; `If-None-Match` is useless: the `ETag` changes on every response (T-006) |
| Sets `session` and `locale` cookies | The client neither stores nor resends them |
| `robots.txt` does not block `/api/` | It is not a permission; the terms rule |

### Terms

- **Terms of use of Blizzard's websites** (rev. 2018-09-26): license "for personal use only"; they exclude "any commercial use of the Site or the Materials therein" and "downloading (other than the page caching) of any portion of the Site", and transmitting the materials to another site. They do not mention scraping or automated requests. [Source](https://www.blizzard.com/en-us/legal/29232b30-6ae1-4d74-b1c5-8bd1df9e0b63/terms-of-use-for-blizzards-websites)
- **EULA** (2024-03-21): forbids unauthorized processes that "intercepts, collects, reads, or 'mines' information" from the *Platform* (app, service and games; websites are not in the definition). [Source](https://www.blizzard.com/en-us/legal/fba4d00f-c7e4-4883-b8b9-1b4500a402ea/blizzard-end-user-license-agreement)
- **Developer API terms**: according to forums (not verified in the official text), they forbid scraping and automated queries except through authorized APIs. They only bind those who register as a developer, but they indicate Blizzard's stance. [Forum](https://us.forums.blizzard.com/en/blizzard/t/questions-about-blizzard-developer-api-terms-of-use/57897)

Reading (not legal advice): querying one's own row from the player's app, at their request and with caching, is similar to using the site in person. Crawling the whole leaderboard from a server, storing it and showing it on another site, or putting it in a paid extra, conflicts with "personal use only" and with the ban on redistribution. Decision: D-012 (closes P-008).

## Matching: local user ↔ leaderboard row

1. **Key:** local name without `#number` + region (`Hearthstone.log`) + mode (`GameType` of the game) + current season.
2. **Exact comparison** of `accountid` with the name. Case and Unicode normalization unchecked: until tested, exact and unnormalized.
3. **Result:**
   - One row → MMR with its `rank`, season and query time.
   - No row in all pages → "not in the public leaderboard (top ~8000)", no value.
   - Two or more rows with the same name → "ambiguous", no value. We do not pick by similar rating: that would be inventing.
   - Network error, invalid response or inconsistent pages → "unknown", different from "not there".
4. **Cost:** if the app remembers the user's last known `rank` or rating, it can go straight to that area (rows are sorted by descending rating) in 1–3 requests; otherwise it must walk from page 1. Strategy and measured figures: [`leaderboard-client.md`](leaderboard-client.md) (D-013).

## P-007: MMR outside the leaderboard

| Option | What it is | Pros | Cons |
|--------|------------|------|------|
| A. By hand | The user types the MMR they see in the game; the app shows it as "entered by you" with a date | Real data, seen on their screen (rule 3) | Gets stale after each game; depends on the user |
| B. Infer deltas | Estimate the change by placement | Automatic | Outside the leaderboard there is no base value, and the change depends on the lobby's average MMR, which is not in the log. It would be an invented value (D-005) |
| C. Don't show it | "No data" | Honest, zero maintenance | Worse experience for most (outside the top) |
| D. `[Net]` verbose | Read the rating if the game writes it | — | **Discarded:** tested on 2026-10-09, the game does not write it |

**Initial recommendation:** C by default with A as an option. Discard B and D.

**User decision (D-011, 2026-10-09):** the leaderboard is the guide. A variant of C: instead of "no data", someone who does not appear is shown as "below the cut-off" (< 8000), because few people play and being outside the top is already information. Conditions to avoid inventing:

| Situation | Shown |
|-----------|-------|
| One row with their name | Their rating, `rank`, season and query time |
| No row, whole leaderboard checked (all `totalPages` pages) | "Below the leaderboard cut-off (< 8000)", with the cut-off taken from the last row |
| Their name appears two or more times | "Ambiguous" |
| Network error, invalid response or incomplete walk | "Unknown" |

Cost: claiming someone is **not** there requires walking all pages (173 in Solo EU, ≈ 52 MB). T-006 does it at most once a day per region and mode; with gzip it is ~3 MB. `If-None-Match` is useless (the `ETag` changes on every response). See [`leaderboard-client.md`](leaderboard-client.md).

## Pending

- Go back to `Verbose=false` in `[Net]` (the experiment gave nothing).
- Leaderboard update frequency: in batches, between a few minutes and under an hour (T-006, measurements 9 and 10).
- Case and Unicode in `accountid` versus the local name (see [`leaderboard-client.md`](leaderboard-client.md#open-risks-and-doubts)).
