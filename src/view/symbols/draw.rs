//! Turns a `SymbolLook` (`symbols/mod.rs`) into UI nodes (SYM-02 to SYM-09,
//! SYM-12): a square node `size` pixels across, holding the fill, frame
//! and icon as vector parts (`symbols/shape.rs`), the "?" for an unknown
//! class, the echelon above the frame, a station's installation bar on
//! top, the condition bar below, and a group's count at the upper right
//! and task-force bracket above.
//! Amplifiers sit outside the square (SYM-04).
//!
//! Every coloured part carries a `Tint` with its full-strength colour, so
//! whoever draws the symbol can fade it as one by scaling each tint's alpha
//! (`markers/mod.rs` does, for folding and the motion rule).

use bevy::color::Mix;
use bevy::prelude::*;

use super::shape::{SymbolMask, SymbolPart};
use super::{ConditionBar, Icon, SymbolLook};
use crate::view::colors;
use crate::view::fonts::Fonts;

/// A part's full-strength colour and the one slot it is drawn with. A UI
/// node has a background colour (and often a border colour) whether it
/// shows one or not, so fading writes `color × alpha` to that slot only.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Tint {
    pub color: Color,
    pub slot: TintSlot,
}

impl Tint {
    pub fn new(color: Color, slot: TintSlot) -> Self {
        Self { color, slot }
    }
}

/// Where a part's colour lives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TintSlot {
    /// A symbol part's `SymbolPart` colour.
    Shape,
    Text,
    Background,
    Border,
}

/// The "?" of an unknown class, as a fraction of the symbol's size.
const QUESTION_SCALE: f32 = 0.6;
/// The echelon text's size and its gap above the frame, as fractions of
/// the symbol's size.
const ECHELON_SCALE: f32 = 0.45;
/// The installation bar: its width and height as fractions of the symbol's
/// size, and how far it sits into the frame's top.
const BAR_WIDTH: f32 = 0.3;
const BAR_HEIGHT: f32 = 0.16;
const BAR_OVERLAP: f32 = 0.05;
/// A group's member count (SYM-09): its data-face size as a fraction of
/// the symbol's size, and how far its text box rises above the frame's
/// top, as a fraction of its font size, so the figure sits level with the
/// frame's upper right corner.
const COUNT_SCALE: f32 = 0.5;
const COUNT_RISE: f32 = 0.55;
/// The data face is monospaced: each figure is this fraction of the font
/// size wide (IBM Plex Mono's advance).
const MONO_ADVANCE: f32 = 0.6;
/// A line of text is this many times its font size high.
const LINE_HEIGHT: f32 = 1.2;
/// The task-force bracket (SYM-09): its width and height as fractions of
/// the symbol's size, its stroke, and its gap above an echelon, in
/// pixels.
const BRACKET_WIDTH: f32 = 0.8;
const BRACKET_HEIGHT: f32 = 0.2;
const BRACKET_STROKE_PX: f32 = 1.5;
const BRACKET_GAP_PX: f32 = 2.0;
/// The condition bar's height in pixels (SYM-12).
pub const CONDITION_BAR_HEIGHT_PX: f32 = 3.0;

/// SYM-09 / SYM-12 / HOV-07: how far outside the frame the condition bar,
/// a group's count and a task force's bracket start. The selection and
/// hover brackets are drawn inside a box that clears the frame by
/// `bracket_margin_px`, so these amplifiers start `amplifier_gap_px`
/// outside that box and the brackets never cover them.
pub fn amplifier_gap(bracket_margin_px: f32, amplifier_gap_px: f32) -> f32 {
    bracket_margin_px + amplifier_gap_px
}

/// How a symbol's amplifiers are spaced and drawn, from the tunables:
/// how far outside the frame they start (`amplifier_gap`), and how much
/// of the condition bar's colour its unfilled part keeps
/// (`condition_track_mix`, SYM-12).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AmplifierStyle {
    pub gap: f32,
    pub track_mix: f32,
}

