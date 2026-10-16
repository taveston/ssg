//! Headless tests of a multi-marker's tray (MUL-07 to MUL-10, MUL-12,
//! GRP-07, GRP-09, HOV-05, INS-03, CAM-10, CAM-14, MRK-12, PLNR-11): what
//! opens it, what it lists and says, its keys, where it sits, and what
//! closes it.

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input::mouse::MouseButtonInput;

use super::*;
use crate::view::multi::{OpenTray, TrayFor};
use crate::view::planner::state::PickTarget;
use crate::view::tray::TrayPlugin;
use crate::view::tray::contents::TrayContents;
use crate::view::tray::draw::{TrayHeaderText, TrayRoot, TrayRowFor};

/// Corvane with markers, multi-markers, ring marks and the tray.
fn tray_app_with(extra: impl FnOnce(&mut App)) -> App {
    use crate::view::rings::marks::RingMarksPlugin;
    symbols_app_with(|app| {
        app.add_plugins((SharedOrbitPlugin, RingMarksPlugin, TrayPlugin));
        extra(app);
    })
}

fn tray_app() -> App {
    tray_app_with(|_| {})
}

fn open_tray(app: &App) -> Option<TrayFor> {
    app.world().resource::<OpenTray>().0
}

fn contents(app: &App) -> TrayContents {
    app.world().resource::<TrayContents>().clone()
}

fn selected(app: &App) -> Option<Entity> {
    app.world().resource::<Selection>().selected
}

fn the_window(app: &mut App) -> Entity {
    let mut windows = app.world_mut().query_filtered::<Entity, With<Window>>();
    windows.single(app.world()).unwrap()
}

fn press_key(app: &mut App, key: KeyCode) {
    let window = the_window(app);
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(KeyboardInput {
            key_code: key,
            logical_key: Key::Unidentified(NativeKey::Unidentified),
            state,
            text: None,
            repeat: false,
            window,
        });
        run(app, 1);
    }
    run(app, 1);
}

fn move_cursor(app: &mut App, to: Vec2) {
    let window = the_window(app);
    app.world_mut()
        .get_mut::<Window>(window)
        .unwrap()
        .set_cursor_position(Some(to));
}

fn mouse(app: &mut App, state: ButtonState) {
    let window = the_window(app);
    app.world_mut().write_message(MouseButtonInput {
        button: MouseButton::Left,
        state,
        window,
    });
    run(app, 1);
}

/// Click the fold marker of `parent` and let the tray open.
fn click_fold_marker(app: &mut App, parent: Entity) {
    let (_, target) = multi_node(app, MultiKey::Fold(parent)).expect("a fold marker");
    click_node(app, target, 1);
    run(app, 2);
}

/// The drawn row for `entity`.
fn row_node(app: &mut App, entity: Entity) -> Entity {
    let mut rows = app.world_mut().query::<(Entity, &TrayRowFor)>();
    rows.iter(app.world())
        .find(|(_, row)| row.entity == entity)
        .map(|(e, _)| e)
        .expect("a row")
}

fn tray_root(app: &mut App) -> Entity {
    let mut roots = app.world_mut().query_filtered::<Entity, With<TrayRoot>>();
    roots.single(app.world()).unwrap()
}

