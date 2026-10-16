//! The paths' marks on screen (PLNR-61, PLNR-63 to PLNR-65, PLNR-67,
//! PLNR-69): the dot at each path's start, the ghosts at its ends and the
//! labels of the arrival ghost and of the coast's event. Drawn by small
//! pools of UI nodes: the dots under every marker, just above the lines;
//! the ghosts in the natural bodies' layer, under every real marker; the
//! labels in the label layer, placed with every other label (`labels.rs`).
//! None of them takes a click (HOV-02).

use bevy::prelude::*;

use super::gather::{Ghost, MapPaths, PathDraw};
use super::ghost::GhostLook;
use super::look::{Emphasis, ghost_opacity};
use super::strokes::{DrawnPath, DrawnPaths, coast_color, path_color};
use crate::sim::body::BodyClass;
use crate::view::fonts::Fonts;
use crate::view::halo::TextHalo;
use crate::view::layers;
use crate::view::markers::marker_half_size;
use crate::view::motion_mode::MotionModeState;
use crate::view::screen::ScreenPositions;
use crate::view::tunables::Tunables;
use crate::view::world::ViewNow;

/// The dots at the paths' starts sit just above the lines (PLNR-69).
pub const DOT_LAYER: i32 = layers::BODIES - 2;
/// Ghosts sit with the natural bodies, beneath every real marker
/// (PLNR-69).
pub const GHOST_LAYER: i32 = layers::BODIES - 1;
/// The size of the paths' labels' text, as a marker's name.
const PATH_LABEL_FONT_PX: f32 = 12.0;

/// A path's start dot (PLNR-61).
#[derive(Component)]
pub struct PathDot;

/// A ghost node (PLNR-63).
#[derive(Component)]
pub struct PathGhost;

/// A path's label: the arrival ghost's, or the coast event's (PLNR-65,
/// PLNR-67). `anchor` is the box on screen it sits beside this frame,
/// `None` while it isn't used. `labels.rs` places it after every ranked
/// label.
#[derive(Component, Debug, Default, Clone, Copy, PartialEq)]
pub struct PathLabel {
    pub anchor: Option<Rect>,
}

/// The text of a path's label, the label's one child.
#[derive(Component)]
pub struct PathLabelText;

/// A dot to draw: its centre and colour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DotMark {
    pub centre: Vec2,
    pub color: Color,
}

/// A ghost to draw: its centre and look.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GhostMark {
    pub centre: Vec2,
    pub look: GhostLook,
}

/// A label to place: the box it sits beside, its text and colour.
#[derive(Debug, Clone, PartialEq)]
pub struct LabelMark {
    pub anchor: Rect,
    pub text: String,
    pub color: Color,
}

/// Every mark the paths want this frame.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Marks {
    pub dots: Vec<DotMark>,
    pub ghosts: Vec<GhostMark>,
    pub labels: Vec<LabelMark>,
}

/// What a ghost is sized and faded by: its object's class and dot.
type GhostObjects<'w, 's> = Query<'w, 's, (&'static BodyClass, Option<&'static MotionModeState>)>;

/// PLNR-63 / PLNR-64: a ghost of `object` on screen at `centre`: a dashed
/// circle the size of its marker class, neutral at `ghost_alpha` times its
/// dot's opacity.
fn ghost_mark(object: Entity, centre: Vec2, objects: &GhostObjects, t: &Tunables) -> GhostMark {
    let (size, dot_alpha) = objects
        .get(object)
        .map_or((t.ship_symbol_px, 1.0), |(class, mode)| {
            (
                marker_half_size(*class, t) * 2.0,
                mode.map_or(1.0, MotionModeState::dot_alpha),
            )
        });
    GhostMark {
        centre,
        look: GhostLook {
            color: crate::view::colors::NEUTRAL.with_alpha(ghost_opacity(t.ghost_alpha, dot_alpha)),
            size_px: size,
            width_px: t.ghost_width_px,
            dash_px: t.ghost_dash_px,
        },
    }
}

