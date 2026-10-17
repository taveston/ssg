//! Headless tests of entering a system from the sector view and going
//! back (SECT-30 to SECT-33, SECT-44, CAM-25): a small sector with most of
//! the system view's plugins running, so every marker, label, ring, tray,
//! inset, planner and path built for a system is built here too, and
//! must go with it.

use bevy::diagnostic::DiagnosticsStore;
use bevy::ecs::observer::Observer;
use bevy::ecs::resource::IsResource;
use bevy::input::mouse::AccumulatedMouseScroll;
use bevy::picking::{InteractionPlugin, PickingPlugin, PickingSystems};

use super::camera::{follow_cursor_with_pointer, press_key};
use super::sector::{
    SEED, central_node, pointer_click_at, rig, screens, sector, sector_app_with, selected,
};
use super::*;
use crate::content::generator::system::generate_system;
use crate::content::start::LoadedSector;
use crate::sim::load::{OfLoadedSystem, SystemLoadApp};
use crate::sim::system::SystemData;
use crate::view::bindings::Bindings;
use crate::view::camera::state::Bookmarks;
use crate::view::camera::stops::ZoomStops;
use crate::view::double_click::MapClicks;
use crate::view::edge::parent_tab::ParentTabPlugin;
use crate::view::frame_cue::FrameCuePlugin;
use crate::view::hover::HoverPlugin;
use crate::view::inset::fidelity::FidelityImages;
use crate::view::inset::inspect::InspectPlugin;
use crate::view::inset::outline::OutlinePlugin;
use crate::view::map_layer::{MapLayer, SectorSelection, WayBack};
use crate::view::markers::brackets::BracketsPlugin;
use crate::view::markers::labels::LabelsPlugin;
use crate::view::multi::{OpenTray, TrayRequest};
use crate::view::paths::PathsPlugin;
use crate::view::planner::PlannerPlugin;
use crate::view::rings::RingsPlugin;
use crate::view::rings::marks::RingMarksPlugin;
use crate::view::rings::picking::RingPickingPlugin;
use crate::view::sector::{EnterRequest, NodeKind};
use crate::view::sidebar::SidebarState;
use crate::view::topbar::{BackButton, TopBarPlugin};
use crate::view::tray::TrayPlugin;

/// A small sector, paused, with the system view's markers, labels, rings,
/// trays, inset, planner, paths, top bar and breadcrumb as well as the
/// sector view, and the system view's shared state reset with a system as
/// `ViewPlugin` has it.
fn entering_app() -> App {
    entering_app_with(|_| {})
}

fn entering_app_with(extra: impl FnOnce(&mut App)) -> App {
    sector_app_with(|app| {
        app.reset_with_system::<Selection>()
            .reset_with_system::<PlannerState>()
            .reset_with_system::<OpenTray>()
            .reset_with_system::<TrayRequest>()
            .reset_with_system::<MultiMarkers>()
            .reset_with_system::<MapClicks>()
            .init_resource::<Assets<bevy::gizmos::GizmoAsset>>()
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<DiagnosticsStore>()
            .init_resource::<AccumulatedMouseScroll>()
            .insert_resource(FidelityImages {
                surveyed: Handle::default(),
                unsurveyed: Handle::default(),
            })
            .add_plugins((
                EdgePlugin,
                GroupingPlugin,
                MotionPlugin,
                MarkersPlugin,
                MultiMarkersPlugin,
                HoveredPlugin,
                LabelsPlugin,
                BracketsPlugin,
                TrueSizePlugin,
                crate::view::belt::BeltPlugin,
            ))
            .add_plugins((
                SharedOrbitPlugin,
                RingsPlugin,
                RingMarksPlugin,
                RingPickingPlugin,
                TrayPlugin,
                InspectPlugin,
                OutlinePlugin,
                PlannerPlugin,
                PathsPlugin,
            ))
            .add_plugins((
                HoverPlugin,
                TopBarPlugin,
                BreadcrumbPlugin,
                ParentTabPlugin,
                FrameCuePlugin,
            ));
        extra(app);
    })
}

