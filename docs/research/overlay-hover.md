# Overlay v2: hover on the game's leaderboard (T-306, T-307, T-308)

Session 033, 2026-10-10. Decision D-045.

## What the user asked for

From the first live test (plan, "Notes from the user's first live test"): no panel listing every opponent's minions; instead, hovering a hero on the game's own leaderboard shows that opponent's last-seen board next to it, as Hearthstone Deck Tracker does. At tavern tier 1 or 2 the same hover lists the minions they could have from the lobby's tribes. The overlay must never be larger than the screen, and its switch must be easy to find.

## Public sources (idea only, no code read)

- Hearthstone Deck Tracker release notes, v1.28.1 (HearthSim): "hover over heroes on the leaderboard as usual to see the last board you encountered", and last-known boards in Duos. Mirror of the release page: <https://git.tdem.in/HearthSim/Hearthstone-Deck-Tracker/releases/tag/v1.28.1>. HDT's code is "All Rights Reserved" and was not opened (CLAUDE.md rule 1).
- HearthstoneJSON's card documentation, <https://hearthstonejson.com/docs/cards.html>, documents `race` but no Battlegrounds field. The fields were checked on the file itself (`https://api.hearthstonejson.com/v1/latest/enUS/cards.json`, fetched once into a temporary folder, not kept): 303 cards have `isBattlegroundsPoolMinion: true`; each has `techLevel` (1 to 7: 26, 41, 53, 69, 62, 38 and 14 cards); 272 have `races` (a list, also `race`), 31 have none (neutral), 6 are `ALL`; 30 have `isBattlegroundsDuosExclusive`. Tribe names match the log's `CARDRACE` values (`BEAST`, `MECHANICAL`, …).

Firestone and Nomi's Kitchen were not looked at.

## What the log gives

Checked on the user's own logs of 2026-10-09 and 2026-10-10 (counts only, nothing printed or kept):

- `PLAYER_LEADERBOARD_PLACE` and `PLAYER_TECH_LEVEL` are on every lobby hero entity and change as the game goes. Sampling every 500 lines of a Solo game, 402 of 406 samples had places 1 to 8 with no gap and a tier for every hero; in a Duos session, 1923 of 1927 samples had places 1,1,2,2,3,3,4,4 (both heroes of a team share their team's place) and 1919 had every tier. The few other samples are the moments around hero select and the end.
- The parser already kept every tag on every entity, so `LogReader::lobby_now` only reads them from the leaderboard heroes it already picks for the report.

## Where the leaderboard is

The game lays out its board by the window's height and centres it, so the leaderboard is a fixed share of the height, left of the centre. Measured on the user's 3840x2160 screenshots of the live test (crops of the leaderboard and of the old overlay panel over it): portraits' left edge near x 505, slots about 185 px apart, the first near y 350, the eighth ending near y 1800. As shares of the height: left edge `0.665 x height` left of the centre, width `0.085`, top `0.16`, height `0.68`. In Duos the same column holds four teams of two portraits, so it is split into four slots.

These numbers are an estimate from cropped screenshots, and a patch can move the leaderboard. So:

1. the numbers live in one place (`crates/desktop/src/leaderboard.rs`) with tests;
2. while the overlay is unlocked, a dashed box shows where the app thinks the leaderboard is; the user drags or resizes it over the real one and it is kept for that game window size in `overlay.json`; "Reset leaderboard box" goes back to the measured default.

Still to check: a full-screen screenshot at 3840x2160 in Solo and in Duos (the user was asked).

## Screen bounds

- The overlay window covers the work area of the monitor holding most of the game window (else the main monitor). Moving a window to a monitor with another DPI makes Windows resize it after the app's own call; found in the replay check (the page was 688 px tall instead of 1032), so the window's real position and size are checked on every pass and set again.
- Panels are cut at the bottom of the screen; the hover card goes right of the leaderboard, or left, or narrower, and is centred on the slot but kept inside the screen.

## Checks

Log replay (T-D02) of the user's Solo game cut at round 3, with a plain stand-in window titled "Hearthstone" (the app's `--overlay-dev` finds it by title; it draws nothing of the game) and the mouse moved by a script:

- 1920x1080 window on the 4K monitor (150 %): slot 3 showed the third-place opponent with the board "seen in round 3" and 36 possible tier 1-2 minions from the tribes seen in the tavern (inferred); slot 1 showed "no board seen yet".
- 1280x720 window: slot 5 showed the fifth-place opponent; off the leaderboard the card hid.
- The same window on the second monitor (1920x1080, 100 %): the overlay moved to that monitor's work area; slot 2 showed the second-place opponent; our own slot (8th) showed nothing.
- Unlocked, with no game: the box showed over the stand-in's leaderboard area, a drag of 20 px right and down was saved exactly in `overlay.json` for `1280x720`.
