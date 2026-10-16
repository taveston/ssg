//! The frame body's parent as an edge tab (MRK-15). Outside the star's
//! frame, while the parent is off the map area, a tab pinned to the edge in
//! the parent's direction says where it is: a chevron pointing that way,
//! the parent's body marker at list size, so it never reads as a gate, and
//! its name.
//!
//! It is placed and styled as a gate's edge tab is (`edge_tab.rs`). The
//! marker inside is rebuilt whenever the parent changes.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::edge::{edge_arrow_angle, edge_point};
use super::edge_tab::{TabChevron, place_tab, spawn_tab, tab_label};
use super::fonts::Fonts;
use super::frame::Frame;
use super::markers::draw::{BodyLook, spawn_list_icon};
use super::screen::{MapArea, ScreenPositions};
use super::tunables::Tunables;
use super::world::ViewNow;
use crate::sim::body::{BodyClass, DisplayName};
use crate::sim::track::Affiliation;
use crate::{Boot, Stage};

/// The tab itself.
#[derive(Component)]
pub struct ParentTab;
/// The holder of the parent's marker.
#[derive(Component)]
struct ParentTabIcon;
/// The parent's name.
#[derive(Component)]
struct ParentTabLabel;

pub struct ParentTabPlugin;

impl Plugin for ParentTabPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_parent_tab.in_set(Boot::Overlay))
            .add_systems(
                Update,
                (fill_parent_tab, place_parent_tab)
                    .chain()
                    .in_set(Stage::Overlay),
            );
    }
}

/// MRK-15 / EDGE-03: an edge tab with the chevron beside it and the
/// marker and the name inside it.
fn spawn_parent_tab(mut commands: Commands, fonts: Res<Fonts>, tunables: Res<Tunables>) {
    spawn_tab(
        &mut commands,
        &fonts,
        tunables.button_stroke_px,
        |content| {
            content.spawn((ParentTabIcon, Pickable::IGNORE, Node::default()));
            content.spawn((ParentTabLabel, tab_label(&fonts, "")));
        },
    )
    .insert(ParentTab);
}

/// The frame body's parent: the body it orbits, or the star. `None` in the
/// star's frame.
fn frame_parent(frame: &Frame, parents: &Query<&ChildOf>) -> Option<Entity> {
    let body = frame.non_star_body()?;
    Some(parents.get(body).map_or(frame.star, ChildOf::parent))
}

/// The tab's parts that are filled for each parent: the marker's holder
/// and the name.
#[derive(SystemParam)]
struct ParentTabParts<'w, 's> {
    icons: Query<'w, 's, Entity, With<ParentTabIcon>>,
    labels: Query<'w, 's, &'static mut Text, With<ParentTabLabel>>,
}

/// MRK-15: puts the parent's marker and name in the tab when the parent
/// changes.
fn fill_parent_tab(
    mut commands: Commands,
    mut built_for: Local<Option<Entity>>,
    frame: Res<Frame>,
    (fonts, tunables): (Res<Fonts>, Res<Tunables>),
    parents: Query<&ChildOf>,
    bodies: Query<(&BodyClass, Option<&Affiliation>, &DisplayName)>,
    mut parts: ParentTabParts,
) {
    let ParentTabParts { icons, labels } = &mut parts;
    let parent = frame_parent(&frame, &parents);
    if *built_for == parent {
        return;
    }
    *built_for = parent;
    let Some((class, affiliation, name)) = parent.and_then(|p| bodies.get(p).ok()) else {
        return;
    };
    for mut text in labels.iter_mut() {
        text.0 = name.0.clone();
    }
    let look = BodyLook {
        class: *class,
        affiliation: affiliation.copied().unwrap_or(Affiliation::Neutral),
        record: None,
        group: None,
    };
    for icon in icons.iter() {
        commands
            .entity(icon)
            .despawn_related::<Children>()
            .with_children(|holder| {
                spawn_list_icon(
                    holder,
                    (&fonts, &tunables),
                    look,
                    tunables.list_symbol_px / 2.0,
                );
            });
    }
}

/// MRK-15: where the tab's chevron sits, in window pixels: nowhere while
/// the parent is inside the map area, else pinned to its edge in the
/// parent's direction as a gate's arrow is (EDGE-01).
pub fn parent_tab_point(
    on_screen: Option<Vec2>,
    world: Option<Vec3>,
    camera: Option<&GlobalTransform>,
    area: &MapArea,
) -> Option<Vec2> {
    let inside = on_screen.is_some_and(|pos| {
        let local = pos - area.offset;
        local.cmpge(Vec2::ZERO).all() && local.cmple(area.size).all()
    });
    if inside {
        return None;
    }
    edge_point(on_screen, world, camera, area)
}

/// MRK-15: shows the tab while the parent is off the map area, on the
/// edge in its direction, with the chevron turned towards it.
fn place_parent_tab(
    view: ViewNow,
    frame: Res<Frame>,
    screen: Res<ScreenPositions>,
    parents: Query<&ChildOf>,
    mut tabs: Query<(&mut Node, &ComputedNode, &Children), With<ParentTab>>,
    mut chevrons: Query<&mut UiTransform, With<TabChevron>>,
) {
    let area = MapArea::of(screen.viewport);
    let camera = view.camera().map(|(_, transform)| transform);
    let at = frame_parent(&frame, &parents).and_then(|parent| {
        let world = if parent == frame.star {
            Some(view.star())
        } else {
            view.position(parent)
        };
        parent_tab_point(screen.get(parent), world, camera.as_ref(), &area)
    });
    for (mut node, computed, children) in &mut tabs {
        let Some(at) = at else {
            node.display = Display::None;
            continue;
        };
        place_tab(&mut node, computed, at, &area);
        let angle = edge_arrow_angle(at - area.offset, area.size);
        for &child in children {
            if let Ok(mut transform) = chevrons.get_mut(child) {
                transform.rotation = Rot2::radians(angle);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area() -> MapArea {
        MapArea::of(Vec2::new(1600.0, 900.0))
    }

    /// MRK-15: while the parent is in the map area there is no tab.
    #[test]
    fn a_parent_in_view_has_no_tab() {
        let area = area();
        let inside = area.offset + Vec2::new(5.0, 5.0);
        assert_eq!(parent_tab_point(Some(inside), None, None, &area), None);
    }

    /// MRK-15: a parent off the map area is pinned to the edge in its
    /// direction, as a gate's arrow is.
    #[test]
    fn a_parent_off_the_map_is_pinned_to_the_edge() {
        let area = area();
        let centre = area.offset + area.size / 2.0;
        let far_right = centre + Vec2::new(5000.0, 0.0);
        let at = parent_tab_point(Some(far_right), None, None, &area).unwrap();
        assert!((at.y - centre.y).abs() < 0.01);
        assert!(at.x > centre.x && at.x < area.offset.x + area.size.x);
    }

    /// MRK-15: a parent behind the camera is pinned by its direction.
    #[test]
    fn a_parent_behind_the_camera_is_pinned_by_direction() {
        let area = area();
        let camera = GlobalTransform::from(Transform::IDENTITY);
        // The camera looks down -Z; the parent is behind it, to the left.
        let parent = Vec3::new(-10.0, 0.0, 5.0);
        let at = parent_tab_point(None, Some(parent), Some(&camera), &area).unwrap();
        assert!(at.x < area.offset.x + area.size.x / 2.0);
    }
}