/// `entering_app` with Bevy's picking running, the pointer following the
/// window's cursor.
fn picking_entering_app() -> App {
    let mut app = entering_app_with(|app| {
        app.add_plugins((PickingPlugin, InteractionPlugin))
            .add_systems(
                PreUpdate,
                follow_cursor_with_pointer.before(PickingSystems::ProcessInput),
            );
    });
    let window = the_window(&mut app);
    app.world_mut()
        .entity_mut(window)
        .insert(bevy::window::PrimaryWindow);
    app.world_mut()
        .spawn(bevy::picking::pointer::PointerId::Mouse);
    run(&mut app, 2);
    app
}

fn layer(app: &App) -> MapLayer {
    *app.world().resource::<MapLayer>()
}

/// Two gated systems of the sector: the one nearest the middle of the
/// map, and another.
fn two_gated(app: &App) -> (usize, usize) {
    let first = central_node(app, NodeKind::Gated);
    let other = sector(app)
        .gated
        .iter()
        .map(|g| g.id)
        .find(|&id| id != first)
        .expect("a second gated system");
    (first, other)
}

/// The records the generator makes for the gated system `id`.
fn generated(app: &App, id: usize) -> SystemData {
    let loaded = app.world().resource::<LoadedSector>();
    generate_system(&loaded.sector, id, &loaded.tunables).unwrap()
}

/// Asks to enter `id`, as the inset's button does, and lets the view
/// settle.
fn enter(app: &mut App, id: usize) {
    app.world_mut().resource_mut::<EnterRequest>().0 = Some(id);
    run(app, SETTLE_FRAMES);
}

/// Asks to go back, as the Back button does, and lets the view settle.
fn go_back(app: &mut App) {
    app.world_mut().resource_mut::<WayBack>().asked = true;
    run(app, SETTLE_FRAMES);
}

/// Every body shown, by name, with its class and orbit radius, sorted.
fn bodies(app: &mut App) -> Vec<(String, BodyClass, Option<u64>)> {
    let mut query = app
        .world_mut()
        .query::<(&DisplayName, &BodyClass, Option<&Orbit>)>();
    let mut found: Vec<_> = query
        .iter(app.world())
        .map(|(name, class, orbit)| (name.0.clone(), *class, orbit.map(|o| o.radius_km as u64)))
        .collect();
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found
}

fn count<F: bevy::ecs::query::QueryFilter>(app: &mut App) -> usize {
    let mut query = app.world_mut().query_filtered::<(), F>();
    query.iter(app.world()).count()
}

/// The world's own entities, leaving out Bevy's resources and observers.
fn own_entities(app: &mut App) -> usize {
    count::<(Without<IsResource>, Without<Observer>)>(app)
}

/// Nothing of a system is left: no body, named orbit or belt, nothing the
/// load spawned, and no marker, label, ring, tab or row for a body.
fn assert_no_system_left(app: &mut App) {
    use crate::sim::body::Belt;
    use crate::view::edge::EdgeTabFor;
    use crate::view::rings::RingStrokeFor;
    assert_eq!(count::<With<BodyClass>>(app), 0, "bodies left");
    assert_eq!(count::<With<NamedOrbit>>(app), 0, "named orbits left");
    assert_eq!(count::<With<Belt>>(app), 0, "belts left");
    assert_eq!(
        count::<With<OfLoadedSystem>>(app),
        0,
        "loaded entities left"
    );
    assert_eq!(count::<With<MarkerFor>>(app), 0, "markers left");
    assert_eq!(count::<With<EdgeTabFor>>(app), 0, "edge tabs left");
    assert_eq!(count::<With<RingStrokeFor>>(app), 0, "ring strokes left");
}

fn back_button_shown(app: &mut App) -> bool {
    let mut buttons = app.world_mut().query_filtered::<&Node, With<BackButton>>();
    buttons.single(app.world()).unwrap().display != Display::None
}

fn crumb_texts(app: &mut App) -> Vec<String> {
    breadcrumb_entries(app)
        .into_iter()
        .map(|(_, t)| t)
        .collect()
}