/// How far a symbol's amplifiers reach beyond its square on each side, in
/// pixels (SYM-04, SYM-13): the echelon and the task-force bracket above,
/// a group's count to the right, the condition bar below. Nothing sits to
/// the left.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct AmplifierReach {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl AmplifierReach {
    /// The box a symbol and its amplifiers take, from the square's box.
    pub fn grow(&self, square: Rect) -> Rect {
        Rect::from_corners(
            square.min - Vec2::new(0.0, self.top),
            square.max + Vec2::new(self.right, self.bottom),
        )
    }

    /// SYM-13: what a symbol's label keeps `label_gap_px` from, and other
    /// labels keep clear of: the frame's box, the square grown by
    /// `margin` (`bracket_margin_px`, HOV-07) on every side, together with
    /// the amplifiers.
    pub fn label_box(&self, square: Rect, margin: f32) -> Rect {
        self.grow(square).union(square.inflate(margin))
    }
}

/// The height of the echelon's line above the frame, or zero without one.
fn echelon_height(look: &SymbolLook, size: f32) -> f32 {
    if look.echelon.is_some() {
        size * ECHELON_SCALE * LINE_HEIGHT
    } else {
        0.0
    }
}

/// SYM-09: how far above the frame a task force's bracket starts: above
/// the echelon if there is one, and outside the brackets' box (`gap`,
/// `amplifier_gap`) in any case.
fn task_force_bracket_rise(look: &SymbolLook, size: f32, gap: f32) -> f32 {
    let echelon = echelon_height(look, size);
    if echelon > 0.0 {
        (echelon + BRACKET_GAP_PX).max(gap)
    } else {
        gap
    }
}

/// SYM-13: how far a symbol's amplifiers reach beyond its `size`-pixel
/// square, so its label can be placed clear of them. `gap` is how far
/// outside the frame the amplifiers start (`amplifier_gap`).
pub fn amplifier_reach(look: &SymbolLook, size: f32, gap: f32) -> AmplifierReach {
    let mut top = echelon_height(look, size);
    if look.task_force {
        top = task_force_bracket_rise(look, size, gap) + size * BRACKET_HEIGHT;
    }
    let mut right = 0.0;
    if let Some(count) = look.count {
        let font = size * COUNT_SCALE;
        let figures = count.to_string().len() as f32;
        right = gap + figures * MONO_ADVANCE * font;
        top = top.max(COUNT_RISE * font);
    }
    let bottom = if look.condition.is_some() {
        gap + CONDITION_BAR_HEIGHT_PX
    } else {
        0.0
    };
    AmplifierReach { top, right, bottom }
}

/// Marks the filled part of a symbol's condition bar (SYM-12).
#[derive(Component)]
pub struct ConditionFill;

/// Marks a group's member count (SYM-09).
#[derive(Component)]
pub struct GroupCount;

/// Marks a task force's bracket (SYM-09).
#[derive(Component)]
pub struct TaskForceBracket;

/// The mask for an icon, if it is drawn from one.
fn icon_mask(icon: Icon) -> Option<SymbolMask> {
    match icon {
        Icon::Warship => Some(SymbolMask::Triangle),
        Icon::Drop => Some(SymbolMask::Drop),
        Icon::Crate => Some(SymbolMask::Crate),
        Icon::UnitBox { crossed } => Some(SymbolMask::UnitBox { crossed }),
        Icon::Question | Icon::None => None,
    }
}

/// SYM-03: a frame's opaque inside, its colour mixed into the ground.
pub fn fill_color(look: &SymbolLook) -> Color {
    colors::GROUND.mix(&look.color.with_alpha(1.0), look.fill_mix)
}

/// A node covering the whole symbol square.
fn full() -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: px(0),
        top: px(0),
        width: percent(100),
        height: percent(100),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    }
}

