//! Fold state: whether each satellite is drawn on its own or folded into
//! its parent, and what each parent has folded into it (GRP-01, GRP-03,
//! GRP-06). Shared state, read by everything that draws a satellite
//! (ARCH-03); `grouping.rs` is the one writer.
//!
//! The rules are plain functions here too, so they can be tested without a
//! camera.

use std::collections::HashMap;

use bevy::prelude::*;

use crate::sim::body::{Affiliation, BodyClass};

/// Per-satellite fold state. `folded` is the hysteresis latch;
/// `effective_hidden` additionally accounts for a folded ancestor, so
/// consumers don't have to walk the hierarchy themselves.
#[derive(Component)]
pub struct SatelliteFoldState {
    pub folded: bool,
    /// 0.0 = sitting on its parent (folded), 1.0 = at its true position.
    pub anim_t: f32,
    pub effective_hidden: bool,
}

impl Default for SatelliteFoldState {
    fn default() -> Self {
        // Everything starts folded: the default view is the whole system,
        // where no moon or station should be drawn individually.
        Self {
            folded: true,
            anim_t: 0.0,
            effective_hidden: true,
        }
    }
}

#[derive(Default)]
pub struct FoldedGroup {
    pub members: Vec<Entity>,
    /// The most pressing affiliation among the members, if any presses.
    pub pressing: Option<Affiliation>,
    pub has_own_ship: bool,
}

/// GRP-06: what the "+N" pill has to show besides the count - an outline
/// in the colour of the most pressing affiliation among its folded
/// satellites, and a blue dot if own ships are parked there.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PillFlags {
    pub pressing: Option<Affiliation>,
    pub has_own_ship: bool,
}

/// GRP-06: how pressing an affiliation is for the pill's outline: hostile,
/// then unknown, then neutral or allied; own presses not at all.
fn urgency(affiliation: Affiliation) -> u8 {
    match affiliation {
        Affiliation::Hostile => 3,
        Affiliation::Unknown => 2,
        Affiliation::Neutral | Affiliation::Allied => 1,
        Affiliation::Own => 0,
    }
}

/// GRP-06: the more pressing of two, if either presses at all.
pub fn more_pressing(a: Option<Affiliation>, b: Option<Affiliation>) -> Option<Affiliation> {
    [a, b]
        .into_iter()
        .flatten()
        .filter(|aff| urgency(*aff) > 0)
        .max_by_key(|aff| urgency(*aff))
}

/// GRP-06, as a plain function over the folded satellites' class and
/// affiliation. "Own ships parked there" is an own *ship*, not any own
/// satellite: a state-owned moon or station is not a parked ship.
pub fn pill_flags(folded: &[(BodyClass, Affiliation)]) -> PillFlags {
    PillFlags {
        pressing: folded.iter().fold(None, |most, (_, affiliation)| {
            more_pressing(most, Some(*affiliation))
        }),
        has_own_ship: folded.iter().any(|(class, affiliation)| {
            *affiliation == Affiliation::Own && *class == BodyClass::Ship
        }),
    }
}

/// GRP-03: a satellite slides out to its true position when it unfolds and
/// back onto its parent when it folds, over `fold_unfold_anim_ms`. 0.0 sits
/// on the parent, 1.0 is the true position; `step` is one frame's travel.
pub fn advance_fold_anim(anim_t: f32, folded: bool, step: f32) -> f32 {
    let target = if folded { 0.0 } else { 1.0 };
    crate::view::anim::advance_towards(anim_t, target, step)
}

/// What each parent currently has folded into it, rebuilt every frame.
#[derive(Resource, Default)]
pub struct FoldSummary(pub HashMap<Entity, FoldedGroup>);

/// GRP-01: a satellite is drawn individually only while its orbit's
/// on-screen radius is at least `unfold_at_px`; below `fold_below_px` it
/// folds into its parent. Between the two it keeps its current state -
/// that band is the hysteresis that stops it flickering at the boundary.
pub fn next_folded(
    on_screen_radius: f32,
    currently_folded: bool,
    unfold_at_px: f32,
    fold_below_px: f32,
) -> bool {
    if on_screen_radius >= unfold_at_px {
        false
    } else if on_screen_radius < fold_below_px {
        true
    } else {
        currently_folded
    }
}

