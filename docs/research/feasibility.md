# Feasibility of a Battlegrounds-only tracker

Date: 2026-10-08. ChatGPT research (codex, with web) cross-checked by Claude against GitHub, npm and official sites. Anything marked "unverified" was not checked against a primary source.

## How existing trackers get their data

- `Power.log` (enabled via `log.config`) plus reading the client's memory: HDT with HearthMirror, Firestone with MindVision.
- Overwolf deprecated the Hearthstone game events (GEP): date 6-Jul-2026, removal planned 10-Aug-2026, "lack of usage/players". [Overwolf](https://dev.overwolf.com/ow-native/live-game-data-gep/supported-games/deprecated/overview/)

## Licenses (verified)

| Component | License | Reusable? |
|-----------|---------|-----------|
| [python-hearthstone](https://github.com/HearthSim/python-hearthstone) | MIT | Yes |
| [python-hslog](https://github.com/HearthSim/python-hslog) | MIT | Yes (`Power.log` parser) |
| [hs-game-converter-csharp-port](https://github.com/Zero-to-Heroes/hs-game-converter-csharp-port) (Firestone) | MIT | Yes |
| [HSTracker](https://github.com/HearthSim/HSTracker) (macOS) | MIT | Yes, with care: it inherits a lot from HDT |
| [hearthstone-battlegrounds-simulator](https://github.com/twanvl/hearthstone-battlegrounds-simulator) | MIT | Yes, but check whether it is up to date |
| [HearthstoneJSON](https://hearthstonejson.com/) | Site CC0; card data © Blizzard | Data yes, art per the Fan Content Policy |
| [HDT](https://github.com/HearthSim/Hearthstone-Deck-Tracker) | "All Rights Reserved" (README) | No |
| [Firestone](https://github.com/Zero-to-Heroes/firestone) | No license; restrictive ToS | No |
| [@firestone-hs/simulate-bgs-battle](https://www.npmjs.com/package/@firestone-hs/simulate-bgs-battle) | "All rights reserved" (v1.1.770, updated 8-Oct-2026) | No |
| HearthMirror, BobsBuddy (HearthSim) | Private repos (404) | No |
| Nomi's Kitchen (HDT and Firestone) | "MIT NON-AI": forbids using the code with AI | Do not read or port |

## Blizzard

- The EULA forbids unauthorized processes that intercept or extract information; Blizzard may allow third-party interfaces. [EULA](https://www.blizzard.com/es-es/legal/588783f5-79da-4e1c-89dd-ebe212764dda/contrato-de-licencia-para-usuario-final-de-blizzard)
- The 2022 Lobby Legends rulebook allows trackers in tournaments if they show only what the player could see. [Rulebook](https://assets.blz-contentstack.com/v3/assets/bltc965041283bac56c/bltb47fca8192f92a36/622fc8f9da5a7125ed751ae9/Battlegrounds_Lobby_Legends_Rulebook_V_1.4.pdf)
- No reliable source was found of BG features removed at Blizzard's request.
- Since patch 33.2 (1-Aug-2025) the game includes a guide to minions and spells by tier and type. [Patch 33.2](https://hearthstone.blizzard.com/en-us/news/24223019)

## MMR: public leaderboard

Endpoint used by the site itself: `https://hearthstone.blizzard.com/en-gb/api/community/leaderboardsData?region=EU&leaderboardId=battlegrounds&page=1` (not officially documented; may change).

Query of 8-Oct-2026:

| Leaderboard | Accounts |
|-------------|----------|
| BG Solo EU | 4294 (the last one, with 8000 MMR) |
| BG Solo US | 1954 |
| BG Solo AP | 1821 |
| BG Duos EU | 1480 |

Conclusion: it covers only the top. Most players do not appear, so another source is needed for them (P-007). Columns: `rank`, `accountid` (BattleTag without number), `rating`; several players may share a name.

## Competition

- **Firestone:** the most complete (overlay, history, MMR, simulator, in-client replays announced 23-Sep-2026, standalone version in beta). Free with ads + Premium (price not retrieved).
- **HDT + HSReplay Tier7:** Bob's Buddy is free; Tier7 is paid. According to ChatGPT, $25 for 6 months (unverified: the page returned 403).
- **HSGuru:** donations, no paywall; no BG overlay of its own was confirmed.
- **BG Know-How:** abandoned in July 2025 "due to time constrains" (verified in its README).
- **Nomi.gg / Nomi's Kitchen:** free plugin for HDT and Firestone, plus an Android beta. Its "Fix Minion Dance" and "Disable abbreviation" run in a **BepInEx** plugin that is injected into the game client.

## Estimated effort (ChatGPT estimate, not data)

| Scope | Build | Maintain |
|-------|-------|----------|
| Proof of feasibility | 40–80 h | — |
| MVP without simulator | 250–500 h | 8–20 h/month |
| Solid product without simulator | 600–1,200 h | 20–40 h/month |
| Complete with simulator | 1,500–3,000+ h | 40–100+ h/month |

## Code signing (Windows)

| Option | Price | Notes |
|--------|-------|-------|
| [SignPath Foundation](https://signpath.org/) | Free | Open source only; automated build from the repo; key in SignPath's HSM. |
| [Certum](https://shop.certum.eu/code-signing.html) Open Source in the Cloud | from €49 | For free software. |
| Certum Standard (cloud / card / code) | from €209 / 169 / 139 | OV. Since 27-Feb-2026, maximum validity of 459 days per certificate (free reissues on 2–3 year plans). |
| Certum EV | from €329–379 | Unnecessary for this project. |
| [Azure Artifact Signing](https://learn.microsoft.com/en-us/azure/artifact-signing/faq) | Price not visible on the site | Individuals only in the US and Canada: not valid for an individual in Spain. |

Neither OV nor EV signing avoids the SmartScreen warning at first: reputation is earned through downloads (unverified against a primary source for 2026).
