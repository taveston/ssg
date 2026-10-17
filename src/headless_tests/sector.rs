//! Headless tests of the sector view (SECT): a small generated sector in
//! the sector view, with the real camera, nodes, links, labels, marks,
//! readout and inset, driven through projected positions and simulated
//! input.

use std::collections::HashMap;

use bevy::input::ButtonState;
use bevy::picking::{InteractionPlugin, PickingPlugin, PickingSystems};

use super::camera::{follow_cursor_with_pointer, key_event, mouse, move_cursor, press_key};
use super::*;
use crate::content::generator::Sector;
use crate::content::generator::network::LinkState;
use crate::content::generator::tunables::GeneratorTunables;
use crate::content::start::{LoadedSector, SectorRequest, Start, load_start};
use crate::view::camera::state::Bookmarks;
use crate::view::camera::stops::{FitView, SectorFit, distance_for_points};
use crate::view::colors;
use crate::view::map_layer::{MapLayer, MapLayerPlugin, SectorSelection};
use crate::view::markers::brackets::LayerMarks;
use crate::view::scale::ly_to_render;
use crate::view::screen::MapArea;
use crate::view::sector::input::NodeTarget;
use crate::view::sector::inset::{EnterButton, GateRowFor, SectorInsetPanel};
use crate::view::sector::labels::NodeLabel;
use crate::view::sector::links::{LinkPieces, LinkStroke};
use crate::view::sector::nodes::{NodeScreen, NodeScreens, node_name};
use crate::view::sector::readout::{SeedLine, TRUTH_LINE, TruthReadout};
use crate::view::sector::{EnterRequest, ForEntry, NodeKind, SectorNode, SectorPlugin};

/// The seed every sector here is made from.
const SEED: u64 = 11;

/// A small sector in the sector view, with the real camera, paused.
fn sector_app() -> App {
    sector_app_with(|_| {})
}

/// `sector_app` with more set up before start-up runs.
pub(super) fn sector_app_with(extra: impl FnOnce(&mut App)) -> App {
    let mut app = headless_app(FRAME);
    let small = GeneratorTunables::default().preset_small;
    let request = SectorRequest {
        gated_count: small,
        seed: Some(SEED),
    };
    load_start(&mut app, Start::Sector(request));
    app.insert_resource(placeholder_fonts());
    extra(&mut app);
    app.add_plugins((SimPlugin, InputPlugin))
        .add_plugins((
            TunablesPlugin,
            BindingsPlugin,
            ProjectionPlugin,
            CameraPlugin,
        ))
        .add_plugins(ScreenPlugin)
        .init_resource::<Selection>()
        .init_resource::<PlannerState>()
        .init_resource::<crate::view::multi::OpenTray>()
        .init_resource::<crate::view::multi::TrayRequest>()
        .init_resource::<crate::view::multi::MultiMarkers>()
        .add_plugins((FramePlugin, crate::view::sidebar::SidebarPlugin))
        .init_resource::<PressStart>()
        .init_resource::<crate::view::double_click::MapClicks>()
        .add_plugins((MapLayerPlugin, SectorPlugin, PlotMarksPlugin));
    app.world_mut().spawn(Window {
        resolution: WindowResolution::new(VIEWPORT.x, VIEWPORT.y),
        ..default()
    });
    run(&mut app, 1);
    compute_camera_headless(&mut app);
    app.world_mut().resource_mut::<GameClock>().toggle_pause();
    run(&mut app, 2);
    app
}

