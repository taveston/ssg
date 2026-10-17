//! Headless tests of markers and labels (MRK, SYM, LBL, EDGE): true-size
//! discs, draw order, symbol size, the plot furniture, gate edge arrows
//! and label placement, on Corvane with a camera that projects.

use super::*;
use crate::view::edge::GateEdgeTargets;
use crate::view::markers::{BodyDetail, body_fill};
use crate::view::plot_marks::NorthDial;

/// EDGE-01: zoomed in and tilted, every gate still has an arrow on
/// the screen edge - including gates behind the camera, which have no
/// projected position at all.
#[test]
fn every_gate_has_an_arrow_when_zoomed_in() {
    let mut app = headless_app(FRAME);
    app.add_plugins((ContentPlugin, SimPlugin))
        .add_plugins((TunablesPlugin, ProjectionPlugin, StopsPlugin))
        .add_plugins((ScreenPlugin, EdgePlugin))
        .init_resource::<Selection>()
        .init_resource::<PlannerState>()
        .add_plugins(crate::view::sidebar::SidebarPlugin)
        .insert_resource(placeholder_fonts());
    app.world_mut().spawn(Window {
        resolution: WindowResolution::new(VIEWPORT.x, VIEWPORT.y),
        ..default()
    });
    run(&mut app, 2);
    app.world_mut().resource_mut::<GameClock>().toggle_pause();
    run(&mut app, 2);

    let halden = body(&mut app, "Halden IV");
    let margin = app.world().resource::<Tunables>().zoom_fit_margin;
    let radius = app
        .world()
        .resource::<ZoomStops>()
        .satellite_radius(halden)
        .unwrap();
    let distance = fit_distance(radius, fov(), TILT, margin);
    let halden_pos = app
        .world()
        .get::<GlobalTransform>(halden)
        .unwrap()
        .translation();
    app.world_mut().spawn(test_camera(halden_pos, distance));
    run(&mut app, 4);

    let gates: Vec<Entity> = ["Gate Ostra", "Gate Brenn", "Gate Soll"]
        .into_iter()
        .map(|name| body(&mut app, name))
        .collect();
    let screen = app.world().resource::<ScreenPositions>();
    let pinned = app.world().resource::<GateEdgeTargets>();
    for gate in gates {
        let on_screen = screen.get(gate).is_some_and(|pos| screen.in_viewport(pos));
        assert!(
            on_screen || pinned.is_pinned(gate),
            "a gate has neither a marker nor an edge arrow"
        );
    }

    // EDGE-01 / EDGE-02: every pinned gate's tab is shown, labelled with
    // its destination alone.
    let pinned: Vec<Entity> = pinned.0.keys().copied().collect();
    assert!(!pinned.is_empty());
    let mut tabs = app
        .world_mut()
        .query::<(Entity, &crate::view::edge::EdgeTabFor, &Node)>();
    let tabs: Vec<(Entity, Entity, Display)> = tabs
        .iter(app.world())
        .map(|(e, t, n)| (e, t.0, n.display))
        .collect();
    for gate in pinned {
        let &(tab, _, display) = tabs.iter().find(|t| t.1 == gate).unwrap();
        assert_eq!(display, Display::Flex);
        let destination = app
            .world()
            .get::<crate::sim::body::GateDestination>(gate)
            .unwrap()
            .0
            .clone();
        assert!(!destination.starts_with("Gate"));
        assert!(
            descendants(&mut app, tab).iter().any(|&e| app
                .world()
                .get::<Text>(e)
                .is_some_and(|t| t.0 == destination)),
            "the tab is labelled {destination}"
        );
    }
}