/// SECT-31 / CAM-25: entering a gated system shows it in the system view:
/// its bodies, at the system fit in the standard orientation, in the
/// star's frame with nothing selected and the sidebar closed, with "Sector
/// / *name*" and a Back button in the top bar. Scrolling out at the fit
/// never leaves it (SECT-30, ZOOM-14).
#[test]
fn entering_a_gated_system_shows_it_at_the_system_fit_in_the_star_frame() {
    let mut app = entering_app();
    let (id, _) = two_gated(&app);
    let data = generated(&app, id);
    app.world_mut().resource_mut::<SectorSelection>().0 = Some(id);
    app.world_mut().resource_mut::<CameraRig>().heading = 0.7;
    run(&mut app, SETTLE_FRAMES);
    assert!(!back_button_shown(&mut app));
    enter(&mut app, id);

    assert_eq!(layer(&app), MapLayer::System);
    let star = body(&mut app, &data.star_name);
    assert_eq!(app.world().get::<BodyClass>(star), Some(&BodyClass::Star));
    for planet in &data.planets {
        body(&mut app, &planet.name);
    }
    let frame = frame(&app);
    assert!(frame.in_star_frame() && frame.star == star);
    assert_eq!(app.world().resource::<Selection>().selected, None);
    assert_eq!(app.world().resource::<SidebarState>().slide, 0.0);
    let tunables = Tunables::default();
    let rig = rig(&app);
    assert_eq!(rig.heading, 0.0);
    assert_eq!(rig.tilt, tunables.standard_tilt_deg.to_radians());
    let radius = app.world().resource::<ZoomStops>().system_radius;
    let fit = fit_distance(radius, fov(), rig.tilt, tunables.zoom_fit_margin);
    assert!(
        (rig.distance - fit).abs() < fit * 1e-4,
        "{} != {fit}",
        rig.distance
    );
    assert_eq!(crumb_texts(&mut app), ["Sector", data.star_name.as_str()]);
    assert!(back_button_shown(&mut app));

    scroll(&mut app, -3.0);
    assert_eq!(
        layer(&app),
        MapLayer::System,
        "scrolling never changes layer"
    );
}

/// SECT-32: Back, the breadcrumb's Sector entry, and a request each go
/// back to the sector view with its camera, sidebar and selection as they
/// were left, and drop the system: nothing of it is left, and the system
/// view's selection, frame, bookmarks and tray are reset.
#[test]
fn going_back_restores_the_sector_view_and_drops_the_system() {
    let mut app = entering_app();
    let (id, _) = two_gated(&app);
    let data = generated(&app, id);
    for way in ["request", "back button", "breadcrumb"] {
        app.world_mut().resource_mut::<SectorSelection>().0 = Some(id);
        app.world_mut().resource_mut::<CameraRig>().heading = 0.4;
        run(&mut app, SETTLE_FRAMES);
        let left = (rig(&app), *app.world().resource::<SidebarState>());
        enter(&mut app, id);

        // Use the system: select a planet, frame it, save a bookmark.
        let planet = body(&mut app, &data.planets[0].name);
        app.world_mut().resource_mut::<Selection>().select(planet);
        app.world_mut().resource_mut::<FrameRequest>().0 = Some(planet);
        run(&mut app, move_frames());
        let saved = Some(crate::view::camera::state::Bookmark {
            rig: rig(&app),
            frame: frame(&app),
        });
        app.world_mut().resource_mut::<Bookmarks>().0[0] = saved;

        match way {
            "request" => go_back(&mut app),
            "back button" => {
                let mut buttons = app.world_mut().query_filtered::<Entity, With<BackButton>>();
                let button = buttons.single(app.world()).unwrap();
                click_node(&mut app, button, 1);
                run(&mut app, SETTLE_FRAMES);
            }
            _ => {
                let (sector_crumb, text) = breadcrumb_entries(&mut app)[0].clone();
                assert_eq!(text, "Sector");
                click_node(&mut app, sector_crumb, 1);
                run(&mut app, SETTLE_FRAMES);
            }
        }

        assert_eq!(layer(&app), MapLayer::Sector, "{way}");
        assert_eq!(rig(&app), left.0, "{way}");
        assert_eq!(*app.world().resource::<SidebarState>(), left.1, "{way}");
        assert_eq!(selected(&app), Some(id), "{way}");
        assert_no_system_left(&mut app);
        assert_eq!(app.world().resource::<Selection>().selected, None);
        assert!(frame(&app).in_star_frame());
        assert!(
            app.world()
                .resource::<Bookmarks>()
                .0
                .iter()
                .all(Option::is_none)
        );
        assert!(app.world().get_resource::<WayBack>().is_none());
        assert!(!back_button_shown(&mut app));
        assert_eq!(crumb_texts(&mut app), ["Sector"]);
    }
}

