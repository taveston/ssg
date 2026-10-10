//! Satellite grouping (`docs/presentation.md` section 2, "Satellites").
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

/// What each parent currently has folded into it, rebuilt every frame.
#[derive(Resource, Default)]
pub struct FoldSummary(pub HashMap<Entity, FoldedGroup>);

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

    let step = if tunables.fold_unfold_anim_ms > 0.0 {
        time.delta_secs() / (tunables.fold_unfold_anim_ms / 1000.0)
    } else {
        1.0
    };

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

        if on_screen_radius >= tunables.unfold_at_px {
            fold.folded = false;
        } else if on_screen_radius < tunables.fold_below_px {
            fold.folded = true;
        }

        let target = if fold.folded { 0.0 } else { 1.0 };
        if fold.anim_t < target {
            fold.anim_t = (fold.anim_t + step).min(target);
        } else if fold.anim_t > target {
            fold.anim_t = (fold.anim_t - step).max(target);
        }
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
            if *ownership == Ownership::Foreign {
                group.has_foreign = true;
            }
            if *ownership == Ownership::Own && *class == BodyClass::Ship {
                group.has_own_ship = true;
            }
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