/// EDGE-01: turning the heading a full circle in Halden IV's frame
/// at Halden IV's satellite frame and tilted, every gate always has its
/// marker on screen or an arrow on the edge, and no arrow hides under the
/// top bar.
#[test]
fn every_gate_has_an_arrow_as_the_heading_turns() {
    let mut app = camera_app();
    app.add_plugins(EdgePlugin)
        .insert_resource(placeholder_fonts());
    run(&mut app, 1);
    let halden = body(&mut app, "Halden IV");
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(halden);
    run(&mut app, 40);
    app.world_mut().resource_mut::<CameraRig>().tilt = 0.6;
    let gates: Vec<Entity> = ["Gate Ostra", "Gate Brenn", "Gate Soll"]
        .into_iter()
        .map(|name| body(&mut app, name))
        .collect();
    for step in 0..24 {
        app.world_mut().resource_mut::<CameraRig>().heading = step as f32 * 0.2618;
        run(&mut app, 2);
        let screen = app.world().resource::<ScreenPositions>();
        let pinned = app.world().resource::<GateEdgeTargets>();
        for &gate in &gates {
            let on_screen = screen.get(gate).is_some_and(|pos| screen.in_viewport(pos));
            assert!(
                on_screen || pinned.is_pinned(gate),
                "heading step {step}: a gate is lost"
            );
            if let Some(&arrow) = pinned.0.get(&gate) {
                assert!(
                    arrow.y > TOP_BAR_HEIGHT_PX,
                    "an arrow under the top bar: {arrow}"
                );
            }
        }
    }
}

/// The marker node drawn for a body, as its width in pixels.
/// A body marker's fill colour and whether its class detail shows.
fn marker_fill_and_detail(app: &mut App, body: Entity) -> (Color, bool) {
    let mut markers = app
        .world_mut()
        .query_filtered::<(Entity, &MarkerFor, &Tint), With<MarkerHalfSize>>();
    let (marker, _, tint) = markers
        .iter(app.world())
        .find(|(_, marker, _)| marker.0 == body)
        .expect("a marker for the body");
    let fill = tint.color;
    let mut details = app
        .world_mut()
        .query_filtered::<(&Node, &ChildOf), With<BodyDetail>>();
    let shown = details
        .iter(app.world())
        .filter(|(_, child_of)| child_of.parent() == marker)
        .all(|(node, _)| node.display != Display::None);
    (fill, shown)
}

fn marker_width_px(app: &mut App, body: Entity) -> f32 {
    let mut markers = app
        .world_mut()
        .query_filtered::<(&MarkerFor, &Node), With<MarkerHalfSize>>();
    let (_, node) = markers
        .iter(app.world())
        .find(|(marker, _)| marker.0 == body)
        .expect("a marker for the body");
    match node.width {
        Val::Px(width) => width,
        other => panic!("marker width {other:?}"),
    }
}

/// MRK-07: at the system fit Halden IV is its class's fixed marker; close
/// in, where its true diameter is larger than that, the marker is a disc of
/// the true on-screen size.
#[test]
fn a_body_grows_into_a_true_size_disc_when_close() {
    let mut app = camera_app_with(|app| {
        app.insert_resource(placeholder_fonts())
            .init_resource::<crate::view::fold::FoldSummary>()
            .add_plugins((EdgePlugin, MarkersPlugin, HoveredPlugin));
    });
    let halden = body(&mut app, "Halden IV");
    run(&mut app, 2);
    let far = marker_width_px(&mut app, halden);
    let planet_px = Tunables::default().planet_marker_px;
    assert_eq!(far, planet_px, "a planet's fixed marker at the system fit");
    // MRK-13: as a marker it is a ground-filled outline with its dot.
    let tunables = Tunables::default();
    assert_eq!(
        marker_fill_and_detail(&mut app, halden),
        (body_fill(false, &tunables), true)
    );

    let distance = 0.01;
    set_frame(&mut app, halden, Vec3::ZERO);
    app.world_mut().resource_mut::<CameraRig>().distance = distance;
    run(&mut app, 3);
    let radius_km = app.world().get::<PhysicalRadius>(halden).unwrap().0;
    let expected =
        2.0 * projected_radius_px(km_to_render(radius_km), distance, fov(), VIEWPORT.y as f32);
    let drawn = marker_width_px(&mut app, halden);
    assert!(
        expected > planet_px,
        "the test view is close enough: {expected}"
    );
    assert!(
        (drawn - expected).abs() < expected * 0.01,
        "drawn {drawn} px, true size {expected} px"
    );
    // MRK-11: the true-size disc is filled, without the marker's dot.
    assert_eq!(
        marker_fill_and_detail(&mut app, halden),
        (body_fill(true, &tunables), false)
    );
}

