//! Links from a pickable UI node back to the body it stands for. Shared by
//! everything that draws a clickable node for a body (markers, pills, ring
//! hit targets) and by the pointer handlers in `hover.rs`, so neither side
//! has to import the other (ARCH-03).

use bevy::ecs::system::SystemParam;
use bevy::picking::hover::HoverMap;
use bevy::picking::pointer::PointerId;
use bevy::prelude::*;

/// A marker (or exposure-ring hit target) -> the body it represents.
/// Hovering and clicking it target that body (HOV-01, INS-01, MOT-05).
#[derive(Component)]
pub struct MarkerFor(pub Entity);

/// On a node while the pointer is over it (HOV-01), so the marker overlay
/// can draw the white ring for hover and selection alike (MRK-01).
#[derive(Component)]
pub struct Hovered;

/// A UI panel the map's drags don't start on: pressing on the top bar or
/// an inset and dragging doesn't pan or rotate the camera (CAM-10).
#[derive(Component)]
pub struct MapInputBlocker;

/// What the mouse pointer is over, as far as the map's own input cares:
/// a panel (top bar, inset), or anything pickable at all.
#[derive(SystemParam)]
pub struct PointerTargets<'w, 's> {
    hover: Option<Res<'w, HoverMap>>,
    parents: Query<'w, 's, &'static ChildOf>,
    blockers: Query<'w, 's, (), With<MapInputBlocker>>,
}

impl PointerTargets<'_, '_> {
    /// Everything the mouse is over this frame.
    fn hits(&self) -> impl Iterator<Item = Entity> + '_ {
        self.hover
            .as_ref()
            .and_then(|map| map.0.get(&PointerId::Mouse))
            .into_iter()
            .flat_map(|hits| hits.keys().copied())
    }

    /// The panel under the mouse, if any: the `MapInputBlocker` that is, or
    /// holds, something the pointer is over (CAM-10, INS-07).
    pub fn panel(&self) -> Option<Entity> {
        self.hits().find_map(|hit| {
            std::iter::successors(Some(hit), |&node| {
                self.parents.get(node).ok().map(ChildOf::parent)
            })
            .find(|&node| self.blockers.contains(node))
        })
    }

    /// Whether the mouse is over nothing pickable: empty map space (INS-03).
    pub fn over_nothing(&self) -> bool {
        self.hits().next().is_none()
    }
}

/// A "+N" count pill -> the parent whose folded satellites it counts.
/// Clicking it does what clicking the parent does (GRP-07).
#[derive(Component)]
pub struct GroupPillFor(pub Entity);

/// A stand-in for a shared orbit (HOV-05): its ring hit target or its
/// identity pill (MOT-14) -> the named orbit. Hovering it highlights the
/// ring and the pill; clicking it selects the orbit's parent and reveals
/// the orbit's row in the inset.
#[derive(Component)]
pub struct OrbitFor(pub Entity);
