//! Hover and click on the map (HOV-01, HOV-05, INS-01, GRP-07): the
//! pointer handlers every pickable node (marker, pill, ring hit target,
//! ring mark) is observed by; `brackets.rs` draws the hover mark. Nothing
//! has a tooltip: a marker's name and type are on the map already, and a
//! fold pill's satellites are in the inset's outline once its parent is
//! selected. A shared orbit's ring or pill selects the orbit's parent
//! with that orbit's row revealed in the inset.
//!
//! A click selects (`selection.rs`); a double-click also makes the body the
//! frame and moves to it (ZOOM-16), or on the frame body is the
//! planetary-view hook (ZOOM-17), through `camera/requests.rs`.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::bindings::Bindings;
use super::camera::requests::{FrameRequest, PlanetaryViewRequest};
use super::frame::Frame;
use super::gesture::{PressStart, was_click};
use super::planner_state::{PickTarget, PlannerState};
use super::selection::Selection;
use super::targets::{GroupPillFor, Hovered, MarkerFor, OrbitFor, PointerTargets};
use super::tunables::Tunables;
use crate::Stage;
use crate::sim::named_orbit::NamedOrbit;

pub struct HoverPlugin;

impl Plugin for HoverPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, clear_on_empty_click.in_set(Stage::Overlay));
    }
}

/// INS-03: a click on empty map space - a primary press and release within
/// `drag_threshold_px` that hits no marker, pill, ring or panel - clears
/// the selection and closes the inset. While the transfer planner is open
/// it does nothing (PLNR-03).
#[allow(clippy::too_many_arguments)]
fn clear_on_empty_click(
    buttons: Res<ButtonInput<MouseButton>>,
    bindings: Res<Bindings>,
    windows: Query<&Window>,
    press: Res<PressStart>,
    tunables: Res<Tunables>,
    targets: PointerTargets,
    planner: Res<PlannerState>,
    mut selection: ResMut<Selection>,
) {
    if !buttons.just_released(bindings.select_button) || planner.is_open() {
        return;
    }
    let Some(release) = windows.single().ok().and_then(Window::cursor_position) else {
        return;
    };
    if was_click(&press, release, tunables.drag_threshold_px) && targets.over_nothing() {
        selection.clear();
    }
}

/// HOV-01 / GRP-07: hovering a marker, label, ring or pill marks it, so
/// it takes the hover brackets (HOV-06).
pub fn on_marker_over(over: On<PointerOver>, mut commands: Commands) {
    commands.entity(over.entity).insert(Hovered);
}

pub fn on_marker_out(out: On<PointerOut>, mut commands: Commands) {
    if let Ok(mut node) = commands.get_entity(out.entity) {
        node.remove::<Hovered>();
    }
}

/// What a click handler needs to tell a selecting click from the release
/// of a drag or a reserved button.
#[derive(SystemParam)]
pub struct ClickCheck<'w, 's> {
    press: Res<'w, PressStart>,
    tunables: Res<'w, Tunables>,
    windows: Query<'w, 's, &'static Window>,
}

impl ClickCheck<'_, '_> {
    /// A click that selects: the primary button, and not the release of a
    /// drag (CAM-10). Right-clicks are reserved for context menus (CAM-13).
    fn selects(&self, click: &On<PointerClick>) -> bool {
        let release = self.windows.single().ok().and_then(Window::cursor_position);
        click.button == PointerButton::Primary
            && release.is_none_or(|at| was_click(&self.press, at, self.tunables.drag_threshold_px))
    }
}

/// What a click on a body does, from its marker or its pill alike. A
/// click selects (INS-01), or, while the transfer planner is open, picks
/// a destination instead (PLNR-03); a double-click also makes it the
/// frame, or, on the frame body other than the star, is the
/// planetary-view trigger (ZOOM-16, ZOOM-17). The frame, not selection,
/// decides.
#[derive(SystemParam)]
pub struct BodyClick<'w> {
    frame: Res<'w, Frame>,
    selection: ResMut<'w, Selection>,
    planner: ResMut<'w, PlannerState>,
    frame_request: ResMut<'w, FrameRequest>,
    planetary: ResMut<'w, PlanetaryViewRequest>,
}