/// SYM-02 to SYM-09, SYM-12: the parts of a symbol, spawned under
/// `parent`, which is the `size`-pixel square they fill; the condition
/// bar, a group's count and a task force's bracket start `style.gap`
/// pixels outside it (`amplifier_gap`).
pub fn spawn_symbol_parts(
    parent: &mut ChildSpawnerCommands,
    fonts: &Fonts,
    look: &SymbolLook,
    size: f32,
    style: AmplifierStyle,
) {
    let color = look.color.with_alpha(look.alpha);
    spawn_frame_and_icon(parent, fonts, look, size, color);
    if let Some(echelon) = look.echelon {
        spawn_echelon(parent, fonts, echelon, size, color);
    }
    if look.installation_bar {
        spawn_installation_bar(parent, size, color);
    }
    if let Some(bar) = look.condition {
        spawn_condition_bar(parent, bar, look.alpha, size, style);
    }
    if let Some(count) = look.count {
        spawn_count(parent, fonts, count, color, (size, style.gap));
    }
    if look.task_force {
        let rise = task_force_bracket_rise(look, size, style.gap);
        spawn_task_force_bracket(parent, color, size, rise);
    }
}

/// SYM-02 / SYM-03: the frame's inside, the frame, and the icon in
/// `color`, all filling the symbol square.
fn spawn_frame_and_icon(
    parent: &mut ChildSpawnerCommands,
    fonts: &Fonts,
    look: &SymbolLook,
    size: f32,
    color: Color,
) {
    let mut image = |mask: SymbolMask, tint: Color| {
        parent.spawn((
            full(),
            SymbolPart {
                mask,
                color: tint,
                size_px: size,
            },
            Tint::new(tint, TintSlot::Shape),
            Pickable::IGNORE,
        ));
    };
    // SYM-03: the inside is always opaque, even on a faded stale track,
    // so nothing beneath shows through; command shows as how much of the
    // colour is mixed into the ground.
    image(SymbolMask::Fill(look.frame), fill_color(look));
    image(
        SymbolMask::Frame {
            shape: look.frame,
            dashed: look.dashed,
        },
        color,
    );
    if let Some(mask) = icon_mask(look.icon) {
        image(mask, color);
    }
    if look.icon == Icon::Question {
        spawn_text(
            parent,
            fonts.data(size * QUESTION_SCALE),
            "?",
            color,
            full(),
        );
    }
}

/// The echelon mark, centred just above the frame.
fn spawn_echelon(
    parent: &mut ChildSpawnerCommands,
    fonts: &Fonts,
    echelon: &str,
    size: f32,
    color: Color,
) {
    let above = Node {
        position_type: PositionType::Absolute,
        bottom: percent(100),
        width: percent(100),
        justify_content: JustifyContent::Center,
        ..default()
    };
    spawn_text(
        parent,
        fonts.data(size * ECHELON_SCALE),
        echelon,
        color,
        above,
    );
}

/// An installation's bar across the top of the frame.
fn spawn_installation_bar(parent: &mut ChildSpawnerCommands, size: f32, color: Color) {
    parent.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: percent(50.0 - BAR_WIDTH * 50.0),
            top: px(size * (BAR_OVERLAP - BAR_HEIGHT)),
            width: percent(BAR_WIDTH * 100.0),
            height: px(size * BAR_HEIGHT),
            ..default()
        },
        BackgroundColor(color),
        Tint::new(color, TintSlot::Background),
        Pickable::IGNORE,
    ));
}

/// SYM-09: a group's member count in the data face, just outside the
/// frame's upper right, `gap` pixels clear of the frame.
fn spawn_count(
    parent: &mut ChildSpawnerCommands,
    fonts: &Fonts,
    count: usize,
    color: Color,
    (size, gap): (f32, f32),
) {
    let font = size * COUNT_SCALE;
    let node = Node {
        position_type: PositionType::Absolute,
        left: px(size + gap),
        top: px(-COUNT_RISE * font),
        ..default()
    };
    parent
        .spawn((node, Pickable::IGNORE))
        .with_children(|holder| {
            holder.spawn((
                GroupCount,
                Text::new(count.to_string()),
                // Beside a narrow symbol, a figure must not wrap.
                TextLayout::linebreak(LineBreak::NoWrap),
                fonts.data(font),
                TextColor(color),
                Tint::new(color, TintSlot::Text),
                Pickable::IGNORE,
            ));
        });
}