/// How close to its parent a folding satellite's slide must be to count as
/// fully folded - the point at which it stops drawing at all.
const FULLY_FOLDED_T: f32 = 0.001;

/// GRP-01 / GRP-03: folded *and* finished sliding in. Until then the slide
/// stays visible, in both directions.
pub fn fully_folded(folded: bool, anim_t: f32) -> bool {
    folded && anim_t <= FULLY_FOLDED_T
}

/// GRP-01: a satellite of a folded satellite stays hidden whatever its own
/// test says. `satellites` maps each satellite to its parent and whether it
/// is itself fully folded; the walk stops at a parent that isn't a
/// satellite (a planet). Returns, per satellite, whether it is hidden.
pub fn hidden_by_ancestry(satellites: &HashMap<Entity, (Entity, bool)>) -> HashMap<Entity, bool> {
    fn walk(
        entity: Entity,
        satellites: &HashMap<Entity, (Entity, bool)>,
        memo: &mut HashMap<Entity, bool>,
    ) -> bool {
        if let Some(&known) = memo.get(&entity) {
            return known;
        }
        let hidden = match satellites.get(&entity) {
            Some(&(parent, own)) => own || walk(parent, satellites, memo),
            // Not a satellite: the walk ends here, visible.
            None => false,
        };
        memo.insert(entity, hidden);
        hidden
    }
    let mut memo = HashMap::new();
    satellites
        .keys()
        .map(|&entity| (entity, walk(entity, satellites, &mut memo)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// GRP-01 / GRP-03: a satellite only stops drawing once it has folded
    /// and slid all the way in.
    #[test]
    fn fully_folded_waits_for_the_slide() {
        assert!(fully_folded(true, 0.0));
        assert!(!fully_folded(true, 0.4), "still sliding in");
        assert!(!fully_folded(false, 0.0), "starting to slide out");
    }

    /// GRP-01: a moon of a folded moon is hidden even if its own orbit is
    /// large enough to show; folding stops at a planet.
    #[test]
    fn satellites_of_folded_satellites_stay_hidden() {
        let mut world = World::new();
        let [planet, moon, submoon, other] = [(); 4].map(|_| world.spawn_empty().id());
        let mut satellites = HashMap::new();
        satellites.insert(moon, (planet, true));
        satellites.insert(submoon, (moon, false));
        satellites.insert(other, (planet, false));
        let hidden = hidden_by_ancestry(&satellites);
        assert!(hidden[&moon]);
        assert!(hidden[&submoon], "hidden by its folded parent");
        assert!(!hidden[&other]);
        assert!(!hidden.contains_key(&planet));
    }

    /// Covers GRP-01.
    #[test]
    fn no_satellites_means_nothing_hidden() {
        assert!(hidden_by_ancestry(&HashMap::new()).is_empty());
    }

    const UNFOLD_AT: f32 = 24.0;
    const FOLD_BELOW: f32 = 20.0;

    /// Covers GRP-01.
    #[test]
    fn unfolds_at_or_above_r_show() {
        assert!(!next_folded(UNFOLD_AT, true, UNFOLD_AT, FOLD_BELOW));
        assert!(!next_folded(100.0, true, UNFOLD_AT, FOLD_BELOW));
    }

    /// Covers GRP-01.
    #[test]
    fn folds_below_fold_threshold() {
        assert!(next_folded(FOLD_BELOW - 0.01, false, UNFOLD_AT, FOLD_BELOW));
        assert!(next_folded(0.0, false, UNFOLD_AT, FOLD_BELOW));
    }

    /// The whole point of the band: inside it, neither threshold fires, so
    /// a satellite sitting there keeps whatever state it arrived with.
    /// Covers GRP-01.
    #[test]
    fn hysteresis_keeps_state_inside_the_band() {
        for radius in [20.0_f32, 22.0, 23.99] {
            assert!(
                next_folded(radius, true, UNFOLD_AT, FOLD_BELOW),
                "{radius}px should stay folded"
            );
            assert!(
                !next_folded(radius, false, UNFOLD_AT, FOLD_BELOW),
                "{radius}px should stay unfolded"
            );
        }
    }

    /// Crossing the band and coming back must not flicker: it takes a full
    /// traverse to flip, in either direction.
    /// Covers GRP-01.
    #[test]
    fn hysteresis_no_flicker_on_a_round_trip() {
        let mut folded = true;
        // Drifting up through the band changes nothing until R_show.
        for radius in [19.0_f32, 20.0, 22.0, 23.9] {
            folded = next_folded(radius, folded, UNFOLD_AT, FOLD_BELOW);
            assert!(folded, "still folded at {radius}px");
        }
        folded = next_folded(24.0, folded, UNFOLD_AT, FOLD_BELOW);
        assert!(!folded);
        // And back down: it stays unfolded until it drops under fold_below.
        for radius in [23.9_f32, 22.0, 20.0] {
            folded = next_folded(radius, folded, UNFOLD_AT, FOLD_BELOW);
            assert!(!folded, "still unfolded at {radius}px");
        }
        folded = next_folded(19.9, folded, UNFOLD_AT, FOLD_BELOW);
        assert!(folded);
    }

    /// GRP-03: unfolding slides out to the true position, folding slides
    /// back onto the parent, and neither overshoots.
    #[test]
    fn fold_anim_travels_to_the_right_end() {
        assert!((advance_fold_anim(0.0, false, 0.25) - 0.25).abs() < 1e-6);
        assert!((advance_fold_anim(1.0, true, 0.25) - 0.75).abs() < 1e-6);
        assert_eq!(advance_fold_anim(0.9, false, 0.25), 1.0);
        assert_eq!(advance_fold_anim(0.1, true, 0.25), 0.0);
        assert_eq!(advance_fold_anim(1.0, false, 0.25), 1.0);
        assert_eq!(advance_fold_anim(0.0, true, 0.25), 0.0);
    }

    /// GRP-03: a satellite that flips state part-way through the animation
    /// turns round from where it is, so the slide never jumps.
    #[test]
    fn fold_anim_reverses_from_where_it_is() {
        let mut t = 0.0;
        for _ in 0..2 {
            t = advance_fold_anim(t, false, 0.25);
        }
        assert!((t - 0.5).abs() < 1e-6);
        // Folds again from half way out.
        t = advance_fold_anim(t, true, 0.25);
        assert!((t - 0.25).abs() < 1e-6);
    }

    /// GRP-03: a whole animation takes `fold_unfold_anim_ms`, however the
    /// frames fall.
    #[test]
    fn fold_anim_takes_the_tunable_duration() {
        let anim_ms = 200.0;
        let mut t = 0.0;
        let mut elapsed = 0.0;
        while t < 1.0 && elapsed < 1.0 {
            t = advance_fold_anim(
                t,
                false,
                crate::view::anim::step_per_frame(1.0 / 60.0, anim_ms),
            );
            elapsed += 1.0 / 60.0;
        }
        assert_eq!(t, 1.0);
        assert!(
            (elapsed - anim_ms / 1000.0).abs() < 2.0 / 60.0,
            "took {elapsed}s"
        );
    }

    /// GRP-06: the outline takes the most pressing affiliation - hostile,
    /// then unknown, then neutral or allied - and the blue dot only an own
    /// *ship*.
    #[test]
    fn pill_flags_follow_the_folded_satellites() {
        use Affiliation::*;
        let flags = |folded: &[(BodyClass, Affiliation)]| pill_flags(folded);
        let halden = [
            (BodyClass::Moon, Own),
            (BodyClass::Ship, Neutral),
            (BodyClass::Ship, Hostile),
            (BodyClass::Ship, Unknown),
            (BodyClass::Ship, Own),
        ];
        assert_eq!(
            flags(&halden),
            PillFlags {
                pressing: Some(Hostile),
                has_own_ship: true
            }
        );
        assert_eq!(
            flags(&[(BodyClass::Ship, Allied), (BodyClass::Ship, Unknown)]).pressing,
            Some(Unknown)
        );
        assert_eq!(flags(&[(BodyClass::Ship, Allied)]).pressing, Some(Allied));
        let own_only = flags(&[(BodyClass::Moon, Own), (BodyClass::Station, Own)]);
        assert_eq!(
            own_only,
            PillFlags::default(),
            "own moons and stations aren't parked ships"
        );
    }

    /// GRP-06: a group with nothing in it raises neither flag.
    #[test]
    fn pill_flags_of_an_empty_group_are_clear() {
        assert_eq!(pill_flags(&[]), PillFlags::default());
    }
}
