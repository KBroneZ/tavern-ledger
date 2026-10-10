//! The player's own shop, turn by turn, and the actions the log records
//! (T-204, T-205, D-046). Rust-only: the Python prototype does not read it.
//!
//! What the log gives (docs/research/shop-and-apm.md):
//! - a shop turn starts when the game's `TURN` goes odd;
//! - offers are pool minions that enter play on the shop (Bob's) side during
//!   a shop turn; a minion frozen last turn comes back as a new entity that
//!   gets `FROZEN=1` before the player's first action of the turn;
//! - rolls, buys, sells, freezes and tier-ups are top-level `PLAY` blocks of
//!   the player's own shop buttons; a roll that spends no gold is free;
//! - gold is the local player's `RESOURCES`, gold spent the change of
//!   `NUM_RESOURCES_SPENT_THIS_GAME` over the turn (gold from sells included);
//! - the tavern tier is `PLAYER_TECH_LEVEL` on the player's leaderboard hero;
//! - each `GameState.SendOption()` line (and each `SendChoices` that is not
//!   the hero pick) is one action the client sent, at the line's time; its
//!   kind is the top-level block that answers it.
//!
//! Parser revision 4 (T-210, D-055, docs/research/parser-data-round.md):
//! - a buy's price is the change of `NUM_RESOURCES_SPENT_THIS_GAME` over its
//!   block (2 when the minion has `BACON_OVERRIDE_BG_COST` 2, else 3);
//! - the gold the player has is `RESOURCES - RESOURCES_USED + TEMP_RESOURCES`;
//!   a block gains what that gold grew by plus what it spent;
//! - a sell's gold is its block's gain (`BACON_SELL_VALUE` when set, else 1);
//! - extra gold is every other block's gain, except the turn's own gold, which
//!   a top-level `TRIGGER` block of the player entity sets (a block that takes
//!   gold away and gives it back gains nothing);
//! - a roll that lowers the roll button's `BACON_FREE_REFRESH_COUNT` used one
//!   of the free rolls the game showed;
//! - a pass to the teammate (Duos) is a top-level `DECK_ACTION` block of the
//!   player's own card.
//!
//! Only the local player's shop: other players' shops are not in the log.
//! Times are milliseconds from the game's first log line, never a clock time.

use serde::{Deserialize, Serialize};

use crate::state::{Board, Entity};

/// Bounds, so a strange log cannot grow a report without end.
pub const MAX_TURNS: usize = 100;
pub const MAX_OFFERS: usize = 400;
/// Offers over the whole game, so a record stays far below upload-game's size cap.
pub const MAX_GAME_OFFERS: usize = 3_000;
/// Any one count of a turn (rolls, freezes...).
pub const MAX_COUNT: u32 = 10_000;
pub const MAX_CARDS: usize = 100;
pub const MAX_ACTIONS: usize = 3_000;
/// No game lasts a day: past that the record stops (upload-game refuses more).
pub const MAX_MS: i64 = 24 * 60 * 60 * 1000;
const MAX_TIER: i64 = 7;
const MAX_GOLD: i64 = 10_000;
const FREE_REFRESHES: &str = "BACON_FREE_REFRESH_COUNT";

const REROLL: &str = "TB_BaconShop_8p_Reroll_Button";
const BUY: &str = "TB_BaconShop_DragBuy";
const BUY_SPELL: &str = "TB_BaconShop_DragBuy_Spell";
const SELL: &str = "TB_BaconShop_DragSell";
const FREEZE: &str = "TB_BaconShopLockAll_Button";
const TIER_UP: &str = "TB_BaconShopTechUp";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    Roll,
    Buy,
    BuySpell,
    Sell,
    Freeze,
    Unfreeze,
    TierUp,
    HeroPower,
    /// A card played from the hand.
    Play,
    /// A minion moved on the board.
    Move,
    /// A pick among cards offered (discover and the like).
    Choose,
    /// A card passed to the teammate (Duos, parser revision 4).
    Pass,
    /// Anything else the client sent, or an option the log does not answer.
    Other,
}

/// One minion the shop showed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Offer {
    pub card_id: Option<String>,
    /// How many rolls the player had made this turn when it appeared (0 is
    /// the turn's first shop).
    pub roll: u32,
    /// Kept from the last turn by a freeze.
    pub frozen: bool,
}