/// SYM-09: the NATO task-force bracket: a line above the frame with its
/// ends turned down, its feet `rise` pixels above the frame.
fn spawn_task_force_bracket(parent: &mut ChildSpawnerCommands, color: Color, size: f32, rise: f32) {
    let width = size * BRACKET_WIDTH;
    parent.spawn((
        TaskForceBracket,
        Node {
            position_type: PositionType::Absolute,
            left: px((size - width) / 2.0),
            bottom: px(size + rise),
            width: px(width),
            height: px(size * BRACKET_HEIGHT),
            border: UiRect {
                left: px(BRACKET_STROKE_PX),
                right: px(BRACKET_STROKE_PX),
                top: px(BRACKET_STROKE_PX),
                bottom: px(0),
            },
            ..default()
        },
        BorderColor::all(color),
        Tint::new(color, TintSlot::Border),
        Pickable::IGNORE,
    ));
}

/// SYM-12: the bar's unfilled part: its colour mixed into the ground at
/// `mix`, so a part-full bar still shows its whole length.
pub fn condition_track_color(bar_color: Color, mix: f32) -> Color {
    colors::GROUND.mix(&bar_color, mix)
}

/// SYM-12: the condition bar under the frame: as wide as the symbol and
/// `CONDITION_BAR_HEIGHT_PX` high, `style.gap` below the frame, an
/// unfilled track of the bar's colour mixed into the ground, filled from
/// the left to the condition.
fn spawn_condition_bar(
    parent: &mut ChildSpawnerCommands,
    bar: ConditionBar,
    alpha: f32,
    size: f32,
    style: AmplifierStyle,
) {
    let track = condition_track_color(bar.color, style.track_mix).with_alpha(alpha);
    let fill = bar.color.with_alpha(alpha);
    parent
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(size + style.gap),
                width: px(size),
                height: px(CONDITION_BAR_HEIGHT_PX),
                ..default()
            },
            BackgroundColor(track),
            Tint::new(track, TintSlot::Background),
            Pickable::IGNORE,
        ))
        .with_children(|holder| {
            holder.spawn((
                ConditionFill,
                Node {
                    width: percent(bar.fill * 100.0),
                    height: percent(100),
                    ..default()
                },
                BackgroundColor(fill),
                Tint::new(fill, TintSlot::Background),
                Pickable::IGNORE,
            ));
        });
}

