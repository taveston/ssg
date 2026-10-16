//! Orbit rings (MRK-05), their tails and exposure rings (MOT-01, MOT-02,
//! MOT-11), and their hover and selection highlight (HOV-03).
//!
//! One stroke per ring (`shared_orbit.rs`): a planet's or unfolded moon's
//! own orbit (GRP-01), or a named orbit with the ships and stations in it
//! (HOV-04), drawn once however many sit there. Ships and stations draw
//! only their tails in dot mode, fading to nothing at their ends, and the
//! full ring only while hovered or selected or as an exposure ring
//! (MRK-09). Rings are drawn finer where the camera is close so they stay
//! true to the circle at any zoom (SCL-02). Each ring is one stroke whose
//! brightness varies along it: the tail behind each body on it runs from
//! the exposure brightness at the dot down to the ring's normal brightness,
//! and an exposure ring is bright all round. Each ring is its own retained
//! gizmo, so it can widen on its own as its tail closes (MOT-11). A hovered
//! ring is white at `orbit_hover_alpha`, a selected body's white at
//! `orbit_selected_alpha` (HOV-03). A ring too large on screen to read as
//! a ring fades out and then isn't drawn, in every frame (MRK-14).

use bevy::color::Mix;
use bevy::prelude::*;

use crate::sim::body::BodyClass;
use crate::view::camera::state::camera_fov;
use crate::view::colors;
use crate::view::hovered::HoveredBodies;
use crate::view::motion_mode::{MotionModeState, degrees_behind, tail_brightness};
use crate::view::scale::km_to_render;
use crate::view::screen::{MapArea, ScreenPositions};
use crate::view::selection::Selection;
use crate::view::shared_orbit::OrbitRings;
use crate::view::tunables::Tunables;
use crate::view::world::{RenderOrigin, WorldPositions};
use crate::{Boot, Stage};

/// How many straight chords a ring starts as, enough for any ring seen
/// from a distance.
const RING_SEGMENTS: usize = 128;
/// How far a drawn chord may stray from the true circle, in pixels.
const MAX_RING_ERROR_PX: f32 = 0.5;
/// How many times a chord may be halved near the camera: 2^12 pieces is
/// far more than any zoom needs, and bounds the work if the camera sits
/// right on a ring.
const MAX_RING_SPLITS: u32 = 12;
/// A point must be at least this far in front of the camera to be drawn.
const NEAR_EPSILON: f32 = 0.01;

/// One straight piece of a drawn ring, in world space, with the colour at
/// each end.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RingLine {
    pub from: Vec3,
    pub to: Vec3,
    pub from_color: Color,
    pub to_color: Color,
}

/// A ring as drawn this frame: the body at its centre (`None` for the
/// star), its width in physical pixels and its lines.
#[derive(Clone, Debug, Default)]
pub struct DrawnRing {
    pub centre: Option<Entity>,
    pub width: f32,
    pub lines: Vec<RingLine>,
}

/// Every ring drawn this frame, by owner, so `near_arcs.rs` can redraw a
/// close orbit's near side over its body (MRK-12).
#[derive(Resource, Default)]
pub struct DrawnRings(pub std::collections::HashMap<Entity, DrawnRing>);

/// The retained gizmo that draws one ring, for the body or named orbit it
/// stands for.
#[derive(Component)]
pub struct RingStrokeFor(pub Entity);

pub struct RingsPlugin;

impl Plugin for RingsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DrawnRings>()
            .add_systems(Startup, spawn_ring_strokes.in_set(Boot::Overlay))
            // Drawn with the rest of the overlay, once grouping and motion
            // have decided what shows.
            .add_systems(Update, draw_orbit_rings.in_set(Stage::Overlay));
    }
}

/// HOV-04: one retained gizmo per ring: a natural body's own, or a named
/// orbit's, shared by everything in it.
fn spawn_ring_strokes(
    mut commands: Commands,
    mut assets: ResMut<Assets<GizmoAsset>>,
    rings: OrbitRings,
) {
    for owner in rings.owners() {
        commands.spawn((
            RingStrokeFor(owner),
            Gizmo {
                handle: assets.add(GizmoAsset::new()),
                line_config: GizmoLineConfig {
                    width: Tunables::default().orbit_ring_width_px,
                    ..default()
                },
                ..default()
            },
        ));
    }
}

/// How much of a body's orbit is drawn this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RingExtent {
    /// Nothing at all.
    Hidden,
    /// Only the tail behind the dot, fading to nothing at its end.
    Tail,
    /// The whole ring.
    Full,
}