/// One shop phase. Counts are what the log records for this turn: 0 is
/// "none", since the turn is in the log. A value the log lacks is None.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShopTurn {
    /// 1 for the first shop, then one per round.
    pub turn: i64,
    /// From the game's first log line to the start of this shop phase.
    pub start_ms: i64,
    /// To the start of the next shop phase, or the end of the game.
    pub end_ms: i64,
    /// Tavern tier at the end of the shop phase.
    pub tier: Option<i64>,
    /// The turn's gold (`RESOURCES`), without extra gold from cards.
    pub gold: Option<i64>,
    pub gold_spent: Option<i64>,
    pub rolls: u32,
    /// Rolls that spent no gold.
    pub free_rolls: u32,
    /// Minions bought, by card id.
    pub buys: Vec<Option<String>>,
    pub spell_buys: u32,
    /// Minions sold, by card id.
    pub sells: Vec<Option<String>>,
    pub freezes: u32,
    pub offers: Vec<Offer>,
    /// Actions the log records in this turn (see [`PlayerAction`]).
    pub actions: u32,
    // Parser revision 4 (T-210). None in records saved before it ("not
    // recorded"); the gold ones are None too when the turn's gold is not in
    // the log.
    /// Gold beyond the turn's own and beyond sells: cards, trinkets and the
    /// like giving gold, refunds, more gold this turn.
    #[serde(default)]
    pub extra_gold: Option<u32>,
    /// Gold the player got for selling.
    #[serde(default)]
    pub sell_gold: Option<u32>,
    /// Gold spent buying minions.
    #[serde(default)]
    pub buy_gold: Option<u32>,
    /// Gold spent buying tavern spells.
    #[serde(default)]
    pub spell_gold: Option<u32>,
    /// Rolls that used one of the free rolls the game showed on the roll
    /// button; None for the whole game when the log never gives that count.
    #[serde(default)]
    pub free_refreshes: Option<u32>,
    /// Cards passed to the teammate (Duos), by card id.
    #[serde(default)]
    pub passes: Option<Vec<Option<String>>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TierUp {
    pub turn: i64,
    pub tier: i64,
}

/// One action the client sent for the player, as the log records it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerAction {
    /// From the game's first log line.
    pub ms: i64,
    pub turn: i64,
    pub kind: ActionKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShopRecord {
    pub turns: Vec<ShopTurn>,
    pub tier_ups: Vec<TierUp>,
    pub actions: Vec<PlayerAction>,
    /// The first shop phase's start, from the game's first log line.
    pub start_ms: Option<i64>,
    /// The game's end (STATE=COMPLETE), or the log's last line if it never came.
    pub end_ms: Option<i64>,
    /// False when the log stops before the game ends.
    pub ended: bool,
}

/// Who is who, from the collector.
#[derive(Clone, Copy, Debug, Default)]
pub struct Seats {
    /// Entity id of the local player.
    pub local: Option<i64>,
    pub local_pid: Option<i64>,
    pub dummy_pid: Option<i64>,
}

/// The top-level block being read.
#[derive(Clone, Debug)]
struct OpenBlock {
    kind: Option<ActionKind>,
    /// Card id of the target, for buys and sells.
    target: Option<String>,
    spent_before: Option<i64>,
    froze: bool,
    unfroze: bool,
    /// The turn's own gold being set (a TRIGGER block of the player entity).
    refresh: bool,
    /// Card id of the block's entity, for passes.
    card: Option<String>,
    /// The gold the player had when the block started.
    gold_before: Option<i64>,
    /// A roll that used a free roll the game showed.
    used_free: bool,
    /// The action (an option sent) this block answers.
    answers: Option<usize>,
}

#[derive(Clone, Debug)]
struct OpenTurn {
    record: ShopTurn,
    /// Entity id of each offer, in the same order.
    offer_ids: Vec<i64>,
    /// No action of the player's own yet: a freeze seen now is last turn's,
    /// carried over.
    untouched: bool,
}

