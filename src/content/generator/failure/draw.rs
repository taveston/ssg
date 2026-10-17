//! The draw of which links failed (SGEN-70, SGEN-71): shares of working,
//! dormant and collapsed links shifted a little per map, and a failure
//! weight per link that mixes its traffic with chance. The links with the
//! highest weights fail; of those, a random few are dormant and the rest
//! collapse.

use crate::content::generator::network::LinkState;
use crate::content::generator::rng::Rng;
use crate::content::generator::tunables::GeneratorTunables;

/// The draw's shares for one map (SGEN-70): `draw_working_share` and
/// `draw_dormant_share` each shifted by a random amount within ±
/// `state_share_shift`, and the rest collapsed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shares {
    pub working: f64,
    pub dormant: f64,
    pub collapsed: f64,
}

/// This map's shares (SGEN-70). None is below 0.
pub fn draw_shares(rng: &mut Rng, t: &GeneratorTunables) -> Shares {
    let shift = t.state_share_shift;
    let working = (t.draw_working_share + rng.uniform(-shift, shift)).clamp(0.0, 1.0);
    let dormant = (t.draw_dormant_share + rng.uniform(-shift, shift)).clamp(0.0, 1.0 - working);
    Shares {
        working,
        dormant,
        collapsed: 1.0 - working - dormant,
    }
}

/// Each link's failure weight (SGEN-71): β × t + (1 − β) × u, with β =
/// `failure_traffic_weight`, t the mean overall score of its two ends
/// divided by the largest such mean, and u uniform in [0, 1). Busier
/// links failed more. `traffic` holds each link's mean score.
pub fn failure_weights(traffic: &[f64], rng: &mut Rng, t: &GeneratorTunables) -> Vec<f64> {
    let busiest = traffic.iter().copied().fold(0.0, f64::max);
    let beta = t.failure_traffic_weight;
    traffic
        .iter()
        .map(|&mean| {
            let share = if busiest > 0.0 { mean / busiest } else { 0.0 };
            beta * share + (1.0 - beta) * rng.unit()
        })
        .collect()
}

/// Each link's state after the draw (SGEN-71): with L links, the
/// round(d × L) plus round(c × L) with the highest weights fail, d and c
/// being the dormant and collapsed shares; of the failed, round(d × L)
/// chosen at random are dormant (faults are set later) and the rest
/// collapse.
pub fn draw_states(weights: &[f64], shares: Shares, rng: &mut Rng) -> Vec<LinkState> {
    let count = weights.len();
    let dormant = (shares.dormant * count as f64).round() as usize;
    let collapsed = (shares.collapsed * count as f64).round() as usize;
    let failed = (dormant + collapsed).min(count);
    let mut order: Vec<usize> = (0..count).collect();
    order.sort_by(|&a, &b| weights[b].total_cmp(&weights[a]).then(a.cmp(&b)));
    let mut states = vec![LinkState::Working; count];
    for &link in &order[..failed] {
        states[link] = LinkState::Collapsed;
    }
    for pick in rng.sample_indices(failed, dormant.min(failed)) {
        states[order[pick]] = LinkState::Dormant {
            faulted: [false, false],
        };
    }
    states
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SGEN-70: the shares are the draw's, each shifted within the limit,
    /// and sum to 1.
    #[test]
    fn shares_shift_within_their_limit() {
        let t = GeneratorTunables::default();
        let mut rng = Rng::new(70);
        let (mut low, mut high) = (f64::INFINITY, f64::NEG_INFINITY);
        for _ in 0..2000 {
            let s = draw_shares(&mut rng, &t);
            assert!((s.working - 0.25).abs() <= 0.05 + 1e-12);
            assert!((s.dormant - 0.05).abs() <= 0.05 + 1e-12);
            assert!((s.working + s.dormant + s.collapsed - 1.0).abs() < 1e-12);
            low = low.min(s.working);
            high = high.max(s.working);
        }
        assert!(low < 0.21 && high > 0.29, "the shift spans its range");
    }

    /// SGEN-71: weights mix traffic and chance; the busiest links fail
    /// first, with the counts the shares give.
    #[test]
    fn the_heaviest_links_fail_in_the_counts_the_shares_give() {
        let all_traffic = GeneratorTunables {
            failure_traffic_weight: 1.0,
            ..GeneratorTunables::default()
        };
        let traffic = [0.2, 0.4, 0.8, 0.1];
        let weights = failure_weights(&traffic, &mut Rng::new(1), &all_traffic);
        assert_eq!(weights, vec![0.25, 0.5, 1.0, 0.125]);
        let shares = Shares {
            working: 0.5,
            dormant: 0.25,
            collapsed: 0.25,
        };
        let states = draw_states(&weights, shares, &mut Rng::new(2));
        // Links 2 and 1 weigh most and fail; one of them is dormant.
        assert_eq!(states[0], LinkState::Working);
        assert_eq!(states[3], LinkState::Working);
        let failed = [states[1], states[2]];
        assert!(failed.contains(&LinkState::Collapsed));
        assert!(
            failed
                .iter()
                .any(|s| matches!(s, LinkState::Dormant { .. }))
        );
        // With β = 0.5 a weight lies between β t and β t + 0.5.
        let t = GeneratorTunables::default();
        let mixed = failure_weights(&traffic, &mut Rng::new(3), &t);
        for (weight, mean) in mixed.iter().zip(traffic) {
            let base = 0.5 * mean / 0.8;
            assert!(*weight >= base && *weight < base + 0.5);
        }
    }
}
