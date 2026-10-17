//! Which systems carry gates (SGEN-55, SGEN-57): a core picked by score,
//! with a quota for each reason; connectors that join it (`connectors`);
//! and rounds of dropping and adding until exactly N systems are gated.

use bevy::math::DVec3;

use super::Field;
use super::connectors::connect;
use crate::content::generator::graph::groups_within;
use crate::content::generator::network::{Pick, Reason};
use crate::content::generator::tunables::GeneratorTunables;

/// A gated system's id and how it was picked.
pub type Picked = (usize, Pick);

/// Exactly `gated_count` gated systems, by id, each with how it was picked
/// (SGEN-55 to SGEN-57); fewer only if the sector has fewer eligible
/// systems. `range_km` is the link range, which grows whenever a step
/// cannot be done within it (SGEN-60).
pub fn choose(
    gated_count: usize,
    field: &Field,
    range_km: &mut f64,
    t: &GeneratorTunables,
) -> Vec<Picked> {
    let n = gated_count.min(field.eligible.len());
    let mut quota = (t.reason_quota * n as f64).ceil() as usize;
    loop {
        if let Some(mut picked) = attempt(n, quota, field, range_km, t) {
            picked.sort_by_key(|&(id, _)| id);
            return picked;
        }
        // The quota picks and their connectors alone are too many, or the
        // count cannot be met without dropping a quota pick: lower every
        // quota by one and start again (SGEN-57). With no quota every
        // attempt succeeds.
        quota = quota
            .checked_sub(1)
            .expect("an attempt with no quota always succeeds");
    }
}

/// One attempt at the gated set with this quota per reason (SGEN-55,
/// SGEN-57). `None` if the quota is too high for N.
fn attempt(
    n: usize,
    quota: usize,
    field: &Field,
    range_km: &mut f64,
    t: &GeneratorTunables,
) -> Option<Vec<Picked>> {
    let quota_picks = fill_quotas(quota, field);
    if quota > 0 {
        let ids: Vec<usize> = quota_picks.iter().map(|&(id, _)| id).collect();
        if ids.len() + connect(&ids, field, range_km, t).len() > n {
            return None;
        }
    }
    let core_size = (t.core_share * n as f64).round() as usize;
    let mut core: Vec<Picked> = quota_picks;
    for id in field.by_overall() {
        if core.len() >= core_size {
            break;
        }
        if !contains(&core, id) {
            core.push((id, Pick::Score));
        }
    }
    for round in 0..=t.max_gate_rounds {
        let ids: Vec<usize> = core.iter().map(|&(id, _)| id).collect();
        let mut chosen = core.clone();
        chosen.extend(connect(&ids, field, range_km, t));
        if chosen.len() == n {
            return Some(chosen);
        }
        if round == t.max_gate_rounds {
            return fallback(chosen, n, field, range_km, t);
        }
        if chosen.len() > n {
            // Drop the lowest-scoring core system that no quota needs.
            let lowest = lowest_scoring(&core, field, |_| true)?;
            core.retain(|&(id, _)| id != lowest);
        } else if let Some(best) = best_unchosen(&chosen, field, |_| true) {
            core.push((best, Pick::Score));
        } else {
            return fallback(chosen, n, field, range_km, t);
        }
    }
    unreachable!("the last round returns")
}

/// The quota picks (SGEN-55): in turns, each reason takes its
/// highest-scoring eligible system not yet taken, until each has `quota`
/// or runs out of systems. Ties go to the lower id.
fn fill_quotas(quota: usize, field: &Field) -> Vec<Picked> {
    let rankings: Vec<Vec<usize>> = Reason::ALL.iter().map(|&r| field.by_reason(r)).collect();
    let mut next = [0usize; 4];
    let mut counts = [0usize; 4];
    let mut picked: Vec<Picked> = Vec::new();
    loop {
        let mut progressed = false;
        for (r, reason) in Reason::ALL.iter().enumerate() {
            if counts[r] >= quota {
                continue;
            }
            while next[r] < rankings[r].len() && contains(&picked, rankings[r][next[r]]) {
                next[r] += 1;
            }
            if let Some(&id) = rankings[r].get(next[r]) {
                picked.push((id, Pick::Quota(*reason)));
                counts[r] += 1;
                progressed = true;
            }
        }
        if !progressed {
            return picked;
        }
    }
}

/// Whether `id` is among `picked`.
fn contains(picked: &[Picked], id: usize) -> bool {
    picked.iter().any(|&(p, _)| p == id)
}

/// The lowest-scoring of `picked` that is not a quota pick and passes
/// `allowed`; of equal scores, the higher id goes first, so ties keep
/// the lower id.
fn lowest_scoring(
    picked: &[Picked],
    field: &Field,
    allowed: impl Fn(usize) -> bool,
) -> Option<usize> {
    picked
        .iter()
        .filter(|(id, pick)| !matches!(pick, Pick::Quota(_)) && allowed(*id))
        .map(|&(id, _)| id)
        .min_by(|&a, &b| {
            field
                .overall(a)
                .total_cmp(&field.overall(b))
                .then(b.cmp(&a))
        })
}

