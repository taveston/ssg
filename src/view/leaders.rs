//! Velocity leaders (SYM-11): a line from the edge of a ship's or group's
//! frame in the direction it moves across the screen, longer the faster
//! it goes. Only ships in flight have one; a ship parked or orbiting in a
//! named orbit has none, and nor does a stale track or a ship slower than
//! `leader_speed_min_kms` relative to the star.
//!
//! The leader is a child of the symbol's marker, so it moves, fades and
//! hides with it. It is a holder the size of the marker, turned about the
//! marker's centre to the direction of motion, with a thin bar hanging
//! above its centre from the frame's edge outwards.

use bevy::prelude::*;

use super::markers::{MarkerShape, MarkersPlaced, symbol_settings};
use super::plot_marks::screen_angle;
use super::scale::km_to_render_vec3;
use super::symbol_draw::{Tint, TintSlot};
use super::symbols::{FrameShape, symbol_look};
use super::targets::MarkerFor;
use super::tunables::Tunables;
use super::world::ViewNow;
use crate::Stage;
use crate::sim::body::Affiliation;
use crate::sim::rail::km_per_day_to_km_per_s;
use crate::sim::track::{ShipRecord, Track};
use crate::sim::velocity::FlightVelocities;

/// How much of a square frame's half-size its outline takes, as drawn by
/// `symbol_shape.wesl` (`SQUARE_SCALE` there).
const SQUARE_SCALE: f32 = 0.85;
/// A quatrefoil lobe's radius as a fraction of the frame's half-size, as
/// drawn by `symbol_shape.wesl` (`QUATREFOIL_LOBE` there).
const QUATREFOIL_LOBE: f32 = 0.6;
/// Below this many pixels of screen movement per step the motion is
/// along the line of sight, with no direction on screen to point in.
const MIN_SCREEN_STEP_PX: f32 = 1e-3;

pub struct LeadersPlugin;

impl Plugin for LeadersPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (attach_leaders, place_leaders)
                .chain()
                .in_set(Stage::Overlay)
                .after(MarkersPlaced),
        );
    }
}

/// On a symbol's marker once its leader is attached.
#[derive(Component)]
pub struct HasLeader;

/// The leader's holder: the body it belongs to and its frame's shape.
/// It fills the marker and is turned to the direction of motion.
#[derive(Component, Debug, Clone, Copy)]
pub struct Leader {
    pub body: Entity,
    pub frame: FrameShape,
}

/// The leader's line itself, a child of its holder.
#[derive(Component)]
pub struct LeaderBar;

/// SYM-11: the leader's length in pixels for a speed in km/s relative to
/// the star. It rises with the logarithm of speed from `leader_min_px`
/// at `leader_speed_min_kms` to `leader_max_px` at `leader_speed_max_kms`,
/// clamped at both ends; below the minimum speed there is no leader.
pub fn leader_length_px(speed_kms: f32, tunables: &Tunables) -> Option<f32> {
    let (low, high) = (tunables.leader_speed_min_kms, tunables.leader_speed_max_kms);
    if speed_kms.is_nan() || speed_kms < low {
        return None;
    }
    let t = if high > low {
        ((speed_kms.ln() - low.ln()) / (high.ln() - low.ln())).clamp(0.0, 1.0)
    } else {
        1.0
    };
    Some(tunables.leader_min_px + (tunables.leader_max_px - tunables.leader_min_px) * t)
}

/// SYM-11: how far from a symbol's centre its frame's outer edge lies
/// along the screen direction `dir`, for a frame `size` pixels across
/// with an outline `stroke` wide, matching the shapes `symbol_shape.wesl`
/// draws.
pub fn frame_edge_px(frame: FrameShape, size: f32, stroke: f32, dir: Vec2) -> f32 {
    let d = dir.normalize_or(Vec2::Y);
    // The shader draws the outline centred on a shape of this half-size.
    let half = size / 2.0 - stroke / 2.0;
    let centre_line = match frame {
        FrameShape::Circle => half,
        FrameShape::Square => half * SQUARE_SCALE / d.x.abs().max(d.y.abs()),
        FrameShape::Diamond => half / (d.x.abs() + d.y.abs()),
        FrameShape::Quatrefoil => {
            // Four circles; the ray leaves the last one it is inside.
            let lobe = half * QUATREFOIL_LOBE;
            let out = half - lobe;
            [Vec2::X, Vec2::NEG_X, Vec2::Y, Vec2::NEG_Y]
                .into_iter()
                .filter_map(|axis| {
                    let c = axis * out;
                    let b = d.dot(c);
                    let disc = b * b - c.length_squared() + lobe * lobe;
                    (disc >= 0.0).then(|| b + disc.sqrt())
                })
                .fold(0.0, f32::max)
        }
    };
    centre_line + stroke / 2.0
}

