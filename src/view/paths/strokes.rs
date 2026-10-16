//! The paths' lines (PLNR-60, PLNR-61, PLNR-67, PLNR-69): each path laid
//! out in render space from its rail, with each segment's points placed
//! relative to its primary where the primary is now, and drawn as world
//! gizmos: solid where flown, dashed ahead, and the coast after an
//! Intercept dotted in the warning colour. Gizmos sit under every UI node,
//! so paths are in the bottom layer; they are drawn in front of the orbit
//! rings, which share that layer (MRK-12). Lines are never hover or click
//! targets (HOV-02).

use bevy::prelude::*;

use super::gather::{MapPaths, PathDraw};
use super::geometry::{
    Dashes, PathPoint, Piece, dashed_pieces, sample_rail, solid_pieces, split_at_day,
};
use super::look::Emphasis;
use crate::sim::clock::GameClock;
use crate::sim::rail::SegmentKind;
use crate::view::colors;
use crate::view::rings::{RingView, px_per_rad};
use crate::view::scale::km_to_render_vec3;
use crate::view::tunables::Tunables;
use crate::view::world::ViewNow;
use orbits::Rail;

/// How far a drawn line may stray from the true path, in pixels, as for
/// orbit rings (MRK-05).
const MAX_PATH_ERROR_PX: f32 = 0.5;
/// Gizmo depth bias that draws a line in front of everything else in the
/// world: the orbit rings, which a path runs along at its ends (PLNR-69).
const IN_FRONT: f32 = -1.0;

/// The three line widths paths are drawn at, one gizmo each.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathStroke {
    /// Planned, hovered or selected paths, `path_width_px` wide.
    Full,
    /// Paths of ships in flight, `traffic_path_width_px` wide.
    Traffic,
    /// Coasts after an Intercept, `consequence_width_px` wide.
    Coast,
}

impl PathStroke {
    /// PLNR-61 / PLNR-62: the stroke's width.
    pub fn width(self, tunables: &Tunables) -> f32 {
        match self {
            PathStroke::Full => tunables.path_width_px,
            PathStroke::Traffic => tunables.traffic_path_width_px,
            PathStroke::Coast => tunables.consequence_width_px,
        }
    }
}

/// One path as laid out this frame: its pieces of line by style, where
/// it starts and ends, and where its coast ends, in render space.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DrawnPath {
    pub flown: Vec<Piece>,
    pub ahead: Vec<Piece>,
    pub coast: Vec<Piece>,
    pub start: Option<Vec3>,
    pub end: Option<Vec3>,
    pub coast_end: Option<Vec3>,
}

/// Every path laid out this frame, in `MapPaths`' order.
#[derive(Resource, Default, Debug)]
pub struct DrawnPaths(pub Vec<DrawnPath>);

/// One gizmo per stroke width, drawn in front of the rings.
/// Without gizmo assets (a headless test) nothing is drawn, though paths
/// are still laid out.
pub fn spawn_path_strokes(mut commands: Commands, assets: Option<ResMut<Assets<GizmoAsset>>>) {
    let Some(mut assets) = assets else {
        return;
    };
    for stroke in [PathStroke::Full, PathStroke::Traffic, PathStroke::Coast] {
        commands.spawn((
            stroke,
            Gizmo {
                handle: assets.add(GizmoAsset::new()),
                depth_bias: IN_FRONT,
                ..default()
            },
        ));
    }
}

/// PLNR-61 / PLNR-62: a path's colour: own blue, faint as traffic.
pub fn path_color(emphasis: Emphasis, tunables: &Tunables) -> Color {
    match emphasis {
        Emphasis::Full => colors::OWN,
        Emphasis::Traffic => colors::OWN.with_alpha(tunables.traffic_path_alpha),
    }
}

/// PLNR-61 / PLNR-67: the coast's colour: the warning colour, as faint
/// as its path.
pub fn coast_color(emphasis: Emphasis, tunables: &Tunables) -> Color {
    colors::WARNING.with_alpha(path_color(emphasis, tunables).alpha())
}

/// What a path is laid out against this frame: today, the camera, and
/// where each primary is now.
pub struct Layout<'a, 'w, 's> {
    pub today: f64,
    pub view: RingView,
    pub now: &'a ViewNow<'w, 's>,
    pub tunables: &'a Tunables,
}