/// MUL-07 / MUL-08 / MUL-09 / GRP-07 / GRP-09: a click on Halden IV's fold
/// marker opens its tray and selects Halden IV. The header counts what is
/// folded; the station comes first, tagged "station"; TF-1 is one row
/// tagged with its member count by class, closed; no row is selected, as
/// the parent isn't one. Opening Iver's tray closes Halden IV's.
#[test]
fn clicking_a_fold_marker_opens_its_tray_and_selects_the_parent() {
    let mut app = tray_app();
    run(&mut app, SETTLE_FRAMES);
    let halden = body(&mut app, "Halden IV");
    let highport = body(&mut app, "Arden Highport");
    let tf1 = body(&mut app, "TF-1");
    let iver = body(&mut app, "Iver");
    assert_eq!(open_tray(&app), None);

    click_fold_marker(&mut app, halden);
    assert_eq!(
        open_tray(&app),
        Some(TrayFor::Marker(MultiKey::Fold(halden)))
    );
    assert_eq!(selected(&app), Some(halden));
    let fold = multi_marker(&app, MultiKey::Fold(halden)).unwrap();
    let tray = contents(&app);
    assert_eq!(
        tray.header,
        format!("HALDEN IV · {} IN ORBIT", fold.count())
    );
    assert_eq!(tray.lines.len(), fold.count());
    assert_eq!(tray.lines[0].row.entity, highport);
    assert_eq!(tray.lines[0].tag, "station");
    let tf1_line = tray.lines.iter().find(|l| l.row.entity == tf1).unwrap();
    assert_eq!(tf1_line.tag, "×4 · 4 warships");
    assert_eq!(tf1_line.row.open, Some(false));
    assert!(tray.lines.iter().all(|l| !l.selected));
    let headers: Vec<String> = {
        let mut texts = app.world_mut().query::<&TrayHeaderText>();
        texts.iter(app.world()).map(|t| t.0.clone()).collect()
    };
    assert_eq!(
        headers,
        std::slice::from_ref(&tray.header),
        "the header is drawn"
    );
    row_node(&mut app, tf1);

    click_fold_marker(&mut app, iver);
    assert_eq!(open_tray(&app), Some(TrayFor::Marker(MultiKey::Fold(iver))));
    assert_eq!(selected(&app), Some(iver));
}

/// MRK-12 / CAM-10 / HOV-02: the tray is drawn above the markers, labels
/// and brackets and beneath the edge arrows, and is a panel the map's
/// drags don't start on: a drag that starts on it doesn't pan.
#[test]
fn a_press_on_the_tray_never_pans() {
    use crate::view::camera::state::CameraRig;
    use crate::view::layers;
    use crate::view::targets::MapInputBlocker;
    use bevy::picking::backend::HitData;
    use bevy::picking::hover::HoverMap;
    use bevy::picking::pointer::PointerId;

    let mut app = tray_app();
    run(&mut app, SETTLE_FRAMES);
    let root = tray_root(&mut app);
    let z = app.world().get::<GlobalZIndex>(root).unwrap().0;
    assert!(layers::LABELS < z && z < layers::EDGE);
    assert!(app.world().get::<MapInputBlocker>(root).is_some());

    let camera = app.world_mut().spawn_empty().id();
    let hits = [(root, HitData::new(camera, 0.0, None, None))].into();
    let mut map = HoverMap::default();
    map.0.insert(PointerId::Mouse, hits);
    app.world_mut().insert_resource(map);
    let before = *app.world().resource::<CameraRig>();
    move_cursor(&mut app, Vec2::new(500.0, 500.0));
    run(&mut app, 1);
    mouse(&mut app, ButtonState::Pressed);
    for step in 1..=10 {
        move_cursor(&mut app, Vec2::new(500.0 + 30.0 * step as f32, 500.0));
        run(&mut app, 1);
    }
    mouse(&mut app, ButtonState::Released);
    let after = *app.world().resource::<CameraRig>();
    assert_eq!(before.pivot, after.pivot, "no pan");
}

