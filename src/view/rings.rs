//! Orbit rings (MRK-05), their tails and exposure rings (MOT-01, MOT-02,
//! MOT-11), and their hover and selection highlight (HOV-03).
//!
//! Rings are drawn for planets and for any *unfolded* satellite (GRP-01),
//! finer where the camera is close so they stay true to the circle at any
//! zoom (SCL-02). Each ring is one stroke whose brightness varies along it:
//! the tail behind a body runs from the exposure brightness at the dot down
//! to the ring's normal brightness, and an exposure ring is bright all
//! round. Each ring is its own retained gizmo, so it can widen on its own
//! as its tail closes (MOT-11). A hovered body's ring is white at
//! `orbit_hover_alpha`, a selected body's full white (HOV-03).

use bevy::color::Mix;
use bevy::prelude::*;

use crate::sim::body::BodyClass;
use crate::sim::orbit::{Orbit, OrbitPositionKm};
use crate::view::camera::state::camera_fov;
use crate::view::colors;
use crate::view::fold::SatelliteFoldState;
use crate::view::hovered::HoveredBodies;
use crate::view::motion_mode::{MotionModeState, degrees_behind, tail_brightness};
use crate::view::scale::km_to_render;
use crate::view::selection::Selection;
use crate::view::tunables::Tunables;
use crate::view::world::WorldPositions;
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
/// MRK-05: orbits are *faint* rings.
const DIM_RING_ALPHA: f32 = 0.35;
/// The width of an ordinary orbit ring, in pixels; an exposure ring is
/// `exposure_ring_width_px` (MOT-11).
const NORMAL_RING_WIDTH_PX: f32 = 2.0;

/// The retained gizmo that draws one body's ring.
#[derive(Component)]
pub struct RingStrokeFor(pub Entity);

pub struct RingsPlugin;

impl Plugin for RingsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_ring_strokes.in_set(Boot::Overlay))
            // Drawn with the rest of the overlay, once grouping and motion
            // have decided what shows.
            .add_systems(Update, draw_orbit_rings.in_set(Stage::Overlay));
    }
}

/// One retained gizmo per body whose ring can be drawn.
fn spawn_ring_strokes(
    mut commands: Commands,
    mut assets: ResMut<Assets<GizmoAsset>>,
    bodies: Query<(Entity, &BodyClass), With<Orbit>>,
) {
    for (body, class) in &bodies {
        if !ring_is_drawn(*class, false) {
            continue;
        }
        commands.spawn((
            RingStrokeFor(body),
            Gizmo {
                handle: assets.add(GizmoAsset::new()),
                line_config: GizmoLineConfig {
                    width: NORMAL_RING_WIDTH_PX,
                    ..default()
                },
                ..default()
            },
        ));
    }
}

/// Everything orbiting, with what decides how its ring is drawn.
type RingBodies<'w, 's> = Query<
    'w,
    's,
    (
        &'static Orbit,
        &'static OrbitPositionKm,
        &'static BodyClass,
        Option<&'static ChildOf>,
        Option<&'static SatelliteFoldState>,
        Option<&'static MotionModeState>,
    ),
>;

/// MRK-05 / HOV-02: whether a body's orbit ring is drawn at all. Gates and
/// salvage sites sit on star orbits that would only add clutter (gates
/// have their edge arrows instead), and a folded satellite draws nothing:
/// no marker, label or ring (GRP-01).
pub fn ring_is_drawn(class: BodyClass, folded_away: bool) -> bool {
    !matches!(class, BodyClass::Gate | BodyClass::SalvageSite) && !folded_away
}

/// HOV-03: the colour a ring is drawn in instead of its normal one: full
/// white while its object is selected (selected wins), white at
/// `hover_alpha` while hovered, otherwise none.
pub fn ring_highlight(selected: bool, hovered: bool, hover_alpha: f32) -> Option<Color> {
    let white = colors::SELECTION_RING;
    if selected {
        Some(white)
    } else if hovered {
        Some(white.with_alpha(hover_alpha))
    } else {
        None
    }
}

/// The two ends of a ring's brightness: its normal colour and the
/// exposure brightness its tail starts from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RingColors {
    pub normal: Color,
    pub exposure: Color,
}