/// Symbol markers that don't have a leader yet.
type BareSymbols<'w, 's> =
    Query<'w, 's, (Entity, &'static MarkerFor, &'static MarkerShape), Without<HasLeader>>;

/// Gives every symbol's marker a leader, hidden until its ship moves.
fn attach_leaders(
    mut commands: Commands,
    tunables: Res<Tunables>,
    markers: BareSymbols,
    bodies: Query<(&Affiliation, &ShipRecord)>,
) {
    for (marker, marker_for, shape) in &markers {
        if *shape != MarkerShape::Symbol {
            continue;
        }
        let Ok((affiliation, record)) = bodies.get(marker_for.0) else {
            continue;
        };
        let look = symbol_look(*affiliation, record, symbol_settings(&tunables));
        let color = look.color;
        let leader = Leader {
            body: marker_for.0,
            frame: look.frame,
        };
        commands
            .entity(marker)
            .insert(HasLeader)
            .with_children(|parent| {
                parent
                    .spawn((
                        leader,
                        Node {
                            position_type: PositionType::Absolute,
                            left: px(0),
                            top: px(0),
                            width: percent(100),
                            height: percent(100),
                            display: Display::None,
                            ..default()
                        },
                        UiTransform::default(),
                        Pickable::IGNORE,
                    ))
                    .with_children(|holder| {
                        holder.spawn((
                            LeaderBar,
                            Node {
                                position_type: PositionType::Absolute,
                                ..default()
                            },
                            BackgroundColor(color),
                            Tint::new(color, TintSlot::Background),
                            Pickable::IGNORE,
                        ));
                    });
            });
    }
}

/// Where a leader points and how long it is this frame, in screen
/// pixels: the clockwise angle from screen up, and the length.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LeaderPose {
    pub angle: f32,
    pub length_px: f32,
}

/// SYM-11: the leader a body has this frame, if any: only a ship or
/// group in flight, not a stale track, moving at least
/// `leader_speed_min_kms` relative to the star, in its direction across
/// the screen.
fn leader_pose(
    body: Entity,
    view: &ViewNow,
    velocities: &FlightVelocities,
    records: &Query<&ShipRecord>,
    tunables: &Tunables,
) -> Option<LeaderPose> {
    if matches!(records.get(body).ok()?.track, Track::Stale { .. }) {
        return None;
    }
    let velocity = velocities.of(body)?;
    let speed = km_per_day_to_km_per_s(velocity.length()) as f32;
    let length_px = leader_length_px(speed, tunables)?;
    // A short step along the velocity, in render space, seen through the
    // camera: the direction it moves on screen.
    let at = view.position(body)?;
    let (_, camera) = view.camera()?;
    let step = camera.translation().distance(at) * 0.01;
    let along = km_to_render_vec3(velocity.normalize()).normalize_or_zero() * step;
    let (from, to) = (view.project(at)?, view.project(at + along)?);
    if from.distance(to) < MIN_SCREEN_STEP_PX {
        return None;
    }
    Some(LeaderPose {
        angle: screen_angle(from, to),
        length_px,
    })
}

/// The leaders' holders and their bars.
type LeaderHolders<'w, 's> = Query<
    'w,
    's,
    (
        &'static Leader,
        &'static mut Node,
        &'static mut UiTransform,
        &'static Children,
    ),
>;