/// MRK-05 / MRK-09: whether a body ever draws anything on its orbit. Gates
/// and salvage sites sit on star orbits that would only add clutter (gates
/// have their edge arrows instead).
pub fn has_orbit_line(class: BodyClass) -> bool {
    matches!(
        class,
        BodyClass::Planet | BodyClass::Moon | BodyClass::Ship | BodyClass::Station
    )
}

/// MRK-05 / MRK-09 / HOV-02: how much of a body's orbit is drawn. Planets
/// and moons draw their full ring. Ships and stations draw only their tail
/// in dot mode, and the full ring while hovered or selected or once the
/// tail has closed into the exposure ring. A folded satellite draws
/// nothing: no marker, label or ring (GRP-01).
pub fn ring_extent(
    class: BodyClass,
    folded_away: bool,
    ring_mode: bool,
    highlighted: bool,
) -> RingExtent {
    if folded_away || !has_orbit_line(class) {
        return RingExtent::Hidden;
    }
    let natural = matches!(class, BodyClass::Planet | BodyClass::Moon);
    if natural || ring_mode || highlighted {
        RingExtent::Full
    } else {
        RingExtent::Tail
    }
}

/// HOV-03: the colour a ring is drawn in instead of its normal one: white
/// at `selected_alpha` while its object is selected (selected wins), white
/// at `hover_alpha` while hovered, otherwise none.
pub fn ring_highlight(
    selected: bool,
    hovered: bool,
    selected_alpha: f32,
    hover_alpha: f32,
) -> Option<Color> {
    let white = colors::SELECTION;
    if selected {
        Some(white.with_alpha(selected_alpha))
    } else if hovered {
        Some(white.with_alpha(hover_alpha))
    } else {
        None
    }
}

/// MRK-14: how large a ring of `radius` round `center` looks on screen, in
/// pixels: its radius at the scale of its point nearest the camera. A ring
/// the camera sits close to counts as huge, though its far side is small,
/// since near the camera it crosses the view as a near-straight line.
pub fn ring_screen_radius_px(center: Vec3, radius: f32, camera_pos: Vec3, px_per_rad: f32) -> f32 {
    let from_center = camera_pos - center;
    let across = Vec2::new(from_center.x, from_center.z).length();
    let nearest = Vec2::new(across - radius, from_center.y)
        .length()
        .max(NEAR_EPSILON);
    radius * px_per_rad / nearest
}

/// MRK-14: what a ring's alpha is multiplied by, for an on-screen radius
/// `ratio` times half the map area's diagonal: 1 up to `from`, falling
/// evenly to 0 at `to`, and 0 beyond.
pub fn orbit_fade(ratio: f32, from: f32, to: f32) -> f32 {
    if to <= from {
        return if ratio < from { 1.0 } else { 0.0 };
    }
    1.0 - ((ratio - from) / (to - from)).clamp(0.0, 1.0)
}

/// What a ring's size fade is judged against this frame (MRK-14).
#[derive(Clone, Copy, Debug)]
pub struct FadeView {
    pub camera_pos: Vec3,
    pub px_per_rad: f32,
    pub half_diagonal_px: f32,
    pub from: f32,
    pub to: f32,
}

impl FadeView {
    /// The fade for this frame's camera and map area.
    pub fn new(
        (camera, transform, projection): (&Camera, &Transform, &Projection),
        screen: &ScreenPositions,
        tunables: &Tunables,
    ) -> Option<Self> {
        Some(Self {
            camera_pos: transform.translation,
            px_per_rad: px_per_rad(camera, projection)?,
            half_diagonal_px: MapArea::of(screen.viewport).size.length() / 2.0,
            from: tunables.orbit_fade_from,
            to: tunables.orbit_fade_to,
        })
    }

    /// MRK-14: the alpha factor for a ring of `radius` round `center`. 0
    /// means the ring isn't drawn, and so is no target (HOV-02).
    pub fn of(&self, center: Vec3, radius: f32) -> f32 {
        if self.half_diagonal_px <= 0.0 {
            return 1.0;
        }
        let screen = ring_screen_radius_px(center, radius, self.camera_pos, self.px_per_rad);
        orbit_fade(screen / self.half_diagonal_px, self.from, self.to)
    }
}

/// How many pixels one radian of view covers, from the viewport's height
/// and the vertical field of view; `None` before there is a viewport.
pub fn px_per_rad(camera: &Camera, projection: &Projection) -> Option<f32> {
    let size = camera.logical_viewport_size()?;
    Some((size.y / 2.0) / (camera_fov(projection) / 2.0).tan())
}

/// The two ends of a ring's brightness: its normal colour and the
/// exposure brightness its tail starts from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RingColors {
    pub normal: Color,
    pub exposure: Color,
}