/// MRK-05 / MOT-01 / HOV-03: a ring's normal colour is faint, or the
/// highlight; its exposure colour is the bright neutral, or white while
/// highlighted.
pub fn ring_colors(highlight: Option<Color>) -> RingColors {
    RingColors {
        normal: highlight.unwrap_or(colors::ORBIT_LINE.with_alpha(DIM_RING_ALPHA)),
        exposure: if highlight.is_some() {
            colors::SELECTION_RING
        } else {
            colors::NEUTRAL
        },
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
    radius: f32,
    body_angle: f32,
    mode: MotionModeState,
    colors: RingColors,
}

/// MRK-05 / MOT-01 / MOT-02 / MOT-11 / HOV-03: refill every ring stroke:
/// the ring as the camera sees it, bright along the tail (or all round in
/// ring mode), as wide as its closing makes it.
#[allow(clippy::too_many_arguments)]
fn draw_orbit_rings(
    selection: Res<Selection>,
    hovered: Res<HoveredBodies>,
    tunables: Res<Tunables>,
    camera_q: Query<(&Camera, &Transform, &Projection)>,
    bodies: RingBodies,
    positions: Res<WorldPositions>,
    mut strokes: Query<(&RingStrokeFor, &mut Gizmo)>,
    mut assets: ResMut<Assets<GizmoAsset>>,
) {
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
    for (stroke_for, mut gizmo) in &mut strokes {
        let Some(mut asset) = assets.get_mut(&gizmo.handle) else {
            continue;
        };
        asset.clear();
        let body = stroke_for.0;
        let Some(look) = stroke_look(body, &bodies, &positions, (&selection, &hovered, &tunables))
        else {
            continue;
        };
        for segment in ring_segments(look.center, look.radius, view) {
            let at = |angle: f32| {
                let behind = degrees_behind(look.body_angle, angle);
                ring_color_at(look.colors, tail_brightness(behind, &look.mode))
            };
            asset.line_gradient(
                segment.from,
                segment.to,
                at(segment.from_angle),
                at(segment.to_angle),
            );
        }
        let width = look
            .mode
            .ring_width(NORMAL_RING_WIDTH_PX, tunables.exposure_ring_width_px);
        if gizmo.line_config.width != width {
            gizmo.line_config.width = width;
        }
    }
}

/// Where a body's ring is, where the body is on it, and how it is
/// coloured; `None` if the ring isn't drawn.
fn stroke_look(
    body: Entity,
    bodies: &RingBodies,
    positions: &WorldPositions,
    (selection, hovered, tunables): (&Selection, &HoveredBodies, &Tunables),
) -> Option<StrokeLook> {
    let (orbit, local, class, child_of, fold, mode) = bodies.get(body).ok()?;
    if !ring_is_drawn(*class, fold.is_some_and(|f| f.effective_hidden)) {
        return None;
    }
    let center = child_of
        .and_then(|c| positions.0.get(&c.parent()).copied())
        .unwrap_or(Vec3::ZERO);
    let highlight = ring_highlight(
        selection.is_selected(body),
        hovered.contains(body),
        tunables.orbit_hover_alpha,
    );
    Some(StrokeLook {
        center,
        radius: km_to_render(orbit.radius_km),
        body_angle: local.0.z.atan2(local.0.x) as f32,
        mode: mode.copied().unwrap_or_default(),
        colors: ring_colors(highlight),
    })
}

/// `MAX_RING_ERROR_PX` as an angle seen from the camera. Without a viewport
/// yet there is nothing to be accurate to, and rings stay as plain chords.
fn max_error_rad(camera: &Camera, projection: &Projection) -> f32 {
    let Some(size) = camera.logical_viewport_size() else {
        return f32::INFINITY;
    };
    let px_per_rad = (size.y / 2.0) / (camera_fov(projection) / 2.0).tan();
    MAX_RING_ERROR_PX / px_per_rad
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

    /// HOV-03: a selected ring is full white and wins over hover; a hovered
    /// one is white at the hover alpha; otherwise the normal colour.
    #[test]
    fn hovered_and_selected_rings_are_white() {
        use crate::view::colors::SELECTION_RING;
        assert_eq!(ring_highlight(true, false, 0.6), Some(SELECTION_RING));
        assert_eq!(ring_highlight(true, true, 0.6), Some(SELECTION_RING));
        assert_eq!(
            ring_highlight(false, true, 0.6),
            Some(SELECTION_RING.with_alpha(0.6))
        );
        assert_eq!(ring_highlight(false, false, 0.6), None);
    }

    /// HOV-02: planets' and unfolded satellites' rings are drawn (and so
    /// can be hit); gates', salvage sites' and folded satellites' aren't.
    #[test]
    fn which_rings_are_drawn() {
        assert!(ring_is_drawn(BodyClass::Planet, false));
        assert!(ring_is_drawn(BodyClass::Moon, false));
        assert!(!ring_is_drawn(BodyClass::Moon, true));
        assert!(!ring_is_drawn(BodyClass::Gate, false));
        assert!(!ring_is_drawn(BodyClass::SalvageSite, false));
    }

    /// MOT-01 / HOV-03: the tail runs from the exposure colour down to the
    /// normal faint colour; a highlighted ring runs from white down to its
    /// highlight.
    #[test]
    fn the_tail_runs_from_exposure_to_normal() {
        let plain = ring_colors(None);
        assert_eq!(ring_color_at(plain, 1.0), colors::NEUTRAL);
        assert_eq!(
            ring_color_at(plain, 0.0),
            colors::ORBIT_LINE.with_alpha(DIM_RING_ALPHA)
        );
        let hovered = ring_colors(ring_highlight(false, true, 0.6));
        assert_eq!(ring_color_at(hovered, 1.0), colors::SELECTION_RING);
        assert_eq!(
            ring_color_at(hovered, 0.0),
            colors::SELECTION_RING.with_alpha(0.6)
        );
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
