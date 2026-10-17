//! Which bodies and shared orbits the pointer is over this frame, through
//! any of their pickable stand-ins - marker hit area, label, orbit ring or
//! identity mark (HOV-01, HOV-03, HOV-05). Shared state, so the hover
//! brackets and the orbit rings' highlight agree without importing each
//! other (ARCH-03).

use std::collections::HashSet;

use bevy::prelude::*;

use crate::sim::load::SystemLoadApp;

use super::planner::state::PlannerState;
use super::targets::{Hovered, MarkerFor, OrbitFor};
use crate::Stage;

/// The bodies and named orbits hovered this frame.
#[derive(Resource, Default, Debug)]
pub struct HoveredBodies(pub HashSet<Entity>);

impl HoveredBodies {
    /// Whether a body, or a shared orbit, is hovered.
    pub fn contains(&self, entity: Entity) -> bool {
        self.0.contains(&entity)
    }
}

pub struct HoveredPlugin;

impl Plugin for HoveredPlugin {
    fn build(&self, app: &mut App) {
        app.reset_with_system::<HoveredBodies>()
            .add_systems(Update, collect_hovered.in_set(Stage::Screen));
    }
}

/// HOV-03 / HOV-05: hovering any stand-in for a body - its marker, its
/// label or its ring - hovers the body; hovering a shared orbit's ring or
/// identity mark hovers the orbit. While the transfer planner is open,
/// what it can't pick takes no hover mark (PLNR-11).
fn collect_hovered(
    targets: Query<&MarkerFor, With<Hovered>>,
    orbits: Query<&OrbitFor, With<Hovered>>,
    planner: Res<PlannerState>,
    mut hovered: ResMut<HoveredBodies>,
) {
    let now: HashSet<Entity> = targets
        .iter()
        .map(|target| target.0)
        .chain(orbits.iter().map(|orbit| orbit.0))
        .filter(|entity| !planner.unpickable.contains(entity))
        .collect();
    if hovered.0 != now {
        hovered.0 = now;
    }
}
