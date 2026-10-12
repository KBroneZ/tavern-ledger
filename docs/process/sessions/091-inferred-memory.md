# Session 091: What the app infers, and what memory could read instead (T-535, research)

Model: opus
Effort: high
Subagents: haiku for searches; sonnet for the code reading

Work in your worktree **`C:\Users\andia\tavern-ledger\.claude\worktrees\091-inferred-memory`** (branch `091-inferred-memory`, created by the hub) and follow its `CLAUDE.md`. Chat in caveman mode; the user writes in Spanish, answer in English. Everything in the repo is in English. **Tier R3 for the decision** (it widens a Blizzard-limits exception): this session is **research only**; it changes no code and runs no memory tool.

**Other sessions at the same time:** 086 (combat replay, app crates), 087 (Dark Gifts, simulator), 089 (website wording), 090 (leaderboards). Your area: `docs/research/inferred-values.md` (new), plan row T-535, README line. Read everything else; change nothing else. Decision number reserved for the user's answer: **D-104** (do not write it; propose it). Every changed text file LF.

- The user may be busy: do not wait on them. Push the branch and message the hub (`tavern-ledger-2b`, `ListAgents`, use its `[ref]`) with the branch, PR title, a short PR body and the questions for the user.

## Why

The user (2026-10-12): "there are many 'inferred' things in the app that we could now get from memory; fix it." Today D-063 (amending D-004) allows memory reads of **exactly two values** the game shows the player: the lobby's five tribes and the player's own rating, read-only, off by default, never uploaded. Anything more needs **the user's explicit decision in `DECISIONS.md`**, and Absolute rule 3 still holds: no injection (D-006), no game file changes, no automated actions, and **never information the player could not see on their own screen**. So each candidate must be checked one by one before anything is built.

## Task

1. **Inventory** every value the app (window, recap, stats, overlay) and the website mark as inferred, estimated, "not recorded", "not shown" or unknown: what it is, where it shows, why it is inferred (what the log lacks), how often it is inferred in the user's history (counts only), and how wrong the inference can be. Read `crates/tracker/`, `crates/bg-parser/`, `crates/desktop/ui/`, D-010, D-054, D-057, D-058, D-063, D-069, `docs/research/` (memory and log notes).
2. For each value, answer: **(a)** does the game show it on the player's screen, and when (with how you know: public footage descriptions, patch notes, the user's screenshots in the hub's notes, not guesses); **(b)** could the log give it with better parsing instead (always preferred over memory); **(c)** is it plausibly reachable read-only from memory through the same public Mono structures as D-063, and how stable that would be across patches (from our own `crates/game-memory` design and public documentation only; **never read or port HearthMirror, HDT or Firestone code**, absolute rule 1).
3. **Recommendation table:** fix by log / candidate for memory (shown on screen) / leave as inferred / drop (not shown to the player: never). Order by value to the player and risk.
4. **Questions for the user**, ready for the hub to ask one at a time, each with a recommendation, for a D-104 that would amend D-063 for specific named values only.
5. Plan row T-535 (research done; build follows the user's answer), README.

## Acceptance criteria

- [ ] Inventory with counts from the user's history; each value classified with evidence.
- [ ] No code changed; no memory tool run; nothing read from HearthMirror, HDT or Firestone.
- [ ] Questions for D-104 ready; branch handed to the hub.

## Red line

Absolute rules 1 and 3; D-004 as amended by D-063; D-006; D-054. Never print BattleTags, `GameAccountId`, player names, keys or the user's email.
