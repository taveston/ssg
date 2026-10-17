//! The generator's own random numbers: a small seeded generator written
//! here, so no crate is needed and the same seed gives the same numbers
//! everywhere (SGEN-02, SGEN-06), and the derivation of one seed from
//! another and a name, which gives every stage its own stream (SGEN-03).
//!
//! The generator is xoshiro256** (Blackman and Vigna), seeded through
//! SplitMix64. Both are a few lines of integer arithmetic, fast and well
//! tested, and nothing in them depends on the platform.

use bevy::math::DVec3;

/// The odd constant SplitMix64 adds each step: 2^64 divided by the golden
/// ratio, which spreads consecutive seeds across the whole range.
const GOLDEN_GAMMA: u64 = 0x9e37_79b9_7f4a_7c15;
/// FNV-1a's starting value and prime, for hashing a stage's name into a
/// number (the standard 64-bit parameters).
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
/// A uniform number in [0, 1) is built from the top 53 bits of a draw,
/// the precision of an `f64`'s mantissa.
const UNIT_BITS: u32 = 53;

/// One SplitMix64 step: advances `state` and returns a well-mixed number.
/// Used to seed the main generator and to derive seeds, so that nearby
/// inputs (seed 1 and seed 2) give unrelated outputs.
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(GOLDEN_GAMMA);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// FNV-1a over a name's bytes: a fixed number for each name.
fn hash_name(name: &str) -> u64 {
    name.bytes().fold(FNV_OFFSET, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(FNV_PRIME)
    })
}

/// A seed derived from `seed` and a `name`, such as a stage's ("stars")
/// (SGEN-03). The same pair always gives the same seed; different names
/// give unrelated seeds, so each stage draws its own numbers.
pub fn derive_seed(seed: u64, name: &str) -> u64 {
    let mut state = seed ^ hash_name(name);
    splitmix64(&mut state)
}

/// A seed derived from `seed`, a `name` and an `index`, such as a
/// system's id: one stream per item, so items drawn in any order get the
/// same numbers (the system generator's seed per system, SYSG-02).
pub fn derive_indexed_seed(seed: u64, name: &str, index: u64) -> u64 {
    let mut state = derive_seed(seed, name) ^ index.wrapping_mul(GOLDEN_GAMMA);
    splitmix64(&mut state)
}

/// A seeded stream of random numbers (xoshiro256**). Two `Rng`s made
/// from the same seed give the same numbers in the same order.
#[derive(Debug, Clone)]
pub struct Rng {
    state: [u64; 4],
}

impl Rng {
    /// A stream started from `seed`. Any seed is fine, zero included.
    pub fn new(seed: u64) -> Self {
        let mut mix = seed;
        let state = [
            splitmix64(&mut mix),
            splitmix64(&mut mix),
            splitmix64(&mut mix),
            splitmix64(&mut mix),
        ];
        Self { state }
    }

    /// The next raw 64-bit number.
    pub fn next_u64(&mut self) -> u64 {
        let [s0, s1, s2, s3] = self.state;
        let result = s1.wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = s1 << 17;
        let mut next = [s0, s1, s2 ^ s0, s3 ^ s1];
        next[1] ^= next[2];
        next[0] ^= next[3];
        next[2] ^= t;
        next[3] = next[3].rotate_left(45);
        self.state = next;
        result
    }

    /// A number uniform in [0, 1).
    pub fn unit(&mut self) -> f64 {
        let bits = self.next_u64() >> (64 - UNIT_BITS);
        bits as f64 / (1u64 << UNIT_BITS) as f64
    }

    /// A number uniform in [lo, hi).
    pub fn uniform(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.unit()
    }

    /// A number between `lo` and `hi` (both positive) whose logarithm is
    /// uniform: each factor of ten in the range is equally likely. The
    /// round trip through `ln` and `exp` can land a hair outside the range,
    /// so the result is kept inside it.
    pub fn log_uniform(&mut self, lo: f64, hi: f64) -> f64 {
        (lo.ln() + (hi.ln() - lo.ln()) * self.unit())
            .exp()
            .max(lo)
            .min(hi)
    }

    /// A number from the normal distribution with this mean and spread
    /// (standard deviation), by the Box-Muller transform.
    pub fn normal(&mut self, mean: f64, spread: f64) -> f64 {
        // 1 − unit is in (0, 1], so the logarithm is finite.
        let radius = (-2.0 * (1.0 - self.unit()).ln()).sqrt();
        let angle = std::f64::consts::TAU * self.unit();
        mean + spread * radius * angle.cos()
    }

    /// True with probability `p` (0 never, 1 always).
    pub fn chance(&mut self, p: f64) -> bool {
        self.unit() < p
    }

    /// A whole number uniform in [0, n). `n` must be at least 1.
    pub fn below(&mut self, n: usize) -> usize {
        ((u128::from(self.next_u64()) * n as u128) >> 64) as usize
    }

    /// A whole number uniform from `lo` to `hi`, both included.
    pub fn int_between(&mut self, lo: u32, hi: u32) -> u32 {
        lo + self.below((hi - lo) as usize + 1) as u32
    }

