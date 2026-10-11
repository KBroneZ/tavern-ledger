# Dark Gifts: the list, how the log carries them, and the simulator

Session 087, 2026-10-12. Task T-527, decision D-100. The parser and display side is T-528 (section 5 is its brief).

## Summary

- Season 36 has **47 Dark Gifts** in the card data (gift spells `BG36_MidGameEffect_000t…`, offered by the Dark Discovery `BG36_MidGameEffect_010`). **23 act in combat**: 15 change the combat (start of combat, Deathrattle, Reborn, shield, attack, type, golden), 3 only give keywords the board already shows, and 5 are a Rally or Deathrattle whose effect lands outside combat. The other **24 are recruit-phase only** or only stats the board already shows.
- On the board a gift is an **enchantment attached to the minion** (`ATTACHED` = the minion), not `DARK_GIFT_ENTITY`, which only the offers carry. The minion also has `HAS_DARK_GIFT` 1. Combat copies carry both, for the player's and the opponent's minions alike.
- In the user's logs on the PC (6 sessions, 8 games, 84 combats), **53 combats (63%) had a gifted minion**: 33 with a gift that changes the combat, 12 with one that matters on a board taken after start of combat (the boards the app keeps). Until T-528 the simulator played those minions as the plain card, silently.
- The engine now knows every gift: 22 of the 23 with their combat effect (2 approximate: Persisting Horror, and Gilding on a card whose golden card is not modelled), Worship blocks the odds when start of combat is played (its Deity's stats are on no board), the 24 others are known and change nothing; a gift it does not know blocks the odds by name, as a card does.

## 1. Clean room and sources

Gift names and effects from HearthstoneJSON's card data (`cards.json`, enUS and esES, build of 2026-10-11), read only for this note; the summaries below are in our own words. Log facts from the user's own `Power.log` files on this PC (the six sessions of 2026-10-11 still on the PC, 475 MB), read-only by throwaway scripts that print tag names, counts and card ids only: no names, BattleTags or account ids. No tracker or simulator code was read (CLAUDE.md rule 1).

## 2. The gifts

"Acts" says when the gift does something; "Engine" is how `crates/combat-sim/src/gifts.rs` plays it. "Offers": how many Dark Discovery offers in these logs named the gift (54 of the 106 `DARK_GIFT_ENTITY` tags the scan saw named a gift entity it had seen created; the other 52 were on removed copies). "In combat": gifted minions on boards at the first attack of a combat, the player's own / the opponent's. Gifts with a `t` after the number (`000t28t`, `000t29t`, `000t30t`, `000t64t`) are weaker or stronger versions with the same name; the logs show `000t29t` offered and `000t30t` and `000t64t` on boards.

| Gift id (`BG36_MidGameEffect_` + …) | English | esES | What it does (summary) | Acts | Engine | Offers | In combat (own / opp) |
|---|---|---|---|---|---|---|---|
| `000t` | Offensive Sacrifice | Sacrificio ofensivo | +10 Attack; Deathrattle: its Attack goes to another random friendly minion | Deathrattle | Deathrattle: give own Attack | 7 | 1 / 2 |
| `000t2` | Defensive Sacrifice | Sacrificio defensivo | +10 Health; Deathrattle: its full Health goes to another random friendly minion | Deathrattle | Deathrattle: give own full Health | - | - / 3 |
| `000t3` | Charisma | Carisma | Rally: a random minion of your most common type to the hand | Rally (hand) | Rally with no combat effect | 2 | 1 / 7 |
| `000t4` | Incubation | Incubación | +4/+4, stats doubled two turns later | recruit | none (stats on the board) | 1 | - / 2 |
| `000t5` | Mystic Essence | Esencia mística | Deathrattle: a random Tavern spell to the hand | Deathrattle (hand) | Deathrattle with no combat effect | 3 | 3 / 1 |
| `000t7` | Resistance | Resistencia | Start of Combat: double its Health | start of combat | doubles Health | - | 1 / 1 |
| `000t9` | Admiration | Admiración | Start of Combat: gain the Attack of the minion to the left | start of combat | gain left Attack | 4 | 6 / 6 |
| `000t10` | Echoing Voice | Voz resonante | end of turn: trigger its Battlecries | recruit | none | - | - / 2 |
| `000t11` | Double Vision | Visión doble | an extra copy when you get it | recruit | none | 6 | - / - |
| `000t12` | Persisting Horror | Horror persistente | Reborn; comes back with full stats and its bonus keywords | Reborn | Reborn whole (approximate) | - | 2 / 1 |
| `000t13` | Harpy's Talons | Garfas de arpía | Divine Shield and Windfury | keywords | keywords | 5 | 2 / 17 |
| `000t14` | Gilding | Doración | golden (no Triple reward) | golden | golden card's rules (approximate if not modelled) | - | - / 2 |
| `000t15` | Toreth's Blessing | Bendición de Toreth | its Divine Shield takes 3 hits to break | shield | shield takes 3 hits | 2 | - / - |
| `000t16` | Jaws of Death | Fauces de la Muerte | Start of Combat: trigger its Deathrattles | start of combat | trigger own Deathrattles | 3 | 1 / 5 |
| `000t18` | Replication | Replicación | every 2 turns, a plain copy to the hand | recruit | none | 2 | 4 / 4 |
| `000t21` | Time Turning | Tiempo volteado | its end-of-turn effects also at start of turn | recruit | none | 2 | 3 / - |
| `000t22` | Amalgamation | Amalgamado | has all minion types | type | all types | - | 1 / - |
| `000t28` | Battle Scars | Cicatrices de batalla | +3/+3 per Battlecry triggered this game | recruit (stats) | none (stats on the board) | - | - / - |
| `000t28t` | Battle Scars | Cicatrices de batalla | +2/+2 per Battlecry triggered this game | recruit (stats) | none (stats on the board) | - | - / - |
| `000t29` | Death's Embrace | Abrazo de la Muerte | +2/+2 per Deathrattle triggered this game | recruit (stats) | none (stats on the board) | 2 | - / 1 |
| `000t29t` | Death's Embrace | Abrazo de la Muerte | +1/+1 per Deathrattle triggered this game | recruit (stats) | none (stats on the board) | 1 | - / - |
| `000t30` | Spell Siphon | Succión de hechizos | +3/+3 per Tavern spell cast this game | recruit (stats) | none (stats on the board) | 2 | 2 / - |
| `000t30t` | Spell Siphon | Succión de hechizos | +2/+2 per Tavern spell cast this game | recruit (stats) | none (stats on the board) | - | 2 / - |
| `000t50` | Tarecgosa's Blessing | Bendición de Tarecgosa | keeps its bonus keywords and doubles the stats it gains in combat, after the combat | after combat | none | - | 1 / - |
| `000t51` | Steady Growth | Crecimiento constante | end of turn: gains stats | recruit | none | - | - / 1 |
| `000t52` | Fresh Perspective | Perspectiva renovada | Deathrattle: 2 free Refreshes | Deathrattle (shop) | Deathrattle with no combat effect | 3 | 2 / 2 |
| `000t60` | Invulnerability | Invulnerabilidad | Immune while attacking | attack | no damage back when it attacks | - | - / - |
| `000t61` | Golemancy | Golemancia | Deathrattle: summon a Golem with its stats | Deathrattle | summon `BG29_864t` with its Attack and full Health | - | 2 / - |
| `000t62` | Sunken Persistence | Persistencia sumergida | its Spellcrafts are permanent | recruit | none | - | - / - |
| `000t64` | Dexterity | Destreza | +2/+2 whenever you play a card | recruit | none | 1 | 1 / 2 |
| `000t64t` | Dexterity | Destreza | +4/+4 whenever you play a card | recruit | none | - | 1 / - |
| `000t65` | Polarization | Polarización | end of turn: Magnetize a random Mech to it | recruit | none | - | - / - |
| `000t66` | Demonology | Demonología | Rally: a Fodder in your next 3 Refreshes | Rally (shop) | Rally with no combat effect | - | - / - |
| `000t69` | Toxicity | Toxicidad | Venomous | keywords | keywords | - | - / - |
| `000t71` | Hostility | Hostilidad | Start of Combat: double its Attack | start of combat | doubles Attack | - | - / 2 |
| `000t72` | Titanic Strength | Fuerza titánica | +1000 Attack | recruit (stats) | none (stats on the board) | - | - / - |
| `000t73` | Fortitude | Entereza | +5/+5 | recruit (stats) | none (stats on the board) | - | - / 2 |
| `000t74` | Sharpened Sword | Espada afilada | +3 Attack whenever you play a card | recruit | none | 4 | - / 5 |
| `000t75` | Toughened Shield | Escudo reforzado | +3 Health whenever you play a card | recruit | none | 1 | 6 / - |
| `000t78` | Efficiency | Eficiencia | its Activate triggers twice | recruit | none | - | - / - |
| `000t79` | Furtiveness | Furtividad | Stealth | keywords | keywords | - | 3 / 1 |
| `000t80` | Consanguinity | Consanguinidad | Rally: 2 Blood Gems to the hand | Rally (hand) | Rally with no combat effect | 1 | - / - |
| `000t81` | Transcendence | Trascendencia | Start of Combat: triple its stats | start of combat | triples both | - | - / - |
| `000t82` | Affinity | Conexión | every 2 turns, a random minion of its type to the hand | recruit | none | - | - / - |
| `000t88` | Desecration | Profanación | +3/+3 per card discarded this game | recruit (stats) | none (stats on the board) | - | - / - |
| `000t89` | Worship | Adoración | Start of Combat: gain the stats of your Deity | start of combat | blocks when start of combat is played (Deity unknown) | - | - / - |
| `000t90` | Doppelganger | Doble | Start of Combat: summon a copy of it | start of combat | summon a copy (keeps its gifts) | 2 | 9 / - |

## 3. How the log carries a gift

Counts over the 84 combats (the boards at each combat's first attack: 395 of the player's minions, 386 of opponents'). Only `GameState` lines were read.

| Where | What the log shows | Count |
|---|---|---|
| Offer (Dark Discovery) | the offered minion (`SETASIDE`) has `DARK_GIFT_ENTITY` = an entity whose card id is the gift spell `BG36_MidGameEffect_000tNN` (D-058) | 106 tags |
| Gifted minion, shop and combat | an `ENCHANTMENT` in `PLAY` with `ATTACHED` = the minion and a gift enchantment card id (table below) | 121 minions (53 own, 68 opp) |
| Same minions | `HAS_DARK_GIFT` 1 on the minion | 119 of the 121 |
| Exceptions | Gilding enchantment without `HAS_DARK_GIFT` (opponents) | 2 |
| | `HAS_DARK_GIFT` 1 with no gift enchantment attached (1 own, 1 opp; not explained) | 2 |
| Board minions with `DARK_GIFT_ENTITY` | only on 5 of the player's own minion entities that are not combat copies (still marked from the offer); never on a combat copy | 5 |
| Minions with two gifts | two different gift enchantments | 2 (1 own, 1 opp) |

**Gift enchantments.** Most gifts attach one enchantment that carries the gift's text, and some also a hidden "stat holder" (`+0/+0`, "Stats set to", "Attack set to 0") that keeps the numbers it changed. The parser must map the **text enchantment** to its gift and ignore the holders:

| Gift (`BG36_MidGameEffect_` + …) | Enchantment that means "carries this gift" | Holders to ignore |
|---|---|---|
| `000t` | `000te2` | `000te`, `000te3` |
| `000t2` | `000t2e2` | `000t2e`, `000t2e3` |
| `000t3`, `000t5`, `000t7`, `000t9`, `000t10` | the id + `e2` (`000t3e2` …) | `000t7e`, `000t9e` |
| `000t4` | `000t4e2` (and the hidden `000t4e`) | `000t4e3`, `000t4e4` |
| `000t11` | `000t11e` or `000t11e2` | |
| `000t13` | **`EDR_100t13e`** (the logs, 19 minions); `000t13e` exists in the data, unseen | |
| `000t28t`, `000t29t`, `000t30t`, `000t64t` | the id + `e` (`000t28te` …) | `000t64te2` |
| every other gift | the id + `e` (`000t81e` …) | `000t51e2`, `000t61e2`, `000t64e2`, `000t71e2`, `000t74e2`, `000t75e2`, `000t81e2`, `000t89e2` |

Stripping a suffix is not enough: the Golem the Golemancy gift summons carries `000t61e2` (its stats set) but not the gift's Deathrattle `000t61e`, so it is not gifted.

**Golden triples.** 39 gifted minions in combat had a golden card id (24 own, 15 opp). Per game, 113 of the 183 golden-id entities that ever carried a gift had a plain copy of the same card with the same gift earlier in that game: the golden minion keeps the gifts of the copies tripled (the rest had their gift added after the triple, or a plain copy outside `PLAY`). Gilding makes a minion golden without changing its card id: `PREMIUM` 1 with a plain id and the `000t14e` enchantment (2 opponents' minions).

**Combat copies.** Every gifted opponent's minion in combat and 50 of the player's 55 are combat copies (`COPIED_FROM_ENTITY_ID`), and the copies carry the gift enchantment and `HAS_DARK_GIFT` as the originals do; the other 5 are the player's own entities above. Doppelganger's copy keeps the gift: 4 boards had two minions of one card both with `000t90e`, none had three, so the copy's own Doppelganger does not trigger again.

**Golemancy.** Its Golem is `BG29_864t` (Deserted Golem) with the dead minion's stats, seen twice; it is not `BG30_MagicItem_442t` (the round 4 Golem).

**Opponents (D-004, D-054).** The opponents' combat copies in `PLAY` carry the same enchantments and tag (68 gifted minions in 39 combats); the game shows a gift on the minion, so it is what the player saw. Duos teammate combats the game does not play back (`SETASIDE` copies) were not counted here; under D-054 their gifts may only be shown after the game, like the rest of those boards.

## 4. The simulator

- **Board model.** `combat_sim::Minion` has `gifts: Vec<String>`, the gift spell ids (`000tNN`, as the offer names them and as the stats already record them, D-060). The wasm request takes `"gifts": [...]` on a minion (at most 4, card-id characters only; `null` or missing is none). The website editor's `SimMinion` has `gifts`, sent with the request; its controls come with T-526.
- **Rules.** `crates/combat-sim/src/gifts.rs` lists all 47 gifts. A gifted minion gets its own card entry: the card's triggers (its golden card's when gilded and modelled), then each gift's; keyword gifts add their keywords; Amalgamation adds every type. New primitives: give own Attack or full Health to a random friend, multiply stats, gain the Attack of the minion to the left, trigger own Deathrattles, summon a copy (gifts included), a shield that takes 3 hits, no damage back while attacking, Reborn with full stats and keywords. A minion that comes back with plain Reborn is a fresh copy of its card without its gifts (an enchantment does not survive Reborn).
- **Start of combat.** Gift effects at start of combat are played only when the engine plays start of combat (the overlay's shop boards), never on boards taken at the first attack, which already show them (the logged boards, the website's boards from saved games).
- **Coverage (D-088, D-097).** An unknown gift id blocks the odds and is named (`not_modelled`, like a card); Worship blocks only when start of combat is played; Persisting Horror and Gilding on a card whose golden card is not modelled are approximate, with the reason (English and Spanish on the website).
- **Engine `0.6.0`.** The accuracy numbers did not move: the saved history has no gifts yet (export regenerated, only the version changed).

## 5. What T-528 must read (parser, app and website)

1. **Board minions:** for each minion in a board snapshot (combat copies included), the gift enchantments in `PLAY` with `ATTACHED` = the minion's id, mapped to gift ids with the table in section 3 (holders ignored); save them as `gifts: ["BG36_MidGameEffect_000tNN", …]` in board order, empty when none. `HAS_DARK_GIFT` alone is not enough (no gift id; 2 minions had it without an enchantment, 2 had a gift without it); show such a minion as "Dark Gift (unknown)" and do not pass an id to the engine, which then plays it as the plain card and should be flagged by the caller.
2. **Report and upload:** a new optional `gifts` list per minion (parser revision bump, `VALIDATOR_VERSION` and `upload-game` key checks, D-058's pattern); the shop board (`own_board_now`) too, for the overlay's odds.
3. **Odds:** `tracker::odds::engine_minion` fills `gifts` (today `Vec::new()`), and `harness::minion` / `harness_logs::engine_minion` read them, so `combat-validate` scores gifted combats; `tracker` should also use `combat_sim::gifts::has_start_of_combat` where it uses the card's.
4. **Display:** a gift mark on the minion wherever a minion shows (recap, overlay, website boards) with the gift's name from the card data (names only, D-038, D-091), Advanced: the id.
5. **D-054:** gifts on unplayed Duos boards only after the game, as the boards themselves.

## 5b. What T-528 did (session 094, D-107)

All of section 5: `crates/bg-parser/src/gifts.rs` (the table above as code, one pass over the board), parser revision 9 with `gifts` and `gift_unknown` on every board and replay minion, `VALIDATOR_VERSION` 8, the odds and the harness pass the gifts, and a gift mark on every minion picture in the app, the overlay and the website. Counts on the same 6 log sessions, at combat start (the boards the report keeps), which is earlier than the first attack used above: 101 of the player's and 122 of the opponents' gifted minions (per combat), 58 of 84 combats, 5 minions with two gifts, 2 marked with no enchantment (the same two as above), 24 gifted minions summoned in replays (Doppelganger's copies among them). `combat-validate` before and after is in plan.md (notes T-528).

## 6. Harness

`combat-validate` cannot use gifts until T-528 saves them: the history (`games.jsonl`, 41 games) has none. In the logs on the PC, 53 of 84 combats had a gifted minion, 12 with a gift that changes a board taken after start of combat (Deathrattle, Reborn, shield, attack, type or golden gifts); none of these would be blocked by a gift (no Worship seen). Once T-528 lands, `combat-validate --logs` gives the first measured effect.

## 7. Open points

- Persisting Horror: whether the minion comes back with its other gifts and this combat's buffs is not checked (approximate).
- Toreth's Blessing and Invulnerability were offered but never seen in combat; both follow the text.
- Gilding: a golden card id is built as `<id>_G`; cards whose golden id is different (`TB_BaconUps_…`) play the plain card, flagged approximate when the card has a combat effect.
- The 2 minions with `HAS_DARK_GIFT` and no enchantment.