/// The box a label sits beside: a square `size` across round `centre`.
fn anchor(centre: Vec2, size: f32) -> Rect {
    Rect::from_center_size(centre, Vec2::splat(size))
}

/// PLNR-61 / PLNR-63 to PLNR-65 / PLNR-67: the marks of one path, from
/// where its ends land on screen. A mark whose point is off the map isn't
/// drawn.
fn path_marks(
    path: &PathDraw,
    drawn: &DrawnPath,
    on_screen: &impl Fn(Option<Vec3>) -> Option<Vec2>,
    objects: &GhostObjects,
    t: &Tunables,
    marks: &mut Marks,
) {
    let color = path_color(path.emphasis, t);
    let start = on_screen(drawn.start);
    let end = on_screen(drawn.end);
    if let Some(centre) = start {
        marks.dots.push(DotMark { centre, color });
    }
    let ghost = |ghost: &Ghost, at: Option<Vec2>, marks: &mut Marks| {
        let Some(centre) = at else {
            return;
        };
        let mark = ghost_mark(ghost.object, centre, objects, t);
        if let Some(text) = &ghost.label {
            marks.labels.push(LabelMark {
                anchor: anchor(centre, mark.look.size_px),
                text: text.clone(),
                color,
            });
        }
        marks.ghosts.push(mark);
    };
    if let Some(departure) = &path.departure_ghost {
        ghost(departure, start, marks);
    }
    if let Some(arrival) = &path.arrival_ghost {
        ghost(arrival, end, marks);
    }
    if let (Some(coast), Some(centre)) = (&path.coast, on_screen(drawn.coast_end)) {
        marks.labels.push(LabelMark {
            anchor: anchor(centre, 0.0),
            text: coast.label.clone(),
            color: coast_color(Emphasis::Full, t),
        });
    }
}

/// The pooled mark nodes.
type DotNodes<'w, 's> =
    Query<'w, 's, (Entity, &'static mut Node, &'static mut BackgroundColor), With<PathDot>>;
type GhostNodes<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static mut Node, &'static mut GhostLook),
    (With<PathGhost>, Without<PathDot>),
>;
type LabelNodes<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static mut PathLabel, &'static Children),
    (Without<PathGhost>, Without<PathDot>),
>;
type LabelTexts<'w, 's> =
    Query<'w, 's, (&'static mut Text, &'static mut TextColor), With<PathLabelText>>;

/// Every path's marks, written to the pools of nodes, which grow as
/// needed; unused nodes are hidden.
#[allow(clippy::too_many_arguments)]
pub fn place_path_marks(
    mut commands: Commands,
    (paths, drawn, screen): (Res<MapPaths>, Res<DrawnPaths>, Res<ScreenPositions>),
    (view, tunables, fonts): (ViewNow, Res<Tunables>, Res<Fonts>),
    objects: GhostObjects,
    mut dots: DotNodes,
    mut ghosts: GhostNodes,
    mut labels: LabelNodes,
    mut texts: LabelTexts,
) {
    let on_screen =
        |world: Option<Vec3>| view.project(world?).filter(|&pos| screen.in_viewport(pos));
    let mut marks = Marks::default();
    for (path, laid) in paths.0.iter().zip(&drawn.0) {
        path_marks(path, laid, &on_screen, &objects, &tunables, &mut marks);
    }
    write_dots(
        &mut commands,
        &marks.dots,
        &mut dots,
        tunables.departure_dot_px,
    );
    write_ghosts(&mut commands, &marks.ghosts, &mut ghosts);
    write_labels(
        &mut commands,
        &marks.labels,
        (&mut labels, &mut texts),
        &fonts,
    );
}

/// A node centred on `centre`, `size` across, shown or hidden.
fn place(node: &mut Node, centre: Vec2, size: f32) {
    node.display = Display::Flex;
    node.left = px(centre.x - size / 2.0);
    node.top = px(centre.y - size / 2.0);
    node.width = px(size);
    node.height = px(size);
}

