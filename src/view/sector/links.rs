//! The links (SECT-04): a straight line between the centres of the two
//! nodes a link joins, drawn by its state in the orbit colour (MRK-01): a
//! working link solid, a dormant one dashed with dashes and gaps measured
//! on screen, a collapsed one solid, thinner and faint. Links are world
//! gizmos, under every node and label (SECT-05), and are never hover or
//! click targets.

use bevy::prelude::*;

use super::SectorNode;
use crate::content::generator::network::LinkState;
use crate::content::start::LoadedSector;
use crate::view::colors;
use crate::view::map_layer::MapLayer;
use crate::view::paths::geometry::{Dashes, Piece};
use crate::view::rings::{RingView, clip_to_front};
use crate::view::tunables::Tunables;
use crate::view::world::ViewNow;

/// A dormant link is cut into this many straight pieces before its dashes
/// are measured, so dashes stay even on screen under perspective.
const DASH_PIECES: usize = 48;

/// How a link is drawn, one gizmo each (SECT-04).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkStroke {
    Working,
    Dormant,
    Collapsed,
}

impl LinkStroke {
    /// SECT-04: how a link in `state` is drawn.
    pub fn of(state: LinkState) -> Self {
        match state {
            LinkState::Working => LinkStroke::Working,
            LinkState::Dormant { .. } => LinkStroke::Dormant,
            LinkState::Collapsed => LinkStroke::Collapsed,
        }
    }

    /// SECT-04: the line's width in pixels. A dormant link's dashes are as
    /// wide as a working link's line.
    pub fn width(self, tunables: &Tunables) -> f32 {
        match self {
            LinkStroke::Working | LinkStroke::Dormant => tunables.link_working_px,
            LinkStroke::Collapsed => tunables.link_collapsed_px,
        }
    }

    /// SECT-04: the line's colour: the orbit colour at the state's
    /// strength.
    pub fn color(self, tunables: &Tunables) -> Color {
        let alpha = match self {
            LinkStroke::Working => tunables.link_working_alpha,
            LinkStroke::Dormant => tunables.link_dormant_alpha,
            LinkStroke::Collapsed => tunables.link_collapsed_alpha,
        };
        colors::ORBIT_LINE.with_alpha(alpha)
    }
}

/// Every link's pieces of line this frame, with how each is drawn.
#[derive(Resource, Default, Debug)]
pub struct LinkPieces(pub Vec<(LinkStroke, Piece)>);

/// One gizmo per stroke. Without gizmo assets (a headless test) nothing
/// is drawn, though the links are still laid out.
pub fn spawn_link_strokes(mut commands: Commands, assets: Option<ResMut<Assets<GizmoAsset>>>) {
    let Some(mut assets) = assets else {
        return;
    };
    for stroke in [
        LinkStroke::Working,
        LinkStroke::Dormant,
        LinkStroke::Collapsed,
    ] {
        commands.spawn((
            stroke,
            // No depth bias: nothing in the sector view writes depth, and a
            // line biased to the near plane loses pieces far from the
            // camera to the near clip.
            Gizmo {
                handle: assets.add(GizmoAsset::new()),
                ..default()
            },
        ));
    }
}

/// SECT-04: the dashes of a straight line from `from` to `to`: pieces
/// `dashes.on` pixels long on screen with `dashes.off` between, the
/// pattern running on from piece to piece. The line is cut where it
/// passes the camera's near limit.
pub fn dash_pieces(
    (from, to): (Vec3, Vec3),
    view: RingView,
    mut dashes: Dashes,
    project: impl Fn(Vec3) -> Option<Vec2>,
) -> Vec<Piece> {
    let mut pieces: Vec<Piece> = Vec::new();
    // Whether the last dash ran to the end of its piece, so a dash that
    // starts the next piece is the same dash going on.
    let mut running = false;
    for i in 0..DASH_PIECES {
        let at = |k: usize| from.lerp(to, k as f32 / DASH_PIECES as f32);
        let Some((a, b)) = clip_to_front((at(i), at(i + 1)), view) else {
            running = false;
            continue;
        };
        let (Some(sa), Some(sb)) = (project(a), project(b)) else {
            running = false;
            continue;
        };
        let stretches = dashes.walk(sa.distance(sb));
        let ends_on_edge = stretches.last().is_some_and(|&(_, t1)| t1 >= 1.0);
        for (t0, t1) in stretches {
            let piece = Piece {
                from: a.lerp(b, t0),
                to: a.lerp(b, t1),
            };
            match pieces.last_mut() {
                Some(last) if running && t0 <= 0.0 => last.to = piece.to,
                _ => pieces.push(piece),
            }
            running = false;
        }
        running = ends_on_edge;
    }
    pieces
}

/// SECT-04: a link's pieces: one solid piece for a working or collapsed
/// link, cut at the near limit, or its dashes for a dormant one.
pub fn link_pieces(
    ends: (Vec3, Vec3),
    stroke: LinkStroke,
    view: RingView,
    tunables: &Tunables,
    project: impl Fn(Vec3) -> Option<Vec2>,
) -> Vec<Piece> {
    match stroke {
        LinkStroke::Dormant => {
            let dashes = Dashes::new(tunables.link_dash_px, tunables.link_gap_px);
            dash_pieces(ends, view, dashes, project)
        }
        LinkStroke::Working | LinkStroke::Collapsed => clip_to_front(ends, view)
            .map(|(from, to)| Piece { from, to })
            .into_iter()
            .collect(),
    }
}

