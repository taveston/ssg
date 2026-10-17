//! Headless tests of the frame cue (REF-01 to REF-10) and the frame
//! violet (MRK-01, CAM-25), on Corvane with a camera that projects: where
//! each stroke starts and ends on screen, its ticks against the scale
//! bar, the vertical axis as the view tilts, and the cue's layers.

use super::*;
use crate::view::camera::rules::NORTH;
use crate::view::colors;
use crate::view::frame_cue::FrameCuePlugin;
use crate::view::frame_cue::draw::{CueNorthLabel, CuePart};
use crate::view::frame_cue::geometry::{Stroke, StrokeStyle};
use crate::view::frame_cue::stroke::CueStroke;
use crate::view::halo::TextHalo;
use crate::view::layers;
use crate::view::markers::{MarkerDrawn, MarkerShape, box_half};
use crate::view::plot_marks::ScaleBarKm;
use crate::view::screen::screen_angle;

/// Corvane in the star's frame with markers, the plot furniture and the
/// frame cue, paused.
fn cue_app() -> App {
    let mut app = camera_app_with(|app| {
        app.insert_resource(placeholder_fonts())
            .init_resource::<crate::view::fold::FoldSummary>()
            .add_plugins((
                EdgePlugin,
                MarkersPlugin,
                HoveredPlugin,
                PlotMarksPlugin,
                FrameCuePlugin,
            ));
    });
    run(&mut app, 2);
    app
}

/// Every stroke of the cue as laid out this frame, by part.
fn strokes(app: &mut App) -> std::collections::HashMap<CuePart, Option<Stroke>> {
    let mut query = app.world_mut().query::<(&CuePart, &CueStroke)>();
    query
        .iter(app.world())
        .map(|(part, look)| (*part, look.stroke.clone()))
        .collect()
}

fn stroke(app: &mut App, part: CuePart) -> Option<Stroke> {
    strokes(app).remove(&part).flatten()
}

fn north(app: &mut App) -> Stroke {
    stroke(app, CuePart::Arm(0)).expect("a north arm")
}

/// Where a world point lands on screen with the camera as placed.
fn project(app: &mut App, world: Vec3) -> Option<Vec2> {
    let mut cameras = app
        .world_mut()
        .query_filtered::<(&Camera, &Transform), With<OrbitCamera>>();
    let (camera, transform) = cameras.single(app.world()).unwrap();
    camera
        .world_to_viewport(&GlobalTransform::from(*transform), world)
        .ok()
}

/// The frame body's centre on screen and its box's half-size (HOV-07).
fn frame_body_box(app: &mut App) -> (Vec2, f32) {
    let body = frame_body(app);
    let mut markers = app.world_mut().query::<(&MarkerFor, &MarkerDrawn)>();
    let (centre, half) = markers
        .iter(app.world())
        .find(|(marker, _)| marker.0 == body)
        .and_then(|(_, drawn)| drawn.0)
        .expect("the frame body is drawn");
    (centre, box_half(half, &Tunables::default()))
}

fn set_view(app: &mut App, heading: f32, tilt: f32) {
    {
        let mut rig = app.world_mut().resource_mut::<CameraRig>();
        rig.heading = heading;
        rig.tilt = tilt;
    }
    run(app, 2);
}

