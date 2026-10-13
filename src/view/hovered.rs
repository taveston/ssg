//! Which bodies the pointer is over this frame, through any of their
//! pickable stand-ins - marker hit area, label or orbit ring (HOV-01,
//! HOV-03). Shared state, so the markers' white ring and the orbit rings'
//! highlight agree without importing each other (ARCH-03).

use std::collections::HashSet;

use bevy::prelude::*;

use super::targets::{Hovered, MarkerFor};
use crate::Stage;

/// The bodies hovered this frame.
#[derive(Resource, Default, Debug)]
pub struct HoveredBodies(pub HashSet<Entity>);

impl HoveredBodies {
    pub fn contains(&self, body: Entity) -> bool {
        self.0.contains(&body)
    }
}

pub struct HoveredPlugin;

impl Plugin for HoveredPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HoveredBodies>()
            .add_systems(Update, collect_hovered.in_set(Stage::Screen));
    }
}

/// HOV-03: hovering any stand-in for a body - its marker, its label or its
/// ring - hovers the body.
fn collect_hovered(targets: Query<&MarkerFor, With<Hovered>>, mut hovered: ResMut<HoveredBodies>) {
    let now: HashSet<Entity> = targets.iter().map(|target| target.0).collect();
    if hovered.0 != now {
        hovered.0 = now;
    }
}