/// SECT-32 / SYSG-02: entering a system, going back and entering it again
/// gives the same system, and a round trip leaves the world as the last
/// one did: nothing builds up.
#[test]
fn entering_the_same_system_again_gives_the_same_system() {
    let mut app = entering_app();
    let (id, _) = two_gated(&app);
    let mut seen = vec![];
    let mut left_behind = vec![];
    for _ in 0..3 {
        enter(&mut app, id);
        seen.push(bodies(&mut app));
        go_back(&mut app);
        left_behind.push(own_entities(&mut app));
    }
    assert!(!seen[0].is_empty());
    assert_eq!(seen[0], seen[1]);
    assert_eq!(seen[1], seen[2]);
    assert_eq!(left_behind[1], left_behind[2], "entities build up");
}

/// SECT-31 / SECT-32: entering one system, going back and entering
/// another shows only the second's bodies, with the time running, a body
/// selected and hovered, a tray asked for and the planner's state reset
/// between; then going back leaves nothing.
#[test]
fn entering_two_systems_in_turn_leaves_nothing_of_the_first() {
    let mut app = entering_app();
    let (first, second) = two_gated(&app);
    app.world_mut().resource_mut::<GameClock>().toggle_pause();
    app.world_mut().resource_mut::<GameClock>().run_at(5.0);
    for id in [first, second, first] {
        let data = generated(&app, id);
        enter(&mut app, id);
        let star = body(&mut app, &data.star_name);
        let names: std::collections::HashSet<String> = bodies(&mut app)
            .into_iter()
            .map(|(name, ..)| name)
            .collect();
        assert!(names.contains(&data.star_name));
        for planet in &data.planets {
            assert!(names.contains(&planet.name));
        }
        assert_eq!(frame(&app).star, star);
        // Hover and select the first planet, through its marker.
        let planet = body(&mut app, &data.planets[0].name);
        let marker = marker_of(&mut app, planet);
        app.world_mut().entity_mut(marker).insert(Hovered);
        app.world_mut().resource_mut::<Selection>().select(planet);
        app.world_mut().resource_mut::<TrayRequest>().0 = Some(
            crate::view::multi::TrayFor::Marker(crate::view::multi::MultiKey::Fold(star)),
        );
        run(&mut app, SETTLE_FRAMES);
        go_back(&mut app);
        assert_no_system_left(&mut app);
        assert_eq!(app.world().resource::<OpenTray>().0, None);
    }
}

/// SECT-31: the clock and its date carry on across entering and leaving:
/// the rate is kept and the days only ever go forward.
#[test]
fn the_clock_carries_on_across_entering_and_leaving() {
    let mut app = entering_app();
    let (id, _) = two_gated(&app);
    app.world_mut().resource_mut::<GameClock>().toggle_pause();
    app.world_mut().resource_mut::<GameClock>().run_at(5.0);
    run(&mut app, 10);
    let mut last = day(&app);
    for step in 0..4 {
        if step % 2 == 0 {
            enter(&mut app, id);
        } else {
            go_back(&mut app);
        }
        let clock = app.world().resource::<GameClock>();
        assert!(!clock.is_paused());
        assert_eq!(clock.rate_days_per_sec(), 5.0);
        let now = day(&app);
        let expected = 5.0 * SETTLE_FRAMES as f64 * FRAME.as_secs_f64();
        assert!((now - last - expected).abs() < 0.05, "{now} after {last}");
        last = now;
    }
}

