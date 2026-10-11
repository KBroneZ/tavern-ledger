# What the app infers, and what memory could read instead (T-535)

Session 091, 2026-10-12. Research only: no code changed, no memory tool run, nothing read from HearthMirror, HDT or Firestone (rule 1).

**Work in progress.** Counts gathered so far, from the user's saved history (`%APPDATA%\TavernLedger\games.jsonl`, latest record per game, counted by a scratch script; counts only):

- 41 games (2 Solo, 39 Duos); parser revisions 2 (10 games), 3 (8), 5 (6), 6 (16), 7 (1).
- 468 rounds. Combat result from the log: 251 (every round of revision 5 and later). Not in the record (revision 2 and 3 games): 217, of which 176 come from the health rule (inferred) and 41 are unknown.
- 8 rounds missing from the record (gaps).
- Hidden combat boards (D-010, D-054): 200 own-side and 21 opponent-side entries with no board.
- Lobby tribes: inferred in every game (31 with a shop record, all five confirmed in each; 10 without a shop record). The fifth tribe is confirmed on shop turn 2 (3 games), 3 (3), 4 (5), 5 (6), 6 (5), 7 (6), 8 (2), 10 (1).
- Own hero skin grouping: 26 games on a skin; 16 named by the log's link, 10 by the D-019 rule (inferred).
- Free rerolls unknown on 183 of 345 shop turns.
- Hero pick, choices, Old God, keywords, triples and trinkets: recorded in 23 of 41 games ("not recorded" in the 18 older ones).
