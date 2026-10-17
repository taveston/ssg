//! Satellite grouping (`docs/spec/system-view.md`, GRP-01 to GRP-05,
//! GRP-08): measures each satellite's orbit on screen and folds or unfolds
//! it, then summarises what each parent carries for its moon ring
//! (`moon_ring.rs`) and its object marker (`multi_markers/`).
//!
//! A satellite is anything orbiting a body other than the star - moons,
//! stations, ships parked in orbit. Satellites are identified structurally:
//! the simulation only ever gives `ChildOf` to bodies that orbit a parent,
//! so "has `Orbit` and `ChildOf`" is exactly the set of satellites, with no
//! class matching and no special case for future ship types. GRP-08 (ships
//! in transit are never folded) falls out of that: a ship in flight has no
//! orbit to fold, so it loses its fold state when it leaves its orbit and
//! gets a new one when it joins another (PLNR-68).

use std::collections::HashMap;

use bevy::prelude::*;

use crate::sim::load::SystemLoadApp;

use super::anim::step_per_frame;
use super::fold::{
    FoldSummary, SatelliteFoldState, advance_fold_anim, fully_folded, hidden_by_ancestry,
    next_folded,
};
use super::scale::km_to_render;
use super::tunables::Tunables;
use super::world::ViewNow;
use crate::sim::orbit::Orbit;
use crate::sim::track::ShipRecord;
use crate::{Boot, Stage};

pub struct GroupingPlugin;

impl Plugin for GroupingPlugin {
    fn build(&self, app: &mut App) {
        app.reset_with_system::<FoldSummary>()
            .add_load_systems(Boot::Derive, attach_fold_state)
            .add_systems(
                Update,
                (
                    attach_fold_state,
                    drop_fold_state_in_flight,
                    fold_satellites,
                    resolve_hidden,
                    summarise_folds,
                )
                    .chain()
                    .in_set(Stage::Grouping),
            );
    }
}

/// Satellites that haven't been given fold state yet.
type UnfoldedSatellites = (With<Orbit>, With<ChildOf>, Without<SatelliteFoldState>);

/// GRP-04: every satellite starts folded - the opening view is the whole
/// system, where no moon or station is drawn individually. A ship that
/// joins a body's orbit at the end of a transfer becomes a satellite then.
fn attach_fold_state(mut commands: Commands, satellites: Query<Entity, UnfoldedSatellites>) {
    for entity in &satellites {
        commands
            .entity(entity)
            .insert(SatelliteFoldState::default());
    }
}

/// GRP-08 / PLNR-68: a ship that leaves its orbit is in flight and is
/// never folded, so it drops the fold state it had as a satellite.
fn drop_fold_state_in_flight(
    mut commands: Commands,
    in_flight: Query<Entity, (With<SatelliteFoldState>, Without<Orbit>)>,
) {
    for entity in &in_flight {
        commands.entity(entity).remove::<SatelliteFoldState>();
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

/// GRP-05: what each parent has folded into it: natural bodies for its
/// moon ring (GRP-10), and ships, groups and stations (those with a
/// track record) for its object marker (GRP-12).
fn summarise_folds(
    satellites: Query<(Entity, &ChildOf, &SatelliteFoldState, Has<ShipRecord>)>,
    mut summary: ResMut<FoldSummary>,
) {
    summary.0.clear();
    for (entity, child_of, fold, object) in &satellites {
        // Count by the latch, not by `effective_hidden`: a satellite joins
        // or leaves the count the moment it starts folding or unfolding,
        // so the fold marker and the sliding marker hand over cleanly.
        if !fold.folded {
            continue;
        }
        // Only count a satellite against the nearest *visible* ancestor:
        // if its own parent is folded too, it's already represented there.
        let parent = child_of.parent();
        let parent_folded = satellites.get(parent).is_ok_and(|(_, _, parent_fold, _)| {
            fully_folded(parent_fold.folded, parent_fold.anim_t)
        });
        if parent_folded {
            continue;
        }
        let group = summary.0.entry(parent).or_default();
        if object {
            group.objects.push(entity);
        } else {
            group.bodies.push(entity);
        }
    }
}