#[derive(Clone, Debug, Default)]
pub struct ShopTracker {
    turns: Vec<ShopTurn>,
    open: Option<OpenTurn>,
    tier_ups: Vec<TierUp>,
    actions: Vec<PlayerAction>,
    /// An option sent whose block has not come yet.
    pending: Option<usize>,
    block: Option<OpenBlock>,
    spent_at_turn_start: Option<i64>,
    end_ms: Option<i64>,
    last_ms: i64,
    offers_seen: usize,
    /// The log gave the roll button's free rolls at least once this game.
    /// Without it, "no free roll used" would be a guess: unknown instead.
    free_refreshes_seen: bool,
}

impl ShopTracker {
    /// Every line's time, so a log cut short still has an end.
    pub fn saw_line(&mut self, ms: i64) {
        self.last_ms = self.last_ms.max(ms);
    }

    /// The game's TURN changed. Odd turns are shop phases.
    pub fn on_turn(&mut self, turn: i64, ms: i64, board: &Board, seats: Seats) {
        self.close_block(board, seats);
        self.close_turn(board, seats, ms);
        self.pending = None;
        let number = (turn + 1) / 2;
        let fits = self.turns.len() < MAX_TURNS && number <= MAX_TURNS as i64 && ms <= MAX_MS;
        if turn >= 1 && turn % 2 == 1 && fits {
            self.spent_at_turn_start = spent(board, seats);
            self.open = Some(OpenTurn {
                record: ShopTurn {
                    turn: number,
                    start_ms: ms,
                    end_ms: ms,
                    tier: None,
                    gold: None,
                    gold_spent: None,
                    rolls: 0,
                    free_rolls: 0,
                    buys: Vec::new(),
                    spell_buys: 0,
                    sells: Vec::new(),
                    freezes: 0,
                    offers: Vec::new(),
                    actions: 0,
                    extra_gold: Some(0),
                    sell_gold: Some(0),
                    buy_gold: Some(0),
                    spell_gold: Some(0),
                    free_refreshes: Some(0),
                    passes: Some(Vec::new()),
                },
                offer_ids: Vec::new(),
                untouched: true,
            });
        }
    }

    /// STATE=COMPLETE on the game.
    pub fn on_game_over(&mut self, ms: i64) {
        self.end_ms.get_or_insert(ms);
    }

    /// The client sent an option; the next top-level block says what it was.
    pub fn on_option(&mut self, ms: i64) {
        self.pending = self.push_action(ms, ActionKind::Other);
    }

    /// The client sent a choice. The hero pick (MULLIGAN) is not an action
    /// of the shop.
    pub fn on_choice(&mut self, ms: i64, choice_type: &str) {
        if choice_type != "MULLIGAN" {
            self.push_action(ms, ActionKind::Choose);
            self.pending = None;
        }
    }

    fn push_action(&mut self, ms: i64, kind: ActionKind) -> Option<usize> {
        let open = self.open.as_mut()?;
        if self.actions.len() >= MAX_ACTIONS || ms > MAX_MS {
            return None;
        }
        open.record.actions += 1;
        self.actions.push(PlayerAction {
            ms,
            turn: open.record.turn,
            kind,
        });
        Some(self.actions.len() - 1)
    }

    /// A BLOCK_START at the top level of the log.
    pub fn on_top_block(
        &mut self,
        block_type: &str,
        entity: Option<&Entity>,
        target: Option<&Entity>,
        board: &Board,
        seats: Seats,
    ) {
        self.close_block(board, seats);
        let own = entity.is_some_and(|e| Some(e.int("CONTROLLER")) == seats.local_pid);
        // A pass is of the player's own card; any other DECK_ACTION stays "other".
        let kind = block_kind(block_type, entity).map(|k| {
            if k == ActionKind::Pass && !own {
                ActionKind::Other
            } else {
                k
            }
        });
        let answers = kind.and_then(|_| self.pending.take());
        if let (Some(kind), Some(i)) = (kind, answers) {
            self.actions[i].kind = kind;
        }
        let shop_kind = kind.filter(|_| own && self.open.is_some());
        if let (Some(kind), Some(open)) = (shop_kind, self.open.as_mut()) {
            open.untouched = false;
            // Counted now, so the offers it brings carry the new count.
            if kind == ActionKind::Roll {
                open.record.rolls = (open.record.rolls + 1).min(MAX_COUNT);
            }
        }
        let refresh = block_type == "TRIGGER" && entity.is_some_and(|e| seats.local == Some(e.id));
        self.block = Some(OpenBlock {
            kind: shop_kind,
            target: target.and_then(|t| t.card_id.clone()),
            spent_before: spent(board, seats),
            froze: false,
            unfroze: false,
            answers,
            refresh,
            card: entity.and_then(|e| e.card_id.clone()),
            gold_before: gold(board, seats),
            used_free: false,
        });
    }

