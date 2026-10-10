//! Fold state: whether each satellite is drawn on its own or folded into
//! its parent, and what each parent has folded into it (GRP-01, GRP-03,
//! GRP-06). Shared state, read by everything that draws a satellite
//! (ARCH-03); `grouping.rs` is the one writer.
//!
//! The rules are plain functions here too, so they can be tested without a
//! camera.

use std::collections::HashMap;

use bevy::prelude::*;

use crate::sim::body::{BodyClass, Ownership};

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
    pub has_foreign: bool,
    pub has_own_ship: bool,
}

/// GRP-06: what the "+N" pill has to show besides the count - an orange
/// outline if any folded satellite is foreign, and a blue dot if own ships
/// are parked there.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct PillFlags {
    pub has_foreign: bool,
    pub has_own_ship: bool,
}

/// GRP-06, as a plain function over the folded satellites' class and
/// ownership. "Own ships parked there" is an own *ship*, not any own
/// satellite: a state-owned moon is not a parked ship.
pub fn pill_flags(folded: &[(BodyClass, Ownership)]) -> PillFlags {
    PillFlags {
        has_foreign: folded
            .iter()
            .any(|(_, ownership)| *ownership == Ownership::Foreign),
        has_own_ship: folded
            .iter()
            .any(|(class, ownership)| *ownership == Ownership::Own && *class == BodyClass::Ship),
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

#[cfg(test)]
mod tests {
    use super::*;

    const UNFOLD_AT: f32 = 24.0;
    const FOLD_BELOW: f32 = 20.0;

    #[test]
    fn grp_01_unfolds_at_or_above_r_show() {
        assert!(!next_folded(UNFOLD_AT, true, UNFOLD_AT, FOLD_BELOW));
        assert!(!next_folded(100.0, true, UNFOLD_AT, FOLD_BELOW));
    }

    #[test]
    fn grp_01_folds_below_fold_threshold() {
        assert!(next_folded(FOLD_BELOW - 0.01, false, UNFOLD_AT, FOLD_BELOW));
        assert!(next_folded(0.0, false, UNFOLD_AT, FOLD_BELOW));
    }

    /// The whole point of the band: inside it, neither threshold fires, so
    /// a satellite sitting there keeps whatever state it arrived with.
    #[test]
    fn grp_01_hysteresis_keeps_state_inside_the_band() {
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
    #[test]
    fn grp_01_hysteresis_no_flicker_on_a_round_trip() {
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
    fn grp_03_fold_anim_travels_to_the_right_end() {
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
    fn grp_03_fold_anim_reverses_from_where_it_is() {
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
    fn grp_03_fold_anim_takes_the_tunable_duration() {
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

    /// GRP-06: the pill's orange outline tracks any foreign satellite, and
    /// the blue dot only an own *ship*.
    #[test]
    fn grp_06_pill_flags_follow_the_folded_satellites() {
        let halden = [
            (BodyClass::Moon, Ownership::Own),
            (BodyClass::Station, Ownership::Own),
            (BodyClass::Ship, Ownership::Foreign),
            (BodyClass::Ship, Ownership::Own),
        ];
        assert_eq!(
            pill_flags(&halden),
            PillFlags {
                has_foreign: true,
                has_own_ship: true
            }
        );
    }

    /// GRP-06: own moons are not parked ships, and a foreign moon still
    /// raises the outline.
    #[test]
    fn grp_06_pill_flags_distinguish_ships_from_other_satellites() {
        assert_eq!(
            pill_flags(&[(BodyClass::Moon, Ownership::Own)]),
            PillFlags {
                has_foreign: false,
                has_own_ship: false
            }
        );
        assert_eq!(
            pill_flags(&[(BodyClass::Moon, Ownership::Foreign)]),
            PillFlags {
                has_foreign: true,
                has_own_ship: false
            }
        );
        assert_eq!(
            pill_flags(&[(BodyClass::Ship, Ownership::Foreign)]),
            PillFlags {
                has_foreign: true,
                has_own_ship: false
            }
        );
    }

    /// GRP-06: a group with nothing in it raises neither flag.
    #[test]
    fn grp_06_pill_flags_of_an_empty_group_are_clear() {
        assert_eq!(pill_flags(&[]), PillFlags::default());
    }
}
