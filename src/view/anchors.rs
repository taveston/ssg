//! Where each body is shown on screen this frame (INS-06, CAM-22): its own
//! marker, or - while it is folded into its parent - its parent's "+N"
//! pill, sliding from one to the other as it unfolds, so nothing that
//! points at a body (an inset's leader line, the focus brackets) jumps.
//!
//! Shared state, read by the inset and the focus marks (ARCH-03); computed
//! in `Stage::Layout`, after screen positions and fold states.

use std::collections::HashMap;

use bevy::prelude::*;

use super::fold::SatelliteFoldState;
use super::pills::pill_anchor;
use super::screen::ScreenPositions;
use crate::Stage;
use crate::sim::body::BodyClass;

/// Each body's on-screen anchor this frame. A body behind the camera has
/// no entry.
#[derive(Resource, Default, Debug)]
pub struct BodyAnchors(pub HashMap<Entity, Vec2>);

impl BodyAnchors {
    pub fn get(&self, body: Entity) -> Option<Vec2> {
        self.0.get(&body).copied()
    }
}

/// INS-06: where a body is shown. Fully folded (`unfold` 0) it is at its
/// parent's pill; fully unfolded (1) at its own marker; in between, part
/// way, so the hand-over has no jump. Without a pill to go to, it is at its
/// own position.
pub fn shown_at(own: Option<Vec2>, pill: Option<Vec2>, unfold: f32) -> Option<Vec2> {
    match (own, pill) {
        (Some(own), Some(pill)) => Some(pill.lerp(own, unfold.clamp(0.0, 1.0))),
        (own, None) => own,
        (None, Some(pill)) => (unfold < 1.0).then_some(pill),
    }
}

pub struct AnchorsPlugin;

impl Plugin for AnchorsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BodyAnchors>()
            .add_systems(Update, update_anchors.in_set(Stage::Layout));
    }
}

/// A body, what it orbits, and its fold state if it is a satellite.
type BodyFold = (
    Entity,
    Option<&'static ChildOf>,
    Option<&'static SatelliteFoldState>,
);

fn update_anchors(
    screen: Res<ScreenPositions>,
    bodies: Query<BodyFold, With<BodyClass>>,
    mut anchors: ResMut<BodyAnchors>,
) {
    anchors.0.clear();
    for (body, child_of, fold) in &bodies {
        let own = screen.get(body);
        let pill = child_of
            .and_then(|child_of| screen.get(child_of.parent()))
            .map(pill_anchor);
        let unfold = fold.map_or(1.0, |fold| fold.anim_t);
        let pill = pill.filter(|_| fold.is_some());
        if let Some(at) = shown_at(own, pill, unfold) {
            anchors.0.insert(body, at);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// INS-06: folded, a satellite is shown at its parent's pill; unfolded,
    /// at its own marker; part way, part way - no jump at either end.
    #[test]
    fn folded_satellite_is_shown_at_the_pill() {
        let own = Some(Vec2::new(100.0, 0.0));
        let pill = Some(Vec2::new(0.0, 0.0));
        assert_eq!(shown_at(own, pill, 0.0), pill);
        assert_eq!(shown_at(own, pill, 1.0), own);
        assert_eq!(shown_at(own, pill, 0.25), Some(Vec2::new(25.0, 0.0)));
    }

    /// A body with no parent pill is just at its own position.
    /// Covers INS-06.
    #[test]
    fn unparented_bodies_are_at_their_marker() {
        let own = Some(Vec2::new(3.0, 4.0));
        assert_eq!(shown_at(own, None, 0.0), own);
        assert_eq!(shown_at(None, None, 1.0), None);
        assert_eq!(shown_at(None, Some(Vec2::ONE), 0.0), Some(Vec2::ONE));
    }
}
