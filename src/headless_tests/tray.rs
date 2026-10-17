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

/// MUL-07 / MUL-09: a click on the open tray's marker again selects the
/// parent, and the highlight leaves the row the keys had moved to; a
/// tray opened in place of another starts at the top of its rows.
#[test]
fn clicking_the_marker_again_moves_the_highlight_and_a_new_tray_starts_at_the_top() {
    use crate::view::tray::draw::TrayList;
    let mut app = tray_app();
    run(&mut app, SETTLE_FRAMES);
    let halden = body(&mut app, "Halden IV");
    let iver = body(&mut app, "Iver");
    click_fold_marker(&mut app, halden);
    press_key(&mut app, KeyCode::ArrowDown);
    press_key(&mut app, KeyCode::ArrowDown);
    assert!(contents(&app).lines[1].selected);

    click_fold_marker(&mut app, halden);
    assert_eq!(selected(&app), Some(halden));
    assert!(contents(&app).lines.iter().all(|l| !l.selected));

    let list = {
        let mut lists = app.world_mut().query_filtered::<Entity, With<TrayList>>();
        lists.single(app.world()).unwrap()
    };
    app.world_mut().get_mut::<ScrollPosition>(list).unwrap().y = 50.0;
    click_fold_marker(&mut app, iver);
    assert_eq!(app.world().get::<ScrollPosition>(list).unwrap().y, 0.0);
}

/// MRK-12 / CAM-10: the tray is drawn above the markers, labels
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

/// Stands in for Bevy's UI picking backend, which needs a laid-out UI:
/// reports the entity in `UiHit` under the mouse wherever it is, at the
/// UI's order (the camera's plus 0.5), as `bevy_ui` does.
#[derive(Resource, Default)]
struct UiHit(Option<Entity>);

fn pick_ui_stand_in(
    hit: Res<UiHit>,
    cameras: Query<(Entity, &Camera), With<IsDefaultUiCamera>>,
    mut output: MessageWriter<bevy::picking::backend::PointerHits>,
) {
    use bevy::picking::backend::{HitData, PointerHits};
    use bevy::picking::pointer::PointerId;
    let (Some(node), Ok((camera, ui_camera))) = (hit.0, cameras.single()) else {
        return;
    };
    output.write(PointerHits::new(
        PointerId::Mouse,
        vec![(node, HitData::new(camera, 0.0, None, None))],
        ui_camera.order as f32 + 0.5,
    ));
}

/// A press and release of the left button where the cursor is, through
/// Bevy's picking as well as the button input.
fn picked_click(app: &mut App) {
    use bevy::camera::NormalizedRenderTarget;
    use bevy::picking::pointer::{Location, PointerAction, PointerId, PointerInput};
    let window = the_window(app);
    let position = app
        .world()
        .get::<Window>(window)
        .unwrap()
        .cursor_position()
        .unwrap();
    let location = Location {
        target: NormalizedRenderTarget::Window(
            bevy::window::WindowRef::Entity(window)
                .normalize(None)
                .unwrap(),
        ),
        position,
    };
    for (state, action) in [
        (
            ButtonState::Pressed,
            PointerAction::Press(PointerButton::Primary),
        ),
        (
            ButtonState::Released,
            PointerAction::Release(PointerButton::Primary),
        ),
    ] {
        app.world_mut().write_message(PointerInput::new(
            PointerId::Mouse,
            location.clone(),
            action,
        ));
        mouse(app, state);
    }
    run(app, 2);
}

