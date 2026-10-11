//! Orbit rings (MRK-05) and exposure rings (MOT-02, MOT-07).
//!
//! Rings are drawn for planets and for any *unfolded* satellite (GRP-01),
//! finer where the camera is close so they stay true to the circle at any
//! zoom (SCL-02, F4-26).
//! A body in exposure-ring mode (see `motion.rs`) gets a second, brighter
//! and thicker ring drawn over the dim one, white while it is selected
//! (MOT-07).

use bevy::prelude::*;

use crate::Stage;
use crate::sim::body::BodyClass;
use crate::sim::orbit::Orbit;
use crate::view::camera::state::camera_fov;
use crate::view::fold::SatelliteFoldState;
use crate::view::motion_mode::MotionModeState;
use crate::view::scale::km_to_render;
use crate::view::tunables::Tunables;
use crate::view::world::WorldPositions;

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
/// Below this strength an exposure ring isn't worth drawing.
const MIN_RING_STRENGTH: f32 = 0.01;

/// Its own gizmo group so exposure rings can carry a thicker line width
/// than the ordinary orbit rings (MOT-02).
#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct ExposureRingGizmos;

pub struct RingsPlugin;

impl Plugin for RingsPlugin {
    fn build(&self, app: &mut App) {
        app.init_gizmo_group::<ExposureRingGizmos>()
            .add_systems(Startup, configure_exposure_gizmos)
            // Drawn with the rest of the overlay, once grouping and motion
            // have decided what shows.
            .add_systems(Update, draw_orbit_rings.in_set(Stage::Overlay));
    }
}

/// MOT-02: exposure rings are `exposure_ring_width_px` wide.
fn configure_exposure_gizmos(mut config_store: ResMut<GizmoConfigStore>, tunables: Res<Tunables>) {
    let (config, _) = config_store.config_mut::<ExposureRingGizmos>();
    config.line.width = tunables.exposure_ring_width_px;
}

/// Everything orbiting, with what decides how its ring is drawn.
type RingBodies<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Orbit,
        &'static BodyClass,
        Option<&'static ChildOf>,
        Option<&'static SatelliteFoldState>,
        Option<&'static MotionModeState>,
    ),
>;

/// MRK-05 / MOT-02: every visible orbit's faint ring, and the bright one
/// over it in exposure-ring mode.
fn draw_orbit_rings(
    mut gizmos: Gizmos,
    mut exposure: Gizmos<ExposureRingGizmos>,
    selection: Res<crate::view::selection::Selection>,
    camera_q: Query<(&Camera, &Transform, &Projection)>,
    bodies: RingBodies,
    positions: Res<WorldPositions>,
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

    for (entity, orbit, class, child_of, fold, motion) in &bodies {
        // Gates and salvage sites sit on star orbits that would just add
        // clutter; gates have their own edge indicators instead.
        if matches!(class, BodyClass::Gate | BodyClass::SalvageSite) {
            continue;
        }
        // GRP-01: a folded satellite draws nothing - no marker, label,
        // ring or trail.
        if fold.map(|f| f.effective_hidden).unwrap_or(false) {
            continue;
        }

        let center = child_of
            .and_then(|c| positions.0.get(&c.parent()).copied())
            .unwrap_or(Vec3::ZERO);
        let radius = km_to_render(orbit.radius_km);

        let dim = crate::view::colors::ORBIT_LINE.with_alpha(DIM_RING_ALPHA);
        let strength = motion.map(MotionModeState::ring_strength).unwrap_or(0.0);
        let bright = (strength > MIN_RING_STRENGTH).then(|| {
            // MOT-07: a selected object in ring mode keeps its ring
            // highlighted.
            let color = if selection.is_open(entity) {
                crate::view::colors::SELECTION_RING
            } else {
                crate::view::colors::NEUTRAL
            };
            color.with_alpha(strength)
        });

        for (from, to) in ring_segments(center, radius, view) {
            gizmos.line(from, to, dim);
            // MOT-02: the body is moving too fast to read as a position, so
            // the ring itself carries it.
            if let Some(bright) = bright {
                exposure.line(from, to, bright);
            }
        }
    }
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

/// MRK-05 / SCL-02 (F4-26): the straight segments that draw a ring as the
/// camera sees it.
///
/// A ring starts as `RING_SEGMENTS` chords, which is plenty from afar. A
/// chord the camera is close to is split until it strays from the circle by
/// no more than `max_error_rad` as seen from the camera: zoomed in on a
/// planet, the camera sits about one render unit from a planetary orbit
/// 180 units in radius, where a fixed chord would miss the planet by tens of
/// pixels. Chords crossing the camera's near limit are cut there rather than
/// dropped, so the part of the ring in front of the camera always shows.
pub fn ring_segments(center: Vec3, radius: f32, view: RingView) -> Vec<(Vec3, Vec3)> {
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
    out: &mut Vec<(Vec3, Vec3)>,
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
    out.extend(clip_to_front(chord, view));
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
    #[test]
    fn mrk_05_a_distant_ring_is_unchanged() {
        // Morrow's orbit (4.3 AU) seen from the system frame.
        let view = looking_at(Vec3::new(0.0, 975.0, 436.0), Vec3::ZERO);
        let segments = ring_segments(Vec3::ZERO, 387.0, view);
        assert_eq!(segments.len(), RING_SEGMENTS);
    }

    /// F4-26: zoomed in on Halden IV, its own 2 AU orbit is drawn through
    /// the planet and within half a pixel of the true circle, where 128
    /// fixed chords missed it by tens of pixels or dropped out entirely.
    #[test]
    fn mrk_05_a_ring_near_the_camera_follows_the_circle() {
        let radius = 180.0;
        let planet = ring_point(Vec3::ZERO, radius, 4.4);
        // The satellite frame: about one unit away at the default tilt.
        let pitch: f32 = 1.15;
        let camera = planet + Vec3::new(0.0, pitch.sin(), pitch.cos()) * 1.07;
        let view = looking_at(camera, planet);
        let segments = ring_segments(Vec3::ZERO, radius, view);

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

    /// F4-26: a chord crossing the near limit is cut there, not dropped.
    #[test]
    fn mrk_05_chords_through_the_near_limit_are_cut_not_dropped() {
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