/// MRK-12: natural bodies are layered by depth, nearer on top, and the
/// order follows the view as it turns; symbols sit above every body and
/// labels above every marker.
#[test]
fn nearer_bodies_draw_over_farther_and_symbols_over_bodies() {
    let mut app = camera_app_with(|app| {
        app.insert_resource(placeholder_fonts())
            .init_resource::<crate::view::fold::FoldSummary>()
            .add_plugins((EdgePlugin, MarkersPlugin, HoveredPlugin));
    });
    let names = ["Pell", "Iver", "Halden IV", "Morrow"];
    let bodies: Vec<Entity> = names.iter().map(|n| body(&mut app, n)).collect();
    let layers_agree_with_depth = |app: &mut App| {
        run(app, 2);
        let camera = {
            let mut q = app
                .world_mut()
                .query_filtered::<&Transform, With<OrbitCamera>>();
            *q.single(app.world()).unwrap()
        };
        let mut markers = app
            .world_mut()
            .query_filtered::<(&MarkerFor, &GlobalZIndex), With<crate::view::markers::MarkerShape>>(
            );
        let layer_of: std::collections::HashMap<Entity, i32> =
            markers.iter(app.world()).map(|(m, z)| (m.0, z.0)).collect();
        let positions = app.world().resource::<WorldPositions>();
        let depth = |body: Entity| (positions.0[&body] - camera.translation).dot(*camera.forward());
        let mut by_depth = bodies.clone();
        by_depth.sort_by(|a, b| depth(*b).total_cmp(&depth(*a)));
        let layers: Vec<i32> = by_depth.iter().map(|b| layer_of[b]).collect();
        assert!(layers.windows(2).all(|w| w[0] < w[1]), "{layers:?}");
        assert!(layers.iter().all(|&l| l < crate::view::layers::SYMBOLS));
    };
    layers_agree_with_depth(&mut app);
    app.world_mut().resource_mut::<CameraRig>().heading += std::f32::consts::PI;
    layers_agree_with_depth(&mut app);

    let ship = body(&mut app, "Iver Yard");
    let mut z = app.world_mut().query::<(
        &MarkerFor,
        &GlobalZIndex,
        Has<crate::view::markers::MarkerLabel>,
    )>();
    for (marker_for, layer, is_label) in z.iter(app.world()) {
        if marker_for.0 == ship && !is_label {
            assert_eq!(layer.0, crate::view::layers::SYMBOLS);
        }
        if is_label {
            assert_eq!(layer.0, crate::view::layers::LABELS);
        }
    }
}

/// SYM-04 / MRK-03: a ship's symbol fits `ship_symbol_px` at every zoom; it
/// never grows to its true size as natural bodies do.
#[test]
fn a_symbol_keeps_its_size_at_every_zoom() {
    let mut app = camera_app_with(|app| {
        app.insert_resource(placeholder_fonts())
            .init_resource::<crate::view::fold::FoldSummary>()
            .add_plugins((EdgePlugin, MarkersPlugin, HoveredPlugin));
    });
    let symbol_px = app.world().resource::<Tunables>().ship_symbol_px;
    let orison = body(&mut app, "Orison");
    let halden = body(&mut app, "Halden IV");
    set_frame(&mut app, halden, Vec3::ZERO);
    let mut shown = 0;
    for distance in [2.0, 1.0, 0.5, 0.25] {
        app.world_mut().resource_mut::<CameraRig>().distance = distance;
        run(&mut app, 20);
        let mut markers = app
            .world_mut()
            .query_filtered::<(&MarkerFor, &Node), With<MarkerHalfSize>>();
        let (_, node) = markers
            .iter(app.world())
            .find(|(m, _)| m.0 == orison)
            .unwrap();
        if node.display == Display::Flex {
            shown += 1;
            assert_eq!(node.width, Val::Px(symbol_px), "at {distance}");
        }
    }
    assert!(shown >= 2, "the symbol was on screen at {shown} zooms");
}