/// HOV-02 / MUL-10: with an orbit ring under the tray, a click there hits
/// the tray's row, not the ring: the row's thing is selected, the ring's
/// isn't, and the tray stays open. Without the tray over it, the same
/// point hits the ring.
#[test]
fn a_click_on_the_tray_over_a_ring_hits_the_row() {
    use crate::view::rings::picking::{RingPaths, RingPickingPlugin, distance_to_segments};
    use bevy::picking::{InteractionPlugin, PickingPlugin, PickingSystems};

    let mut app = tray_app_with(|app| {
        app.add_plugins((PickingPlugin, InteractionPlugin, RingPickingPlugin))
            .init_resource::<UiHit>()
            .add_systems(
                PreUpdate,
                (
                    super::camera::follow_cursor_with_pointer.before(PickingSystems::ProcessInput),
                    pick_ui_stand_in.in_set(PickingSystems::Backend),
                ),
            );
    });
    let window = the_window(&mut app);
    app.world_mut()
        .entity_mut(window)
        .insert(bevy::window::PrimaryWindow);
    app.world_mut()
        .spawn(bevy::picking::pointer::PointerId::Mouse);
    run(&mut app, SETTLE_FRAMES);
    let halden = body(&mut app, "Halden IV");
    let highport = body(&mut app, "Arden Highport");
    click_fold_marker(&mut app, halden);
    assert!(open_tray(&app).is_some());
    // The click selected Halden IV, so the inset is sliding open and the
    // map area narrowing, carrying the rings with it (INS-07). Let the
    // slide finish so the point found below stays on its ring.
    run(&mut app, SETTLE_FRAMES);

    // A point on a drawn ring, clear of every other ring, that stands for
    // neither Halden IV, the selection, nor the row's station, so a click
    // that reached the ring would show.
    let tolerance = Tunables::default().ring_hit_px;
    let (ring, point) = {
        let paths = &app.world().resource::<RingPaths>().0;
        let clear_of_others = |ring: Entity, point: Vec2| {
            paths.iter().all(|(&other, segments)| {
                other == ring || distance_to_segments(point, segments) > 3.0 * tolerance
            })
        };
        paths
            .iter()
            .filter(|(ring, _)| {
                let body = app.world().get::<MarkerFor>(**ring).map(|m| m.0);
                body.is_some_and(|body| body != halden && body != highport)
            })
            .flat_map(|(&ring, segments)| {
                segments.iter().map(move |&(a, b)| (ring, a.lerp(b, 0.5)))
            })
            .find(|&(ring, point)| clear_of_others(ring, point))
            .expect("a drawn ring clear of the others")
    };
    move_cursor(&mut app, point);
    run(&mut app, 2);
    assert!(hovered(&app).contains(&ring), "the ring is under the point");

    // The tray's row now lies over the ring.
    let row = row_node(&mut app, highport);
    app.world_mut().resource_mut::<UiHit>().0 = Some(row);
    run(&mut app, 2);
    assert_eq!(hovered(&app), [row], "the row wins over the ring");
    picked_click(&mut app);
    assert_eq!(selected(&app), Some(highport));
    assert!(open_tray(&app).is_some());
}

/// MUL-09 / MUL-10 / CAM-10: a click on a row's toggle opens its members,
/// but the release of a drag over it doesn't.
#[test]
fn a_drag_released_on_a_rows_toggle_leaves_it_alone() {
    use crate::view::gesture::PressStart;
    use crate::view::tray::TrayState;
    let mut app = tray_app();
    run(&mut app, SETTLE_FRAMES);
    let halden = body(&mut app, "Halden IV");
    let tf1 = body(&mut app, "TF-1");
    click_fold_marker(&mut app, halden);
    let row = row_node(&mut app, tf1);
    let toggle = app.world().get::<Children>(row).unwrap()[0];
    let open = |app: &App| app.world().resource::<TrayState>().open_rows.contains(&tf1);

    let press = Vec2::new(500.0, 500.0);
    app.world_mut().resource_mut::<PressStart>().0 = Some(press);
    move_cursor(&mut app, press + Vec2::new(100.0, 0.0));
    click_node(&mut app, toggle, 1);
    assert!(!open(&app), "a drag's release isn't a click");

    move_cursor(&mut app, press);
    click_node(&mut app, toggle, 1);
    assert!(open(&app));
}