/// REF-01 / REF-02 / REF-03 / MRK-01: in the star's frame, four dotted
/// arms in the frame violet leave the star, each starting half its box
/// plus the gap from its centre and running to the map area's edge,
/// below the top bar. The north arm is at full strength, fading to
/// `ref_alpha_far`; the others at `ref_minor_factor` of it.
#[test]
fn four_arms_leave_the_frame_body_clear_of_its_box_and_reach_the_map_edge() {
    let mut app = cue_app();
    let t = Tunables::default();
    let (centre, half) = frame_body_box(&mut app);
    let all = strokes(&mut app);
    let arm = |i: usize| all[&CuePart::Arm(i)].clone().expect("an arm");
    for i in 0..4 {
        let a = arm(i);
        assert!(
            (a.start.distance(centre) - (half + t.ref_gap_px)).abs() < 1.0,
            "arm {i} starts {} px out",
            a.start.distance(centre)
        );
        assert_eq!(
            a.style,
            StrokeStyle {
                dot_px: t.ref_dot_px,
                pitch_px: t.ref_dot_pitch_px,
                arrow_px: 0.0,
            }
        );
        let strength = if i == 0 { 1.0 } else { t.ref_minor_factor };
        assert_eq!(a.alpha_start, t.ref_alpha_near * strength);
        assert_eq!(a.alpha_end, t.ref_alpha_far * strength);
    }
    let end = |i: usize| arm(i).at(arm(i).length);
    assert!(
        (end(0).y - TOP_BAR_HEIGHT_PX).abs() < 0.5,
        "north: {}",
        end(0)
    );
    assert!(
        (end(1).x - VIEWPORT.x as f32).abs() < 0.5,
        "east: {}",
        end(1)
    );
    assert!(
        (end(2).y - VIEWPORT.y as f32).abs() < 0.5,
        "south: {}",
        end(2)
    );
    assert!(end(3).x.abs() < 0.5, "west: {}", end(3));
    assert!(arm(0).dir.y < -0.99, "north is up at the standard heading");

    let mut looks = app.world_mut().query::<&CueStroke>();
    assert!(
        looks
            .iter(app.world())
            .all(|look| look.color == colors::REFERENCE)
    );
}

