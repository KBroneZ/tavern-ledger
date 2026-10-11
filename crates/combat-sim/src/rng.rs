//! A small seedable random number generator (SplitMix64). Deterministic for a
//! given seed on every platform, including WebAssembly, so tests and
//! bug reports can replay a combat exactly. Not for anything secret.

#[derive(Clone, Debug)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number in `0..n`. `n` must be above 0. The bias of the
    /// multiply-shift method is below 2^-32 for the sizes used here (at most
    /// a few dozen choices).
    pub fn below(&mut self, n: usize) -> usize {
        debug_assert!(n > 0);
        ((u128::from(self.next_u64()) * n as u128) >> 64) as usize
    }

    pub fn coin(&mut self) -> bool {
        self.next_u64() >> 63 == 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_gives_the_same_sequence() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        let xs: Vec<u64> = (0..5).map(|_| a.next_u64()).collect();
        let ys: Vec<u64> = (0..5).map(|_| b.next_u64()).collect();
        assert_eq!(xs, ys);
        assert_ne!(Rng::new(1).next_u64(), Rng::new(2).next_u64());
    }

    #[test]
    fn below_stays_in_range_and_covers_it() {
        let mut rng = Rng::new(7);
        let mut seen = [0u32; 3];
        for _ in 0..3000 {
            seen[rng.below(3)] += 1;
        }
        assert!(seen.iter().all(|&c| c > 900 && c < 1100), "{seen:?}");
    }

    #[test]
    fn coin_is_roughly_fair() {
        let mut rng = Rng::new(9);
        let heads = (0..10_000).filter(|_| rng.coin()).count();
        assert!((4800..5200).contains(&heads), "{heads}");
    }
}
