//! Satellite grouping (`docs/spec/system-view.md`, GRP-01 to GRP-05,
//! GRP-08): measures each satellite's orbit on screen and folds or unfolds
//! it, then summarises what each parent carries for its "+N" pill.
//!
//! A satellite is anything orbiting a body other than the star - moons,
//! stations, ships parked in orbit. Satellites are identified structurally:
//! the simulation only ever gives `ChildOf` to bodies that orbit a parent,
//! so "has `Orbit` and `ChildOf`" is exactly the set of satellites, with no
//! class matching and no special case for future ship types. GRP-08 (ships
//! in transit are never folded) falls out of that: a ship in transit has no
//! parent to fold into.

use std::collections::HashMap;

use bevy::prelude::*;

use super::anim::step_per_frame;
use super::fold::{
    FoldSummary, SatelliteFoldState, advance_fold_anim, fully_folded, hidden_by_ancestry,
    next_folded, pill_flags,
};
use super::scale::km_to_render;
use super::tunables::Tunables;
use super::world::ViewNow;
use crate::sim::body::{BodyClass, Ownership};
use crate::sim::orbit::Orbit;
use crate::{Boot, Stage};

pub struct GroupingPlugin;

impl Plugin for GroupingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FoldSummary>()
            .add_systems(Startup, attach_fold_state.in_set(Boot::Derive))
            .add_systems(
                Update,
                (fold_satellites, resolve_hidden, summarise_folds)
                    .chain()
                    .in_set(Stage::Grouping),
            );
    }
}

/// Satellites that haven't been given fold state yet.
type UnfoldedSatellites = (With<Orbit>, With<ChildOf>, Without<SatelliteFoldState>);

/// GRP-04: every satellite starts folded - the opening view is the whole
/// system, where no moon or station is drawn individually.
fn attach_fold_state(mut commands: Commands, satellites: Query<Entity, UnfoldedSatellites>) {
    for entity in &satellites {
        commands
            .entity(entity)
            .insert(SatelliteFoldState::default());
    }
}

/// GRP-01 to GRP-03: measure each satellite's orbit on screen, run the
/// hysteresis latch, and slide it towards its new state.
fn fold_satellites(
    time: Res<Time>,
    tunables: Res<Tunables>,
    view: ViewNow,
    mut satellites: Query<(&Orbit, &ChildOf, &mut SatelliteFoldState)>,
) {
    let Some((camera, camera_transform)) = view.camera() else {
        return;
    };
    // Measure orbit radius along the camera's least-foreshortened screen
    // axis, flattened into the orbital plane. This depends only on zoom and
    // tilt, never on where the satellite currently is in its orbit, so the
    // test can't flicker as a satellite comes round (GRP-02: per satellite,
    // so outer ones unfold first).
    let right = camera_transform.right().as_vec3();
    let right_flat = Vec3::new(right.x, 0.0, right.z).normalize_or_zero();
    let step = step_per_frame(time.delta_secs(), tunables.fold_unfold_anim_ms);

    for (orbit, child_of, mut fold) in &mut satellites {
        let Some(centre) = view.position(child_of.parent()) else {
            continue;
        };
        let edge = centre + right_flat * km_to_render(orbit.radius_km);
        let (Ok(centre), Ok(edge)) = (
            camera.world_to_viewport(&camera_transform, centre),
            camera.world_to_viewport(&camera_transform, edge),
        ) else {
            // Off-screen or behind the camera: leave the latch alone.
            continue;
        };
        fold.folded = next_folded(
            centre.distance(edge),
            fold.folded,
            tunables.unfold_at_px,
            tunables.fold_below_px,
        );
        fold.anim_t = advance_fold_anim(fold.anim_t, fold.folded, step);
    }
}

/// GRP-01: a satellite of a fully folded satellite is hidden too.
fn resolve_hidden(mut satellites: Query<(Entity, &ChildOf, &mut SatelliteFoldState)>) {
    let tree: HashMap<Entity, (Entity, bool)> = satellites
        .iter()
        .map(|(entity, child_of, fold)| {
            (
                entity,
                (child_of.parent(), fully_folded(fold.folded, fold.anim_t)),
            )
        })
        .collect();
    let hidden = hidden_by_ancestry(&tree);
    for (entity, _, mut fold) in &mut satellites {
        fold.effective_hidden = hidden.get(&entity).copied().unwrap_or(fold.folded);
    }
}

/// GRP-05 / GRP-06: what each parent's "+N" pill counts and flags.
fn summarise_folds(
    satellites: Query<(Entity, &ChildOf, &SatelliteFoldState)>,
    classes: Query<(&BodyClass, &Ownership)>,
    mut summary: ResMut<FoldSummary>,
) {
    summary.0.clear();
    for (entity, child_of, fold) in &satellites {
        // Count by the latch, not by `effective_hidden`: a satellite joins
        // or leaves the count the moment it starts folding or unfolding,
        // so the "+N" and the sliding marker hand over cleanly.
        if !fold.folded {
            continue;
        }
        // Only count a satellite against the nearest *visible* ancestor:
        // if its own parent is folded too, it's already represented there.
        let parent = child_of.parent();
        let parent_folded = satellites
            .get(parent)
            .is_ok_and(|(_, _, parent_fold)| fully_folded(parent_fold.folded, parent_fold.anim_t));
        if parent_folded {
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
