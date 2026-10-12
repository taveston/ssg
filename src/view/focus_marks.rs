//! How focus shows on screen: four corner brackets around the focused
//! object (CAM-22), in the neutral colour so they are never mistaken for the
//! white selection ring, and a chip under the breadcrumb reading "Focus:
//! *name*" with a × that ends the focus (CAM-09).

use bevy::prelude::*;

use super::anchors::BodyAnchors;
use super::camera::requests::ReleaseFocusRequest;
use super::colors;
use super::focus::Focus;
use super::fonts::Fonts;
use super::markers::ring_outer_half;
use super::targets::MapInputBlocker;
use super::tunables::Tunables;
use crate::sim::body::{BodyClass, DisplayName};
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
/// The chip sits just under the top bar, aligned with the breadcrumb.
const CHIP_TOP_PX: f32 = 58.0;
const CHIP_LEFT_PX: f32 = 14.0;
const CHIP_FONT_PX: f32 = 12.0;

/// The focus brackets (CAM-22).
#[derive(Component)]
struct FocusBrackets;

/// The focus chip (CAM-09), and its text.
#[derive(Component)]
struct FocusChip;
#[derive(Component)]
struct FocusChipText;

pub struct FocusMarksPlugin;

impl Plugin for FocusMarksPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (spawn_brackets, spawn_chip).in_set(Boot::Overlay))
            .add_systems(Update, (place_brackets, update_chip).in_set(Stage::Overlay));
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
            FocusBrackets,
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

/// CAM-22: the brackets sit on the focused object wherever it is shown -
/// its marker, or its parent's pill while it is folded (INS-06).
fn place_brackets(
    focus: Res<Focus>,
    anchors: Res<BodyAnchors>,
    tunables: Res<Tunables>,
    classes: Query<&BodyClass>,
    mut brackets: Query<&mut Node, With<FocusBrackets>>,
) {
    let at = focus
        .body
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

fn spawn_chip(mut commands: Commands, fonts: Res<Fonts>) {
    commands
        .spawn((
            FocusChip,
            MapInputBlocker,
            Node {
                position_type: PositionType::Absolute,
                top: px(CHIP_TOP_PX),
                left: px(CHIP_LEFT_PX),
                padding: UiRect::axes(px(8), px(3)),
                column_gap: px(8),
                border: UiRect::all(px(1)),
                display: Display::None,
                ..default()
            },
            BackgroundColor(colors::PANEL),
            BorderColor::all(colors::PANEL_BORDER),
        ))
        .with_children(|chip| {
            chip.spawn((
                FocusChipText,
                Text::new(""),
                fonts.label(CHIP_FONT_PX),
                TextColor(colors::NEUTRAL),
            ));
            // CAM-08 / CAM-09: the × ends focus.
            chip.spawn((
                Text::new("×"),
                fonts.label(CHIP_FONT_PX),
                TextColor(colors::NEUTRAL),
            ))
            .observe(
                |_click: On<PointerClick>, mut release: ResMut<ReleaseFocusRequest>| {
                    release.0 = true;
                },
            );
        });
}

/// CAM-09: the chip shows while something is focused, with its name.
fn update_chip(
    focus: Res<Focus>,
    names: Query<&DisplayName>,
    mut chips: Query<&mut Node, With<FocusChip>>,
    mut texts: Query<&mut Text, With<FocusChipText>>,
) {
    let name = focus.body.and_then(|body| names.get(body).ok());
    for mut node in &mut chips {
        node.display = if name.is_some() {
            Display::Flex
        } else {
            Display::None
        };
    }
    if let Some(name) = name {
        for mut text in &mut texts {
            *text = Text::new(chip_label(&name.0));
        }
    }
}

/// CAM-09: the chip's wording.
pub fn chip_label(name: &str) -> String {
    format!("Focus: {name}")
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

    /// Covers CAM-09.
    #[test]
    fn chip_reads_focus_and_the_name() {
        assert_eq!(chip_label("Halden IV"), "Focus: Halden IV");
    }
}