    /// A BLOCK_END at the top level of the log.
    pub fn on_top_block_end(&mut self, board: &Board, seats: Seats) {
        self.close_block(board, seats);
    }

    fn close_block(&mut self, board: &Board, seats: Seats) {
        let Some(block) = self.block.take() else {
            return;
        };
        let Some(open) = self.open.as_mut() else {
            return;
        };
        let turn = &mut open.record;
        let cost = match (block.spent_before, spent(board, seats)) {
            (Some(before), Some(after)) => Some(after.saturating_sub(before)),
            _ => None,
        };
        // Gold the block gave: what the player's gold grew by, plus what it spent.
        let gain = match (block.gold_before, gold(board, seats)) {
            (Some(before), Some(after)) => after
                .saturating_sub(before)
                .saturating_add(cost.unwrap_or(0)),
            _ => 0,
        };
        let is_sell = block.kind == Some(ActionKind::Sell);
        if !block.refresh && !is_sell && gain > 0 {
            add(&mut turn.extra_gold, gain);
        }
        match block.kind {
            Some(ActionKind::Roll) => {
                if cost.is_some_and(|c| c <= 0) {
                    turn.free_rolls = (turn.free_rolls + 1).min(turn.rolls);
                }
                if block.used_free {
                    add(&mut turn.free_refreshes, 1);
                }
            }
            Some(ActionKind::Buy) if turn.buys.len() < MAX_CARDS => {
                turn.buys.push(block.target);
                add(&mut turn.buy_gold, cost.unwrap_or(0));
            }
            Some(ActionKind::BuySpell) => {
                turn.spell_buys = (turn.spell_buys + 1).min(MAX_COUNT);
                add(&mut turn.spell_gold, cost.unwrap_or(0));
            }
            Some(ActionKind::Sell) if turn.sells.len() < MAX_CARDS => {
                turn.sells.push(block.target);
                add(&mut turn.sell_gold, gain);
            }
            Some(ActionKind::Pass) => {
                if let Some(passes) = turn.passes.as_mut().filter(|p| p.len() < MAX_CARDS) {
                    passes.push(block.card);
                }
            }
            Some(ActionKind::Freeze) if block.froze || !block.unfroze => {
                turn.freezes = (turn.freezes + 1).min(MAX_COUNT)
            }
            _ => {}
        }
        // A freeze button that only cleared the shop's freeze is an unfreeze.
        if block.kind == Some(ActionKind::Freeze) && block.unfroze && !block.froze {
            if let Some(action) = block.answers.and_then(|i| self.actions.get_mut(i)) {
                action.kind = ActionKind::Unfreeze;
            }
        }
    }

    /// A tag changed on `entity`; `old` is its value before.
    pub fn on_tag(
        &mut self,
        entity: &Entity,
        tag: &str,
        old: Option<i64>,
        board: &Board,
        seats: Seats,
    ) {
        match tag {
            "FROZEN" => self.on_frozen(entity),
            "PLAYER_TECH_LEVEL" => self.on_tier(entity, old, board, seats),
            "ZONE" | "CONTROLLER" => self.on_maybe_offer(entity, seats),
            "RESOURCES" | "TEMP_RESOURCES" | "RESOURCES_USED" => {
                self.on_gold_outside_blocks(entity, tag, old, seats)
            }
            FREE_REFRESHES => self.on_free_refresh(entity, old),
            _ => {}
        }
    }

