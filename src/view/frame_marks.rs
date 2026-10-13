//! How the reference frame shows on screen: four corner brackets around the
//! frame body's marker, unless it is the star (CAM-22), in the neutral
//! colour so they are never mistaken for the white selection ring. The
//! breadcrumb (`topbar.rs`) names the frame too.

use bevy::prelude::*;

use super::anchors::BodyAnchors;
use super::colors;
use super::frame::Frame;
use super::markers::ring_outer_half;
use super::tunables::Tunables;
use crate::sim::body::BodyClass;
use crate::{Boot, Stage};

/// Length of each bracket's arms, as a percentage of the box side.
const BRACKET_ARM_PERCENT: f32 = 30.0;
/// Gap between the white selection ring and the brackets, so both read
/// on one marker (CAM-22).
const BRACKET_CLEARANCE_PX: f32 = 2.0;

/// CAM-22: the bracket box's side - `focus_bracket_px`, or larger if needed
/// to clear the marker and its selection ring, so both stay readable on one
/// marker (F5-08).
pub fn bracket_side(focus_bracket_px: f32, ring_outer_half: f32) -> f32 {
    focus_bracket_px.max(2.0 * (ring_outer_half + BRACKET_CLEARANCE_PX))
}
const BRACKET_LINE_PX: f32 = 1.5;

/// The frame brackets (CAM-22).
#[derive(Component)]
struct FrameBrackets;

pub struct FrameMarksPlugin;

impl Plugin for FrameMarksPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_brackets.in_set(Boot::Overlay))
            .add_systems(Update, place_brackets.in_set(Stage::Overlay));
    }
}

/// CAM-22: one corner bracket of the box, at the given corner.
fn corner(top: bool, right: bool) -> impl Bundle {
    let line = |on: bool| if on { px(BRACKET_LINE_PX) } else { px(0) };
    let edge = |on: bool| if on { px(0) } else { Val::Auto };
    (
        Node {
            position_type: PositionType::Absolute,
            width: percent(BRACKET_ARM_PERCENT),
            height: percent(BRACKET_ARM_PERCENT),
            top: edge(top),
            bottom: edge(!top),
            left: edge(!right),
            right: edge(right),
            border: UiRect {
                top: line(top),
                bottom: line(!top),
                left: line(!right),
                right: line(right),
            },
            ..default()
        },
        BorderColor::all(colors::NEUTRAL),
    )
}

fn spawn_brackets(mut commands: Commands) {
    commands
        .spawn((
            FrameBrackets,
            Node {
                position_type: PositionType::Absolute,
                display: Display::None,
                ..default()
            },
            // Brackets don't take clicks from the marker inside them.
            Pickable::IGNORE,
        ))
        .with_children(|brackets| {
            for (top, right) in [(true, true), (true, false), (false, true), (false, false)] {
                brackets.spawn((corner(top, right), Pickable::IGNORE));
            }
        });
}

/// CAM-22: the brackets sit on the frame body wherever it is shown - its
/// marker, or its parent's pill while it is folded (INS-06). The star's
/// frame has none.
fn place_brackets(
    frame: Res<Frame>,
    anchors: Res<BodyAnchors>,
    tunables: Res<Tunables>,
    classes: Query<&BodyClass>,
    mut brackets: Query<&mut Node, With<FrameBrackets>>,
) {
    let at = frame
        .non_star_body()
        .and_then(|body| Some((anchors.get(body)?, *classes.get(body).ok()?)));
    for mut node in &mut brackets {
        let Some((at, class)) = at else {
            node.display = Display::None;
            continue;
        };
        let side = bracket_side(tunables.focus_bracket_px, ring_outer_half(class));
        node.display = Display::Flex;
        node.width = px(side);
        node.height = px(side);
        node.left = px(at.x - side / 2.0);
        node.top = px(at.y - side / 2.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CAM-22 (F5-08): the brackets clear the marker and its ring, and are
    /// never smaller than `focus_bracket_px`.
    #[test]
    fn brackets_clear_the_selection_ring() {
        // A planet: 6 px half-size, ring 2 px out and 2 px wide.
        let side = bracket_side(14.0, 10.0);
        assert!(side / 2.0 > 10.0, "outside the ring");
        assert_eq!(bracket_side(40.0, 5.0), 40.0, "never below the tunable");
    }
}
