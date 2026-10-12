# Session 004: Leaderboard client (T-006)

Model: sonnet
Effort: medium
Subagents: haiku

Work in `C:\Users\andia\tavern-ledger` and follow its `CLAUDE.md`: caveman in chat; documents in Spanish; public README and product in English. **Tier R2** (network): TDD + `/code-review` + `/security-review`. Branch and PR; never push directly to `main`.

**Red line (D-004, D-006):** local logs only, plus the public endpoint the web page itself uses. No memory reading, injection or game automation.

## Effort and models per phase

| Phase | Effort | Subagents (if needed) |
|-------|--------|-----------------------|
| Reading, D-012 and plan | low | Haiku (searches) |
| Preliminary measurements (gzip, rating distribution) | medium | — |
| Tests and client | medium | Sonnet if code needs to be delegated |
| Reviews (`/code-review`, `/security-review`) | high | — |
| Docs and PR | low | — |

**Autonomy:** Claude decides the internal design, names and concrete limits within what is below. **The user approves** any new dependency and the PR merge.

## At the start

1. Title **`#004 Leaderboard client`**.
2. Check that PR KBroneZ/tavern-ledger#3 is merged and `main` is up to date. If not, warn.
3. Read: `README.md`, `docs/plan/plan.md` (T-006), `docs/decisions/DECISIONS.md` (D-003, D-005, D-011, P-008) and all of `docs/research/mmr-sources.md`.
4. Run `python tools/check_logs.py` and note in the plan if there are new games.
5. This prompt file goes into this session's PR.

## First: record D-012 (decision already made)

The user closed P-008 in session 003, after the merge. Move it from pending to made, dated 2026-10-09, in this session's PR:

> **D-012.** The leaderboard MMR is a free feature (D-003). The query comes only from each user's desktop app, for their own row, on request or after a game, with cache and the minimum number of requests; no crawling from a server, no storing other players' rows and never in paid extras. The concrete design for spending few requests is up to Claude. Closes P-008.
> Reason: user's decision; the Blizzard sites' terms grant a "personal use only" license and exclude commercial use and downloading except page caching.

## Context

- Endpoint, parameters, response shape and behavior: `docs/research/mmr-sources.md`. Note: an out-of-range page returns **200 with `rows: []`**; there is no search by name; ~300 KB per page; no rate-limit headers; weak `ETag`; it sets cookies that must not be stored.
- D-011 fixes the four output states: rating (one row), "below the cut-off" (no row after checking **all** pages; cut-off = rating of the last row), "ambiguous" (repeated name), "unknown" (error or incomplete traversal).
- Local input data: name (from `Power.log`, without `#number`), region (`Region: XX` in `Hearthstone.log`, around line 48), mode (`GameType`). `Net.log` carries an authentication token: **it is not read**. Never print BattleTags or `GameAccountId` in chat, docs, tests or tool logs.
- Rule 4 of `CLAUDE.md`: cache, few requests, timeout, identifiable User-Agent (`TavernLedger/<version> (+https://github.com/KBroneZ/tavern-ledger)`).
- The desktop stack (P-002) is still undecided: the client is a **Python prototype** like `tools/parse_bg.py`, meant to be ported. What is valuable is the design, the states and the tests.

## Task

1. **Preliminary measurements (few manual requests, ≤ 10 in total):**
   - Does it respond compressed with `Accept-Encoding: gzip`? Bytes with and without.
   - Does it respond `304` to `If-None-Match`? How often does a page's `ETag` change?
   - Rating distribution per page near the cut-off (how many pages are there between 8000 and 9000 in Solo EU), to size the strategy. Counts only; do not store real responses.
2. **Few-requests strategy** (Claude decides, document it with numbers). Starting points:
   - If the user's last `rank`/rating is known, request first the page where it should be and the neighboring ones (rows are sorted by descending rating).
   - If the last state was "below the cut-off", after a game it is enough to look at the last pages near the cut-off (whoever gets in does so from the bottom), unless a lot of time has passed.
   - Full traversal only when there is no previous state or the cache has expired, and at most once a day per region and mode.
   - Pause between requests, daily request cap, `timeout`, retries with backoff and stop at the first serious error → "unknown".
3. **Client** `tools/leaderboard.py` (read-only CLI + testable functions):
   - Injectable transport (no network in tests). Prefer the standard library (`urllib`); `requests` is already pinned in `requirements.txt` as a transitive dependency, but using it directly counts as a new dependency → ask.
   - Strict response validation (types, increasing `rank`, non-increasing `rating`, consistent `totalPages`); anything odd → "unknown", never a value.
   - Local cache in `.local/` (already in `.gitignore`) with only what is necessary: own state, cut-off, `ETag` and time. No rows from other players.
   - Exact name comparison (D-011); document the uncertainty about case/Unicode.
   - Output without names: state, rating, rank, cut-off, season, region, mode, time and number of requests made.
4. **Tests** (`tests/test_leaderboard.py`, unittest) with **synthetic** responses (invented names), written before the code: the four states, empty out-of-range page, repeated name, incomplete traversal, malformed response, timeout, `304`, expired cache, request caps.
5. **Docs:** client section in `docs/research/mmr-sources.md` (or a new report if it grows), `PROVENANCE.md` if anything is added, plan (T-006), `DECISIONS.md` (D-012 and whatever comes up) and README (usage command).

## Out of scope

- Fixing `tools/parse_bg.py` for Solo (final health and repeated placements): goes in another session.
- Choosing the stack (P-002), app, web or overlay.
- Any server, account, spending or acceptance of terms.

## Acceptance criteria

- [ ] D-012 recorded and P-008 removed from pending.
- [ ] Preliminary measurements documented (gzip, `304`, distribution near the cut-off) with the number of requests used.
- [ ] Strategy documented with expected requests per case (user in the top, below the cut-off, no previous state).
- [ ] Client with the four D-011 states; never a number from outside the leaderboard.
- [ ] Tests without network, green; `/code-review` and `/security-review` with no open CRITICAL or HIGH findings.
- [ ] No player name or real response in the repo.
- [ ] Plan, decisions and README up to date in the same PR; CI green.