    /// The index of one of `weights`, each chosen in proportion to its
    /// weight. Weights need not sum to 1; zero weights are never chosen.
    pub fn weighted_index(&mut self, weights: &[f64]) -> usize {
        let total: f64 = weights.iter().sum();
        let mut target = self.unit() * total;
        for (index, weight) in weights.iter().enumerate() {
            if target < *weight {
                return index;
            }
            target -= weight;
        }
        // Rounding can leave the target just past the end: take the last
        // index with any weight.
        weights.iter().rposition(|w| *w > 0.0).unwrap_or(0)
    }

    /// One of `choices`, each chosen in proportion to its weight.
    pub fn weighted<T: Copy>(&mut self, choices: &[(T, f64)]) -> T {
        let weights: Vec<f64> = choices.iter().map(|(_, w)| *w).collect();
        choices[self.weighted_index(&weights)].0
    }

    /// A direction uniform over the sphere, as a unit vector.
    pub fn direction(&mut self) -> DVec3 {
        let z = self.uniform(-1.0, 1.0);
        let angle = std::f64::consts::TAU * self.unit();
        let ring = (1.0 - z * z).sqrt();
        DVec3::new(ring * angle.cos(), ring * angle.sin(), z)
    }

    /// `count` distinct indices from [0, n), in the order drawn.
    pub fn sample_indices(&mut self, n: usize, count: usize) -> Vec<usize> {
        let mut pool: Vec<usize> = (0..n).collect();
        let count = count.min(n);
        for i in 0..count {
            let pick = i + self.below(n - i);
            pool.swap(i, pick);
        }
        pool.truncate(count);
        pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SGEN-02: a stream is fixed by its seed alone.
    #[test]
    fn the_same_seed_gives_the_same_numbers() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        let first: Vec<u64> = (0..100).map(|_| a.next_u64()).collect();
        let second: Vec<u64> = (0..100).map(|_| b.next_u64()).collect();
        assert_eq!(first, second);
        assert_ne!(Rng::new(43).next_u64(), first[0]);
    }

    /// SGEN-02: the numbers are fixed by the algorithm, not the machine:
    /// SplitMix64 from 0 and xoshiro256** from the state [1, 2, 3, 4]
    /// give their published first outputs.
    #[test]
    fn the_stream_matches_its_reference_values() {
        let mut state = 0;
        assert_eq!(splitmix64(&mut state), 0xe220_a839_7b1d_cdaf);
        let mut rng = Rng {
            state: [1, 2, 3, 4],
        };
        let first: Vec<u64> = (0..4).map(|_| rng.next_u64()).collect();
        assert_eq!(
            first,
            vec![11520, 0, 1_509_978_240, 1_215_971_899_390_074_240]
        );
    }

    /// SGEN-03: a derived seed depends on both the seed and the name.
    #[test]
    fn derived_seeds_differ_by_name_and_by_seed() {
        assert_eq!(derive_seed(7, "stars"), derive_seed(7, "stars"));
        assert_ne!(derive_seed(7, "stars"), derive_seed(7, "volume"));
        assert_ne!(derive_seed(7, "stars"), derive_seed(8, "stars"));
        assert_eq!(
            derive_indexed_seed(7, "system", 3),
            derive_indexed_seed(7, "system", 3)
        );
        assert_ne!(
            derive_indexed_seed(7, "system", 3),
            derive_indexed_seed(7, "system", 4)
        );
    }

    #[test]
    fn draws_stay_in_their_ranges() {
        let mut rng = Rng::new(1);
        for _ in 0..10_000 {
            let u = rng.unit();
            assert!((0.0..1.0).contains(&u));
            let l = rng.log_uniform(0.5, 2.0);
            assert!((0.5..=2.0).contains(&l));
            let k = rng.int_between(3, 7);
            assert!((3..=7).contains(&k));
            assert!((rng.direction().length() - 1.0).abs() < 1e-12);
        }
        // An empty range gives its one value exactly, as when a companion's
        // mass range is cut down to its primary's mass.
        assert_eq!(rng.log_uniform(0.45, 0.45), 0.45);
    }

    #[test]
    fn weighted_choices_follow_their_weights() {
        let mut rng = Rng::new(5);
        let mut counts = [0usize; 3];
        for _ in 0..30_000 {
            counts[rng.weighted_index(&[0.5, 0.0, 1.5])] += 1;
        }
        assert_eq!(counts[1], 0);
        let share = counts[0] as f64 / 30_000.0;
        assert!((share - 0.25).abs() < 0.01, "{share}");
    }

    #[test]
    fn the_normal_draw_has_its_mean_and_spread() {
        let mut rng = Rng::new(9);
        let draws: Vec<f64> = (0..40_000).map(|_| rng.normal(1.0, 2.0)).collect();
        let mean = draws.iter().sum::<f64>() / draws.len() as f64;
        let var = draws.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / draws.len() as f64;
        assert!((mean - 1.0).abs() < 0.05, "{mean}");
        assert!((var.sqrt() - 2.0).abs() < 0.05, "{var}");
    }

    #[test]
    fn sampled_indices_are_distinct() {
        let mut rng = Rng::new(3);
        let mut picks = rng.sample_indices(20, 8);
        picks.sort_unstable();
        picks.dedup();
        assert_eq!(picks.len(), 8);
        assert!(picks.iter().all(|&i| i < 20));
    }
}
