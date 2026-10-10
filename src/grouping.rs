//! Satellite grouping (`docs/spec/system-view.md`, GRP-01 to GRP-08).
//!
//! A satellite is anything orbiting a body other than the star - moons,
//! stations, ships parked in orbit. It is drawn individually only while its
//! orbit's on-screen radius is at least R_show; otherwise it folds into its
//! parent and the parent carries a "+N" count pill instead.
//!
//! Satellites are identified structurally: `scene.rs` only ever inserts
//! `ChildOf` for bodies spawned under a parent, so "has `OrbitalBody` and
//! `ChildOf`" is exactly the set of satellites, with no class matching and
//! no special case for future ship types.

use std::collections::HashMap;

use bevy::prelude::*;

use crate::Stage;
use crate::orbits::OrbitalBody;
use crate::system_data::{BodyClass, Ownership};
use crate::tunables::Tunables;

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
    crate::anim::advance_towards(anim_t, target, step)
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

pub struct GroupingPlugin;

impl Plugin for GroupingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FoldSummary>()
            .add_systems(Update, update_grouping.in_set(Stage::Grouping));
    }
}

#[allow(clippy::too_many_arguments)]
fn update_grouping(
    time: Res<Time>,
    tunables: Res<Tunables>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    transforms: Query<&GlobalTransform>,
    parents: Query<&ChildOf>,
    classes: Query<(&BodyClass, &Ownership)>,
    mut satellites: Query<(Entity, &OrbitalBody, &ChildOf, &mut SatelliteFoldState)>,
    mut summary: ResMut<FoldSummary>,
) {
    let Ok((camera, camera_transform)) = camera_q.single() else {
        return;
    };

    // Measure orbit radius along the camera's least-foreshortened screen
    // axis, flattened into the orbital plane. This depends only on zoom and
    // tilt, never on where the satellite currently is in its orbit, so the
    // test can't flicker as a satellite comes round.
    let right = camera_transform.right().as_vec3();
    let right_flat = Vec3::new(right.x, 0.0, right.z).normalize_or_zero();

    let step = crate::anim::step_per_frame(time.delta_secs(), tunables.fold_unfold_anim_ms);

    for (entity, orbital, child_of, mut fold) in &mut satellites {
        let Ok(parent_transform) = transforms.get(child_of.parent()) else {
            continue;
        };
        let parent_world = parent_transform.translation();

        let on_screen_radius = match (
            camera.world_to_viewport(camera_transform, parent_world),
            camera.world_to_viewport(
                camera_transform,
                parent_world + right_flat * orbital.orbit_radius,
            ),
        ) {
            (Ok(centre), Ok(edge)) => centre.distance(edge),
            // Off-screen or behind the camera: leave the latch alone.
            _ => {
                let _ = entity;
                continue;
            }
        };

        fold.folded = next_folded(
            on_screen_radius,
            fold.folded,
            tunables.unfold_at_px,
            tunables.fold_below_px,
        );

        fold.anim_t = advance_fold_anim(fold.anim_t, fold.folded, step);
    }

    // A satellite of a folded satellite stays hidden no matter what its own
    // test said. Hierarchies here are shallow, so a small memoised walk is
    // plenty. "Hidden" means *fully* folded, so the slide in and out of the
    // parent stays visible in both directions.
    let folded_now: HashMap<Entity, bool> = satellites
        .iter()
        .map(|(entity, _, _, fold)| (entity, fold.folded && fold.anim_t <= 0.001))
        .collect();

    let mut cache: HashMap<Entity, bool> = HashMap::new();
    let hidden: Vec<(Entity, bool)> = satellites
        .iter()
        .map(|(entity, _, _, _)| {
            let hidden = ancestor_hidden(entity, &folded_now, &parents, &mut cache);
            (entity, hidden)
        })
        .collect();

    let hidden_lookup: HashMap<Entity, bool> = hidden.iter().copied().collect();
    for (entity, _, _, mut fold) in &mut satellites {
        fold.effective_hidden = hidden_lookup.get(&entity).copied().unwrap_or(fold.folded);
    }

    summary.0.clear();
    for (entity, _, child_of, fold) in &satellites {
        // Count by the latch, not by `effective_hidden`: a satellite joins
        // or leaves the count the moment it starts folding or unfolding,
        // so the "+N" and the sliding marker hand over cleanly.
        if !fold.folded {
            continue;
        }
        let parent = child_of.parent();
        // Only count a satellite against the nearest *visible* ancestor:
        // if its own parent is folded too, it's already represented there.
        if folded_now.get(&parent).copied().unwrap_or(false) {
            continue;
        }
        let group = summary.0.entry(parent).or_default();
        group.members.push(entity);
        if let Ok((class, ownership)) = classes.get(entity) {
            // GRP-06, one member at a time: the flags are a disjunction, so
            // folding each member's own flags in is the same as asking
            // `pill_flags` about the whole group.
            let flags = pill_flags(&[(*class, *ownership)]);
            group.has_foreign |= flags.has_foreign;
            group.has_own_ship |= flags.has_own_ship;
        }
    }
}

fn ancestor_hidden(
    entity: Entity,
    folded: &HashMap<Entity, bool>,
    parents: &Query<&ChildOf>,
    cache: &mut HashMap<Entity, bool>,
) -> bool {
    if let Some(known) = cache.get(&entity) {
        return *known;
    }
    let own = folded.get(&entity).copied().unwrap_or(false);
    let hidden = own
        || match parents.get(entity) {
            Ok(child_of) => {
                let parent = child_of.parent();
                // Only satellites appear in `folded`; a planet parent ends
                // the walk.
                folded.contains_key(&parent) && ancestor_hidden(parent, folded, parents, cache)
            }
            Err(_) => false,
        };
    cache.insert(entity, hidden);
    hidden
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
            t = advance_fold_anim(t, false, crate::anim::step_per_frame(1.0 / 60.0, anim_ms));
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