/// MUL-10 / CAM-14 / CAM-18: with the tray open, Down and Up step the
/// selection through the rows and stop at the ends; Right opens TF-1 to
/// list its members one level in, and stepping onto a member keeps TF-1
/// selected; Left from a member closes TF-1 and selects it again. The
/// arrows never pan the map meanwhile.
#[test]
fn the_tray_keys_step_open_and_close_rows_without_panning() {
    let mut app = tray_app();
    run(&mut app, SETTLE_FRAMES);
    let halden = body(&mut app, "Halden IV");
    let highport = body(&mut app, "Arden Highport");
    let tf1 = body(&mut app, "TF-1");
    click_fold_marker(&mut app, halden);
    let offset = frame(&app).offset;

    press_key(&mut app, KeyCode::ArrowDown);
    assert_eq!(selected(&app), Some(highport), "the first row");
    let second = contents(&app).lines[1].row.entity;
    press_key(&mut app, KeyCode::ArrowDown);
    assert_eq!(selected(&app), Some(second));
    press_key(&mut app, KeyCode::ArrowUp);
    press_key(&mut app, KeyCode::ArrowUp);
    assert_eq!(selected(&app), Some(highport), "stops at the top");
    assert!(contents(&app).lines[0].selected);

    // MUL-08: the rows scroll, as if only four fit.
    let row_px = Tunables::default().tray_row_px;
    let list = {
        let mut lists = app
            .world_mut()
            .query_filtered::<Entity, With<crate::view::tray::draw::TrayList>>();
        lists.single(app.world()).unwrap()
    };
    app.world_mut().get_mut::<ComputedNode>(list).unwrap().size = Vec2::new(240.0, 4.0 * row_px);
    for _ in 0..40 {
        if selected(&app) == Some(tf1) {
            break;
        }
        press_key(&mut app, KeyCode::ArrowDown);
    }
    assert_eq!(selected(&app), Some(tf1));
    assert!(contents(&app).hint.contains("→ open group"));
    let at = contents(&app)
        .lines
        .iter()
        .position(|l| l.row.entity == tf1)
        .unwrap();
    let scrolled = app.world().get::<ScrollPosition>(list).unwrap().y;
    assert_eq!(
        scrolled,
        (at as f32 - 3.0) * row_px,
        "the selected row is in view"
    );
    press_key(&mut app, KeyCode::ArrowRight);
    let tray = contents(&app);
    let at = tray.lines.iter().position(|l| l.row.entity == tf1).unwrap();
    let members = app
        .world()
        .get::<crate::sim::group::ShipGroup>(tf1)
        .unwrap()
        .members
        .clone();
    let listed: Vec<Entity> = tray.lines[at + 1..=at + members.len()]
        .iter()
        .map(|l| l.row.entity)
        .collect();
    assert_eq!(listed, members);
    assert!(tray.lines[at + 1].row.depth == 1);

    press_key(&mut app, KeyCode::ArrowDown);
    assert_eq!(selected(&app), Some(tf1), "a member selects its group");
    assert!(contents(&app).lines[at + 1].selected);
    press_key(&mut app, KeyCode::ArrowLeft);
    let tray = contents(&app);
    assert_eq!(tray.lines[at].row.open, Some(false));
    assert!(tray.lines[at].selected);
    assert_eq!(selected(&app), Some(tf1));

    assert_eq!(frame(&app).offset, offset, "the arrows panned");
}

/// INS-03 / MUL-10: Esc closes the tray and keeps the selection; a second
/// Esc clears the selection.
#[test]
fn esc_closes_the_tray_before_the_selection() {
    use crate::view::inset::fidelity::FidelityImages;
    use crate::view::inset::inspect::InspectPlugin;
    let mut app = tray_app_with(|app| {
        app.add_plugins(InspectPlugin)
            .insert_resource(FidelityImages {
                surveyed: Handle::default(),
                unsurveyed: Handle::default(),
            })
            .init_resource::<crate::view::camera::requests::PlanetaryViewRequest>();
    });
    run(&mut app, SETTLE_FRAMES);
    let halden = body(&mut app, "Halden IV");
    click_fold_marker(&mut app, halden);
    press_key(&mut app, KeyCode::Escape);
    assert_eq!(open_tray(&app), None);
    assert_eq!(selected(&app), Some(halden));
    press_key(&mut app, KeyCode::Escape);
    assert_eq!(selected(&app), None);
}

/// MUL-10: a click on a row selects its thing and the tray stays open; the
/// release of a drag leaves it open; a click on the map closes it and
/// keeps the selection.
#[test]
fn a_click_elsewhere_closes_the_tray_and_a_drag_does_not() {
    let mut app = tray_app();
    run(&mut app, SETTLE_FRAMES);
    let halden = body(&mut app, "Halden IV");
    let highport = body(&mut app, "Arden Highport");
    click_fold_marker(&mut app, halden);
    let row = row_node(&mut app, highport);
    click_node(&mut app, row, 1);
    run(&mut app, 2);
    assert_eq!(selected(&app), Some(highport));
    assert!(open_tray(&app).is_some());
    assert!(contents(&app).lines[0].selected);

    move_cursor(&mut app, Vec2::new(300.0, 700.0));
    run(&mut app, 1);
    mouse(&mut app, ButtonState::Pressed);
    move_cursor(&mut app, Vec2::new(400.0, 700.0));
    run(&mut app, 1);
    mouse(&mut app, ButtonState::Released);
    assert!(open_tray(&app).is_some(), "a drag's release isn't a click");

    move_cursor(&mut app, Vec2::new(300.0, 700.0));
    run(&mut app, 1);
    mouse(&mut app, ButtonState::Pressed);
    mouse(&mut app, ButtonState::Released);
    run(&mut app, 1);
    assert_eq!(open_tray(&app), None);
    assert_eq!(selected(&app), Some(highport));
}