/// REF-01 / REF-04: the arms turn with the view: the north arm points
/// the way the north dial does, at any heading and tilt, and the "N" sits
/// `ref_n_inset_px` inside its end, in the frame violet with the label
/// halo.
#[test]
fn the_north_arm_points_with_the_dial_and_carries_the_n() {
    let mut app = cue_app();
    let t = Tunables::default();
    for (heading, tilt) in [(0.5, 0.0), (-2.0, 0.7), (2.8, 1.2)] {
        set_view(&mut app, heading, tilt);
        let arm = north(&mut app);
        let mut arrows = app
            .world_mut()
            .query_filtered::<&UiTransform, With<NorthArrow>>();
        let dial = arrows.single(app.world()).unwrap().rotation.as_radians();
        let angle = screen_angle(Vec2::ZERO, arm.dir);
        let turn = (angle - dial + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
            - std::f32::consts::PI;
        assert!(turn.abs() < 0.01, "arm {angle}, dial {dial}");

        let mut labels = app
            .world_mut()
            .query_filtered::<(&Node, &Children), With<CueNorthLabel>>();
        let (node, children) = labels.single(app.world()).unwrap();
        let (Val::Px(left), Val::Px(top), Val::Px(width), Val::Px(height)) =
            (node.left, node.top, node.width, node.height)
        else {
            panic!("the N is placed in pixels");
        };
        let at = Vec2::new(left + width / 2.0, top + height / 2.0);
        let wanted = arm.at(arm.length - t.ref_n_inset_px);
        assert!(at.distance(wanted) < 0.01, "N at {at}, wanted {wanted}");
        let text = children[0];
        assert_eq!(app.world().get::<Text>(text).unwrap().0, "N");
        assert_eq!(
            app.world().get::<TextColor>(text).unwrap().0,
            colors::REFERENCE
        );
        assert!(app.world().get::<TextHalo>(text).is_some(), "LBL-05 halo");
    }
}

/// REF-05: ticks sit at multiples of the scale bar's distance, measured
/// along the plane: across the screen at the standard heading they are
/// the scale bar's length apart; along north, seen at a tilt, they crowd
/// closer. None falls inside an arm's gap.
#[test]
fn ticks_fall_at_multiples_of_the_scale_bar_and_crowd_with_tilt() {
    let mut app = cue_app();
    let (centre, half) = frame_body_box(&mut app);
    let gap = Tunables::default().ref_gap_px;
    let mut lines = app
        .world_mut()
        .query_filtered::<&Node, With<ScaleBarLine>>();
    let Val::Px(bar_px) = lines.single(app.world()).unwrap().width else {
        panic!("the scale bar has a pixel width");
    };
    assert!(app.world().resource::<ScaleBarKm>().0 > 0.0);
    let east = stroke(&mut app, CuePart::Arm(1)).unwrap();
    let from_centre: Vec<f32> = east
        .ticks
        .iter()
        .map(|&at| east.at(at).distance(centre))
        .collect();
    assert!(from_centre.len() >= 3, "{from_centre:?}");
    // The bar's drawn length is rounded to a whole pixel.
    assert!((from_centre[0] - bar_px).abs() < 1.0, "{from_centre:?}");
    for (k, at) in from_centre.iter().enumerate() {
        let wanted = from_centre[0] * (k + 1) as f32;
        assert!(
            (at - wanted).abs() < 0.5,
            "tick {k} at {at}, wanted {wanted}"
        );
    }
    let north = north(&mut app);
    let first_north = north.at(north.ticks[0]).distance(centre);
    assert!(first_north < from_centre[0] * 0.95, "foreshortened");
    for arm in strokes(&mut app).values().flatten() {
        for &at in &arm.ticks {
            assert!(arm.at(at).distance(centre) >= half + gap - 0.5);
            assert!(at <= arm.length);
        }
    }
}

/// REF-03: near edge-on, the north arm runs away from the camera and ends
/// where the plane's horizon crosses the view, below the map's top edge;
/// its ticks crowd towards the horizon and stop short of it.
#[test]
fn the_north_arm_ends_at_the_horizon_seen_near_edge_on() {
    let mut app = cue_app();
    set_view(&mut app, 0.0, 85f32.to_radians());
    let arm = north(&mut app);
    let eye = {
        let mut cameras = app
            .world_mut()
            .query_filtered::<&Transform, With<OrbitCamera>>();
        cameras.single(app.world()).unwrap().translation
    };
    let horizon = project(&mut app, eye + NORTH * 10.0).unwrap();
    let end = arm.at(arm.length);
    assert!(end.y > TOP_BAR_HEIGHT_PX + 10.0, "stops short of the top");
    assert!(end.distance(horizon) < 1.0, "end {end}, horizon {horizon}");
    let gaps: Vec<f32> = arm.ticks.windows(2).map(|w| w[1] - w[0]).collect();
    assert!(gaps.windows(2).all(|g| g[1] <= g[0] + 0.01), "{gaps:?}");
}

/// REF-06 / REF-07 / REF-08: the vertical axis runs up the screen from
/// the gap beyond the star's box, dotted as the arms are; edge-on it is
/// `ref_up_fraction` of the map area's height, shortened by the tilt, and
/// it is nothing seen straight down. The lower half starts at the centre,
/// runs down for `ref_down_fraction` of the upper's length, dotted and
/// without an arrowhead.
#[test]
fn the_vertical_axis_shortens_with_tilt_and_vanishes_straight_down() {
    let mut app = cue_app();
    let t = Tunables::default();
    let (centre, half) = frame_body_box(&mut app);
    set_view(&mut app, 0.0, 0.0);
    assert!(stroke(&mut app, CuePart::Upper).is_none());
    assert!(stroke(&mut app, CuePart::Lower).is_none());
    for degrees in [30.0_f32, 60.0, 85.0] {
        set_view(&mut app, 0.3, degrees.to_radians());
        let upper = stroke(&mut app, CuePart::Upper).expect("an upper half");
        let lower = stroke(&mut app, CuePart::Lower).expect("a lower half");
        let map_height = VIEWPORT.y as f32 - TOP_BAR_HEIGHT_PX;
        let full = t.ref_up_fraction * map_height * degrees.to_radians().sin();
        // Near edge-on the axis would pass the map's top edge, where it
        // is clipped as the arms are.
        let room = (upper.start.y - TOP_BAR_HEIGHT_PX) / -upper.dir.y;
        let wanted = full.min(room);
        assert!(
            (upper.length - wanted).abs() < 0.5,
            "at {degrees}°: {} px, wanted {wanted}",
            upper.length
        );
        assert!((upper.start.distance(centre) - (half + t.ref_gap_px)).abs() < 1.0);
        assert!(upper.dir.y < -0.99, "up the screen");
        let dotted = |arrow_px| StrokeStyle {
            dot_px: t.ref_dot_px,
            pitch_px: t.ref_dot_pitch_px,
            arrow_px,
        };
        assert_eq!(upper.style, dotted(t.ref_arrow_px));
        assert!(lower.start.distance(centre) < 1.0);
        assert!(lower.dir.y > 0.99, "down the screen");
        assert!((lower.length - full * t.ref_down_fraction).abs() < 0.5);
        assert_eq!(lower.style, dotted(0.0));
        assert_eq!(
            (upper.alpha_start, upper.alpha_end),
            (t.ref_alpha_near, t.ref_alpha_far)
        );
    }
    // REF-08: just off straight down, the halves are too short to draw.
    set_view(&mut app, 0.0, 0.02);
    assert!(stroke(&mut app, CuePart::Upper).is_none());
}

/// REF-06 / REF-07: close in on Halden IV, drawn as a true-size disc, the
/// upper half starts at the pole (its true radius above the centre) and
/// the lower half at the centre.
#[test]
fn over_a_true_size_disc_the_vertical_axis_starts_at_the_pole() {
    let mut app = cue_app();
    let halden = body(&mut app, "Halden IV");
    set_frame(&mut app, halden, Vec3::ZERO);
    app.world_mut().resource_mut::<CameraRig>().distance = 0.03;
    set_view(&mut app, 0.0, 70f32.to_radians());
    let (_, half) = frame_body_box(&mut app);
    assert!(half > Tunables::default().planet_marker_px, "a disc");
    let position = app.world().resource::<WorldPositions>().0[&halden];
    let radius = km_to_render(app.world().get::<PhysicalRadius>(halden).unwrap().0);
    let centre = project(&mut app, position).unwrap();
    let pole = project(&mut app, position + Vec3::Y * radius).unwrap();
    let upper = stroke(&mut app, CuePart::Upper).unwrap();
    let lower = stroke(&mut app, CuePart::Lower).unwrap();
    assert!(
        upper.start.distance(pole) < 0.5,
        "{} vs {pole}",
        upper.start
    );
    assert!(lower.start.distance(centre) < 0.5);
    // The arms still start clear of the disc's box (REF-02).
    let arm = stroke(&mut app, CuePart::Arm(1)).unwrap();
    assert!(arm.start.distance(centre) >= half);
}

/// REF-01: the cue holds still as the frame body does: with the clock
/// running in Halden IV's frame, the arms stay put on screen.
#[test]
fn the_cue_holds_still_with_the_frame_body() {
    let mut app = cue_app();
    let halden = body(&mut app, "Halden IV");
    set_frame(&mut app, halden, Vec3::ZERO);
    run(&mut app, 3);
    let before = strokes(&mut app);
    app.world_mut().resource_mut::<GameClock>().toggle_pause();
    run(&mut app, 30);
    let after = strokes(&mut app);
    for i in 0..4 {
        let (a, b) = (
            before[&CuePart::Arm(i)].clone().unwrap(),
            after[&CuePart::Arm(i)].clone().unwrap(),
        );
        assert!(a.start.distance(b.start) < 0.5 && a.dir.distance(b.dir) < 1e-3);
    }
}

/// REF-09 / MRK-12 / REF-10: the arms, ticks, lower half and "N" lie
/// under every natural body; the upper half lies over every body and
/// under the symbols. No part of the cue is a pick target.
#[test]
fn the_cue_layers_round_the_bodies_and_is_never_picked() {
    let mut app = cue_app();
    let mut markers = app
        .world_mut()
        .query::<(&MarkerShape, &GlobalZIndex, &MarkerFor)>();
    let classes: Vec<i32> = markers
        .iter(app.world())
        .filter(|(shape, ..)| **shape == MarkerShape::Disc)
        .map(|(_, z, _)| z.0)
        .collect();
    let (lowest_body, highest_body) = (
        *classes.iter().min().unwrap(),
        *classes.iter().max().unwrap(),
    );
    let mut parts = app
        .world_mut()
        .query::<(&CuePart, &GlobalZIndex, &Pickable)>();
    for (part, z, pickable) in parts.iter(app.world()) {
        match part {
            CuePart::Upper => {
                assert!(z.0 > highest_body && z.0 < layers::SYMBOLS, "{z:?}");
            }
            _ => assert!(z.0 < lowest_body, "{part:?} at {z:?}"),
        }
        assert_eq!(*pickable, Pickable::IGNORE);
    }
    let mut labels = app
        .world_mut()
        .query_filtered::<(&GlobalZIndex, &Pickable, &Children), With<CueNorthLabel>>();
    let (z, pickable, children) = labels.single(app.world()).unwrap();
    assert!(z.0 < lowest_body);
    assert_eq!(*pickable, Pickable::IGNORE);
    let text = children[0];
    assert_eq!(app.world().get::<Pickable>(text), Some(&Pickable::IGNORE));
}

/// CAM-25 / MRK-01: the breadcrumb entry for the current frame is in the
/// frame violet, the system's in the star's frame and the frame body's
/// otherwise; the ancestors are neutral and the sector secondary.
#[test]
fn the_current_frame_entry_of_the_breadcrumb_is_violet() {
    let mut app = camera_app_with(|app| {
        app.insert_resource(placeholder_fonts())
            .add_plugins(BreadcrumbPlugin);
    });
    app.world_mut().spawn((BreadcrumbRoot, Node::default()));
    run(&mut app, 2);
    let colours = |app: &mut App| -> Vec<Color> {
        breadcrumb_entries(app)
            .into_iter()
            .map(|(entry, _)| app.world().get::<TextColor>(entry).unwrap().0)
            .collect()
    };
    assert_eq!(
        colours(&mut app),
        [colors::PANEL_BORDER, colors::REFERENCE],
        "Sector / Corvane"
    );
    let highport = body(&mut app, "Arden Highport");
    set_frame(&mut app, highport, Vec3::ZERO);
    run(&mut app, 2);
    assert_eq!(
        colours(&mut app),
        [
            colors::PANEL_BORDER,
            colors::NEUTRAL,
            colors::NEUTRAL,
            colors::REFERENCE
        ]
    );
}

/// REF-06: seen at 75° with the frame body high on the map, the vertical
/// axis's upper half is clipped at the map area's edge, as the arms are,
/// so its arrowhead ends on that edge and never runs under the top bar.
#[test]
fn the_vertical_axis_stops_at_the_map_edge_when_the_frame_body_is_high() {
    let mut app = cue_app();
    let t = Tunables::default();
    let star = frame_body(&app);
    set_view(&mut app, 0.0, 75f32.to_radians());
    let reach = app.world().resource::<CameraRig>().distance;
    // Move the pivot south of the star until the star sits under the top
    // bar by less than half the axis's length.
    let wanted_px = t.ref_up_fraction * (VIEWPORT.y as f32 - TOP_BAR_HEIGHT_PX);
    let mut placed = None;
    for step in 1..200 {
        set_frame(&mut app, star, -NORTH * reach * 0.02 * step as f32);
        run(&mut app, 2);
        let (centre, _) = frame_body_box(&mut app);
        if centre.y < TOP_BAR_HEIGHT_PX + wanted_px / 2.0 {
            placed = Some(centre);
            break;
        }
    }
    let centre = placed.expect("the star rises to just under the top bar");
    assert!(centre.y > TOP_BAR_HEIGHT_PX, "the star is still on the map");
    let upper = stroke(&mut app, CuePart::Upper).expect("an upper half");
    let end = upper.at(upper.length);
    assert!(
        (end.y - TOP_BAR_HEIGHT_PX).abs() < 0.5,
        "the arrowhead ends at {end}, the map's edge is at {TOP_BAR_HEIGHT_PX}"
    );
    assert_eq!(upper.style.arrow_px, t.ref_arrow_px);
}