/// MRK-05 / MOT-01 / MOT-02 / HOV-03: a ring's normal colour is faint, or
/// the highlight; its exposure colour is the neutral at `exposure_alpha`,
/// so it never matches the white selected ring, or white while
/// highlighted.
pub fn ring_colors(highlight: Option<Color>, ring_alpha: f32, exposure_alpha: f32) -> RingColors {
    RingColors {
        normal: highlight.unwrap_or(colors::ORBIT_LINE.with_alpha(ring_alpha)),
        exposure: if highlight.is_some() {
            colors::SELECTION
        } else {
            colors::NEUTRAL.with_alpha(exposure_alpha)
        },
    }
}

/// MRK-05 / HOV-03: a ring's width before its tail closes: the highlight
/// width while hovered or selected, else the normal width.
pub fn ring_base_width(highlighted: bool, tunables: &Tunables) -> f32 {
    if highlighted {
        tunables.orbit_highlight_width_px
    } else {
        tunables.orbit_ring_width_px
    }
}

/// MOT-01: a point's colour, `brightness` of the way from the normal
/// colour to the exposure colour.
pub fn ring_color_at(colors: RingColors, brightness: f32) -> Color {
    colors
        .normal
        .mix(&colors.exposure, brightness.clamp(0.0, 1.0))
}

/// What one ring stroke needs this frame, if its ring shows.
struct StrokeLook {
    center: Vec3,
    /// The body the ring is centred on; `None` for the star.
    centre_body: Option<Entity>,
    radius: f32,
    /// Where each drawn body sits on the ring: each carries a tail.
    body_angles: Vec<f32>,
    mode: MotionModeState,
    colors: RingColors,
    base_width: f32,
}

impl RingColors {
    /// MRK-14: both ends of the brightness with their alpha multiplied by
    /// `factor`.
    fn faded(self, factor: f32) -> Self {
        let fade = |c: Color| c.with_alpha(c.alpha() * factor);
        Self {
            normal: fade(self.normal),
            exposure: fade(self.exposure),
        }
    }
}

/// MRK-05 / MOT-01 / MOT-02 / MOT-11 / HOV-03 / HOV-04 / MRK-14: refill
/// every ring stroke: the ring as the camera sees it, bright along each
/// body's tail (or all round in ring mode), as wide as its closing makes
/// it, and faded by its size on screen.
#[allow(clippy::too_many_arguments)]
pub fn draw_orbit_rings(
    screen: Res<ScreenPositions>,
    selection: Res<Selection>,
    hovered: Res<HoveredBodies>,
    tunables: Res<Tunables>,
    camera_q: Query<(&Camera, &Transform, &Projection), With<Camera3d>>,
    rings: OrbitRings,
    positions: Res<WorldPositions>,
    origin: Res<RenderOrigin>,
    mut strokes: Query<(&RingStrokeFor, &mut Gizmo)>,
    mut assets: ResMut<Assets<GizmoAsset>>,
    mut drawn: ResMut<DrawnRings>,
) {
    drawn.0.clear();
    // This frame's camera and positions (CAM-05), so rings and markers
    // agree.
    let Ok((camera, camera_transform, projection)) = camera_q.single() else {
        return;
    };
    let view = RingView {
        camera_pos: camera_transform.translation,
        camera_forward: camera_transform.forward().as_vec3(),
        max_error_rad: max_error_rad(camera, projection),
    };
    let fade = FadeView::new((camera, camera_transform, projection), &screen, &tunables);
    for (stroke_for, mut gizmo) in &mut strokes {
        let Some(mut asset) = assets.get_mut(&gizmo.handle) else {
            continue;
        };
        asset.clear();
        let Some(mut look) = stroke_look(
            stroke_for.0,
            &rings,
            (&positions, origin.star),
            (&selection, &hovered, &tunables),
        ) else {
            continue;
        };
        let factor = fade.map_or(1.0, |fade| fade.of(look.center, look.radius));
        if factor <= 0.0 {
            continue;
        }
        look.colors = look.colors.faded(factor);
        let segments = ring_segments(look.center, look.radius, view);
        let width = look
            .mode
            .ring_width(look.base_width, tunables.exposure_ring_width_px);
        let mut ring = DrawnRing {
            centre: look.centre_body,
            width,
            lines: Vec::new(),
        };
        for segment in segments
            .into_iter()
            .flat_map(|segment| split_at_angles(segment, &look.body_angles))
        {
            let (from, to) = ends_brightness(&segment, &look.body_angles, &look.mode);
            let line = RingLine {
                from: segment.from,
                to: segment.to,
                from_color: ring_color_at(look.colors, from),
                to_color: ring_color_at(look.colors, to),
            };
            asset.line_gradient(line.from, line.to, line.from_color, line.to_color);
            ring.lines.push(line);
        }
        drawn.0.insert(stroke_for.0, ring);
        if gizmo.line_config.width != width {
            gizmo.line_config.width = width;
        }
    }
}