/// MRK-16 / MRK-17: the scale bar is a round length within its maximum;
/// the dial's arrow follows the heading; clicking the dial turns north up
/// as N does (CAM-03).
#[test]
fn the_scale_bar_fits_and_the_north_dial_follows_and_resets_the_heading() {
    let mut app = camera_app_with(|app| {
        app.insert_resource(placeholder_fonts())
            .add_plugins(PlotMarksPlugin);
    });
    run(&mut app, 2);
    let max = Tunables::default().scale_bar_max_px;
    let mut lines = app
        .world_mut()
        .query_filtered::<&Node, With<ScaleBarLine>>();
    let Val::Px(width) = lines.single(app.world()).unwrap().width else {
        panic!("the scale bar has a pixel width");
    };
    // A round length (1, 2 or 5) is at least 0.4 of the next one up.
    assert!(width <= max && width >= 0.4 * max - 1.0, "{width} px");

    let arrow_angle = |app: &mut App| {
        let mut arrows = app
            .world_mut()
            .query_filtered::<&UiTransform, With<NorthArrow>>();
        arrows.single(app.world()).unwrap().rotation.as_radians()
    };
    assert!(arrow_angle(&mut app).abs() < 1e-3, "north up at the start");
    {
        let mut rig = app.world_mut().resource_mut::<CameraRig>();
        rig.heading = 0.5;
        rig.tilt = 0.0;
    }
    run(&mut app, 2);
    assert!(
        (arrow_angle(&mut app).abs() - 0.5).abs() < 1e-2,
        "seen straight down, north turns with the heading"
    );

    let mut dials = app.world_mut().query_filtered::<Entity, With<NorthDial>>();
    let dial = dials.single(app.world()).unwrap();
    let camera = app.world_mut().spawn_empty().id();
    app.world_mut().trigger(PointerClick {
        entity: dial,
        pointer: bevy::picking::events::Pointer::new(
            bevy::picking::pointer::PointerId::Mouse,
            bevy::picking::pointer::Location {
                target: bevy::camera::NormalizedRenderTarget::None {
                    width: VIEWPORT.x,
                    height: VIEWPORT.y,
                },
                position: Vec2::ZERO,
            },
        ),
        button: PointerButton::Primary,
        hit: bevy::picking::backend::HitData::new(camera, 0.0, None, None),
        duration: Duration::from_millis(80),
        count: 1,
    });
    run(&mut app, move_frames());
    let heading = app.world().resource::<CameraRig>().heading;
    assert!(heading.abs() < 1e-3, "heading {heading}");
    assert!(arrow_angle(&mut app).abs() < 1e-3);
}