/// `sector_app` with Bevy's picking running, the pointer following the
/// window's cursor.
fn picking_sector_app() -> App {
    let mut app = sector_app_with(|app| {
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

/// A primary click at `at`, as the mouse and Bevy's pointer see it, so a
/// hit from any picking backend turns into a click.
fn pointer_click_at(app: &mut App, at: Vec2) {
    use bevy::camera::NormalizedRenderTarget;
    use bevy::input::mouse::MouseButtonInput;
    use bevy::picking::pointer::{Location, PointerAction, PointerId, PointerInput};
    move_cursor(app, at);
    run(app, 2);
    let window = the_window(app);
    let location = Location {
        target: NormalizedRenderTarget::Window(
            bevy::window::WindowRef::Entity(window)
                .normalize(None)
                .unwrap(),
        ),
        position: at,
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
        let input = PointerInput::new(PointerId::Mouse, location.clone(), action);
        app.world_mut().write_message(input);
        app.world_mut().write_message(MouseButtonInput {
            button: MouseButton::Left,
            state,
            window,
        });
        run(app, 1);
    }
    run(app, 1);
}

fn sector(app: &App) -> &Sector {
    &app.world().resource::<LoadedSector>().sector
}

fn screens(app: &App) -> HashMap<usize, NodeScreen> {
    app.world().resource::<NodeScreens>().0.clone()
}

fn rig(app: &App) -> CameraRig {
    *app.world().resource::<CameraRig>()
}

fn selected(app: &App) -> Option<usize> {
    app.world().resource::<SectorSelection>().0
}

/// The entities carrying `T` whose `ForEntry` is `id`.
fn for_entry<T: Component>(app: &mut App, id: usize) -> Vec<Entity> {
    let mut query = app
        .world_mut()
        .query_filtered::<(Entity, &ForEntry), With<T>>();
    query
        .iter(app.world())
        .filter(|(_, entry)| entry.0 == id)
        .map(|(entity, _)| entity)
        .collect()
}

/// The node drawn nearest the middle of the map of `kind`, by entry id.
fn central_node(app: &App, kind: NodeKind) -> usize {
    let centre = VIEWPORT.as_vec2() / 2.0;
    screens(app)
        .into_iter()
        .filter(|(_, s)| s.kind == kind)
        .min_by(|a, b| {
            a.1.centre
                .distance(centre)
                .total_cmp(&b.1.centre.distance(centre))
        })
        .map(|(id, _)| id)
        .expect("a node of that kind on screen")
}

/// Where a world point lands on screen through the real camera.
fn project(app: &mut App, world: Vec3) -> Option<Vec2> {
    let mut cameras = app
        .world_mut()
        .query_filtered::<(&Camera, &Transform), With<OrbitCamera>>();
    let (camera, transform) = cameras.single(app.world()).ok()?;
    camera
        .world_to_viewport(&GlobalTransform::from(*transform), world)
        .ok()
}

/// The sector fit for the rig's heading at `tilt`, in this window with no
/// sidebar.
fn sector_fit(app: &App, heading: f32, tilt: f32) -> f32 {
    let view = FitView {
        heading,
        tilt,
        fov_y: fov(),
        margin: Tunables::default().zoom_fit_margin,
        viewport: VIEWPORT.as_vec2(),
    };
    distance_for_points(&app.world().resource::<SectorFit>().points, view)
}

/// Every text under `root`, in spawn order of the hierarchy.
fn texts_under(app: &mut App, root: Entity) -> Vec<String> {
    descendants(app, root)
        .into_iter()
        .filter_map(|node| app.world().get::<Text>(node).map(|t| t.0.clone()))
        .collect()
}

fn panel(app: &mut App) -> Entity {
    let mut panels = app
        .world_mut()
        .query_filtered::<Entity, With<SectorInsetPanel>>();
    panels.single(app.world()).unwrap()
}

/// No layout runs headless: give every label `size` and let them settle.
fn settle_node_labels(app: &mut App, size: Vec2) {
    for _ in 0..12 {
        let mut labels = app
            .world_mut()
            .query_filtered::<&mut ComputedNode, With<NodeLabel>>();
        for mut computed in labels.iter_mut(app.world_mut()) {
            computed.size = size;
        }
        run(app, 2);
    }
}

/// The labels shown, by entry, with their box and text.
fn shown_labels(app: &mut App, size: Vec2) -> Vec<(usize, Rect, String)> {
    let mut labels = app
        .world_mut()
        .query_filtered::<(&ForEntry, &Node, &Text, &Visibility), With<NodeLabel>>();
    labels
        .iter(app.world())
        .filter(|(.., visibility)| **visibility != Visibility::Hidden)
        .filter_map(|(entry, node, text, _)| {
            let min = crate::view::node_box::node_min(node)?;
            Some((entry.0, Rect::from_corners(min, min + size), text.0.clone()))
        })
        .collect()
}

/// SECT-01 / SECT-03 / SECT-40: a game started with a sector starts in
/// the sector view with no system: one node per catalogue entry, a gated
/// system's a star marker `sector_node_px` across, an ungated system's a
/// circle `sector_dot_px` across at `sector_dot_alpha`, a brown dwarf's
/// the same at `sector_faint_alpha`.
#[test]
fn a_sector_start_shows_a_node_for_every_entry() {
    let mut app = sector_app();
    let t = Tunables::default();
    assert_eq!(*app.world().resource::<MapLayer>(), MapLayer::Sector);
    let mut bodies = app.world_mut().query::<&BodyClass>();
    assert_eq!(bodies.iter(app.world()).count(), 0, "no system is loaded");
    let entries = sector(&app).entries.len();
    let mut nodes = app
        .world_mut()
        .query::<(Entity, &SectorNode, &Node, Option<&BorderColor>)>();
    let found: Vec<_> = nodes
        .iter(app.world())
        .map(|(e, n, ui, border)| (e, *n, ui.width, border.map(|b| b.top)))
        .collect();
    assert_eq!(found.len(), entries);
    let mut kinds = std::collections::HashSet::new();
    for (entity, node, width, border) in found {
        kinds.insert(node.kind);
        match node.kind {
            NodeKind::Gated => {
                assert_eq!(width, px(t.sector_node_px));
                let ticks = descendants(&mut app, entity)
                    .into_iter()
                    .filter(|&e| {
                        app.world()
                            .get::<crate::view::markers::BodyDetail>(e)
                            .is_some()
                    })
                    .count();
                assert_eq!(ticks, t.star_tick_count as usize, "MRK-13: a star marker");
            }
            NodeKind::Ungated => {
                assert_eq!(width, px(t.sector_dot_px));
                assert_eq!(border.unwrap().alpha(), t.sector_dot_alpha);
            }
            NodeKind::BrownDwarf => {
                assert_eq!(width, px(t.sector_dot_px));
                assert_eq!(border.unwrap().alpha(), t.sector_faint_alpha);
            }
        }
    }
    assert_eq!(kinds.len(), 3, "gated, ungated and brown dwarf nodes");
}

/// SECT-03 / SECT-05 / MRK-03: each node sits on its star's projected
/// position, the same size at any zoom, and a nearer node is layered
/// over a farther one.
#[test]
fn nodes_sit_on_their_stars_nearer_over_farther() {
    let mut app = sector_app();
    let check = |app: &mut App| {
        let drawn = screens(app);
        assert!(drawn.len() > 3);
        let mut nodes = app
            .world_mut()
            .query::<(&SectorNode, &Node, &GlobalZIndex)>();
        let placed: Vec<(SectorNode, Node, i32)> = nodes
            .iter(app.world())
            .map(|(n, ui, z)| (*n, ui.clone(), z.0))
            .collect();
        for (node, ui, _) in &placed {
            let Some(screen) = drawn.get(&node.id) else {
                continue;
            };
            let at = project(app, node.world).unwrap();
            assert!(
                screen.centre.distance(at) <= 0.75,
                "node {} off its star",
                node.id
            );
            let (Val::Px(left), Val::Px(width)) = (ui.left, ui.width) else {
                panic!("placed in pixels");
            };
            assert!((left + width / 2.0 - screen.centre.x).abs() < 1e-3);
        }
        let z_of: HashMap<usize, i32> = placed.iter().map(|(n, _, z)| (n.id, *z)).collect();
        let mut by_depth: Vec<(usize, f32)> = drawn.iter().map(|(&id, s)| (id, s.depth)).collect();
        by_depth.sort_by(|a, b| a.1.total_cmp(&b.1));
        for pair in by_depth.windows(2) {
            assert!(z_of[&pair[0].0] > z_of[&pair[1].0], "nearer over farther");
        }
    };
    let t = Tunables::default();
    let sized = |app: &App| {
        let by_kind = |s: &NodeScreen| crate::view::sector::nodes::node_px(s.kind, &t) / 2.0;
        screens(app).values().all(|s| s.half == by_kind(s))
    };
    check(&mut app);
    assert!(sized(&app));
    scroll(&mut app, 2.0);
    run(&mut app, SETTLE_FRAMES);
    check(&mut app);
    assert!(sized(&app), "the same size at any zoom");
}

/// SECT-04: each link is a straight line between its nodes' centres: a
/// working or collapsed link one solid piece, a dormant one dashes along
/// the same line; each drawn by its state.
#[test]
fn links_run_between_node_centres_by_state() {
    let app = sector_app();
    let sector = sector(&app).clone();
    let pieces = &app.world().resource::<LinkPieces>().0;
    let world =
        |id: usize| crate::view::scale::sector_km_to_render_vec3(sector.entries[id].position_km);
    let on_line = |p: Vec3, a: Vec3, b: Vec3| {
        let t = (p - a).dot(b - a) / (b - a).length_squared();
        (a.lerp(b, t) - p).length() < 1e-2 && (-1e-4..=1.0001).contains(&t)
    };
    let mut seen = [0usize; 3];
    for link in &sector.links {
        let (a, b) = (world(link.ends[0]), world(link.ends[1]));
        let stroke = LinkStroke::of(link.state);
        let mine: Vec<_> = pieces
            .iter()
            .filter(|(s, p)| *s == stroke && on_line(p.from, a, b) && on_line(p.to, a, b))
            .collect();
        match link.state {
            LinkState::Working | LinkState::Collapsed => {
                let whole = mine
                    .iter()
                    .any(|(_, p)| p.from.distance(a) < 1e-3 && p.to.distance(b) < 1e-3);
                assert!(whole, "a solid line from centre to centre");
                seen[if link.state == LinkState::Working {
                    0
                } else {
                    2
                }] += 1;
            }
            LinkState::Dormant { .. } => {
                assert!(mine.len() > 1, "a dormant link is dashed");
                seen[1] += 1;
            }
        }
    }
    assert!(
        seen.iter().all(|&n| n > 0),
        "every state is drawn: {seen:?}"
    );
}

/// SECT-06: gated nodes are labelled with their names, clear of each
/// other and of every node; an ungated node's designation shows only
/// while it is selected or hovered.
#[test]
fn gated_nodes_are_labelled_and_others_only_when_picked() {
    let mut app = sector_app();
    let size = Vec2::new(44.0, 14.0);
    settle_node_labels(&mut app, size);
    let sector = sector(&app).clone();
    let shown = shown_labels(&mut app, size);
    assert!(shown.len() > 10, "{} labels", shown.len());
    let margin = Tunables::default().bracket_margin_px;
    let boxes: Vec<Rect> = screens(&app)
        .values()
        .map(|s| Rect::from_center_half_size(s.centre, Vec2::splat(s.half + margin)))
        .collect();
    for (i, (id, rect, text)) in shown.iter().enumerate() {
        assert!(sector.gated_system(*id).is_some(), "only gated labels show");
        assert_eq!(*text, node_name(&sector, *id));
        let overlaps = |a: Rect, b: Rect| {
            let shared = a.intersect(b);
            shared.width() > 0.0 && shared.height() > 0.0
        };
        assert!(boxes.iter().all(|b| !overlaps(*b, *rect)), "clear of nodes");
        for (_, other, _) in &shown[i + 1..] {
            assert!(!overlaps(*rect, *other), "labels never overprint");
        }
    }
    let ungated = central_node(&app, NodeKind::Ungated);
    app.world_mut().resource_mut::<SectorSelection>().0 = Some(ungated);
    settle_node_labels(&mut app, size);
    let shown = shown_labels(&mut app, size);
    let label = shown
        .iter()
        .find(|(id, ..)| *id == ungated)
        .expect("selected label");
    assert_eq!(label.2, sector.entries[ungated].designation);

    app.world_mut().resource_mut::<SectorSelection>().0 = None;
    let other = central_node(&app, NodeKind::BrownDwarf);
    for target in for_entry::<NodeTarget>(&mut app, other) {
        app.world_mut().entity_mut(target).insert(Hovered);
    }
    settle_node_labels(&mut app, size);
    let shown = shown_labels(&mut app, size);
    assert!(shown.iter().any(|(id, ..)| *id == other), "hovered label");
    assert!(
        !shown.iter().any(|(id, ..)| *id == ungated),
        "unselected, it hides"
    );
}

/// SECT-07 / HOV-06 / HOV-07: hovering a node draws the hover mark round
/// its box, selecting one the selection mark.
#[test]
fn hovering_and_selecting_a_node_marks_it() {
    let mut app = sector_app();
    let t = Tunables::default();
    let hovered_id = central_node(&app, NodeKind::Gated);
    let selected_id = central_node(&app, NodeKind::Ungated);
    for target in for_entry::<NodeTarget>(&mut app, hovered_id) {
        app.world_mut().entity_mut(target).insert(Hovered);
    }
    app.world_mut().resource_mut::<SectorSelection>().0 = Some(selected_id);
    run(&mut app, 2);
    let marks = app.world().resource::<LayerMarks>().0.clone();
    let drawn = screens(&app);
    let mark_at = |id: usize| {
        let s = drawn[&id];
        marks
            .iter()
            .find(|m| m.bbox.centre == s.centre)
            .map(|m| (m.alpha, m.bbox.half - Vec2::splat(s.half)))
    };
    let margin = Vec2::splat(t.bracket_margin_px);
    assert_eq!(mark_at(hovered_id), Some((t.bracket_hover_alpha, margin)));
    assert_eq!(mark_at(selected_id), Some((1.0, margin)));
    assert_eq!(marks.len(), 2);
}

/// SECT-08: the readout's two lines sit `plot_margin_px` in from the top
/// left of the map area: the truth line and "Seed *seed* · *N* gated
/// systems".
#[test]
fn the_truth_readout_names_the_seed_and_gated_count() {
    let mut app = sector_app();
    let t = Tunables::default();
    let mut roots = app
        .world_mut()
        .query_filtered::<(Entity, &Node), With<TruthReadout>>();
    let (root, node) = roots.single(app.world()).unwrap();
    assert_eq!(node.display, Display::Flex);
    assert_eq!(node.left, px(t.plot_margin_px));
    assert_eq!(node.top, px(TOP_BAR_HEIGHT_PX + t.plot_margin_px));
    let gated = sector(&app).gated.len();
    let mut lines = app.world_mut().query_filtered::<&Text, With<SeedLine>>();
    let line = lines.single(app.world()).unwrap().0.clone();
    assert_eq!(line, format!("Seed {SEED} · {gated} gated systems"));
    let letters: String = texts_under(&mut app, root)
        .into_iter()
        .filter(|t| *t != line)
        .collect();
    let wanted: String = TRUTH_LINE.to_uppercase().split_whitespace().collect();
    assert_eq!(letters, wanted);
}

/// SECT-09: the scale bar reads a round number of light years and the
/// north dial shows; there is no frame cue.
#[test]
fn the_scale_bar_reads_light_years_and_no_frame_cue_is_drawn() {
    let mut app = sector_app_with(|app| {
        app.init_resource::<crate::view::fold::FoldSummary>()
            .add_plugins(crate::view::frame_cue::FrameCuePlugin);
    });
    run(&mut app, 2);
    let mut bars = app.world_mut().query::<(&Text, &TextFont)>();
    let length = bars
        .iter(app.world())
        .map(|(text, _)| text.0.clone())
        .find(|text| text.ends_with(" ly"))
        .expect("a length in light years");
    let figure: f64 = length
        .trim_end_matches(" ly")
        .replace(',', "")
        .parse()
        .unwrap();
    let leading = figure / 10f64.powf(figure.log10().floor());
    assert!(
        [1.0, 2.0, 5.0].iter().any(|r| (leading - r).abs() < 1e-9),
        "{length}"
    );
    let mut dials = app
        .world_mut()
        .query_filtered::<&Node, With<crate::view::plot_marks::NorthDial>>();
    assert_eq!(dials.single(app.world()).unwrap().display, Display::Flex);
    let mut cue = app
        .world_mut()
        .query_filtered::<&Node, With<crate::view::frame_cue::draw::CuePart>>();
    assert!(
        cue.iter(app.world())
            .all(|node| node.display == Display::None)
    );
}

/// SECT-02: the clock runs and its keys work in the sector view, and
/// nothing on the map moves with time.
#[test]
fn the_clock_runs_and_nothing_moves_with_it() {
    let mut app = sector_app_with(|app| {
        app.add_plugins((
            bevy::diagnostic::DiagnosticsPlugin,
            crate::view::topbar::TopBarPlugin,
        ));
    });
    let before = screens(&app);
    let day = day(&app);
    press_key(&mut app, KeyCode::Space);
    assert!(
        !app.world().resource::<GameClock>().is_paused(),
        "Space resumes"
    );
    run(&mut app, 30);
    assert!(super::day(&app) > day);
    assert_eq!(screens(&app), before);
}

/// SECT-10 / SECT-12 / CAM-02: the view starts at the sector fit, north
/// up at the standard tilt, over the sector's centre with nothing
/// selected, and every node is inside the map area.
#[test]
fn the_view_starts_at_the_sector_fit_north_up() {
    let mut app = sector_app();
    let t = Tunables::default();
    let start = rig(&app);
    let tilt = t.standard_tilt_deg.to_radians();
    assert_eq!((start.heading, start.tilt), (0.0, tilt));
    assert!(start.pivot.length() < 1e-4);
    let fit = sector_fit(&app, 0.0, tilt);
    assert!((start.distance - fit).abs() < fit * 1e-4);
    assert_eq!(selected(&app), None);
    let area = MapArea::of(VIEWPORT.as_vec2()).rect();
    let nodes: Vec<Vec3> = app.world().resource::<SectorFit>().points.clone();
    assert_eq!(nodes.len(), sector(&app).entries.len());
    for world in nodes {
        let at = project(&mut app, world).unwrap();
        assert!(area.contains(at), "{at} outside the map area");
    }
    assert_eq!(screens(&app).len(), sector(&app).entries.len());
}

/// SECT-11 / ZOOM-01 / ZOOM-13: wheel notches zoom in, anchored on the
/// plane point under the cursor, down to a view radius of
/// `sector_min_view_radius_ly`; zooming out past the sector fit does
/// nothing and never changes the layer.
#[test]
fn zoom_runs_between_the_inner_limit_and_the_sector_fit() {
    let mut app = sector_app();
    let t = Tunables::default();
    let fit = rig(&app).distance;
    for _ in 0..3 {
        scroll(&mut app, -1.0);
    }
    assert!(
        (rig(&app).distance - fit).abs() < fit * 1e-4,
        "no zoom past the fit"
    );
    assert_eq!(*app.world().resource::<MapLayer>(), MapLayer::Sector);

    let cursor = Vec2::new(1000.0, 600.0);
    move_cursor(&mut app, cursor);
    run(&mut app, 1);
    let r = rig(&app);
    let pose = (r.pivot, r.heading, r.tilt, r.distance);
    let under = plane_point_under(cursor, pose);
    scroll(&mut app, 1.0);
    run(&mut app, 2);
    assert!(rig(&app).distance < fit * 0.95, "a notch zooms in");
    let at = project(&mut app, under).unwrap();
    assert!(at.distance(cursor) < 1.0, "anchored on the cursor: {at}");

    for _ in 0..80 {
        scroll(&mut app, 1.0);
    }
    let tilt = rig(&app).tilt;
    let radius = ly_to_render(t.sector_min_view_radius_ly as f64);
    let inner = fit_distance(radius, fov(), tilt, t.zoom_fit_margin);
    assert!(
        (rig(&app).distance - inner).abs() < inner * 1e-3,
        "the inner limit"
    );
}

/// The plane point under a screen position for a camera pose.
fn plane_point_under(
    screen: Vec2,
    (pivot, heading, tilt, distance): (Vec3, f32, f32, f32),
) -> Vec3 {
    use crate::view::camera::rules::{camera_transform, plane_point};
    let transform = camera_transform(pivot, heading, tilt, distance);
    let half_tan = (fov() / 2.0).tan();
    let size = VIEWPORT.as_vec2();
    let ndc = Vec2::new(2.0 * screen.x / size.x - 1.0, 1.0 - 2.0 * screen.y / size.y);
    let local = Vec3::new(ndc.x * half_tan * size.x / size.y, ndc.y * half_tan, -1.0);
    let direction = Dir3::new(transform.rotation * local).unwrap();
    plane_point(Ray3d::new(transform.translation, direction)).unwrap()
}

/// SECT-12 / ZOOM-10 / CAM-03: Home returns to the sector fit in the
/// standard orientation; N turns north up keeping the scale.
#[test]
fn home_and_n_return_to_the_standard_view() {
    let mut app = sector_app();
    let start = rig(&app);
    {
        let mut rig = app.world_mut().resource_mut::<CameraRig>();
        rig.heading = 1.1;
        rig.tilt = 0.3;
    }
    *app.world_mut().resource_mut::<Frame>() =
        Frame::star_frame(Entity::PLACEHOLDER, Vec3::new(40.0, 0.0, 10.0));
    run(&mut app, 2);
    press_key(&mut app, KeyCode::KeyN);
    run(&mut app, move_frames());
    let north = rig(&app);
    assert!(
        north
            .heading
            .rem_euclid(std::f32::consts::TAU)
            .min(std::f32::consts::TAU - north.heading.rem_euclid(std::f32::consts::TAU))
            < 1e-4
    );
    assert!((north.tilt - start.tilt).abs() < 1e-5);
    press_key(&mut app, KeyCode::Home);
    run(&mut app, move_frames());
    let home = rig(&app);
    assert!((home.distance - start.distance).abs() < start.distance * 1e-3);
    assert!(home.pivot.length() < 1e-3);
    assert!((home.tilt - start.tilt).abs() < 1e-5);
}

/// SECT-12: the bookmark keys do nothing in the sector view: nothing is
/// saved, and a slot holding a view isn't recalled.
#[test]
fn bookmarks_do_nothing_in_the_sector_view() {
    let mut app = sector_app();
    key_event(&mut app, KeyCode::ControlLeft, ButtonState::Pressed);
    run(&mut app, 1);
    press_key(&mut app, KeyCode::Digit1);
    key_event(&mut app, KeyCode::ControlLeft, ButtonState::Released);
    run(&mut app, 1);
    assert!(app.world().resource::<Bookmarks>().0[0].is_none());
    let mut far = rig(&app);
    far.distance *= 0.2;
    app.world_mut().resource_mut::<Bookmarks>().0[1] = Some(crate::view::camera::state::Bookmark {
        rig: far,
        frame: Frame::default(),
    });
    let before = rig(&app);
    press_key(&mut app, KeyCode::Digit2);
    run(&mut app, move_frames());
    assert_eq!(rig(&app).distance, before.distance);
}

/// SECT-10 / CAM-19: drags pan the view, never further from the sector's
/// centre than `pan_bound_factor` times the sector fit's radius.
#[test]
fn panning_stops_at_the_sector_pan_bound() {
    let mut app = sector_app();
    let t = Tunables::default();
    let bound = t.pan_bound_factor * app.world().resource::<SectorFit>().radius;
    for _ in 0..8 {
        move_cursor(&mut app, Vec2::new(1500.0, 450.0));
        run(&mut app, 1);
        mouse(&mut app, MouseButton::Left, ButtonState::Pressed);
        run(&mut app, 1);
        for step in 1..=10 {
            move_cursor(&mut app, Vec2::new(1500.0 - 140.0 * step as f32, 450.0));
            run(&mut app, 1);
        }
        mouse(&mut app, MouseButton::Left, ButtonState::Released);
        run(&mut app, 2);
    }
    let pivot = rig(&app).pivot;
    assert!(
        pivot.length() > bound * 0.99,
        "panned to the bound: {pivot}"
    );
    assert!(pivot.length() <= bound * 1.001, "never past it: {pivot}");
}

/// SECT-20: a click on a node selects it and opens its inset in the
/// sidebar; a click on empty space clears it. The system view's inset
/// stays hidden.
#[test]
fn clicking_a_node_selects_it_and_empty_space_clears_it() {
    let mut app = picking_sector_app();
    let id = central_node(&app, NodeKind::Gated);
    let at = screens(&app)[&id].centre;
    pointer_click_at(&mut app, at);
    assert_eq!(selected(&app), Some(id));
    run(&mut app, SETTLE_FRAMES);
    let sidebar = *app.world().resource::<crate::view::sidebar::SidebarState>();
    assert_eq!(sidebar.slide, 1.0);
    let panel = panel(&mut app);
    assert_eq!(
        app.world().get::<Node>(panel).unwrap().display,
        Display::Flex
    );
    let name = node_name(sector(&app), id);
    assert!(texts_under(&mut app, panel).contains(&name));
    pointer_click_at(&mut app, Vec2::new(8.0, 600.0));
    assert_eq!(selected(&app), None);
}

/// SECT-20 / INS-03: Esc and the inset's Close button clear the
/// selection.
#[test]
fn esc_and_close_clear_the_selection() {
    let mut app = sector_app();
    let id = central_node(&app, NodeKind::Gated);
    app.world_mut().resource_mut::<SectorSelection>().0 = Some(id);
    run(&mut app, 2);
    press_key(&mut app, KeyCode::Escape);
    assert_eq!(selected(&app), None);
    app.world_mut().resource_mut::<SectorSelection>().0 = Some(id);
    run(&mut app, 2);
    let panel = panel(&mut app);
    let close = descendants(&mut app, panel)
        .into_iter()
        .find(|&e| {
            app.world()
                .get::<crate::view::buttons::ConsoleButton>(e)
                .is_some()
                && texts_under(&mut app, e) == ["Close"]
        })
        .expect("a Close button");
    click_node(&mut app, close, 1);
    assert_eq!(selected(&app), None);
}

/// SECT-21: the inset's header names the entry, says what it is, and has
/// Close; a gated system's also has Enter system, which asks for it to
/// be entered. There is no Focus button.
#[test]
fn the_inset_header_names_the_entry_and_offers_entering_a_gated_one() {
    use crate::view::sector::inset::content::class_line;
    let mut app = sector_app();
    let gated = central_node(&app, NodeKind::Gated);
    let ungated = central_node(&app, NodeKind::Ungated);
    for (id, enterable) in [(gated, true), (ungated, false)] {
        app.world_mut().resource_mut::<SectorSelection>().0 = Some(id);
        run(&mut app, 2);
        let panel = panel(&mut app);
        let texts = texts_under(&mut app, panel);
        let sector = sector(&app).clone();
        let entry = &sector.entries[id];
        assert!(texts.contains(&node_name(&sector, id)));
        assert!(texts.contains(&class_line(entry, sector.gated_system(id))));
        assert!(texts.contains(&"Close".to_string()));
        assert!(!texts.contains(&"Focus".to_string()));
        let mut buttons = app.world_mut().query::<(Entity, &EnterButton)>();
        let enter: Vec<_> = buttons.iter(app.world()).map(|(e, b)| (e, b.0)).collect();
        assert_eq!(enter.len(), usize::from(enterable));
        if let Some(&(button, for_id)) = enter.first() {
            assert_eq!(for_id, id);
            assert!(texts.contains(&"Enter system".to_string()));
            click_node(&mut app, button, 1);
            assert_eq!(app.world().resource::<EnterRequest>().0, Some(id));
        }
    }
}

/// SECT-24: a click on a gate row selects the destination's node and
/// leaves the camera where it is.
#[test]
fn a_gate_row_selects_its_destination_without_moving_the_camera() {
    let mut app = sector_app();
    let id = central_node(&app, NodeKind::Gated);
    app.world_mut().resource_mut::<SectorSelection>().0 = Some(id);
    run(&mut app, 2);
    let mut rows = app.world_mut().query::<(Entity, &GateRowFor)>();
    let rows: Vec<(Entity, usize)> = rows.iter(app.world()).map(|(e, r)| (e, r.0)).collect();
    assert_eq!(rows.len(), sector(&app).links_of(id).count());
    let (row, destination) = rows[0];
    let before = rig(&app);
    click_node(&mut app, row, 1);
    run(&mut app, 5);
    assert_eq!(selected(&app), Some(destination));
    assert_eq!(rig(&app), before);
}

/// SECT-34: in the sector view the breadcrumb reads "Sector" alone, in
/// the frame colour.
#[test]
fn the_breadcrumb_reads_sector_alone() {
    let mut app = sector_app_with(|app| {
        app.add_plugins(BreadcrumbPlugin);
    });
    app.world_mut().spawn((BreadcrumbRoot, Node::default()));
    run(&mut app, 2);
    let entries = breadcrumb_entries(&mut app);
    let names: Vec<&str> = entries.iter().map(|(_, t)| t.as_str()).collect();
    assert_eq!(names, ["Sector"]);
    let color = app.world().get::<TextColor>(entries[0].0).unwrap().0;
    assert_eq!(color, colors::REFERENCE);
}

/// SECT-44: a screenshot of the sector view selects and hovers entries by
/// a gated system's name or a designation, and fails on a name that is
/// no entry's.
#[test]
fn the_debug_shot_picks_sector_entries_by_name() {
    use crate::view::debug_shot::{DebugShotPlugin, Shot, ShotPhase, ShotPlan, sector_entry_named};
    let probe = sector_app();
    let gated = central_node(&probe, NodeKind::Gated);
    let dwarf = central_node(&probe, NodeKind::BrownDwarf);
    let sector = sector(&probe).clone();
    let name = node_name(&sector, gated);
    let designation = sector.entries[dwarf].designation.clone();
    assert_eq!(sector_entry_named(&sector, &name), Some(gated));
    assert_eq!(sector_entry_named(&sector, &designation), Some(dwarf));
    assert_eq!(sector_entry_named(&sector, "Nowhere"), None);

    let spec = format!(
        "out=unused.png;sector=small;seed={SEED};select={name};hover={designation};wait=0.1"
    );
    let plan = ShotPlan::parse(&spec).unwrap();
    let mut app = sector_app_with(|app| {
        app.insert_resource(Shot::new(plan))
            .add_plugins(DebugShotPlugin);
    });
    let mut frames = 0;
    while app.world().resource::<Shot>().phase != ShotPhase::Capturing {
        run(&mut app, 1);
        frames += 1;
        assert!(frames < 600, "no capture after {frames} frames");
    }
    assert_eq!(selected(&app), Some(gated));
    let hovered = for_entry::<NodeTarget>(&mut app, dwarf);
    assert!(
        hovered
            .iter()
            .all(|&t| app.world().get::<Hovered>(t).is_some())
    );

    let plan = ShotPlan::parse("out=unused.png;sector=small;select=Nowhere").unwrap();
    let mut app = sector_app_with(|app| {
        app.insert_resource(Shot::new(plan))
            .add_plugins(DebugShotPlugin);
    });
    run(&mut app, 120);
    assert_eq!(app.world().resource::<Shot>().phase, ShotPhase::Failed);
}
