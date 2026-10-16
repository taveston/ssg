//! Hover and click on the map (HOV-01, HOV-05, INS-01): the pointer
//! handlers every pickable node (marker, multi-marker, ring hit target,
//! ring mark) is observed by; `markers/brackets.rs` draws the hover mark.
//! Nothing has a tooltip: a marker's name and type are on the map
//! already. A shared orbit's ring or identity mark selects the orbit's
//! parent with that orbit's row revealed in the inset, and asks for the
//! identity mark's tray. A click on a multi-marker is
//! `multi_markers/click.rs`, through `BodyClick`; a station's marker with
//! things docked is a multi-marker, so a click on it asks for its tray
//! and selects the station without picking it (MUL-07, MUL-12).
//!
//! A click selects (`selection.rs`); a double-click also makes the body the
//! frame and moves to it (ZOOM-16), or on the frame body is the
//! planetary-view hook (ZOOM-17), through `camera/requests.rs`. The second
//! click of a double-click acts on what the first click hit, wherever on
//! the map it lands (`double_click.rs`).

use bevy::ecs::system::SystemParam;
use bevy::input::InputSystems;
use bevy::picking::{PickingSettings, PickingSystems};
use bevy::prelude::*;

use super::bindings::Bindings;
use super::camera::requests::{FrameRequest, PlanetaryViewRequest};
use super::double_click::{MapClicks, double_click_interval, record_press};
use super::frame::Frame;
use super::gesture::{PressStart, was_click};
use super::multi::{MultiKey, MultiMarkers, TrayFor, TrayRequest};
use super::planner::state::{PickTarget, PlannerState};
use super::selection::Selection;
use super::targets::{Hovered, MarkerFor, OrbitFor, PointerTargets};
use super::tunables::Tunables;
use crate::Stage;
use crate::sim::named_orbit::NamedOrbit;

pub struct HoverPlugin;

impl Plugin for HoverPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MapClicks>()
            .add_systems(
                PreUpdate,
                record_press
                    .after(InputSystems)
                    .before(PickingSystems::ProcessInput),
            )
            .add_systems(Update, clear_on_empty_click.in_set(Stage::Overlay));
    }
}

/// INS-03: a click on empty map space - a primary press and release within
/// `drag_threshold_px` that hits no marker, multi-marker, ring, tray or panel - clears
/// the selection and closes the inset. While the transfer planner is open
/// it does nothing (PLNR-03). The second click of a double-click whose
/// target slid away meanwhile lands here too, and completes the
/// double-click instead (ZOOM-16).
fn clear_on_empty_click(
    buttons: Res<ButtonInput<MouseButton>>,
    bindings: Res<Bindings>,
    check: ClickCheck,
    targets: PointerTargets,
    mut body_click: BodyClick,
) {
    if buttons.just_released(bindings.select_button)
        && check.released_as_click()
        && targets.over_nothing()
    {
        body_click.empty_click();
    }
}

/// HOV-01 / MUL-11: hovering a marker, label, ring or multi-marker marks
/// it, so it takes the hover brackets (HOV-06).
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
    /// Whether the cursor is in the window and still within the drag
    /// threshold of where the press started (CAM-10).
    pub fn released_as_click(&self) -> bool {
        let release = self.windows.single().ok().and_then(Window::cursor_position);
        release.is_some_and(|at| was_click(&self.press, at, self.tunables.drag_threshold_px))
    }

    /// A click that selects: the primary button, and not the release of a
    /// drag (CAM-10). Right-clicks are reserved for context menus (CAM-13).
    pub fn selects(&self, click: &On<PointerClick>) -> bool {
        let release = self.windows.single().ok().and_then(Window::cursor_position);
        click.button == PointerButton::Primary
            && release.is_none_or(|at| was_click(&self.press, at, self.tunables.drag_threshold_px))
    }
}

/// What a click on a body does, from its marker or its fold marker alike. A
/// click selects (INS-01), or, while the transfer planner is open, picks
/// a destination instead (PLNR-03); a double-click also makes it the
/// frame, or, on the frame body other than the star, is the
/// planetary-view trigger (ZOOM-16, ZOOM-17). The frame, not selection,
/// decides.
///
/// A click counts as a double-click when Bevy counts it so (a second click
/// on the same node), or when it follows a click on a map target soon
/// enough (`double_click.rs`): then it acts on that first click's target.
#[derive(SystemParam)]
pub struct BodyClick<'w> {
    frame: Res<'w, Frame>,
    selection: ResMut<'w, Selection>,
    planner: ResMut<'w, PlannerState>,
    frame_request: ResMut<'w, FrameRequest>,
    planetary: ResMut<'w, PlanetaryViewRequest>,
    tray: ResMut<'w, TrayRequest>,
    clicks: ResMut<'w, MapClicks>,
    time: Res<'w, Time<Real>>,
    picking: Option<Res<'w, PickingSettings>>,
}