impl BodyClick<'_> {
    fn click(&mut self, body: Entity, count: u8) {
        if !self.planner.pick(PickTarget::Body(body)) {
            self.selection.select(body);
        }
        if count >= 2 {
            double_click(
                &self.frame,
                body,
                &mut self.frame_request,
                &mut self.planetary,
            );
        }
    }

    /// HOV-05: a click on a shared orbit selects its parent and opens the
    /// inset with that orbit's row expanded; a double-click is a
    /// double-click on the parent (ZOOM-16).
    fn click_orbit(&mut self, orbit: Entity, parent: Entity, count: u8) {
        if !self.planner.pick(PickTarget::Orbit(orbit)) {
            self.selection.select_through_orbit(parent, orbit);
        }
        if count >= 2 {
            double_click(
                &self.frame,
                parent,
                &mut self.frame_request,
                &mut self.planetary,
            );
        }
    }
}

/// ZOOM-16 / ZOOM-17: a double-click on the frame body, unless it is the
/// star, asks for the planetary view; on anything else (the star
/// included), for a frame jump to it.
pub fn double_click(
    frame: &Frame,
    body: Entity,
    frame_request: &mut FrameRequest,
    planetary: &mut PlanetaryViewRequest,
) {
    if frame.non_star_body() == Some(body) {
        planetary.0 = Some(body);
    } else {
        frame_request.0 = Some(body);
    }
}

pub fn on_marker_click(
    click: On<PointerClick>,
    markers: Query<&MarkerFor>,
    check: ClickCheck,
    mut body_click: BodyClick,
) {
    if !check.selects(&click) {
        return;
    }
    if let Ok(marker_for) = markers.get(click.entity) {
        body_click.click(marker_for.0, click.count);
    }
}

/// HOV-05: clicking a shared orbit's ring or identity pill selects the
/// orbit's parent with the orbit's row revealed.
pub fn on_orbit_click(
    click: On<PointerClick>,
    orbits: Query<&OrbitFor>,
    named: Query<&NamedOrbit>,
    check: ClickCheck,
    mut body_click: BodyClick,
) {
    if !check.selects(&click) {
        return;
    }
    if let Ok(orbit_for) = orbits.get(click.entity)
        && let Ok(orbit) = named.get(orbit_for.0)
    {
        body_click.click_orbit(orbit_for.0, orbit.parent, click.count);
    }
}