fn spawn_text(
    parent: &mut ChildSpawnerCommands,
    font: TextFont,
    text: &str,
    color: Color,
    node: Node,
) {
    parent
        .spawn((node, Pickable::IGNORE))
        .with_children(|holder| {
            holder.spawn((
                Text::new(text),
                font,
                TextColor(color),
                Tint::new(color, TintSlot::Text),
                Pickable::IGNORE,
            ));
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SYM-05: filled triangle, drop, crate and unit boxes come from masks;
    /// "?" is text and a station has nothing.
    #[test]
    fn icons_map_to_their_masks() {
        assert_eq!(icon_mask(Icon::Warship), Some(SymbolMask::Triangle));
        assert_eq!(icon_mask(Icon::Drop), Some(SymbolMask::Drop));
        assert_eq!(icon_mask(Icon::Crate), Some(SymbolMask::Crate));
        assert_eq!(
            icon_mask(Icon::UnitBox { crossed: true }),
            Some(SymbolMask::UnitBox { crossed: true })
        );
        assert_eq!(icon_mask(Icon::Question), None);
        assert_eq!(icon_mask(Icon::None), None);
    }

    /// SYM-09 / SYM-12 / HOV-07: the condition bar starts below the
    /// brackets' box, a group's count to its right and a task force's
    /// bracket above it, so the brackets round a symbol never cover them.
    #[test]
    fn the_amplifiers_clear_the_bracket_box() {
        use super::super::FrameShape;
        use crate::view::markers::brackets::BracketBox;
        use crate::view::tunables::Tunables;
        let tunables = Tunables::default();
        let half = tunables.ship_symbol_px / 2.0;
        let bbox = BracketBox::around_square(Vec2::ZERO, half, tunables.bracket_margin_px);
        let gap = amplifier_gap(tunables.bracket_margin_px, tunables.amplifier_gap_px);
        let bar_top = half + gap;
        assert!(bar_top > bbox.centre.y + bbox.half.y);
        let count_left = half + gap;
        assert!(count_left > bbox.centre.x + bbox.half.x);
        let task_force = SymbolLook {
            frame: FrameShape::Circle,
            color: colors::OWN,
            fill_mix: 0.3,
            icon: Icon::Warship,
            dashed: false,
            alpha: 1.0,
            echelon: None,
            installation_bar: false,
            condition: None,
            count: Some(4),
            task_force: true,
        };
        let size = half * 2.0;
        let bracket_feet = half + task_force_bracket_rise(&task_force, size, gap);
        assert!(bracket_feet > bbox.half.y - bbox.centre.y);
        let with_echelon = SymbolLook {
            echelon: Some("II"),
            ..task_force
        };
        assert!(
            task_force_bracket_rise(&with_echelon, size, gap)
                >= echelon_height(&with_echelon, size)
        );
    }

    /// SYM-13: a symbol's amplifiers reach above it (echelon, then the
    /// task-force bracket), to its right (a group's count, wider for more
    /// figures) and below it (the condition bar); a bare symbol reaches
    /// nowhere.
    #[test]
    fn amplifiers_reach_beyond_the_square() {
        use super::super::FrameShape;
        let bare = SymbolLook {
            frame: FrameShape::Circle,
            color: colors::OWN,
            fill_mix: 0.3,
            icon: Icon::Warship,
            dashed: false,
            alpha: 1.0,
            echelon: None,
            installation_bar: false,
            condition: None,
            count: None,
            task_force: false,
        };
        let size = 22.0;
        assert_eq!(amplifier_reach(&bare, size, 5.0), AmplifierReach::default());
        let bar = ConditionBar {
            fill: 0.45,
            color: colors::WARNING,
        };
        let group = SymbolLook {
            count: Some(4),
            task_force: true,
            condition: Some(bar),
            ..bare
        };
        let reach = amplifier_reach(&group, size, 5.0);
        assert_eq!(reach.bottom, 5.0 + CONDITION_BAR_HEIGHT_PX);
        assert!(reach.right > 0.0);
        assert!(reach.top >= size * BRACKET_HEIGHT + 5.0);
        assert!(reach.right > 5.0);
        let many = amplifier_reach(
            &SymbolLook {
                count: Some(12),
                ..group
            },
            size,
            5.0,
        );
        assert!(many.right > reach.right);
        let with_echelon = amplifier_reach(
            &SymbolLook {
                echelon: Some("II"),
                ..group
            },
            size,
            5.0,
        );
        assert!(with_echelon.top > reach.top);
        let square = Rect::from_corners(Vec2::ZERO, Vec2::splat(size));
        let grown = reach.grow(square);
        assert_eq!(grown.min, Vec2::new(0.0, -reach.top));
        assert_eq!(
            grown.max,
            Vec2::new(size + reach.right, size + reach.bottom)
        );
    }

    /// SYM-03: every inside is opaque, even on a faded stale track; a
    /// civilian's is plain ground, and a larger mix is nearer the colour.
    #[test]
    fn symbol_insides_are_opaque_mixes_into_the_ground() {
        use super::super::FrameShape;
        let look = |fill_mix: f32, alpha: f32| SymbolLook {
            frame: FrameShape::Circle,
            color: colors::OWN,
            fill_mix,
            icon: Icon::None,
            dashed: false,
            alpha,
            echelon: None,
            installation_bar: false,
            condition: None,
            count: None,
            task_force: false,
        };
        assert_eq!(fill_color(&look(0.0, 1.0)), colors::GROUND);
        for (mix, alpha) in [(0.16, 1.0), (0.30, 1.0), (0.30, 0.55)] {
            assert_eq!(fill_color(&look(mix, alpha)).alpha(), 1.0);
        }
        let blue = |c: Color| c.to_srgba().blue;
        assert!(blue(fill_color(&look(0.30, 1.0))) > blue(fill_color(&look(0.16, 1.0))));
    }
}