fn mark_node() -> Node {
    Node {
        position_type: PositionType::Absolute,
        display: Display::None,
        ..default()
    }
}

/// PLNR-61: the start dots, filled circles `size` across.
fn write_dots(commands: &mut Commands, wanted: &[DotMark], nodes: &mut DotNodes, size: f32) {
    let mut pool: Vec<_> = nodes.iter_mut().collect();
    pool.sort_by_key(|(e, ..)| e.index_u32());
    for (i, (_, node, background)) in pool.iter_mut().enumerate() {
        match wanted.get(i) {
            Some(dot) => {
                place(node, dot.centre, size);
                background.0 = dot.color;
            }
            None => node.display = Display::None,
        }
    }
    for dot in wanted.iter().skip(pool.len()) {
        let mut node = Node {
            border_radius: BorderRadius::MAX,
            ..mark_node()
        };
        place(&mut node, dot.centre, size);
        commands.spawn((
            PathDot,
            node,
            BackgroundColor(dot.color),
            GlobalZIndex(DOT_LAYER),
            Pickable::IGNORE,
        ));
    }
}

/// PLNR-63: the ghosts.
fn write_ghosts(commands: &mut Commands, wanted: &[GhostMark], nodes: &mut GhostNodes) {
    let mut pool: Vec<_> = nodes.iter_mut().collect();
    pool.sort_by_key(|(e, ..)| e.index_u32());
    for (i, (_, node, look)) in pool.iter_mut().enumerate() {
        match wanted.get(i) {
            Some(ghost) => {
                place(node, ghost.centre, ghost.look.size_px);
                look.set_if_neq(ghost.look);
            }
            None => node.display = Display::None,
        }
    }
    for ghost in wanted.iter().skip(pool.len()) {
        let mut node = mark_node();
        place(&mut node, ghost.centre, ghost.look.size_px);
        commands.spawn((
            PathGhost,
            node,
            ghost.look,
            BackgroundColor(Color::NONE),
            GlobalZIndex(GHOST_LAYER),
            Pickable::IGNORE,
        ));
    }
}

/// PLNR-65 / PLNR-67: the labels, each anchored for `labels.rs` to place.
fn write_labels(
    commands: &mut Commands,
    wanted: &[LabelMark],
    (nodes, texts): (&mut LabelNodes, &mut LabelTexts),
    fonts: &Fonts,
) {
    let mut pool: Vec<_> = nodes.iter_mut().collect();
    pool.sort_by_key(|(e, ..)| e.index_u32());
    for (i, (_, label, children)) in pool.iter_mut().enumerate() {
        let mark = wanted.get(i);
        label.set_if_neq(PathLabel {
            anchor: mark.map(|m| m.anchor),
        });
        let (Some(mark), Some(&child)) = (mark, children.first()) else {
            continue;
        };
        if let Ok((mut text, mut color)) = texts.get_mut(child) {
            if text.0 != mark.text {
                text.0.clone_from(&mark.text);
            }
            color.0 = mark.color;
        }
    }
    for mark in wanted.iter().skip(pool.len()) {
        commands
            .spawn((
                PathLabel {
                    anchor: Some(mark.anchor),
                },
                Node {
                    position_type: PositionType::Absolute,
                    ..default()
                },
                GlobalZIndex(layers::LABELS),
                Visibility::Hidden,
                Pickable::IGNORE,
            ))
            .with_children(|label| {
                label.spawn((
                    PathLabelText,
                    Text::new(mark.text.clone()),
                    TextLayout::linebreak(LineBreak::NoWrap),
                    fonts.data(PATH_LABEL_FONT_PX),
                    TextColor(mark.color),
                    TextHalo::ground(),
                    Pickable::IGNORE,
                ));
            });
    }
}