impl BodyClick<'_> {
    /// ZOOM-16: if this click is the second click of a double-click, whose
    /// first click hit this target or another one that has since moved,
    /// complete the double-click on what the first click hit and say so.
    /// This click's own target then does nothing more: the first click
    /// already selected what it hit and opened its tray. A click Bevy
    /// already counts as a double (`count` 2 or more) landed on the same
    /// target, so it takes that target's own double-click path, which
    /// also keeps its tray open (MUL-07).
    pub fn completes_double(&mut self, count: u8) -> bool {
        if count >= 2 {
            return false;
        }
        let interval = double_click_interval(self.picking.as_deref());
        let Some(body) = self.clicks.take_double(interval) else {
            return false;
        };
        double_click(
            &self.frame,
            body,
            &mut self.frame_request,
            &mut self.planetary,
        );
        true
    }

    /// INS-03 / PLNR-03: a click on empty map space clears the selection,
    /// unless the planner is open or the click completes a double-click.
    fn empty_click(&mut self) {
        if self.completes_double(1) {
            return;
        }
        self.clicks.forget();
        if !self.planner.is_open() {
            self.selection.clear();
        }
    }

    /// Note a click on a map target standing for `body`: a single click
    /// may start a double-click on it; a double-click ends one.
    fn note(&mut self, body: Entity, count: u8) {
        if count >= 2 {
            self.clicks.forget();
        } else {
            self.clicks.remember(body, self.time.elapsed());
        }
    }

    /// MUL-07: ask for a multi-marker's tray, in place of any open one.
    pub fn open_tray(&mut self, tray: TrayFor) {
        self.tray.0 = Some(tray);
    }

    pub fn click(&mut self, body: Entity, count: u8) {
        if !self.planner.pick(PickTarget::Body(body)) {
            self.selection.select(body);
        }
        self.note(body, count);
        if count >= 2 {
            double_click(
                &self.frame,
                body,
                &mut self.frame_request,
                &mut self.planetary,
            );
        }
    }

    /// MUL-07: select `body` and, on a double-click, focus it (ZOOM-16),
    /// without picking it for the planner: a click on a multi-marker other
    /// than a fold marker picks nothing (MUL-12).
    pub fn select_only(&mut self, body: Entity, count: u8) {
        if self.planner.is_open() {
            return;
        }
        self.selection.select(body);
        self.note(body, count);
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
        self.note(parent, count);
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

/// INS-01 / MUL-07 / MUL-12: a click on a body's marker, label or lone
/// ring selects the body, or picks it while the planner is open. A
/// station with things docked is drawn as a station marker (MUL-03), a
/// multi-marker: the click opens its tray and selects the station, its
/// first row, and with the planner open picks nothing.
pub fn on_marker_click(
    click: On<PointerClick>,
    markers: Query<&MarkerFor>,
    multis: Res<MultiMarkers>,
    check: ClickCheck,
    mut body_click: BodyClick,
) {
    if !check.selects(&click) || body_click.completes_double(click.count) {
        return;
    }
    let Ok(marker_for) = markers.get(click.entity) else {
        return;
    };
    let body = marker_for.0;
    let station = MultiKey::Station(body);
    if multis.get(station).is_some() {
        body_click.open_tray(TrayFor::Marker(station));
        body_click.select_only(body, click.count);
    } else {
        body_click.click(body, click.count);
    }
}

/// HOV-05: clicking a shared orbit's ring or identity mark selects the
/// orbit's parent with the orbit's row revealed, and opens the identity
/// mark's tray (MUL-07). While the planner is open it picks the orbit's
/// band, as the ring does (PLNR-11).
pub fn on_orbit_click(
    click: On<PointerClick>,
    orbits: Query<&OrbitFor>,
    named: Query<&NamedOrbit>,
    check: ClickCheck,
    mut body_click: BodyClick,
) {
    if !check.selects(&click) || body_click.completes_double(click.count) {
        return;
    }
    if let Ok(orbit_for) = orbits.get(click.entity)
        && let Ok(orbit) = named.get(orbit_for.0)
    {
        body_click.click_orbit(orbit_for.0, orbit.parent, click.count);
        body_click.open_tray(TrayFor::Orbit(orbit_for.0));
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
    /// that stands for it.
    fn click_world() -> (World, Entity, Entity) {
        let mut world = World::new();
        world.init_resource::<Frame>();
        world.init_resource::<Selection>();
        world.init_resource::<PlannerState>();
        world.init_resource::<FrameRequest>();
        world.init_resource::<PlanetaryViewRequest>();
        world.init_resource::<TrayRequest>();
        world.init_resource::<MultiMarkers>();
        world.init_resource::<PressStart>();
        world.init_resource::<Tunables>();
        world.init_resource::<MapClicks>();
        world.init_resource::<Time<Real>>();
        let body = world.spawn_empty().id();
        let marker = world.spawn(MarkerFor(body)).observe(on_marker_click).id();
        (world, body, marker)
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

    /// INS-01 / ZOOM-16 / ZOOM-17: clicking a marker selects its body on
    /// one click, asks for a frame jump on a double-click, and for the
    /// planetary view on a double-click of the frame body.
    #[test]
    fn clicking_a_marker_selects_and_a_double_click_focuses() {
        for (count, framed) in [(1, false), (2, false), (2, true)] {
            let (mut world, body, marker) = click_world();
            if framed {
                let frame = *world.resource::<Frame>();
                *world.resource_mut::<Frame>() = frame.on(body, Vec3::ZERO);
            }
            click(&mut world, marker, count);
            let (selected, frame_request, planetary) = outcome(&world);
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

    /// MUL-07 / HOV-05: a double-click on the same shared ring, soon after
    /// its first click, asks for the parent's frame and asks for the tray
    /// again, so the tray the first click opened stays open.
    #[test]
    fn a_double_click_on_one_target_keeps_its_tray() {
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
        world.resource_mut::<TrayRequest>().0 = None;
        world
            .resource_mut::<MapClicks>()
            .pressed(Duration::from_millis(100));
        click(&mut world, target, 2);
        assert_eq!(outcome(&world), (Some(parent), Some(parent), None));
        assert_eq!(
            world.resource::<TrayRequest>().0,
            Some(TrayFor::Orbit(orbit))
        );
    }

    /// PLNR-03 / INS-01: while the planner is open, a click on a marker
    /// or a shared ring picks it as the destination and leaves the
    /// selection as it was.
    #[test]
    fn an_open_planner_picks_what_is_clicked_instead_of_selecting_it() {
        let (mut world, body, marker) = click_world();
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