    /// The player's gold changed outside any top-level block (blocks count
    /// their gain as they close): a rise is extra gold.
    fn on_gold_outside_blocks(
        &mut self,
        entity: &Entity,
        tag: &str,
        old: Option<i64>,
        seats: Seats,
    ) {
        let Some(open) = self.open.as_mut() else {
            return;
        };
        if seats.local != Some(entity.id) || self.block.is_some() {
            return;
        }
        let (old, new) = (old.unwrap_or(0), entity.int(tag));
        let gain = if tag == "RESOURCES_USED" {
            old.saturating_sub(new)
        } else {
            new.saturating_sub(old)
        };
        if gain > 0 {
            add(&mut open.record.extra_gold, gain);
        }
    }

    /// The roll button's free rolls went down during a roll: that roll used one.
    fn on_free_refresh(&mut self, entity: &Entity, old: Option<i64>) {
        let button = entity.card_id.as_deref() == Some(REROLL);
        self.free_refreshes_seen |= button;
        let Some(block) = self.block.as_mut() else {
            return;
        };
        let fewer = entity.int(FREE_REFRESHES) < old.unwrap_or(0);
        if block.kind == Some(ActionKind::Roll) && button && fewer {
            block.used_free = true;
        }
    }

    fn on_frozen(&mut self, entity: &Entity) {
        let frozen = entity.int("FROZEN") != 0;
        if let Some(block) = self.block.as_mut() {
            if block.kind == Some(ActionKind::Freeze) {
                if frozen {
                    block.froze = true;
                } else {
                    block.unfroze = true;
                }
                return;
            }
        }
        let Some(open) = self.open.as_mut() else {
            return;
        };
        if frozen && open.untouched {
            if let Some(i) = open.offer_ids.iter().position(|&id| id == entity.id) {
                open.record.offers[i].frozen = true;
            }
        }
    }

    fn on_tier(&mut self, entity: &Entity, old: Option<i64>, board: &Board, seats: Seats) {
        let Some(open) = self.open.as_ref() else {
            return;
        };
        if !is_own_hero(entity, board, seats) {
            return;
        }
        let tier = entity.int("PLAYER_TECH_LEVEL");
        // The tier is first set to 1 at the start: not a tier-up.
        let up = tier > old.unwrap_or(1) && (2..=MAX_TIER).contains(&tier);
        if up && self.tier_ups.len() < MAX_TURNS {
            self.tier_ups.push(TierUp {
                turn: open.record.turn,
                tier,
            });
        }
    }

    /// An entity was created, shown or moved: is it a new shop offer?
    pub fn on_maybe_offer(&mut self, entity: &Entity, seats: Seats) {
        let Some(open) = self.open.as_mut() else {
            return;
        };
        let offered = seats
            .dummy_pid
            .is_some_and(|bob| entity.int("CONTROLLER") == bob)
            && entity.is("CARDTYPE", "MINION")
            && entity.is("ZONE", "PLAY")
            && entity.int("IS_BACON_POOL_MINION") != 0;
        let full = open.record.offers.len() >= MAX_OFFERS || self.offers_seen >= MAX_GAME_OFFERS;
        if !offered || full || open.offer_ids.contains(&entity.id) {
            return;
        }
        open.offer_ids.push(entity.id);
        self.offers_seen += 1;
        open.record.offers.push(Offer {
            card_id: entity.card_id.clone(),
            roll: open.record.rolls,
            frozen: open.untouched && entity.int("FROZEN") != 0,
        });
    }

    fn close_turn(&mut self, board: &Board, seats: Seats, ms: i64) {
        let Some(open) = self.open.take() else {
            return;
        };
        let mut turn = open.record;
        let local = seats.local.and_then(|id| board.get(id));
        turn.gold = local
            .and_then(|p| tag_int(p, "RESOURCES"))
            .filter(|g| (0..=MAX_GOLD).contains(g));
        if turn.gold.is_none() {
            // Without the turn's gold the player's gold tags are not in the log.
            turn.extra_gold = None;
            turn.sell_gold = None;
            turn.buy_gold = None;
            turn.spell_gold = None;
        }
        // Without the turn's gold the player's gold tags are not in the log.
        turn.gold_spent = match (turn.gold, self.spent_at_turn_start, spent(board, seats)) {
            (Some(_), Some(before), Some(after)) => {
                Some(after.saturating_sub(before).clamp(0, MAX_GOLD))
            }
            _ => None,
        };
        turn.tier = own_hero(board, seats)
            .and_then(|h| tag_int(h, "PLAYER_TECH_LEVEL"))
            .filter(|t| (1..=MAX_TIER).contains(t));
        turn.end_ms = ms.clamp(turn.start_ms, MAX_MS);
        self.turns.push(turn);
    }