/// LBL-01 / LBL-03 / LBL-04 / SYM-13: in Halden IV's frame with its
/// satellites unfolded, every shown label sits clear of every other label
/// and of every marker (with a symbol's amplifiers) and multi-marker but
/// its own, the selection's label is never hidden, and labels fade in
/// over `label_fade_ms` rather than appearing at once.
#[test]
fn labels_keep_clear_of_each_other_and_of_markers() {
    use crate::view::markers::labels::{LabelState, LabelsPlugin, label_rect, overlaps};
    use crate::view::markers::{MarkerLabel, MarkersPlugin};
    use crate::view::multi::{BackFrame, frames_rect};

    let mut app = camera_app_with(|app| {
        app.insert_resource(placeholder_fonts()).add_plugins((
            EdgePlugin,
            GroupingPlugin,
            MotionPlugin,
            MarkersPlugin,
            MultiMarkersPlugin,
            HoveredPlugin,
            LabelsPlugin,
        ));
    });
    let halden = body(&mut app, "Halden IV");
    let ivc = body(&mut app, "Halden IVc");
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(halden);
    run(&mut app, 60 + move_frames());

    // No layout runs headless: give every marker the size its node asks
    // for, and every label the size a short name has.
    const LABEL_SIZE: Vec2 = Vec2::new(80.0, 14.0);
    fn size_nodes(app: &mut App) {
        let mut markers = app
            .world_mut()
            .query_filtered::<(&Node, &mut ComputedNode), With<MarkerHalfSize>>();
        for (node, mut computed) in markers.iter_mut(app.world_mut()) {
            if let (Val::Px(w), Val::Px(h)) = (node.width, node.height) {
                computed.size = Vec2::new(w, h);
            }
        }
        let mut labels = app
            .world_mut()
            .query_filtered::<&mut ComputedNode, With<MarkerLabel>>();
        for mut computed in labels.iter_mut(app.world_mut()) {
            computed.size = LABEL_SIZE;
        }
    }
    size_nodes(&mut app);
    app.world_mut().resource_mut::<Selection>().select(ivc);
    run(&mut app, 2);
    size_nodes(&mut app);
    run(&mut app, 1);

    let mut labels = app
        .world_mut()
        .query_filtered::<(&LabelState, &ChildOf, &Node, &MarkerFor), With<MarkerLabel>>();
    let fading = labels
        .iter(app.world())
        .any(|(state, ..)| state.shown.is_some() && state.alpha > 0.0 && state.alpha < 1.0);
    assert!(fading, "labels fade in over label_fade_ms");
    run(&mut app, 30);

    let marker_rect = |app: &App, marker: Entity| -> Option<Rect> {
        let node = app.world().get::<Node>(marker)?;
        if node.display == Display::None {
            return None;
        }
        let (Val::Px(l), Val::Px(t), Val::Px(w), Val::Px(h)) =
            (node.left, node.top, node.width, node.height)
        else {
            return None;
        };
        let square = Rect::from_corners(Vec2::new(l, t), Vec2::new(l + w, t + h));
        // LBL-01 / SYM-13: a label sits beside the marker's box, with a
        // symbol's amplifiers.
        let reach = app
            .world()
            .get::<crate::view::symbols::draw::AmplifierReach>(marker);
        let back = app.world().get::<BackFrame>(marker).and_then(|b| b.0);
        let margin = app.world().resource::<Tunables>().bracket_margin_px;
        Some(crate::view::markers::labels::marker_box(square, (reach, back), margin).0)
    };
    let gap = app.world().resource::<Tunables>().label_gap_px;
    let mut shown: Vec<(Entity, Entity, Rect)> = Vec::new();
    let listed: Vec<(LabelState, Entity, Entity)> = labels
        .iter(app.world())
        .map(|(state, child_of, _, marker_for)| (*state, child_of.parent(), marker_for.0))
        .collect();
    for (state, marker, body) in listed {
        let (Some(placement), Some(rect)) = (state.shown, marker_rect(&app, marker)) else {
            continue;
        };
        if state.alpha <= 0.0 {
            continue;
        }
        shown.push((
            marker,
            body,
            label_rect(rect, LABEL_SIZE, placement.slot, gap),
        ));
    }
    assert!(shown.len() >= 3, "{} labels shown", shown.len());
    assert!(
        shown.iter().any(|(_, body, _)| *body == ivc),
        "the selection's label is never hidden"
    );
    let mut markers = app
        .world_mut()
        .query_filtered::<Entity, With<MarkerHalfSize>>();
    let mut marker_rects: Vec<(Entity, Rect)> = markers
        .iter(app.world())
        .filter_map(|m| Some((m, marker_rect(&app, m)?)))
        .collect();
    // MUL-13: no label runs over a multi-marker's two frames either.
    let margin = app.world().resource::<Tunables>().bracket_margin_px;
    let mut multis = app
        .world_mut()
        .query::<(Entity, &crate::view::multi::MultiNode, &Node, &BackFrame)>();
    let multi_rects: Vec<(Entity, Rect)> = multis
        .iter(app.world())
        .filter_map(|(e, _, node, back)| {
            let (min, Val::Px(w)) = (node_min(node)?, node.width) else {
                return None;
            };
            let half = w / 2.0;
            Some((
                e,
                frames_rect(min + Vec2::splat(half), half, back.0).inflate(margin),
            ))
        })
        .collect();
    assert!(!multi_rects.is_empty(), "some multi-markers are drawn");
    marker_rects.extend(multi_rects);
    for (i, (own, _, rect)) in shown.iter().enumerate() {
        for (j, (_, _, other)) in shown.iter().enumerate() {
            assert!(i == j || !overlaps(*rect, *other), "two labels overprint");
        }
        for (marker, marker_box) in &marker_rects {
            assert!(
                marker == own || !overlaps(*rect, *marker_box),
                "a label overprints a marker"
            );
        }
    }
}

