# Session 066: Game viewer on the website (T-504, T-201)

Model: opus
Effort: high (the report reader and the CSP change), medium for the rest
Subagents: haiku for searches; sonnet for code and reviews (`code-reviewer`, `a11y-architect`); opus for `security-reviewer`

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\066-web-game-viewer`** (branch `066-web-game-viewer`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R2** (a page that reads user data, a CSP change): TDD + `/code-review` + `/security-review`.

**Another session runs at the same time:** 065 (personal dashboard, public profile and games list, also in `web/`). To stay out of its way:
- Work only in your worktree. Never touch the main checkout or other worktrees.
- Your files: new ones under `web/src/pages/game*`, `web/src/components/game/`, `web/src/lib/game/` (your strings go in `web/src/lib/game/strings.ts`), plus **you own** the picture source (`web/src/lib/data/pictures.ts`) and the CSP (`web/src/lib/config.ts` CSP lines, the CSP tests in `web/test/build/build.test.ts`). 065 owns `/me/`, the profile and the games list. Shared design-system files (`web/src/components/data/`, `web/src/styles/data.css`, `web/src/lib/data/strings.ts`, `format.ts`): add, do not rewrite; if you need a change there, keep it small and tell the hub so 065 knows.
- The games list (065) will link each row to your page: agree the URL with 065 through the hub early (for example `/game/?u=<user>&s=<session>&g=<index>` on a static site). Say it to the hub in your first hour.
- Nothing in `crates/`. `supabase/` only if the viewer cannot work without a change; then stop and tell the hub first (an RLS change is R2 plus a second AI).
- Shared files (`plan.md`, `README.md`, `DECISIONS.md`): only your task's rows and notes. Your decision number is **D-079**, only if needed (065 has D-073; D-074 to D-078 are the user's website decisions). Keep every changed text file LF (`git ls-files --eol`, index side; the checkout uses `core.autocrlf=true`).
- To catch up with `main`: `git fetch origin && git merge origin/main`. No rebase, never force-push. Stop any dev server you start before you hand over.

Working rules (user's decisions):
- Decide technical and design matters yourself (Lamplight glass D-052, the design system D-070). Ask the user, **through the hub**: new dependencies, anything that shows more than the data rules allow. **The CSP change in D-074 is already approved** (one image host, below); any other CSP change needs the user's OK.
- The user is asleep: do not wait on them. Put open questions in your hand-over.
- `gh` is not available in task sessions. When your branch is ready (tests green, reviews done, no open CRITICAL or HIGH), push it and message the hub (`tavern-ledger-2b`, find it with `ListAgents`, use its `[ref]`) with the branch, the PR title and a short PR body (summary, review results, test plan). The hub opens the PR, waits for CI and merges it. **Merging to `main` deploys the site:** afterwards check every page answers 200 and the CSP header is exactly the old one plus the image host, and tell the hub.

## Why

Round 5 puts everything the app knows on the website (read **`docs/plan/web-master-plan.md`**, "Information architecture": the `/game/<id>/` row). The game viewer shows one game in full, the way the app's recap does (D-062, D-065): summary bar, turns timeline (result, damage, board), boards met, shop per turn behind "See shops", choices, lobby with triples and trinkets; Simple and Advanced.

## At the start

1. Conversation title: **`#066 Web game viewer`**.
2. `git fetch origin && git merge origin/main`.
3. Read the master plan, D-038, D-052, D-062, D-065, D-070, D-071, **D-074**, `docs/research/web-data-api.md`, the report shape (`crates/bg-parser` report types, `supabase/functions/upload-game/report_keys.json`, synthetic fixtures only), the app's recap (`crates/desktop/ui/`, for what it shows and how), and `web/src/components/data/`.

## Task

1. **Reading one game.** The full report is a gzipped file in Storage (`games` bucket, policy `games_bucket_read_own_or_public`: the owner, or everyone when the profile is public); `game_summaries` has its `storage_path`. Fetch it with the publishable key (and the session when signed in), decompress in the browser (`DecompressionStream`, no new dependency), validate its shape before use (untrusted input: unknown keys ignored, bad values shown as unknown, never a crash), size-capped.
2. **The page:** summary bar (hero, place, mode, date, rounds, key numbers with their game counts), turns timeline (per turn: result from the log or inferred and marked, damage, tavern tier, gold), boards met per opponent (by hero, never by player name), the player's board per turn, shop per turn folded ("See shops"), choices (discovers, trinkets, quests, Dark Gifts) and the lobby (heroes with triples and trinkets); Duos teammate. Simple shows the summary and the timeline; Advanced the rest. Per-turn bars from the design system.
3. **Pictures (D-074):** switch `pictures.ts` to HearthstoneJSON's image host (`art.hearthstonejson.com`, the URL pattern and size the app uses, D-038) and add exactly that host to the CSP's `img-src`; keep the local placeholder as the fallback when a picture fails or a card id is unknown; the crop with no white edges (D-065). The fan content notice stays in the footer. Update the build tests that pin the CSP and the `<img>` rule.
4. **Privacy:** the owner, or a public profile, only; a private or missing game says so without hinting at data. No other player's name anywhere (the report has none; check it stays so). No BattleTags, no ids beyond what the URL needs.
5. **States:** loading with reserved space (CLS 0), not found, private, a report from a newer parser than the site knows ("made by a newer app version", shown as far as known), network error with a retry.
6. Accessibility WCAG 2.2 AA (axe clean), keyboard, reduced motion; phone and desktop; within the performance budget (the report is fetched once, no layout shift).
7. Tests: unit tests for the report reader and validation (synthetic reports, including broken and oversized ones) and the page's data shaping; the build tests in every build mode. Plan rows **T-504** and **T-201** done with notes.

## Out of scope

Combat replay attack by attack (T-215), heroes and curious-stats pages (T-505), leaderboards, Spanish strings (T-508; keep strings in one place), `supabase/` changes without the hub, any other CSP change.

## Acceptance criteria

- [ ] One game shown in full from its stored report, Simple and Advanced, for the owner and for a public profile; private stays private; all states.
- [ ] Pictures from HearthstoneJSON with the placeholder fallback; CSP = old header + that one host (diff pasted).
- [ ] axe clean, keyboard, CLS 0; `test` and `test:dist` green (paste counts).
- [ ] `/code-review` and `/security-review` with no open CRITICAL or HIGH; branch handed to the hub; post-merge checks reported.

## Red line

No real player data in fixtures or screenshots, no Blizzard art in the repo, no CSP change beyond D-074's host, no new dependency without the user's OK; never print keys, tokens, the user's email or BattleTags.