/// Clicking a "+N" pill does exactly what clicking its parent does
/// (GRP-07).
pub fn on_pill_click(
    click: On<PointerClick>,
    pills: Query<&GroupPillFor>,
    check: ClickCheck,
    mut body_click: BodyClick,
) {
    if !check.selects(&click) {
        return;
    }
    if let Ok(pill) = pills.get(click.entity) {
        body_click.click(pill.0, click.count);
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::camera::NormalizedRenderTarget;
    use bevy::picking::backend::HitData;
    use bevy::picking::pointer::{Location, PointerId};

    use super::*;

    /// A world with the click handlers' resources, a body, and a marker
    /// and a pill that both stand for it.
    fn click_world() -> (World, Entity, Entity, Entity) {
        let mut world = World::new();
        world.init_resource::<Frame>();
        world.init_resource::<Selection>();
        world.init_resource::<PlannerState>();
        world.init_resource::<FrameRequest>();
        world.init_resource::<PlanetaryViewRequest>();
        world.init_resource::<PressStart>();
        world.init_resource::<Tunables>();
        let body = world.spawn_empty().id();
        let marker = world.spawn(MarkerFor(body)).observe(on_marker_click).id();
        let pill = world.spawn(GroupPillFor(body)).observe(on_pill_click).id();
        (world, body, marker, pill)
    }

    fn mouse() -> bevy::picking::events::Pointer {
        bevy::picking::events::Pointer::new(
            PointerId::Mouse,
            Location {
                target: NormalizedRenderTarget::None {
                    width: 1600,
                    height: 900,
                },
                position: Vec2::ZERO,
            },
        )
    }

    fn click(world: &mut World, entity: Entity, count: u8) {
        let camera = world.spawn_empty().id();
        world.trigger(PointerClick {
            entity,
            pointer: mouse(),
            button: PointerButton::Primary,
            hit: HitData::new(camera, 0.0, None, None),
            duration: Duration::from_millis(80),
            count,
        });
    }

    /// What a run of clicks left behind: selection, frame request and
    /// planetary-view request.
    fn outcome(world: &World) -> (Option<Entity>, Option<Entity>, Option<Entity>) {
        (
            world.resource::<Selection>().selected,
            world.resource::<FrameRequest>().0,
            world.resource::<PlanetaryViewRequest>().0,
        )
    }

    /// GRP-07: clicking a pill does exactly what clicking its parent's
    /// marker does - select on one click, ask for a frame jump on a
    /// double-click, and the planetary view on a double-click of the frame
    /// body.
    /// Covers ZOOM-16, ZOOM-17.
    #[test]
    fn clicking_a_pill_is_clicking_its_parent() {
        for (count, framed) in [(1, false), (2, false), (2, true)] {
            let mut results = Vec::new();
            for on_pill in [false, true] {
                let (mut world, body, marker, pill) = click_world();
                if framed {
                    let frame = *world.resource::<Frame>();
                    *world.resource_mut::<Frame>() = frame.on(body, Vec3::ZERO);
                }
                click(&mut world, if on_pill { pill } else { marker }, count);
                results.push((outcome(&world), body));
            }
            assert_eq!(results[0], results[1], "{count} clicks, framed {framed}");
            let ((selected, frame_request, planetary), body) = results[0];
            assert_eq!(selected, Some(body));
            assert_eq!(frame_request.is_some(), count == 2 && !framed);
            assert_eq!(planetary.is_some(), count == 2 && framed);
        }
    }

    /// HOV-05: a click on a shared orbit's ring selects the orbit's parent
    /// with the orbit revealed; a double-click asks for the parent's
    /// frame, as a double-click on the parent would.
    #[test]
    fn clicking_a_shared_orbit_selects_its_parent_and_reveals_it() {
        let (mut world, parent, ..) = click_world();
        let orbit = world
            .spawn(NamedOrbit {
                parent,
                name: "High".to_string(),
                radius_km: 1.0,
                period_days: 1.0,
            })
            .id();
        let target = world.spawn(OrbitFor(orbit)).observe(on_orbit_click).id();
        click(&mut world, target, 1);
        assert_eq!(outcome(&world), (Some(parent), None, None));
        assert_eq!(world.resource::<Selection>().revealed_orbit, Some(orbit));
        click(&mut world, target, 2);
        assert_eq!(outcome(&world), (Some(parent), Some(parent), None));
    }

    /// PLNR-03 / INS-01: while the planner is open, a click on a marker
    /// or a shared ring picks it as the destination and leaves the
    /// selection as it was.
    #[test]
    fn an_open_planner_picks_what_is_clicked_instead_of_selecting_it() {
        let (mut world, body, marker, _) = click_world();
        let ship = world.spawn_empty().id();
        world.resource_mut::<Selection>().select(ship);
        world.resource_mut::<PlannerState>().open(ship);
        click(&mut world, marker, 1);
        assert_eq!(world.resource::<Selection>().selected, Some(ship));
        assert_eq!(
            world.resource::<PlannerState>().clicked,
            Some(PickTarget::Body(body))
        );
        let orbit = world
            .spawn(NamedOrbit {
                parent: body,
                name: "High".to_string(),
                radius_km: 1.0,
                period_days: 1.0,
            })
            .id();
        let ring = world.spawn(OrbitFor(orbit)).observe(on_orbit_click).id();
        click(&mut world, ring, 1);
        assert_eq!(world.resource::<Selection>().selected, Some(ship));
        assert_eq!(
            world.resource::<PlannerState>().clicked,
            Some(PickTarget::Orbit(orbit))
        );
    }

    /// ZOOM-16 / ZOOM-17: a double-click on the star asks for a frame jump
    /// to it even in the star's frame; the star has no planetary view.
    #[test]
    fn double_clicking_the_star_asks_for_its_frame() {
        let mut world = World::new();
        let star = world.spawn_empty().id();
        let frame = Frame::star_frame(star, Vec3::X);
        let mut request = FrameRequest::default();
        let mut planetary = PlanetaryViewRequest::default();
        double_click(&frame, star, &mut request, &mut planetary);
        assert_eq!(request.0, Some(star));
        assert_eq!(planetary.0, None);
    }

    /// HOV-01: hovering a marker marks it for the hover brackets and
    /// leaving it clears it.
    #[test]
    fn hovering_a_marker_rings_it_without_a_tooltip() {
        let mut world = World::new();
        let body = world.spawn_empty().id();
        let marker = world
            .spawn(MarkerFor(body))
            .observe(on_marker_over)
            .observe(on_marker_out)
            .id();
        let camera = world.spawn_empty().id();
        let hit = HitData::new(camera, 0.0, None, None);

        world.trigger(PointerOver {
            entity: marker,
            pointer: mouse(),
            hit: hit.clone(),
        });
        world.flush();
        assert!(world.get::<Hovered>(marker).is_some());

        world.trigger(PointerOut {
            entity: marker,
            pointer: mouse(),
            hit,
        });
        world.flush();
        assert!(world.get::<Hovered>(marker).is_none());
    }
}