/// Where a ring is, where the bodies on it are, and how it is coloured;
/// `None` if the ring isn't drawn.
fn stroke_look(
    owner: Entity,
    rings: &OrbitRings,
    (positions, star): (&WorldPositions, Vec3),
    (selection, hovered, tunables): (&Selection, &HoveredBodies, &Tunables),
) -> Option<StrokeLook> {
    let ring = rings.ring(owner)?;
    let body_angles: Vec<f32> = ring.drawn().map(|m| m.angle).collect();
    let mode = ring.mode();
    let (selected, is_hovered) = ring.highlight(selection, hovered);
    let highlight = ring_highlight(
        selected,
        is_hovered,
        tunables.orbit_selected_alpha,
        tunables.orbit_hover_alpha,
    );
    let extent = ring_extent(
        ring.class(),
        body_angles.is_empty(),
        mode.ring_mode,
        highlight.is_some(),
    );
    let mut colors = ring_colors(
        highlight,
        tunables.orbit_ring_alpha,
        tunables.exposure_ring_alpha,
    );
    match extent {
        RingExtent::Hidden => return None,
        // MRK-09: past the tail's end there is no ring to fade into.
        RingExtent::Tail => colors.normal = colors.normal.with_alpha(0.0),
        RingExtent::Full => {}
    }
    let center = ring
        .centre
        .and_then(|parent| positions.0.get(&parent).copied())
        .unwrap_or(star);
    Some(StrokeLook {
        center,
        centre_body: ring.centre,
        radius: km_to_render(ring.radius_km),
        body_angles,
        mode,
        colors,
        base_width: ring_base_width(highlight.is_some(), tunables),
    })
}

/// `MAX_RING_ERROR_PX` as an angle seen from the camera. Without a viewport
/// yet there is nothing to be accurate to, and rings stay as plain chords.
fn max_error_rad(camera: &Camera, projection: &Projection) -> f32 {
    px_per_rad(camera, projection).map_or(f32::INFINITY, |px| MAX_RING_ERROR_PX / px)
}

/// How the camera sees a ring: where it is, which way it looks, and how
/// far a drawn chord may stray from the true circle.
#[derive(Clone, Copy, Debug)]
pub struct RingView {
    pub camera_pos: Vec3,
    pub camera_forward: Vec3,
    /// The largest allowed gap between a chord and the circle, as an angle
    /// seen from the camera (half a pixel, converted).
    pub max_error_rad: f32,
}

/// The point at `angle` on a ring in the orbital (XZ) plane.
fn ring_point(center: Vec3, radius: f32, angle: f32) -> Vec3 {
    center + Vec3::new(radius * angle.cos(), 0.0, radius * angle.sin())
}

/// MOT-01: a segment split where the body sits on it, so the tail's
/// brightness never bleeds ahead of the body. A segment the body isn't on
/// comes back whole.
pub fn split_at_angle(segment: RingSegment, angle: f32) -> Vec<RingSegment> {
    use std::f32::consts::TAU;
    let span = (segment.to_angle - segment.from_angle).rem_euclid(TAU);
    let into = (angle - segment.from_angle).rem_euclid(TAU);
    if span <= 0.0 || into <= 0.0 || into >= span {
        return vec![segment];
    }
    let cut = segment.from.lerp(segment.to, into / span);
    vec![
        RingSegment {
            to: cut,
            to_angle: angle,
            ..segment
        },
        RingSegment {
            from: cut,
            from_angle: angle,
            ..segment
        },
    ]
}

/// HOV-04 / MOT-01: a segment split at every body on the ring, so no
/// body's tail bleeds ahead of it.
pub fn split_at_angles(segment: RingSegment, angles: &[f32]) -> Vec<RingSegment> {
    angles.iter().fold(vec![segment], |pieces, &angle| {
        pieces
            .into_iter()
            .flat_map(|piece| split_at_angle(piece, angle))
            .collect()
    })
}

/// HOV-04 / MOT-01: the brightness at each end of a segment on a ring with
/// bodies at `angles`: each body lays its own tail along the ring, and a
/// point takes the brightest. With no body on the ring there is no tail.
pub fn ends_brightness(
    segment: &RingSegment,
    angles: &[f32],
    mode: &MotionModeState,
) -> (f32, f32) {
    angles
        .iter()
        .map(|&angle| {
            let (from, to) = segment_behind(segment, angle);
            (tail_brightness(from, mode), tail_brightness(to, mode))
        })
        .fold((0.0, 0.0), |(a, b), (from, to)| (a.max(from), b.max(to)))
}

