# Leaderboard client (T-006)

Date: 2026-10-09. Python prototype, [`tools/leaderboard.py`](../../tools/leaderboard.py), with network-free tests in [`tests/test_leaderboard.py`](../../tests/test_leaderboard.py). Endpoint, response shape and terms: [`mmr-sources.md`](mmr-sources.md). Decisions: D-011 (four states), D-012 (allowed use) and D-013 (strategy).

No player name comes out of the measurements or enters the repo: the measurement scripts only printed counts, ratings and hashes, and did not save responses.

## Summary

- **Gzip works:** a page goes from 308 KB down to ~17 KB. Walking all of Solo EU costs ~3 MB, not 52 MB.
- **`If-None-Match` is no use:** the `ETag` changes on every request even when the content is the same. There is never a `304`. The client does not use it.
- **Ratings pile up near the cutoff:** from page 125 to 173 (≈ 1200 players) there are only 50 MMR points. One game moves a player in that zone by dozens of pages.
- **Consequence:** going straight to the known page only saves requests for those at the top. Near the cutoff, and to claim "below the cutoff", the full walk is needed (174 requests in Solo EU), at most once a day.

## Prior measurements

10 manual requests (budget: ≤ 10), Solo EU, season 19, with the project's User-Agent, 15 s timeout and 2 s pause.

| # | Time (local) | Request | Result |
|---|--------------|---------|--------|
| 1 | 00:49 | Page 1, no gzip | 200, 308,464 bytes, weak `ETag`, no `Cache-Control`, `Vary`, `Age` or `Last-Modified`. `totalSize` 4310 (4305 in T-004, hours earlier) |
| 2 | 00:49 | Page 1, `Accept-Encoding: gzip` | 200, `Content-Encoding: gzip`, **16,984 bytes** (18 times smaller), same decompressed body. `ETag` different from #1 |
| 3 | 00:49 | Page 1, gzip, `If-None-Match` with the `ETag` of #1 | **200, not 304**, a third, different `ETag` with the same body size |
| 4-8 | 00:50 | Pages 173, 150, 125, 100 and 70, gzip | Rating distribution (below). 0 repeated names within each page; 0-2 non-ASCII names per page |
| 9 | 00:53 | Page 150 again | Identical rows (same hash) as 3 minutes before |
| 10 | 01:49 | Page 150 again | **Different rows** (different hash, last row 8018 instead of 8017) and `totalSize` 4313 |

### Rating distribution (Solo EU, 4310 rows, 173 pages)

| Page | Ranks | Rating (first → last row) | Players per MMR point (approx.) |
|------|-------|---------------------------|---------------------------------|
| 1 | 1-25 | 19,118 → 15,009 | < 0.01 |
| 70 | 1726-1750 | 8506 → 8493 | ≈ 2 (pages 70-100) |
| 100 | 2476-2500 | 8157 → 8147 | ≈ 9 (pages 100-125) |
| 125 | 3101-3125 | 8052 → 8050 | ≈ 19 (pages 125-150) |
| 150 | 3726-3750 | 8019 → 8017 | ≈ 30 (pages 150-173) |
| 173 | 4301-4310 | 8000 → 8000 | — (last, 10 rows) |

Between 8000 and 9000 there are more than 120 of the 173 pages (page 70 is already below 8510). `rank` values are consecutive even with ties, and the client requires it: confirmed by the real test below with pages 170-173, all at or near 8000.

### Real client test

