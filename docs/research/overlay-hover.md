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

## Session 058 (2026-10-11): wrong board, board across the top, text size, Simple and Advanced (T-333 to T-335, D-067)

### Why the hover sometimes showed another opponent (T-335)

The game writes every packet from the server twice to `Power.log`: when it arrives (`GameState.DebugPrintPower`, the lines the parser reads) and when the client plays it on screen (`PowerTaskList.DebugPrintPower`). A Battlegrounds combat arrives whole when it starts, so the log already has its outcome (health, `PLAYER_LEADERBOARD_PLACE`, the next turn) while the game is still playing the fight. On the user's three sessions of 2026-10-11 (counts and times only), each place change reached the `GameState` lines a median of 17 to 25 s, and up to 72 s, before the matching `PowerTaskList` line. During that time the overlay matched the hovered slot with the new places while the screen still showed the old order: a reproduction on the real Duos fixture has teams 3 and 4 swapped in the log while the screen still showed them the other way round. It also meant the overlay showed a combat's result before the game did.

Both kinds of line carry the same `TAG_CHANGE`s in the same order: 98,716 of each in one session, and the same count at every game start and at the end of all three sessions; at most about 10,800 were waiting to be played. The fix (`crates/tracker/src/playback.rs`) keeps each view of the live state with the number of arrived `TAG_CHANGE`s it was built from and shows the newest one the played lines have reached. A log without played lines is not held back, and a view waiting more than 120 s is shown anyway, so a log that never catches up cannot freeze the overlay.

In Duos the hover card now gives each hero of the team their own row with their own board, matched by seat, so a teammate's board is never shown as if it were the hovered hero's. Which of a team's two portraits is on top is not in the log, so the hover still works per team.

### What the other trackers show (public material only)

Rule 1 of `CLAUDE.md` holds: no code of HDT, Firestone or their tools was opened, and for Nomi's Kitchen only its public site was read, not its code or its repository.

- **Hearthstone Deck Tracker (HearthSim).** Release notes v1.28.1 (page dated "18 May", no year shown; <https://git.tdem.in/HearthSim/Hearthstone-Deck-Tracker/releases/tag/v1.28.1>): "Hover over heroes on the leaderboard as usual to see the last board you encountered", last-known boards for Duos, and a fix for "the last known board state in Battlegrounds sometimes showing a board from the wrong hero" (the same kind of bug as T-335). HSReplay's overlay page (<https://hsreplay.net/battlegrounds/overlay/>, search snippet only; the site did not load) calls it "last known board" with minions, stats and buffs. Where it appears on screen and how big it is was not found in any public text; the user's own description is "the board shown at the top, like in Deck Tracker".
- **Firestone (Zero-to-Heroes).** Feature wiki (mirror updated 2025-05-28, <https://github-wiki-see.page/m/Zero-to-Heroes/firestone/wiki/Firestone-features>): hovering the leaderboard shows the last opponent's info with their last board, and whether they levelled up or made triples since; battle odds can be set to show only after the fight; a minion list by tier filtered by the lobby's tribes; the Battlegrounds window is meant for a second screen and can be an overlay on one screen. No size or scale setting was found.
- **nomi.gg (Nomi's Kitchen).** Home page (Patch 36.6.3, no date, <https://nomi.gg/>): an overlay with "Opponent boards, lobby tribes, fight odds and hero stats", set up in "about a minute", free, "shows full stat numbers". No public detail on the hover layout or settings was found.

What we took: the board where the game's own board strip is (across the top), one row of up to seven minions with attack and health as the game draws them (HDT, the user's request); the turn it was seen, said as "2 turns ago" (Firestone's "since you last fought them"); few words by default and the rest behind a switch, as the user asked ("things must be for people who are not so pro"). Nothing about how their overlays look in detail was found in public text, so the layout is ours, in the Lamplight glass look (D-052).

### Board across the top (T-333) and text size (T-334)

- The card asks for 85% of the game window's height as its width (the game lays its board out by the height) and sits at the top of the game window, centred, inside the screen (`hover.rs`, `screen_fit::place_card`). It is click-through and shows only while the mouse rests on the leaderboard, so it never blocks play.
- Each minion: picture (the 256x art cropped as in 053 and 056, no white edges), attack bottom left and health bottom right in round badges, golden as a ring and a star, keyword letters (Advanced), name under it. A Duos team has two rows, square pictures so they take less height.
- Text follows the screen: the page's base size is 1.32% of the overlay window's height (at least 13 px), the card's is 1.65% of its width (`overlay-scale.js`). Tested: at 3840x2160 at 150% the body text is about 28 physical pixels (1.3% of the screen's height) and the card's about 30; at 1920x1080 at 100%, 14 and 15. The game's own leaderboard text was not measured (no full screenshot), so 1.25% of the height is the floor the tests check.
- Rendered off screen (headless Edge, the real overlay page with its calls stubbed and invented data) at 2560x1440 at 150% and 1920x1080 at 100%, Solo and Duos, Simple and Advanced, English and Spanish.

### Simple and Advanced (D-067)

Simple (the default) shows the board, the hero, tier, health, record, triple and trinket counts, and the turn the board was seen, with every source mark but the card-database word ("inferred", "unknown", "from the leaderboard"). Advanced adds keyword letters, trinket names, the minions an opponent could have (always labelled "possible", D-050), the card-database word (the legend credits the card data) and the keyword key in the legend. The switch is in the overlay's toolbar (unlock the overlay to see it) and is kept in `overlay.json`; a switch in the Settings tab is left for after session 055's work there is merged.