/// MOT-01: how far behind the body each end of a segment is, in degrees.
/// Segments run in the direction of motion, so the start is the segment's
/// own span further behind than its end. Measuring from the end keeps a
/// piece that starts exactly at the body (split there) a full turn behind,
/// not zero, so it is never shaded as tail.
pub fn segment_behind(segment: &RingSegment, body_angle: f32) -> (f32, f32) {
    let span = (segment.to_angle - segment.from_angle)
        .rem_euclid(std::f32::consts::TAU)
        .to_degrees();
    let to = degrees_behind(body_angle, segment.to_angle);
    (to + span, to)
}

/// One straight piece of a drawn ring, with the orbit angle at each end so
/// the tail's brightness can be laid along it (MOT-01).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RingSegment {
    pub from: Vec3,
    pub to: Vec3,
    pub from_angle: f32,
    pub to_angle: f32,
}

/// MRK-05 / SCL-02: the straight segments that draw a ring as the
/// camera sees it.
///
/// A ring starts as `RING_SEGMENTS` chords, which is plenty from afar. A
/// chord the camera is close to is split until it strays from the circle by
/// no more than `max_error_rad` as seen from the camera: zoomed in on a
/// planet, the camera sits about one render unit from a planetary orbit
/// 180 units in radius, where a fixed chord would miss the planet by tens of
/// pixels. Chords crossing the camera's near limit are cut there rather than
/// dropped, so the part of the ring in front of the camera always shows.
pub fn ring_segments(center: Vec3, radius: f32, view: RingView) -> Vec<RingSegment> {
    let mut segments = Vec::new();
    let step = std::f32::consts::TAU / RING_SEGMENTS as f32;
    for i in 0..RING_SEGMENTS {
        let from = i as f32 * step;
        split_arc(center, radius, (from, from + step), view, 0, &mut segments);
    }
    segments
}

fn split_arc(
    center: Vec3,
    radius: f32,
    (from, to): (f32, f32),
    view: RingView,
    depth: u32,
    out: &mut Vec<RingSegment>,
) {
    let mid = (from + to) / 2.0;
    // The chord's largest gap from the arc, at its middle: r(1 - cos(a/2))
    // for an arc of angle a, written so it stays accurate for tiny angles.
    let gap = 2.0 * radius * ((to - from) / 4.0).sin().powi(2);
    let distance = (ring_point(center, radius, mid) - view.camera_pos)
        .length()
        .max(NEAR_EPSILON);
    if gap / distance > view.max_error_rad && depth < MAX_RING_SPLITS {
        split_arc(center, radius, (from, mid), view, depth + 1, out);
        split_arc(center, radius, (mid, to), view, depth + 1, out);
        return;
    }
    let chord = (
        ring_point(center, radius, from),
        ring_point(center, radius, to),
    );
    // A cut end lies on the chord, not the circle; its orbit angle is read
    // back from where it is, which is close enough to shade it.
    let angle_of = |p: Vec3| (p.z - center.z).atan2(p.x - center.x);
    out.extend(
        clip_to_front(chord, view).map(|(cut_from, cut_to)| RingSegment {
            from: cut_from,
            to: cut_to,
            from_angle: if cut_from == chord.0 {
                from
            } else {
                angle_of(cut_from)
            },
            to_angle: if cut_to == chord.1 {
                to
            } else {
                angle_of(cut_to)
            },
        }),
    );
}