/// SECT-30: a double-click on a gated system's node enters it; on any
/// other node it only selects it.
#[test]
fn a_double_click_enters_a_gated_node_and_only_selects_another() {
    let mut app = picking_entering_app();
    let ungated = central_node(&app, NodeKind::Ungated);
    let at = screens(&app)[&ungated].centre;
    pointer_click_at(&mut app, at);
    pointer_click_at(&mut app, at);
    run(&mut app, 2);
    assert_eq!(layer(&app), MapLayer::Sector);
    assert_eq!(selected(&app), Some(ungated));
    assert_eq!(app.world().resource::<EnterRequest>().0, None);

    // Clear the selection, so the first click opens the inset and the
    // map slides under the pointer before the second.
    app.world_mut().resource_mut::<SectorSelection>().0 = None;
    run(&mut app, SETTLE_FRAMES);
    let gated = central_node(&app, NodeKind::Gated);
    let data = generated(&app, gated);
    let at = screens(&app)[&gated].centre;
    pointer_click_at(&mut app, at);
    pointer_click_at(&mut app, at);
    run(&mut app, SETTLE_FRAMES);
    assert_eq!(layer(&app), MapLayer::System);
    body(&mut app, &data.star_name);
    assert_eq!(
        selected(&app),
        Some(gated),
        "the sector keeps its selection"
    );
}

/// SECT-30 / CAM-18: Enter, through the bindings, enters the selected
/// node's system if it is a gated one, and does nothing otherwise.
#[test]
fn the_enter_key_enters_the_selected_gated_system() {
    let mut app = entering_app();
    let ungated = central_node(&app, NodeKind::Ungated);
    let gated = central_node(&app, NodeKind::Gated);
    app.world_mut().resource_mut::<SectorSelection>().0 = Some(ungated);
    run(&mut app, 2);
    press_key(&mut app, KeyCode::Enter);
    run(&mut app, 2);
    assert_eq!(layer(&app), MapLayer::Sector);

    app.world_mut().resource_mut::<SectorSelection>().0 = Some(gated);
    app.world_mut().resource_mut::<Bindings>().enter_system = vec![KeyCode::KeyX];
    press_key(&mut app, KeyCode::Enter);
    run(&mut app, 2);
    assert_eq!(layer(&app), MapLayer::Sector, "Enter is unbound");
    press_key(&mut app, KeyCode::KeyX);
    run(&mut app, 2);
    assert_eq!(layer(&app), MapLayer::System);
    let star = generated(&app, gated).star_name;
    body(&mut app, &star);
}

/// SECT-33 / CAM-25: in a system the game started with there is no
/// sector: no Back button, and the breadcrumb's Sector entry does
/// nothing.
#[test]
fn a_system_started_without_a_sector_has_no_way_back() {
    let mut app = camera_app_with(|app| {
        app.insert_resource(placeholder_fonts())
            .init_resource::<DiagnosticsStore>()
            .add_plugins((
                crate::view::map_layer::MapLayerPlugin,
                TopBarPlugin,
                BreadcrumbPlugin,
            ));
    });
    run(&mut app, 2);
    assert!(!back_button_shown(&mut app));
    let entries = breadcrumb_entries(&mut app);
    assert_eq!(entries[0].1, "Sector");
    let before = frame(&app);
    click_node(&mut app, entries[0].0, 1);
    run(&mut app, SETTLE_FRAMES);
    assert_eq!(layer(&app), MapLayer::System);
    assert_eq!(frame(&app), before);
    assert!(app.world().get_resource::<WayBack>().is_none());
    assert!(!back_button_shown(&mut app));
    body(&mut app, "Corvane");
}