/// MUL-09 / MUL-10: stepping the selection with the keys moves the
/// highlight from row to row without building the rows again.
#[test]
fn stepping_moves_the_highlight_without_rebuilding_the_rows() {
    let mut app = tray_app();
    run(&mut app, SETTLE_FRAMES);
    let halden = body(&mut app, "Halden IV");
    click_fold_marker(&mut app, halden);
    let things: Vec<Entity> = contents(&app).lines.iter().map(|l| l.row.entity).collect();
    let nodes = |app: &mut App| things.iter().map(|&t| row_node(app, t)).collect::<Vec<_>>();
    let highlighted = |app: &mut App| {
        nodes(app)
            .into_iter()
            .map(|row| app.world().get::<BackgroundColor>(row).unwrap().0 != Color::NONE)
            .collect::<Vec<bool>>()
    };
    let before = nodes(&mut app);
    assert!(highlighted(&mut app).iter().all(|&h| !h));

    press_key(&mut app, KeyCode::ArrowDown);
    press_key(&mut app, KeyCode::ArrowDown);
    assert_eq!(nodes(&mut app), before, "the same rows");
    let lit = highlighted(&mut app);
    assert!(lit[1] && lit.iter().filter(|&&h| h).count() == 1, "{lit:?}");
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
    // Past Highport, Halden IV's things share the High orbit, so they
    // follow by name and TF-1 is always the eleventh row.
    assert_eq!(at, 10, "TF-1's row");
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

/// MUL-10: when framing Iver unfolds Iver Yard, its fold marker goes, and
/// the tray follows the yard and what is docked at it to the station
/// marker they now make; the selection stays.
#[test]
fn the_tray_follows_its_things_out_of_a_fold_marker() {
    let mut app = tray_app();
    run(&mut app, SETTLE_FRAMES);
    let iver = body(&mut app, "Iver");
    let yard = body(&mut app, "Iver Yard");
    click_fold_marker(&mut app, iver);
    assert!(open_tray(&app).is_some());
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(iver);
    run(&mut app, 60 + move_frames());
    assert!(multi_marker(&app, MultiKey::Fold(iver)).is_none());
    assert_eq!(
        open_tray(&app),
        Some(TrayFor::Marker(MultiKey::Station(yard)))
    );
    assert_eq!(contents(&app).header, "IVER YARD · 2 DOCKED");
    assert_eq!(selected(&app), Some(iver));
}

/// The TEST-12 convoy: TF-2 and two haulers, together in Halden IV's
/// High band.
fn convoy(app: &mut App) -> [Entity; 3] {
    ["TF-2", "Varga Courser", "Varga Drifter"].map(|n| body(app, n))
}

/// Zoom in on Halden IV until the convoy's marker is drawn and click it,
/// opening its tray; returns its things in row order.
fn open_the_convoys_tray(app: &mut App) -> Vec<Entity> {
    let halden = body(app, "Halden IV");
    let key = MultiKey::Together(convoy(app).into_iter().min().unwrap());
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(halden);
    run(app, 60 + move_frames());
    zoom_in_until(app, 12, |app| {
        multi_marker(app, key).is_some_and(|m| m.drawn)
    });
    let (_, target) = multi_node(app, key).expect("the convoy's marker");
    click_node(app, target, 1);
    run(app, 2);
    assert_eq!(open_tray(app), Some(TrayFor::Marker(key)));
    contents(app).lines.iter().map(|l| l.row.entity).collect()
}

fn phase(app: &App, thing: Entity) -> f64 {
    app.world().get::<Orbit>(thing).unwrap().phase0_rad
}

fn set_phase(app: &mut App, thing: Entity, phase0_rad: f64) {
    app.world_mut().get_mut::<Orbit>(thing).unwrap().phase0_rad = phase0_rad;
}

fn sorted(mut things: Vec<Entity>) -> Vec<Entity> {
    things.sort();
    things
}

/// MUL-10: the convoy's marker is keyed by its lead. When the lead moves
/// half way round the band, the other two are still together under a new
/// key, and the tray stays open on them, listing just them.
#[test]
fn the_tray_stays_on_the_rest_when_a_convoys_lead_leaves() {
    let mut app = tray_app();
    let rows = open_the_convoys_tray(&mut app);
    let lead = *rows.iter().min().unwrap();
    let selection = selected(&app);
    let away = phase(&app, lead) + std::f64::consts::PI;
    set_phase(&mut app, lead, away);
    run(&mut app, 10);
    let rest: Vec<Entity> = rows.into_iter().filter(|&t| t != lead).collect();
    let key = MultiKey::Together(*rest.iter().min().unwrap());
    assert_eq!(open_tray(&app), Some(TrayFor::Marker(key)));
    let listed = contents(&app).lines.iter().map(|l| l.row.entity).collect();
    assert_eq!(sorted(listed), sorted(rest));
    assert_eq!(selected(&app), selection);
}

/// Adds a hauler, Outrider, to Halden IV's "High" band opposite the
/// convoy, before the system is spawned. Spawned last, it comes before
/// every other entity in `Entity` order, so a marker it joins takes its
/// key from it.
fn add_an_outrider_opposite_the_convoy(mut waiting: ResMut<SystemDataResource>) {
    use crate::sim::track::ShipClass;
    let data = waiting.0.as_mut().unwrap();
    let halden = data
        .planets
        .iter_mut()
        .find(|p| p.name == "Halden IV")
        .unwrap();
    let convoy_phase = halden
        .children
        .iter()
        .find(|c| c.name == "TF-2")
        .unwrap()
        .phase0_rad;
    halden.children.push(BodyBlueprint {
        class: BodyClass::Ship,
        affiliation: Affiliation::Own,
        record: Some(ShipRecord::new(ShipClass::Cargo, false)),
        orbit: OrbitPlacement::Named("High"),
        phase0_rad: convoy_phase + std::f64::consts::PI,
        ..blueprint("Outrider", BodyClass::Ship, 0.0)
    });
}

/// MUL-10: a ship that comes before the convoy's lead moves to the
/// convoy's phase and takes over the marker's key; the tray stays open
/// on it, now listing four things.
#[test]
fn the_tray_stays_when_a_thing_joins_ahead_of_the_first() {
    let mut app = tray_app_with(|app| {
        app.add_systems(
            Startup,
            add_an_outrider_opposite_the_convoy.in_set(Boot::World),
        );
    });
    let rows = open_the_convoys_tray(&mut app);
    let lead = *rows.iter().min().unwrap();
    let newcomer = body(&mut app, "Outrider");
    assert!(newcomer < lead, "Outrider comes before the convoy");
    let joined = phase(&app, lead);
    set_phase(&mut app, newcomer, joined);
    run(&mut app, 10);
    assert_eq!(
        open_tray(&app),
        Some(TrayFor::Marker(MultiKey::Together(newcomer)))
    );
    let listed = contents(&app).lines.iter().map(|l| l.row.entity).collect();
    let mut want = rows;
    want.push(newcomer);
    assert_eq!(sorted(listed), sorted(want));
}

/// MUL-10: when every thing in the convoy leaves its orbit, ships in
/// flight join no marker, so none holds any of the tray's things: it
/// closes and the selection stays.
#[test]
fn the_tray_closes_when_everything_parts_and_the_selection_stays() {
    let mut app = tray_app();
    let rows = open_the_convoys_tray(&mut app);
    let selection = selected(&app);
    assert!(selection.is_some_and(|s| rows.contains(&s)));
    for thing in rows {
        app.world_mut()
            .entity_mut(thing)
            .remove::<(Orbit, InNamedOrbit)>()
            .insert(InFlight);
    }
    run(&mut app, 10);
    assert_eq!(open_tray(&app), None);
    assert_eq!(selected(&app), selection);
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
/// band revealed, and opens the mark's tray, listing the band's things
/// under the orbit's name (MUL-08).
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
    assert_eq!(tray.header, format!("HIGH ORBIT · {things} THINGS"));
    assert_eq!(tray.lines.len(), things);
    assert!(tray.anchor.is_some());
}

/// HOV-05 / MUL-07 / MUL-10: paused in Halden IV's frame, the High band
/// is drawn in dot mode, with no identity mark. A click on its ring
/// selects Halden IV with the band revealed, opens no tray, and closes the
/// one that was open.
#[test]
fn clicking_a_shared_ring_in_dot_mode_opens_no_tray_and_closes_an_open_one() {
    use crate::view::multi::{MultiMarkers, TrayRequest};
    use crate::view::rings::picking::{RingPaths, RingPickingPlugin, RingTarget};
    use crate::view::targets::OrbitFor;
    let mut app = tray_app_with(|app| {
        app.add_message::<bevy::picking::backend::PointerHits>()
            .add_plugins(RingPickingPlugin);
    });
    let halden = body(&mut app, "Halden IV");
    let orison = body(&mut app, "Orison");
    let high = app.world().get::<InNamedOrbit>(orison).unwrap().0;
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(halden);
    run(&mut app, 60 + eased_frames());

    let some_marker = app
        .world()
        .resource::<MultiMarkers>()
        .markers
        .iter()
        .filter(|marker| marker.drawn)
        .map(|marker| marker.key)
        .next()
        .expect("a multi-marker in Halden IV's frame");
    app.world_mut().resource_mut::<TrayRequest>().0 = Some(TrayFor::Marker(some_marker));
    run(&mut app, 2);
    assert_eq!(open_tray(&app), Some(TrayFor::Marker(some_marker)));

    let ring = {
        let mut rings = app
            .world_mut()
            .query::<(Entity, &RingTarget, Option<&OrbitFor>)>();
        rings
            .iter(app.world())
            .find(|(_, ring, orbit_for)| ring.0 == high && orbit_for.is_some())
            .map(|(e, ..)| e)
            .expect("the High band's ring target")
    };
    let on_ring = app.world().resource::<RingPaths>().0[&ring][0].0;
    move_cursor(&mut app, on_ring);
    run(&mut app, 1);
    mouse(&mut app, ButtonState::Pressed);
    click_node(&mut app, ring, 1);
    assert_eq!(
        app.world().resource::<TrayRequest>().0,
        None,
        "the click asks for no tray"
    );
    mouse(&mut app, ButtonState::Released);
    run(&mut app, 1);
    assert_eq!(selected(&app), Some(halden));
    assert_eq!(
        app.world().resource::<Selection>().revealed_orbit,
        Some(high)
    );
    assert_eq!(open_tray(&app), None, "the open tray closed");
}

/// MUL-08: the tray's panel and notch are filled fully opaque, so nothing
/// on the map shows through them.
#[test]
fn the_trays_fill_is_opaque() {
    use crate::view::tray::draw::{TrayNotch, TrayPanel};
    let mut app = tray_app();
    run(&mut app, SETTLE_FRAMES);
    let halden = body(&mut app, "Halden IV");
    click_fold_marker(&mut app, halden);
    let mut fills = app
        .world_mut()
        .query_filtered::<&BackgroundColor, Or<(With<TrayPanel>, With<TrayNotch>)>>();
    let alphas: Vec<f32> = fills.iter(app.world()).map(|fill| fill.0.alpha()).collect();
    assert_eq!(alphas, [1.0, 1.0]);
}