/// The part of a segment in front of the camera's near limit, if any.
pub fn clip_to_front((from, to): (Vec3, Vec3), view: RingView) -> Option<(Vec3, Vec3)> {
    let depth = |p: Vec3| (p - view.camera_pos).dot(view.camera_forward);
    let (d_from, d_to) = (depth(from), depth(to));
    match (d_from > NEAR_EPSILON, d_to > NEAR_EPSILON) {
        (true, true) => Some((from, to)),
        (false, false) => None,
        (from_in_front, _) => {
            let t = (NEAR_EPSILON - d_from) / (d_to - d_from);
            let cut = from + (to - from) * t;
            Some(if from_in_front {
                (from, cut)
            } else {
                (cut, to)
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// MRK-14: with the default limits (4 and 12), a ring is whole up to
    /// `orbit_fade_from` times half the diagonal, fades evenly to nothing at
    /// `orbit_fade_to`, and stays gone beyond it.
    #[test]
    fn large_rings_fade_out_between_the_two_limits() {
        let tunables = crate::view::tunables::Tunables::default();
        let (from, to) = (tunables.orbit_fade_from, tunables.orbit_fade_to);
        assert_eq!((from, to), (4.0, 12.0));
        assert_eq!(orbit_fade(0.2, from, to), 1.0);
        assert_eq!(orbit_fade(3.0, from, to), 1.0);
        assert_eq!(orbit_fade(4.0, from, to), 1.0);
        assert!((orbit_fade(8.0, from, to) - 0.5).abs() < 1e-6);
        assert_eq!(orbit_fade(12.0, from, to), 0.0);
        assert_eq!(orbit_fade(40.0, from, to), 0.0);
    }

    /// MRK-14: seen from straight above, a ring's screen radius is its
    /// angular radius; with the camera right by the ring, as in a planet's
    /// frame looking at the planet's own orbit, it counts as huge.
    #[test]
    fn a_ring_beside_the_camera_counts_as_huge() {
        let above = ring_screen_radius_px(Vec3::ZERO, 10.0, Vec3::Y * 1000.0, 1000.0);
        assert!((above - 10.0).abs() < 0.01, "{above}");
        let planet = Vec3::new(180.0, 0.0, 0.0);
        let beside = ring_screen_radius_px(Vec3::ZERO, 180.0, planet + Vec3::Y, 1000.0);
        assert!(beside > 100_000.0, "{beside}");
    }

    /// MRK-14: the fade multiplies both ends of a ring's brightness, so a
    /// hovered or selected ring fades too.
    #[test]
    fn the_fade_applies_to_highlighted_rings() {
        let hovered = ring_colors(ring_highlight(false, true, 0.85, 0.6), 0.35, 0.5);
        let faded = hovered.faded(0.5);
        assert!((faded.normal.alpha() - 0.3).abs() < 1e-6);
        assert!((faded.exposure.alpha() - 0.5).abs() < 1e-6);
    }

    /// HOV-03: a selected ring is white at the selected alpha and wins over
    /// hover; a hovered one is white at the hover alpha; otherwise the
    /// normal colour.
    #[test]
    fn hovered_and_selected_rings_are_white() {
        use crate::view::colors::SELECTION;
        let selected = Some(SELECTION.with_alpha(0.85));
        assert_eq!(ring_highlight(true, false, 0.85, 0.6), selected);
        assert_eq!(ring_highlight(true, true, 0.85, 0.6), selected);
        assert_eq!(
            ring_highlight(false, true, 0.85, 0.6),
            Some(SELECTION.with_alpha(0.6))
        );
        assert_eq!(ring_highlight(false, false, 0.85, 0.6), None);
    }

    /// MRK-05 / MRK-09: planets and unfolded moons draw their full ring;
    /// gates, salvage sites and folded satellites draw nothing.
    #[test]
    fn natural_bodies_draw_full_rings() {
        use RingExtent::*;
        assert_eq!(ring_extent(BodyClass::Planet, false, false, false), Full);
        assert_eq!(ring_extent(BodyClass::Moon, false, false, false), Full);
        assert_eq!(ring_extent(BodyClass::Moon, true, false, true), Hidden);
        assert_eq!(ring_extent(BodyClass::Gate, false, false, true), Hidden);
        assert_eq!(
            ring_extent(BodyClass::SalvageSite, false, false, true),
            Hidden
        );
    }

    /// MRK-09: ships and stations draw only their tail in dot mode, and the
    /// full ring while hovered or selected, or as an exposure ring.
    #[test]
    fn ships_and_stations_draw_only_their_tail_in_dot_mode() {
        use RingExtent::*;
        for class in [BodyClass::Ship, BodyClass::Station] {
            assert_eq!(ring_extent(class, false, false, false), Tail);
            assert_eq!(ring_extent(class, false, false, true), Full);
            assert_eq!(ring_extent(class, false, true, false), Full);
            assert_eq!(ring_extent(class, true, true, true), Hidden);
        }
    }

    /// MOT-01 / HOV-03: the tail runs from the exposure colour down to the
    /// normal faint colour; a highlighted ring runs from white down to its
    /// highlight.
    #[test]
    fn the_tail_runs_from_exposure_to_normal() {
        let plain = ring_colors(None, 0.35, 0.5);
        assert_eq!(ring_color_at(plain, 1.0), colors::NEUTRAL.with_alpha(0.5));
        assert_eq!(
            ring_color_at(plain, 0.0),
            colors::ORBIT_LINE.with_alpha(0.35)
        );
        let hovered = ring_colors(ring_highlight(false, true, 0.85, 0.6), 0.35, 0.5);
        assert_eq!(ring_color_at(hovered, 1.0), colors::SELECTION);
        assert_eq!(
            ring_color_at(hovered, 0.0),
            colors::SELECTION.with_alpha(0.6)
        );
    }

    /// MOT-02 / HOV-03: an exposure ring is neutral, so it never matches
    /// the white selected ring.
    #[test]
    fn an_exposure_ring_never_matches_the_selected_ring() {
        let tunables = Tunables::default();
        let exposure = ring_colors(
            None,
            tunables.orbit_ring_alpha,
            tunables.exposure_ring_alpha,
        )
        .exposure;
        assert!(exposure.alpha() < 1.0);
        let selected = ring_highlight(
            true,
            false,
            tunables.orbit_selected_alpha,
            tunables.orbit_hover_alpha,
        );
        assert_ne!(Some(exposure), selected);
    }

    /// MRK-05 / HOV-03: normal rings are `orbit_ring_width_px` wide and
    /// highlighted ones `orbit_highlight_width_px`.
    #[test]
    fn highlighted_rings_take_the_highlight_width() {
        let tunables = Tunables::default();
        assert_eq!(
            ring_base_width(false, &tunables),
            tunables.orbit_ring_width_px
        );
        assert_eq!(
            ring_base_width(true, &tunables),
            tunables.orbit_highlight_width_px
        );
    }

    /// MOT-01: a segment the body sits on is split at the body, so the
    /// part ahead of it isn't shaded as tail; others are left whole.
    #[test]
    fn segments_are_split_at_the_body() {
        let segment = RingSegment {
            from: Vec3::ZERO,
            to: Vec3::X * 10.0,
            from_angle: 1.0,
            to_angle: 1.2,
        };
        let split = split_at_angle(segment, 1.05);
        assert_eq!(split.len(), 2);
        assert!((split[0].to - Vec3::X * 2.5).length() < 1e-4);
        assert_eq!((split[0].to_angle, split[1].from_angle), (1.05, 1.05));
        assert_eq!(split_at_angle(segment, 2.0), vec![segment]);
        // Across the wrap from just under a full turn to just over zero.
        let wrap = RingSegment {
            from_angle: 6.2,
            to_angle: 0.05,
            ..segment
        };
        assert_eq!(split_at_angle(wrap, 0.0).len(), 2);
    }

    /// MOT-01: after a split at the body, the piece behind it runs from
    /// its span behind up to 0 at the body; the piece ahead starts a full
    /// turn behind, so it gets no tail brightness.
    #[test]
    fn the_piece_ahead_of_the_body_is_not_tail() {
        let segment = RingSegment {
            from: Vec3::ZERO,
            to: Vec3::X,
            from_angle: 1.0,
            to_angle: 1.2,
        };
        let split = split_at_angle(segment, 1.05);
        let (behind_from, behind_to) = segment_behind(&split[0], 1.05);
        assert!((behind_from - 0.05_f32.to_degrees()).abs() < 1e-3);
        assert!(behind_to.abs() < 1e-3);
        let (ahead_from, ahead_to) = segment_behind(&split[1], 1.05);
        assert!(ahead_from >= 359.99 && ahead_to > ahead_from - 360.0);
        let state = MotionModeState {
            tail_degrees: 90.0,
            closing: 0.0,
            ring_mode: false,
        };
        assert_eq!(tail_brightness(ahead_from, &state), 0.0);
        assert_eq!(tail_brightness(ahead_to, &state), 0.0);
        assert_eq!(tail_brightness(behind_to, &state), 1.0);
    }

    /// HOV-04: on a shared ring each body lays its own tail, and a point
    /// takes the brighter; a segment is split at every body.
    #[test]
    fn each_body_on_a_shared_ring_has_its_own_tail() {
        let state = MotionModeState {
            tail_degrees: 90.0,
            closing: 0.0,
            ring_mode: false,
        };
        let segment = RingSegment {
            from: Vec3::ZERO,
            to: Vec3::X,
            from_angle: 1.0,
            to_angle: 1.2,
        };
        let pieces = split_at_angles(segment, &[1.05, 1.15]);
        assert_eq!(pieces.len(), 3);
        // The piece ending at the second body is at full brightness there,
        // whichever body is listed first, and its start is as bright as the
        // nearer body alone would make it.
        for angles in [[1.05, 1.15], [1.15, 1.05]] {
            let (behind, at_body) = ends_brightness(&pieces[1], &angles, &state);
            assert_eq!(at_body, 1.0);
            let (lone, _) = ends_brightness(&pieces[1], &[1.15], &state);
            assert_eq!(behind, lone);
        }
        assert_eq!(ends_brightness(&segment, &[], &state), (0.0, 0.0));
    }

    /// MOT-01: each segment carries the orbit angle at its ends, so the
    /// tail can be shaded along it.
    #[test]
    fn segments_carry_their_orbit_angles() {
        let view = looking_at(Vec3::new(0.0, 975.0, 436.0), Vec3::ZERO);
        for segment in ring_segments(Vec3::ZERO, 387.0, view) {
            for (point, angle) in [
                (segment.from, segment.from_angle),
                (segment.to, segment.to_angle),
            ] {
                assert!((point - ring_point(Vec3::ZERO, 387.0, angle)).length() < 1e-2);
            }
        }
    }

    /// The segments as plain end-point pairs.
    fn ends(segments: Vec<RingSegment>) -> Vec<(Vec3, Vec3)> {
        segments.into_iter().map(|s| (s.from, s.to)).collect()
    }

    /// Half a pixel at a 900 px tall view with the default 45 degree fov.
    fn half_pixel_rad() -> f32 {
        let px_per_rad = 450.0 / (std::f32::consts::FRAC_PI_4 / 2.0).tan();
        MAX_RING_ERROR_PX / px_per_rad
    }

    fn looking_at(camera_pos: Vec3, target: Vec3) -> RingView {
        RingView {
            camera_pos,
            camera_forward: (target - camera_pos).normalize(),
            max_error_rad: half_pixel_rad(),
        }
    }

    /// The worst gap between a segment's middle and the true circle, as an
    /// angle seen from the camera.
    fn worst_error(segments: &[(Vec3, Vec3)], center: Vec3, radius: f32, view: RingView) -> f32 {
        segments
            .iter()
            .map(|&(a, b)| {
                let mid = (a + b) / 2.0;
                let gap = ((mid - center).length() - radius).abs();
                gap / (mid - view.camera_pos).length()
            })
            .fold(0.0, f32::max)
    }

    /// From afar, a ring is the same 128 chords as ever, all in view.
    /// Covers MRK-05.
    #[test]
    fn a_distant_ring_is_unchanged() {
        // Morrow's orbit (4.3 AU) seen from the system fit.
        let view = looking_at(Vec3::new(0.0, 975.0, 436.0), Vec3::ZERO);
        let segments = ends(ring_segments(Vec3::ZERO, 387.0, view));
        assert_eq!(segments.len(), RING_SEGMENTS);
    }

    /// MRK-05: zoomed in on Halden IV, its own 2 AU orbit is drawn through
    /// the planet and within half a pixel of the true circle, where 128
    /// fixed chords missed it by tens of pixels or dropped out entirely.
    /// Covers MRK-05.
    #[test]
    fn a_ring_near_the_camera_follows_the_circle() {
        let radius = 180.0;
        let planet = ring_point(Vec3::ZERO, radius, 4.4);
        // The satellite frame: about one unit away at the default tilt.
        let elevation: f32 = 1.15;
        let camera = planet + Vec3::new(0.0, elevation.sin(), elevation.cos()) * 1.07;
        let view = looking_at(camera, planet);
        let segments = ends(ring_segments(Vec3::ZERO, radius, view));

        let error = worst_error(&segments, Vec3::ZERO, radius, view);
        assert!(
            error <= view.max_error_rad * 1.01,
            "worst error {error} rad"
        );
        let through_planet = segments
            .iter()
            .map(|&(a, b)| {
                let along = b - a;
                let t = ((planet - a).dot(along) / along.length_squared()).clamp(0.0, 1.0);
                planet.distance(a + along * t)
            })
            .fold(f32::INFINITY, f32::min);
        let allowed = view.max_error_rad * camera.distance(planet) * 1.5;
        assert!(
            through_planet <= allowed,
            "the orbit passes {through_planet} from the planet, allowed {allowed}"
        );
        // Fine only where it needs to be.
        assert!(segments.len() < 1_000, "{} segments", segments.len());
    }

    /// MRK-05: a chord crossing the near limit is cut there, not dropped.
    /// Covers MRK-05.
    #[test]
    fn chords_through_the_near_limit_are_cut_not_dropped() {
        let view = looking_at(Vec3::ZERO, Vec3::NEG_Z);
        let crossing = (Vec3::new(0.0, 0.0, -5.0), Vec3::new(0.0, 0.0, 5.0));
        let (kept_from, kept_to) = clip_to_front(crossing, view).unwrap();
        assert_eq!(kept_from, crossing.0);
        assert!((kept_to.z + NEAR_EPSILON).abs() < 1e-6);
        let reversed = clip_to_front((crossing.1, crossing.0), view).unwrap();
        assert!((reversed.0.z + NEAR_EPSILON).abs() < 1e-6);
        assert_eq!(clip_to_front((Vec3::Z, Vec3::Z * 2.0), view), None);
        let in_front = (Vec3::NEG_Z, Vec3::NEG_Z * 2.0);
        assert_eq!(clip_to_front(in_front, view), Some(in_front));
    }
}