impl Layout<'_, '_, '_> {
    /// PLNR-60: a rail position on `day`, placed relative to its primary
    /// where the primary is now (SCL-06).
    fn at(&self, primary: Entity, kind: &SegmentKind, day: f64) -> Option<Vec3> {
        Some(self.now.position(primary)? + km_to_render_vec3(kind.state_at(day).position))
    }

    /// PLNR-60: the crossing's points, segment by segment, from the day
    /// it starts to the day it ends.
    fn crossing(&self, path: &PathDraw) -> Vec<PathPoint> {
        let mut points: Vec<PathPoint> = Vec::new();
        for segment in path.trajectory.segments() {
            let primary = match segment.kind {
                SegmentKind::Conic { primary, .. } | SegmentKind::Spiral { primary, .. } => primary,
                SegmentKind::Parked { .. } => continue,
            };
            let from = segment.start_day.max(path.crossing_start_day);
            let to = segment
                .end_day
                .unwrap_or(f64::INFINITY)
                .min(path.crossing_end_day);
            let at = |day| self.at(primary, &segment.kind, day);
            let part = sample_rail(&at, (from, to), self.view);
            let skip = usize::from(points.last().map(|p| p.day) == part.first().map(|p| p.day));
            points.extend(part.into_iter().skip(skip));
        }
        points
    }

    /// PLNR-60 / PLNR-61 / PLNR-67: the path laid out: flown solid, ahead
    /// dashed on screen, and its coast dotted.
    pub fn lay_out(&self, path: &PathDraw) -> DrawnPath {
        let t = self.tunables;
        let project = |p: Vec3| self.now.project(p);
        let points = self.crossing(path);
        let (flown, ahead) = split_at_day(&points, self.today);
        let mut drawn = DrawnPath {
            flown: solid_pieces(&flown, self.view),
            ahead: dashed_pieces(
                &ahead,
                self.view,
                Dashes::new(t.path_dash_px, t.path_gap_px),
                project,
            ),
            start: points.first().map(|p| p.world),
            end: points.last().map(|p| p.world),
            ..default()
        };
        if let Some(coast) = &path.coast {
            let kind = SegmentKind::Conic {
                primary: coast.primary,
                conic: coast.conic,
            };
            let at = |day| self.at(coast.primary, &kind, day);
            let points = sample_rail(&at, (coast.from_day, coast.to_day), self.view);
            let dots = Dashes::new(t.consequence_dot_px, t.consequence_gap_px);
            drawn.coast = dashed_pieces(&points, self.view, dots, project);
            drawn.coast_end = points.last().map(|p| p.world);
        }
        drawn
    }
}

/// PLNR-60 / PLNR-61 / PLNR-62 / PLNR-67: lay out every path and refill
/// the three stroke gizmos with its lines.
pub fn draw_paths(
    paths: Res<MapPaths>,
    now: ViewNow,
    (clock, tunables): (Res<GameClock>, Res<Tunables>),
    cameras: Query<(&Camera, &Transform, &Projection), With<Camera3d>>,
    mut strokes: Query<(&PathStroke, &mut Gizmo)>,
    mut assets: Option<ResMut<Assets<GizmoAsset>>>,
    mut drawn: ResMut<DrawnPaths>,
) {
    drawn.0.clear();
    let mut lines: Vec<(PathStroke, Piece, Color)> = Vec::new();
    if let Ok((camera, transform, projection)) = cameras.single() {
        let layout = Layout {
            today: clock.elapsed_days(),
            view: RingView {
                camera_pos: transform.translation,
                camera_forward: transform.forward().as_vec3(),
                max_error_rad: px_per_rad(camera, projection)
                    .map_or(f32::INFINITY, |px| MAX_PATH_ERROR_PX / px),
            },
            now: &now,
            tunables: &tunables,
        };
        for path in &paths.0 {
            let laid = layout.lay_out(path);
            let stroke = match path.emphasis {
                Emphasis::Full => PathStroke::Full,
                Emphasis::Traffic => PathStroke::Traffic,
            };
            let color = path_color(path.emphasis, &tunables);
            let coast = coast_color(path.emphasis, &tunables);
            lines.extend(
                laid.flown
                    .iter()
                    .chain(&laid.ahead)
                    .map(|&p| (stroke, p, color)),
            );
            lines.extend(laid.coast.iter().map(|&p| (PathStroke::Coast, p, coast)));
            drawn.0.push(laid);
        }
    }
    let Some(assets) = assets.as_mut() else {
        return;
    };
    for (stroke, mut gizmo) in &mut strokes {
        let width = stroke.width(&tunables);
        if gizmo.line_config.width != width {
            gizmo.line_config.width = width;
        }
        let Some(mut asset) = assets.get_mut(&gizmo.handle) else {
            continue;
        };
        asset.clear();
        for (_, piece, color) in lines.iter().filter(|(s, ..)| s == stroke) {
            asset.line(piece.from, piece.to, *color);
        }
    }
}
