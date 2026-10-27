# Implementing a card in the simulator

The combat simulator (`crates/combat-sim`) only plays a card it knows. Any other card makes it say "not modelled" instead of giving odds. If you notice a card that is missing or plays wrong, this page is how to fix it, usually in a short pull request.

**Clean room.** Work only from the card's text (HearthstoneJSON's `cards.json`, or the game) and from your own logged games. Never read or port another tracker's or simulator's code (see [CONTRIBUTING.md](../CONTRIBUTING.md), rule 1).

## Where a card's rules live

- `crates/combat-sim/src/cards.rs` defines what a card can do: `Trigger` (when: Deathrattle, Rally, Avenge, Start of Combat…) and `Action` (what: summon a token, buff, give a keyword, deal damage…). `NO_COMBAT_EFFECT` lists cards whose text never changes a combat (shop-only effects); they play as plain minions. `TOKENS` holds the printed facts of every minion a card can summon.
- `crates/combat-sim/src/cards_roundN.rs` (`EFFECTS`, `EFFECTS_2` … `EFFECTS_8`) lists the cards that act in combat, each as `exact(&[...])` when every part of its text is played as written, or `approx("reason", &[...])` when some part is not (the reason is shown to players, in English, with its Spanish in `web/src/lib/sim/reasons.ts`). Add new cards to the newest round file.
- Every minion card has a golden form (`<id>_G`, or an older `TB_BaconUps_…` id) with doubled numbers; list both, the tests check it.

## Step by step

1. **Find the card.** Its id and text are in `cards.json` (`https://api.hearthstonejson.com/v1/<build>/enUS/cards.json`; `<build>` is in your game's log, `BuildNumber=`).
2. **Pick its rules.**
   - Nothing in the text acts during combat: add the id (and its golden id) to `NO_COMBAT_EFFECT`.
   - The text acts in combat and the existing `Trigger` and `Action` variants can say it: add both forms to the newest `EFFECTS_N` list with `exact` or `approx`.
   - A new kind of effect: add an `Action` (or `Trigger`) variant in `cards.rs` and play it in `combat_effects.rs` (or a `combat_*.rs` file of its own, like `combat_deity.rs`), with tests.
   - A token it summons: add its printed facts to `TOKENS` (the tests fail when a summoned token is missing).
3. **Test it on a small board** whose outcome is certain, in the round's tests file (for example `combat_round8_tests.rs`):

   ```rust
   #[test]
   fn my_card_gives_its_neighbours_two_attack_when_it_dies() {
       let card = Minion::new("BG99_123", 2, 1).tier(2);
       with(vec![plain(1, 5), card, plain(1, 5)], target(), |c| {
           kill(c, &[1]);
           assert_eq!(stats(c, 0), [(3, 5), (3, 5)]);
       });
   }
   ```

4. **Check it against real combats**, if you have logged games with the card. Make a corpus of your own combats and compare the engine before and after your change (both tools read only local files; keep the corpus out of the repository):

   ```
   cargo run --release -p combat-sim --bin combat-corpus -- --history "%APPDATA%\TavernLedger\games.jsonl" --out .local/corpus.jsonl.gz
   cargo run --release -p combat-sim --bin combat-conformance -- --corpus .local/corpus.jsonl.gz --cards <build>=.local/cards-<build>.json --baseline .local/baseline.json --bless
   ```

   Make your change, then run the second command again without `--bless`: it lists every combat that moved, as a regression or an improvement, with the attack where the replay now leaves the game. Say in the pull request what moved.

5. **Run the checks**: `cargo fmt --all --check`, `cargo test -p combat-sim`, and `cargo test -p sim-wasm` (the website's engine). If the simulator's version changes, rebuild the website's engine as `docs/development.md` explains. If the synthetic conformance baseline moved on purpose, rewrite it with `CONFORMANCE_BLESS=1 cargo test -p combat-sim --test conformance`.

## When it is not exact

Mark a card `approx` with a short reason when you cannot be sure: a random choice not checked against real games, a value the board does not show (a game-long counter), or a rule the text leaves open. Players see that the odds are approximate and why, which is better than a confident wrong number.