/// SYM-11: turn and size each leader to its ship's motion this frame, or
/// hide it.
fn place_leaders(
    view: ViewNow,
    velocities: FlightVelocities,
    records: Query<&ShipRecord>,
    tunables: Res<Tunables>,
    mut holders: LeaderHolders,
    mut bars: Query<&mut Node, (With<LeaderBar>, Without<Leader>)>,
) {
    let size = tunables.ship_symbol_px;
    let stroke = tunables.symbol_stroke_px;
    for (leader, mut node, mut transform, children) in &mut holders {
        let pose = leader_pose(leader.body, &view, &velocities, &records, &tunables);
        let Some(pose) = pose else {
            if node.display != Display::None {
                node.display = Display::None;
            }
            continue;
        };
        node.display = Display::Flex;
        let rotation = Rot2::radians(pose.angle);
        if transform.rotation != rotation {
            transform.rotation = rotation;
        }
        let dir = Vec2::new(pose.angle.sin(), -pose.angle.cos());
        let edge = frame_edge_px(leader.frame, size, stroke, dir);
        for &child in children {
            if let Ok(mut bar) = bars.get_mut(child) {
                // The holder's centre is the symbol's; the bar hangs
                // above it, from the frame's edge outwards.
                bar.left = px(size / 2.0 - stroke / 2.0);
                bar.width = px(stroke);
                bar.top = px(size / 2.0 - edge - pose.length_px);
                bar.height = px(pose.length_px);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SYM-11: the leader grows with the logarithm of speed between its
    /// two lengths, clamped at both ends, and is absent below the minimum
    /// speed.
    #[test]
    fn leader_length_grows_with_the_log_of_speed_and_clamps() {
        let t = Tunables::default();
        assert_eq!(leader_length_px(t.leader_speed_min_kms * 0.99, &t), None);
        assert_eq!(leader_length_px(0.0, &t), None);
        assert_eq!(
            leader_length_px(t.leader_speed_min_kms, &t),
            Some(t.leader_min_px)
        );
        let max = leader_length_px(t.leader_speed_max_kms, &t).unwrap();
        assert!((max - t.leader_max_px).abs() < 1e-4);
        assert_eq!(
            leader_length_px(t.leader_speed_max_kms * 10.0, &t),
            Some(t.leader_max_px)
        );
        // Halfway in the logarithm is halfway in length.
        let mid = (t.leader_speed_min_kms * t.leader_speed_max_kms).sqrt();
        let half = (t.leader_min_px + t.leader_max_px) / 2.0;
        assert!((leader_length_px(mid, &t).unwrap() - half).abs() < 1e-3);
    }

    /// SYM-11: the leader starts at the frame's outer edge: a circle's is
    /// the same all round, a diamond reaches furthest at its points and a
    /// square at its corners.
    #[test]
    fn the_leader_starts_at_the_frame_edge() {
        let (size, stroke) = (22.0, 1.5);
        let up = Vec2::NEG_Y;
        let diagonal = Vec2::new(1.0, -1.0);
        let circle = frame_edge_px(FrameShape::Circle, size, stroke, up);
        assert_eq!(circle, size / 2.0);
        assert_eq!(
            frame_edge_px(FrameShape::Circle, size, stroke, diagonal),
            circle
        );
        let diamond_point = frame_edge_px(FrameShape::Diamond, size, stroke, up);
        let diamond_side = frame_edge_px(FrameShape::Diamond, size, stroke, diagonal);
        assert!((diamond_point - size / 2.0).abs() < 1e-4);
        assert!(diamond_side < diamond_point);
        let square_side = frame_edge_px(FrameShape::Square, size, stroke, up);
        let square_corner = frame_edge_px(FrameShape::Square, size, stroke, diagonal);
        assert!(square_side < size / 2.0);
        let rim = stroke / 2.0;
        assert!(((square_corner - rim) / (square_side - rim) - 2f32.sqrt()).abs() < 1e-3);
        let lobe_tip = frame_edge_px(FrameShape::Quatrefoil, size, stroke, up);
        assert!((lobe_tip - size / 2.0).abs() < 1e-4);
        assert!(frame_edge_px(FrameShape::Quatrefoil, size, stroke, diagonal) < lobe_tip);
    }
}
