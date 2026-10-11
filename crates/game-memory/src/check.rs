//! Tries the two reads of D-063 once against the running game and prints
//! what came out: the lobby's tribes (names) and the player's own rating, or
//! why not. Read-only (query and read rights only). Prints nothing else from
//! the game.

fn main() {
    let check = game_memory::check();
    match check.tribes {
        Ok(tribes) => println!("lobby tribes: {}", tribes.join(", ")),
        Err(e) => println!("lobby tribes: {e}"),
    }
    match check.rating {
        Ok(r) => println!(
            "own rating: solo {}, duos {}",
            r.solo.map_or("none".into(), |v| v.to_string()),
            r.duos.map_or("none".into(), |v| v.to_string())
        ),
        Err(e) => println!("own rating: {e}"),
    }
}