/// Where `body`'s label sits relative to its marker's node, and in which
/// slot.
pub(super) fn label_place(
    app: &mut App,
    body: Entity,
) -> (Vec2, crate::view::markers::labels::Placement) {
    use crate::view::markers::MarkerLabel;
    use crate::view::markers::labels::LabelState;
    let marker = marker_of(app, body);
    let mut labels = app
        .world_mut()
        .query_filtered::<(&ChildOf, &Node, &LabelState), With<MarkerLabel>>();
    let (node, state) = labels
        .iter(app.world())
        .find(|(child_of, ..)| child_of.parent() == marker)
        .map(|(_, node, state)| (node.clone(), *state))
        .expect("every marker has a label");
    let (Val::Px(left), Val::Px(top)) = (node.left, node.top) else {
        panic!("label not placed in pixels");
    };
    (
        Vec2::new(left, top),
        state.shown.expect("the label is shown"),
    )
}

/// LBL-01 / HOV-07: a label sits `label_gap_px` from its marker's box
/// (the box the brackets go round), whether or not the brackets show, so
/// a moon's and a task force's labels stay put when each is hovered and
/// then selected.
#[test]
fn labels_do_not_move_when_their_object_is_hovered_or_selected() {
    use crate::view::markers::labels::{label_rect, marker_box};
    use crate::view::symbols::draw::AmplifierReach;

    let mut app = symbols_app_with(|_| {});
    let halden = body(&mut app, "Halden IV");
    let ivc = body(&mut app, "Halden IVc");
    let tf1 = body(&mut app, "TF-1");
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(halden);
    run(&mut app, 60 + move_frames());
    const LABEL_SIZE: Vec2 = Vec2::new(70.0, 14.0);
    settle_labels(&mut app, LABEL_SIZE);

    let tunables = app.world().resource::<Tunables>().clone();
    for object in [ivc, tf1] {
        // Close enough that TF-1 overlaps no other symbol (MUL-05).
        zoom_in_until(&mut app, 12, |app| drawn_alone(app, object));
        settle_labels(&mut app, LABEL_SIZE);
        let before = label_place(&mut app, object);
        // The label's place is measured from the box, not the marker.
        let marker = marker_of(&mut app, object);
        let half = app.world().get::<MarkerHalfSize>(marker).unwrap().0;
        let drawn = Rect::from_corners(Vec2::ZERO, half * 2.0);
        let reach = app.world().get::<AmplifierReach>(marker).copied();
        let (bbox, _) = marker_box(drawn, (reach.as_ref(), None), tunables.bracket_margin_px);
        assert!(bbox.min.x <= -tunables.bracket_margin_px);
        let want = label_rect(bbox, LABEL_SIZE, before.1.slot, tunables.label_gap_px).min;
        assert!(
            (before.0 - want).length() < 1e-3,
            "label at {}, want {want}",
            before.0
        );

        let pointer = app.world_mut().spawn((MarkerFor(object), Hovered)).id();
        settle_labels(&mut app, LABEL_SIZE);
        assert_eq!(label_place(&mut app, object), before, "moved on hover");
        app.world_mut().entity_mut(pointer).despawn();
        app.world_mut().resource_mut::<Selection>().select(object);
        settle_labels(&mut app, LABEL_SIZE);
        assert_eq!(label_place(&mut app, object), before, "moved on select");
        app.world_mut().resource_mut::<Selection>().clear();
        settle_labels(&mut app, LABEL_SIZE);
    }
}