    /// The record so far. A game in progress has its open turn up to the
    /// last line read.
    pub fn record(&self, board: &Board, seats: Seats) -> ShopRecord {
        let mut copy = self.clone();
        let end = copy.end_ms.unwrap_or(copy.last_ms).min(MAX_MS);
        copy.close_block(board, seats);
        copy.close_turn(board, seats, end);
        let start_ms = copy.turns.first().map(|t| t.start_ms);
        // A turn (shop and combat) lasts until the next shop phase starts;
        // the last one until the game's end.
        let ends: Vec<i64> = copy
            .turns
            .iter()
            .skip(1)
            .map(|t| t.start_ms)
            .chain([end])
            .collect();
        for (turn, next) in copy.turns.iter_mut().zip(ends) {
            turn.end_ms = next.max(turn.start_ms);
            if !copy.free_refreshes_seen {
                turn.free_refreshes = None;
            }
        }
        ShopRecord {
            turns: copy.turns,
            tier_ups: copy.tier_ups,
            actions: copy.actions,
            start_ms,
            end_ms: start_ms.map(|start| end.max(start)),
            ended: copy.end_ms.is_some(),
        }
    }
}

/// Adds to a count the record keeps, within its bound.
fn add(count: &mut Option<u32>, n: i64) {
    if let Some(c) = count.as_mut() {
        let n = u32::try_from(n.clamp(0, MAX_GOLD)).unwrap_or(0);
        *c = c.saturating_add(n).min(MAX_COUNT);
    }
}

fn block_kind(block_type: &str, entity: Option<&Entity>) -> Option<ActionKind> {
    match block_type {
        "MOVE_MINION" => return Some(ActionKind::Move),
        "DECK_ACTION" => return Some(ActionKind::Pass),
        "PLAY" => {}
        _ => return None,
    }
    let card = entity.and_then(|e| e.card_id.as_deref()).unwrap_or("");
    Some(match card {
        REROLL => ActionKind::Roll,
        BUY => ActionKind::Buy,
        BUY_SPELL => ActionKind::BuySpell,
        SELL => ActionKind::Sell,
        FREEZE => ActionKind::Freeze,
        _ if card.starts_with(TIER_UP) => ActionKind::TierUp,
        _ if entity.is_some_and(|e| e.is("CARDTYPE", "HERO_POWER")) => ActionKind::HeroPower,
        _ => ActionKind::Play,
    })
}

/// A tag's number, or None when the entity does not have it (unknown, not 0).
fn tag_int(entity: &Entity, tag: &str) -> Option<i64> {
    match entity.tags.get(tag) {
        Some(crate::power::Value::Int(v)) => Some(*v),
        _ => None,
    }
}

/// Gold spent so far in the game. The log leaves out tags that are 0, so a
/// player entity without it has spent nothing; no player entity is unknown.
fn spent(board: &Board, seats: Seats) -> Option<i64> {
    let local = board.get(seats.local?)?;
    Some(local.int("NUM_RESOURCES_SPENT_THIS_GAME"))
}

/// The gold the player has right now. The log leaves out tags that are 0.
fn gold(board: &Board, seats: Seats) -> Option<i64> {
    let p = board.get(seats.local?)?;
    Some(
        p.int("RESOURCES")
            .saturating_sub(p.int("RESOURCES_USED"))
            .saturating_add(p.int("TEMP_RESOURCES")),
    )
}

fn own_hero(board: &Board, seats: Seats) -> Option<&Entity> {
    let pid = seats.local_pid?;
    crate::collector::leaderboard_heroes(board)
        .into_iter()
        .find(|(p, _)| *p == pid)
        .map(|(_, h)| h)
}

fn is_own_hero(entity: &Entity, board: &Board, seats: Seats) -> bool {
    entity.is("CARDTYPE", "HERO") && own_hero(board, seats).is_some_and(|h| h.id == entity.id)
}
