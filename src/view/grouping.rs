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

use super::fold::{FoldSummary, SatelliteFoldState, advance_fold_anim, next_folded, pill_flags};
use super::scale::km_to_render;
use super::tunables::Tunables;
use crate::sim::body::{BodyClass, Ownership};
use crate::sim::orbit::Orbit;
use crate::{Boot, Stage};

pub struct GroupingPlugin;

impl Plugin for GroupingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FoldSummary>()
            .add_systems(Startup, attach_fold_state.in_set(Boot::Derive))
            .add_systems(Update, update_grouping.in_set(Stage::Grouping));
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

#[allow(clippy::too_many_arguments)]
fn update_grouping(
    time: Res<Time>,
    tunables: Res<Tunables>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    transforms: Query<&GlobalTransform>,
    parents: Query<&ChildOf>,
    classes: Query<(&BodyClass, &Ownership)>,
    mut satellites: Query<(Entity, &Orbit, &ChildOf, &mut SatelliteFoldState)>,
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

    let step = crate::view::anim::step_per_frame(time.delta_secs(), tunables.fold_unfold_anim_ms);

    for (entity, orbital, child_of, mut fold) in &mut satellites {
        let Ok(parent_transform) = transforms.get(child_of.parent()) else {
            continue;
        };
        let parent_world = parent_transform.translation();

        let on_screen_radius = match (
            camera.world_to_viewport(camera_transform, parent_world),
            camera.world_to_viewport(
                camera_transform,
                parent_world + right_flat * km_to_render(orbital.radius_km),
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