/// SECT-04: lay out every link with this frame's camera and refill the
/// stroke gizmos. Nothing is drawn while the system view is shown.
pub fn draw_links(
    (layer, tunables): (Res<MapLayer>, Res<Tunables>),
    sector: Option<Res<LoadedSector>>,
    now: ViewNow,
    nodes: Query<&SectorNode>,
    mut strokes: Query<(&LinkStroke, &mut Gizmo)>,
    mut assets: Option<ResMut<Assets<GizmoAsset>>>,
    mut pieces: ResMut<LinkPieces>,
) {
    pieces.0.clear();
    if let (Some(loaded), Some((_, camera)), true) = (&sector, now.camera(), layer.is_sector()) {
        let world: Vec<Vec3> = {
            let mut by_id = vec![Vec3::ZERO; loaded.sector.entries.len()];
            for node in &nodes {
                by_id[node.id] = node.world;
            }
            by_id
        };
        let view = RingView {
            camera_pos: camera.translation(),
            camera_forward: camera.forward().as_vec3(),
            max_error_rad: f32::INFINITY,
        };
        for link in &loaded.sector.links {
            let stroke = LinkStroke::of(link.state);
            let ends = (world[link.ends[0]], world[link.ends[1]]);
            let laid = link_pieces(ends, stroke, view, &tunables, |p| now.project(p));
            pieces
                .0
                .extend(laid.into_iter().map(|piece| (stroke, piece)));
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
        let color = stroke.color(&tunables);
        for (_, piece) in pieces.0.iter().filter(|(s, _)| s == stroke) {
            asset.line(piece.from, piece.to, color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A camera straight above the origin, looking down.
    fn view() -> RingView {
        RingView {
            camera_pos: Vec3::new(0.0, 100.0, 0.0),
            camera_forward: Vec3::NEG_Y,
            max_error_rad: f32::INFINITY,
        }
    }

    /// Straight down, one render unit is one pixel.
    fn project(p: Vec3) -> Option<Vec2> {
        Some(Vec2::new(p.x, p.z))
    }

    /// SECT-04: a working link is solid at `link_working_px` and
    /// `link_working_alpha`; a collapsed one solid, `link_collapsed_px`
    /// at `link_collapsed_alpha`; a dormant one at `link_dormant_alpha`;
    /// all in the orbit colour.
    #[test]
    fn links_are_drawn_by_their_state() {
        let t = Tunables::default();
        let ends = (Vec3::ZERO, Vec3::new(200.0, 0.0, 0.0));
        for stroke in [LinkStroke::Working, LinkStroke::Collapsed] {
            let pieces = link_pieces(ends, stroke, view(), &t, project);
            assert_eq!(
                pieces,
                vec![Piece {
                    from: ends.0,
                    to: ends.1
                }]
            );
        }
        assert_eq!(LinkStroke::Working.width(&t), t.link_working_px);
        assert_eq!(LinkStroke::Collapsed.width(&t), t.link_collapsed_px);
        let alpha = |s: LinkStroke| s.color(&t).alpha();
        assert_eq!(alpha(LinkStroke::Working), t.link_working_alpha);
        assert_eq!(alpha(LinkStroke::Dormant), t.link_dormant_alpha);
        assert_eq!(alpha(LinkStroke::Collapsed), t.link_collapsed_alpha);
        let rgb = |c: Color| c.to_srgba().with_alpha(1.0);
        assert_eq!(rgb(LinkStroke::Dormant.color(&t)), rgb(colors::ORBIT_LINE));
        assert_eq!(LinkStroke::of(LinkState::Collapsed), LinkStroke::Collapsed);
        let dormant = LinkState::Dormant {
            faulted: [true, false],
        };
        assert_eq!(LinkStroke::of(dormant), LinkStroke::Dormant);
    }

    /// SECT-04: a dormant link is dashed on screen: dashes
    /// `link_dash_px` long with `link_gap_px` between, from its start.
    #[test]
    fn a_dormant_link_is_dashed_on_screen() {
        let t = Tunables::default();
        let ends = (Vec3::ZERO, Vec3::new(100.0, 0.0, 0.0));
        let pieces = link_pieces(ends, LinkStroke::Dormant, view(), &t, project);
        let period = t.link_dash_px + t.link_gap_px;
        assert_eq!(pieces.len(), (100.0 / period).ceil() as usize);
        for (k, piece) in pieces.iter().enumerate() {
            let start = k as f32 * period;
            assert!(
                (piece.from.x - start).abs() < 1e-3,
                "dash {k} starts at {}",
                piece.from.x
            );
            let end = (start + t.link_dash_px).min(100.0);
            assert!(
                (piece.to.x - end).abs() < 1e-3,
                "dash {k} ends at {}",
                piece.to.x
            );
        }
    }

    /// A link that runs behind the camera is cut at its near limit.
    #[test]
    fn a_link_behind_the_camera_is_cut() {
        let t = Tunables::default();
        let ends = (Vec3::ZERO, Vec3::new(0.0, 200.0, 0.0));
        let pieces = link_pieces(ends, LinkStroke::Working, view(), &t, project);
        assert_eq!(pieces.len(), 1);
        assert!(pieces[0].to.y < 100.0);
    }
}