/// SECT-44: a screenshot with `enter` enters the named gated system
/// before anything else, so `select` names one of its bodies; a name that
/// is no gated system's fails the shot.
#[test]
fn the_debug_shot_enters_a_system_by_name() {
    use crate::view::debug_shot::{DebugShotPlugin, Shot, ShotPhase, ShotPlan};
    let probe = entering_app();
    let gated = central_node(&probe, NodeKind::Gated);
    let ungated = central_node(&probe, NodeKind::Ungated);
    let data = generated(&probe, gated);
    let designation = sector(&probe).entries[ungated].designation.clone();
    let planet = data.planets[0].name.clone();

    let shot = |spec: &str| {
        let plan = ShotPlan::parse(spec).unwrap();
        entering_app_with(|app| {
            app.insert_resource(Shot::new(plan))
                .add_plugins(DebugShotPlugin);
        })
    };
    let spec = format!(
        "out=unused.png;sector=small;seed={SEED};enter={};select={planet};wait=0.1",
        data.star_name
    );
    let mut app = shot(&spec);
    let mut frames = 0;
    while app.world().resource::<Shot>().phase != ShotPhase::Capturing {
        run(&mut app, 1);
        frames += 1;
        assert!(frames < 600, "no capture after {frames} frames");
    }
    assert_eq!(layer(&app), MapLayer::System);
    let selected = app.world().resource::<Selection>().selected.unwrap();
    assert_eq!(app.world().get::<DisplayName>(selected).unwrap().0, planet);

    for name in ["Nowhere", designation.as_str()] {
        let mut app = shot(&format!(
            "out=unused.png;sector=small;seed={SEED};enter={name}"
        ));
        run(&mut app, 120);
        assert_eq!(
            app.world().resource::<Shot>().phase,
            ShotPhase::Failed,
            "{name}"
        );
        assert_eq!(layer(&app), MapLayer::Sector);
    }
}

/// SECT-32: a body hovered, clicked and framed through Bevy's picking, as
/// the mouse does it, is let go of with its system: going back with the
/// pointer still over where its marker was, and entering again, leaves
/// nothing of it and asks nothing of a despawned entity.
#[test]
fn a_body_picked_with_the_mouse_is_let_go_of_with_its_system() {
    use super::camera::move_cursor;
    use crate::view::markers::picking::{MarkerPickingPlugin, MarkerTarget};
    let mut app = entering_app_with(|app| {
        app.add_plugins((PickingPlugin, InteractionPlugin, MarkerPickingPlugin))
            .add_systems(
                PreUpdate,
                follow_cursor_with_pointer.before(PickingSystems::ProcessInput),
            );
    });
    let window = the_window(&mut app);
    app.world_mut()
        .entity_mut(window)
        .insert(bevy::window::PrimaryWindow);
    app.world_mut()
        .spawn(bevy::picking::pointer::PointerId::Mouse);
    run(&mut app, 2);
    let gated = central_node(&app, NodeKind::Gated);
    let data = generated(&app, gated);
    for _visit in 0..2 {
        enter(&mut app, gated);
        let planet = body(&mut app, &data.planets[0].name);
        let mut targets = app
            .world_mut()
            .query_filtered::<(Entity, &MarkerFor), With<MarkerTarget>>();
        let target = targets
            .iter(app.world())
            .find(|(_, marker_for)| marker_for.0 == planet)
            .map(|(target, _)| target)
            .expect("a hit target for the planet");
        let at = app
            .world()
            .resource::<ScreenPositions>()
            .get(planet)
            .expect("the planet on screen");
        move_cursor(&mut app, at);
        run(&mut app, 3);
        assert!(hovered(&app).contains(&target), "the marker is hovered");
        pointer_click_at(&mut app, at);
        assert_eq!(app.world().resource::<Selection>().selected, Some(planet));
        app.world_mut().resource_mut::<FrameRequest>().0 = Some(planet);
        run(&mut app, move_frames());
        assert_eq!(frame(&app).body, planet);

        go_back(&mut app);
        assert_eq!(layer(&app), MapLayer::Sector);
        move_cursor(&mut app, at + Vec2::splat(5.0));
        run(&mut app, 3);
        assert_no_system_left(&mut app);
        assert!(frame(&app).in_star_frame());
    }
}