/// The highest-scoring eligible system not in `chosen` that passes
/// `allowed`, ties to the lower id.
fn best_unchosen(
    chosen: &[Picked],
    field: &Field,
    allowed: impl Fn(usize) -> bool,
) -> Option<usize> {
    field
        .by_overall()
        .into_iter()
        .find(|&id| !contains(chosen, id) && allowed(id))
}

/// Whether `ids` form one group within range: every one joined to another
/// by a chain of hops no longer than `range_km`.
fn joined(ids: &[usize], field: &Field, range_km: f64) -> bool {
    let points: Vec<DVec3> = ids.iter().map(|&id| field.positions[id]).collect();
    groups_within(&points, range_km).count() <= 1
}

/// The fallback once the rounds run out (SGEN-57): remove the
/// lowest-scoring chosen system that is not a quota pick and whose loss
/// leaves the network joined, or add the highest-scoring unchosen
/// eligible system within link range of a chosen one, until there are
/// `n`. The range grows if no system is in range. `None` if nothing can
/// be removed.
fn fallback(
    mut chosen: Vec<Picked>,
    n: usize,
    field: &Field,
    range_km: &mut f64,
    t: &GeneratorTunables,
) -> Option<Vec<Picked>> {
    while chosen.len() > n {
        let ids: Vec<usize> = chosen.iter().map(|&(id, _)| id).collect();
        let keeps_joined = |id: usize| {
            let rest: Vec<usize> = ids.iter().copied().filter(|&other| other != id).collect();
            joined(&rest, field, *range_km)
        };
        let removed = lowest_scoring(&chosen, field, keeps_joined)?;
        chosen.retain(|&(id, _)| id != removed);
    }
    while chosen.len() < n {
        let near = |id: usize| {
            chosen.is_empty()
                || chosen
                    .iter()
                    .any(|&(c, _)| field.distance(c, id) <= *range_km)
        };
        match best_unchosen(&chosen, field, near) {
            Some(id) => chosen.push((id, Pick::Score)),
            None => *range_km *= 1.0 + t.link_range_growth,
        }
    }
    Some(chosen)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::generator::network::Scores;
    use bevy::math::DVec3;

    fn scores(settlement: f64, extraction: f64, industry: f64, science: f64) -> Option<Scores> {
        Some(Scores {
            settlement,
            extraction,
            industry,
            science,
        })
    }

    /// SGEN-55: quotas fill in turns, each reason taking its best system
    /// not yet taken; ties go to the lower id.
    #[test]
    fn quotas_fill_in_turns_by_score() {
        let positions = vec![DVec3::ZERO; 6];
        let all = vec![
            scores(0.9, 0.8, 0.0, 0.0),
            scores(0.8, 0.9, 0.0, 0.0),
            scores(0.1, 0.1, 0.5, 0.0),
            scores(0.1, 0.1, 0.5, 0.0),
            scores(0.0, 0.0, 0.0, 0.2),
            None,
        ];
        let field = Field::new(&positions, &all);
        let picked = fill_quotas(1, &field);
        assert_eq!(
            picked,
            vec![
                (0, Pick::Quota(Reason::Settlement)),
                (1, Pick::Quota(Reason::Extraction)),
                (2, Pick::Quota(Reason::Industry)),
                (4, Pick::Quota(Reason::Science)),
            ]
        );
        // With a quota of two, settlement's second-best is taken by
        // extraction first, so settlement falls to the next.
        let two = fill_quotas(2, &field);
        assert_eq!(two.len(), 5);
        assert!(!contains(&two, 5));
    }

    /// SGEN-57: the result has exactly N systems, all eligible, with the
    /// quota picks kept.
    #[test]
    fn the_choice_has_exactly_n_systems() {
        let t = GeneratorTunables::default();
        // Twenty eligible systems in a row, one parsec-ish apart.
        let positions: Vec<DVec3> = (0..20).map(|i| DVec3::new(i as f64, 0.0, 0.0)).collect();
        let all: Vec<Option<Scores>> = (0..20)
            .map(|i| {
                scores(
                    i as f64 / 20.0,
                    0.1,
                    0.0,
                    if i % 7 == 0 { 0.5 } else { 0.0 },
                )
            })
            .collect();
        let field = Field::new(&positions, &all);
        for n in [8, 10, 15] {
            let mut range = 1.5;
            let picked = choose(n, &field, &mut range, &t);
            assert_eq!(picked.len(), n);
            let ids: Vec<usize> = picked.iter().map(|&(id, _)| id).collect();
            assert!(joined(&ids, &field, range));
        }
    }
}