Outside the measurement budget: 4 requests from the client itself (01:04) with a hand-seeded cache (last rank on page 172, the day's walk already spent) and a name that belongs to no player. It requested pages 172, 171, 173 and 170; all passed the strict validation (rows per page, consecutive `rank`, rating order, season and region) and, since it did not find the name, it returned the saved state with its time. No errors or retries.

### Update frequency

Page 150 (dense zone, where any change shows) returned the same rows at 00:50 and 00:53: the leaderboard does not change every few seconds. At 01:49 (56 minutes later) it had already changed: different rows and `totalSize` from 4310 to 4313. Between the T-004 query and this session's it had gone from 4305 to 4310.

Conclusion: the leaderboard updates in batches, with a frequency between a few minutes and under an hour. Pinning it down would take many more requests and does not change the strategy: querying more than once every few minutes adds nothing (hence the 5 min minimum wait), and a full walk (~7 min) may coincide with a batch, a case the last-page check detects.

## Strategy (D-013)

The client stores, per region and mode, only its own state: state, rating, rank, cutoff, season, number of pages, query time, time of the last full walk and a hash of the name (to know whether the cache belongs to another account without storing the name). No rows from other players.

1. **Less than 5 min since the last attempt:** returns what is stored, 0 requests.
2. **Last state "rating" with a full walk from ≤ 7 days ago:** requests the page of the last rank and up to two on each side (max 5). If the name appears once, that is the rating; if it appears twice on the same page, "ambiguous". That there is no other one in the rest is guaranteed by the last full walk.
3. **If it cannot be resolved that way** (no state, it was "below", "ambiguous" or "unknown", the player is not in the window or the season changed): full walk, **at most one a day** per region and mode (UTC day). Page 1 to learn the size, from the last page down to 2, and the last page again to check that the leaderboard did not change during the walk.
4. **If today's walk is already spent:** returns the last stored state **with its time** ("as of"), or "unknown" if there is none. Never a made-up new value.

Limits (`Limits` in the code): 1.5 s between requests, 15 s timeout, 2 retries waiting 5 and 15 s (only network down, timeout or 5xx), 60 targeted requests per day per region and mode, 600 pages at most per walk. A 403 or 429 stops immediately and pauses all queries for 1 hour. Any other code (including a `304` that is never requested) or a strange response stops immediately with "unknown".

### Expected requests (Solo EU, 173 pages)

| Case | Requests | Approx. time |
|------|----------|--------------|
| No previous state (first time, cache deleted, new season) | 174 | ~7 min (1.5 s pause + ~1 s response) |
| At the top, after a game | 1 (up to 5 if it changed page) | 1-12 s |
| Near the cutoff (≈ 8000-8150), after a game | 5 and, if not in the window, 174 the first time of the day; afterwards, the stored value with its time | — |
| Below the cutoff | 174 the first query of the day; afterwards, the stored state with its time | ~7 min |
| Daily cap per region and mode | 174 + 60 targeted (+ retries) | — |

A player at ~8100 (≈ 10-20 players per point) moves ±50 ranks with only ±3-5 MMR points: the ±2 page window almost never finds them after a game. At the top (page 1: 160 points per place) the first request is enough.

### Discarded options

| Option | Why not |
|--------|---------|
| `If-None-Match` / `304` | The `ETag` changes on every response (measurements 1-3) |
| Look only at the last pages to keep saying "below" after a game | Whoever enters does so from the bottom, but with 20-30 players per point, +50 MMR is 40-60 pages above the end: almost a whole walk, and it would also rest "below" on an assumption about how much is gained per game (D-005). D-011 asks to check all pages |
| Binary search by rating | Requires knowing the new rating; estimating it would be inferring deltas (discarded in D-011) |
| Stop the walk when the name is found | It would not be known whether the name is repeated ("ambiguous") |

## Validation of each response

Anything odd gives "unknown", never a value:

- JSON object, `region` equal to the one requested, `seasonId` integer ≥ 1, `rows` list, `totalPages` and `totalSize` integers (not accepting `true` as 1) with `totalPages = ⌈totalSize / 25⌉`.
- 25 rows on each page except the last, which has `totalSize − 25·(totalPages − 1)`.
- `rank` consecutive from `25·(page − 1) + 1`; `rating` non-increasing within the page and across pages; name non-empty text of ≤ 64 characters.
- Empty page within range → error. Out of range (200 with `rows: []` and `totalPages: 0`) → in the targeted search it means the leaderboard shrank and it falls back to the full walk; in the full walk, error.
- During a walk: same season, `totalPages` and `totalSize` on all pages, and the last page identical at the start and at the end.
- Transport: no cookies (not stored or resent), no following redirects, at most 1 MB compressed and 2 MB decompressed, truncated gzip or unknown encoding → error. Error messages do not repeat server text.

## Open risks and doubts

- **Rows crossing pages during a walk.** If someone moves up or down next to the player between two requests, the player may appear twice (false "ambiguous") or not at all (false "below"). The last-page check detects a new batch during the walk (measurements 9 and 10: the leaderboard changes in batches, between minutes and under an hour). If the batch only changes middle pages and not the last one, it is not detected; low residual risk, and the next day's walk fixes it.
- **Case and Unicode.** The comparison is exact, without normalizing (D-011). 0-8 % of the names on each page have non-ASCII characters. If the local log and the web wrote the same name with a different Unicode normalization, the client would say "below" instead of the rating. Check with the own row of a user who is at the top before porting.
- **Name repeated between walks.** If another player with the same name enters after the last full walk and outside the window, the targeted search would give the own rating without seeing the other. The daily walk fixes it; confidence expires after 7 days.
- **A failed walk spends the day's.** On purpose: a day with an unstable network does not turn into several walks.
- **US and AP not measured.** If they have more than 600 pages, the client answers "unknown" (sanity limit).

## Usage

```
python tools/leaderboard.py --name "<your BattleTag>" --mode solo --region EU
python tools/leaderboard.py --name "<your BattleTag>" --mode duos --hearthstone-log "<Logs>\Hearthstone_<date>\Hearthstone.log" --json
```

It reads the region from `Hearthstone.log` only with the closed pattern `Region: XX` in the first 500 lines. It does not read `Net.log`. The cache goes in `.local/leaderboard-cache.json` (outside git).
