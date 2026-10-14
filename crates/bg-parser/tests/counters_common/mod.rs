//! Synthetic log pieces shared by the counters' tests: the local player is
//! entity 2, the opponent's dummy player entity 3, and entity 5 stands for a
//! Duos teammate's hero.
#![allow(dead_code)]

pub const T: &str = "D 12:00:00.0000000 GameState.DebugPrintPower() - ";

pub fn line(indent: usize, body: &str) -> String {
    format!("{T}{}{body}", "    ".repeat(indent))
}

pub fn tag(entity: &str, tag: &str, value: &str) -> String {
    line(
        1,
        &format!("TAG_CHANGE Entity={entity} tag={tag} value={value} "),
    )
}

/// An enchantment as the game creates it: one `FULL_ENTITY` packet.
pub fn enchantment(
    id: i64,
    card: &str,
    attached: i64,
    zone: &str,
    nums: (i64, i64),
) -> Vec<String> {
    vec![
        line(1, &format!("FULL_ENTITY - Creating ID={id} CardID={card}")),
        line(2, "tag=CARDTYPE value=ENCHANTMENT"),
        line(2, &format!("tag=ATTACHED value={attached}")),
        line(2, &format!("tag=ZONE value={zone}")),
        line(2, &format!("tag=TAG_SCRIPT_DATA_NUM_1 value={}", nums.0)),
        line(2, &format!("tag=TAG_SCRIPT_DATA_NUM_2 value={}", nums.1)),
        line(2, &format!("tag=ENTITY_ID value={id}")),
    ]
}

/// A Solo game in its first shop turn: no values yet.
pub fn start() -> Vec<String> {
    vec![
        line(0, "CREATE_GAME"),
        line(1, "GameEntity EntityID=1"),
        line(2, "tag=CARDTYPE value=GAME"),
        line(2, "tag=ENTITY_ID value=1"),
        line(1, "Player EntityID=2 PlayerID=2 GameAccountId=[hi=1 lo=1]"),
        line(2, "tag=PLAYER_ID value=2"),
        line(2, "tag=CARDTYPE value=PLAYER"),
        line(2, "tag=HERO_ENTITY value=10"),
        line(2, "tag=ENTITY_ID value=2"),
        line(1, "Player EntityID=3 PlayerID=10 GameAccountId=[hi=0 lo=0]"),
        line(2, "tag=PLAYER_ID value=10"),
        line(2, "tag=CARDTYPE value=PLAYER"),
        line(2, "tag=HERO_ENTITY value=11"),
        line(2, "tag=BACON_DUMMY_PLAYER value=1"),
        line(2, "tag=ENTITY_ID value=3"),
        "D 12:00:00.0000000 GameState.DebugPrintGame() - BuildNumber=253216".into(),
        "D 12:00:00.0000000 GameState.DebugPrintGame() - GameType=GT_BATTLEGROUNDS".into(),
        line(1, "FULL_ENTITY - Creating ID=5 CardID=SYN_TEAMMATE_HERO"),
        line(2, "tag=CARDTYPE value=HERO"),
        line(2, "tag=ENTITY_ID value=5"),
        tag("GameEntity", "TURN", "1"),
    ]
}