/// MUL-08 / MUL-10: the tray sits right of its marker's box by
/// `tray_gap_px` and inside the map area, and follows the marker as the
/// view zooms.
#[test]
fn the_tray_follows_its_marker_inside_the_map_area() {
    use crate::view::screen::MapArea;
    let mut app = tray_app();
    run(&mut app, SETTLE_FRAMES);
    let halden = body(&mut app, "Halden IV");
    click_fold_marker(&mut app, halden);
    let root = tray_root(&mut app);
    let size = Vec2::new(260.0, 300.0);
    app.world_mut().get_mut::<ComputedNode>(root).unwrap().size = size;
    let gap = Tunables::default().tray_gap_px;
    let mut lefts = Vec::new();
    for _ in 0..2 {
        run(&mut app, 2);
        let anchor = contents(&app).anchor.expect("the marker is drawn");
        let node = app.world().get::<Node>(root).unwrap().clone();
        assert_eq!(node.display, Display::Flex);
        assert_eq!(
            app.world().get::<Visibility>(root),
            Some(&Visibility::Inherited)
        );
        let min = node_min(&node).unwrap();
        assert!((min.x - (anchor.max.x + gap)).abs() < 0.01);
        let viewport = app.world().resource::<ScreenPositions>().viewport;
        let area = MapArea::of(viewport).rect();
        assert!(area.contains(min) && area.contains(min + size), "{min}");
        lefts.push(min.x);
        scroll(&mut app, 1.0);
    }
    assert_ne!(lefts[0], lefts[1], "the tray moved with its marker");
}

/// MUL-10: when its marker stops existing, here because framing Iver
/// unfolds Iver Yard, the tray closes and the selection stays.
#[test]
fn the_tray_closes_when_its_marker_goes() {
    let mut app = tray_app();
    run(&mut app, SETTLE_FRAMES);
    let iver = body(&mut app, "Iver");
    click_fold_marker(&mut app, iver);
    assert!(open_tray(&app).is_some());
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(iver);
    run(&mut app, 60 + move_frames());
    assert!(multi_marker(&app, MultiKey::Fold(iver)).is_none());
    assert_eq!(open_tray(&app), None);
    assert_eq!(selected(&app), Some(iver));
}

/// MUL-07 / MUL-09: a station marker's tray lists the station, then what
/// is docked unindented; a click on the marker selects the station.
#[test]
fn a_station_markers_tray_lists_the_station_then_whats_docked() {
    let mut app = tray_app();
    let iver = body(&mut app, "Iver");
    let yard = body(&mut app, "Iver Yard");
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(iver);
    run(&mut app, 60 + move_frames());
    let target = marker_target(&mut app, yard);
    click_node(&mut app, target, 1);
    run(&mut app, 2);
    assert_eq!(
        open_tray(&app),
        Some(TrayFor::Marker(MultiKey::Station(yard)))
    );
    assert_eq!(selected(&app), Some(yard));
    let tray = contents(&app);
    assert_eq!(tray.header, "IVER YARD · 2 DOCKED");
    assert_eq!(tray.lines.len(), 3);
    assert_eq!(tray.lines[0].row.entity, yard);
    assert!(tray.lines.iter().all(|l| l.row.depth == 0));
    assert!(tray.lines[0].selected);
}

/// The label of a body's marker, which takes clicks as the marker does.
fn marker_target(app: &mut App, body: Entity) -> Entity {
    use crate::view::markers::MarkerLabel;
    let mut targets = app
        .world_mut()
        .query_filtered::<(Entity, &MarkerFor), With<MarkerLabel>>();
    targets
        .iter(app.world())
        .find(|(_, m)| m.0 == body)
        .map(|(e, _)| e)
        .expect("a marker label")
}

/// MUL-12 / PLNR-11: while the planner is open, a click on a fold marker
/// picks its parent and opens the tray; a click on a station marker
/// picks nothing and opens its tray; a row picks its thing, and a row
/// that can't be picked (a group, and its members) is dimmed and does
/// nothing.
#[test]
fn with_the_planner_open_rows_pick_as_their_symbols_would() {
    let mut app = tray_app();
    run(&mut app, SETTLE_FRAMES);
    let halden = body(&mut app, "Halden IV");
    let highport = body(&mut app, "Arden Highport");
    let tf1 = body(&mut app, "TF-1");
    let orison = body(&mut app, "Orison");
    app.world_mut().resource_mut::<Selection>().select(orison);
    {
        let mut planner = app.world_mut().resource_mut::<PlannerState>();
        planner.open(orison);
        // What the planner marks as unpickable: a group (PLNR-11).
        planner.unpickable.insert(tf1);
    }
    let clicked = |app: &mut App| {
        app.world_mut()
            .resource_mut::<PlannerState>()
            .clicked
            .take()
    };

    click_fold_marker(&mut app, halden);
    assert_eq!(clicked(&mut app), Some(PickTarget::Body(halden)));
    assert!(open_tray(&app).is_some());
    let tray = contents(&app);
    let dimmed = |e: Entity| {
        tray.lines
            .iter()
            .find(|l| l.row.entity == e)
            .unwrap()
            .dimmed
    };
    assert!(dimmed(tf1) && !dimmed(highport));

    let row = row_node(&mut app, highport);
    click_node(&mut app, row, 1);
    assert_eq!(clicked(&mut app), Some(PickTarget::Body(highport)));
    let row = row_node(&mut app, tf1);
    click_node(&mut app, row, 1);
    assert_eq!(clicked(&mut app), None);
    assert_eq!(selected(&app), Some(orison));

    let iver = body(&mut app, "Iver");
    let yard = body(&mut app, "Iver Yard");
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(iver);
    run(&mut app, 60 + move_frames());
    let target = marker_target(&mut app, yard);
    click_node(&mut app, target, 1);
    run(&mut app, 2);
    assert_eq!(clicked(&mut app), None, "a station marker picks nothing");
    assert_eq!(
        open_tray(&app),
        Some(TrayFor::Marker(MultiKey::Station(yard)))
    );
    let row = row_node(&mut app, yard);
    click_node(&mut app, row, 1);
    assert_eq!(clicked(&mut app), Some(PickTarget::Body(yard)));
}

/// HOV-05 / MUL-07: at 4 d/s in Halden IV's frame the High band is an
/// exposure ring; a click on its identity mark selects Halden IV with the
/// band revealed, and opens the mark's tray, listing the band's things.
#[test]
fn clicking_a_shared_orbits_mark_opens_its_tray() {
    use crate::view::rings::marks::{RingMarkFor, ring_things};
    use crate::view::shared_orbit::OrbitRings;
    use bevy::ecs::system::RunSystemOnce;
    let mut app = tray_app();
    let halden = body(&mut app, "Halden IV");
    let orison = body(&mut app, "Orison");
    let high = app.world().get::<InNamedOrbit>(orison).unwrap().0;
    let mut clock = app.world_mut().resource_mut::<GameClock>();
    clock.toggle_pause();
    clock.run_at(4.0);
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(halden);
    run(&mut app, 60 + eased_frames());
    let mark = {
        let mut marks = app.world_mut().query::<(Entity, &RingMarkFor)>();
        marks
            .iter(app.world())
            .find(|(_, mark)| mark.owner == high)
            .map(|(e, _)| e)
            .unwrap()
    };
    click_node(&mut app, mark, 1);
    run(&mut app, 2);
    assert_eq!(selected(&app), Some(halden));
    assert_eq!(
        app.world().resource::<Selection>().revealed_orbit,
        Some(high)
    );
    assert_eq!(open_tray(&app), Some(TrayFor::Orbit(high)));
    let things = app
        .world_mut()
        .run_system_once(
            move |rings: OrbitRings, sim: Res<crate::sim::tunables::SimTunables>| {
                let together_km = sim.together_km;
                ring_things(&rings.ring(high).unwrap(), together_km).0.len()
            },
        )
        .unwrap();
    let tray = contents(&app);
    assert_eq!(tray.header, format!("{things} OBJECTS"));
    assert_eq!(tray.lines.len(), things);
    assert!(tray.anchor.is_some());
}
