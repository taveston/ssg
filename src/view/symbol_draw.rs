//! Turns a `SymbolLook` (`symbols.rs`) into UI nodes (SYM-02 to SYM-07):
//! a square node `size` pixels across, holding the fill, frame and icon
//! as vector parts (`symbol_shape.rs`), the "?" for an unknown class, the
//! echelon above the frame and a station's installation bar on top.
//! Amplifiers sit outside the square (SYM-04).
//!
//! Every coloured part carries a `Tint` with its full-strength colour, so
//! whoever draws the symbol can fade it as one by scaling each tint's alpha
//! (`markers.rs` does, for folding and the motion rule).

use bevy::color::Mix;
use bevy::prelude::*;

use super::colors;
use super::fonts::Fonts;
use super::symbol_shape::{SymbolMask, SymbolPart};
use super::symbols::{Icon, SymbolLook};

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

/// SYM-02 to SYM-07: the parts of a symbol, spawned under `parent`, which
/// is the `size`-pixel square they fill.
pub fn spawn_symbol_parts(
    parent: &mut ChildSpawnerCommands,
    fonts: &Fonts,
    look: &SymbolLook,
    size: f32,
) {
    let color = look.color.with_alpha(look.alpha);
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
            fonts.mono(size * QUESTION_SCALE),
            "?",
            color,
            full(),
        );
    }
    if let Some(echelon) = look.echelon {
        let above = Node {
            position_type: PositionType::Absolute,
            bottom: percent(100),
            width: percent(100),
            justify_content: JustifyContent::Center,
            ..default()
        };
        spawn_text(
            parent,
            fonts.mono(size * ECHELON_SCALE),
            echelon,
            color,
            above,
        );
    }
    if look.installation_bar {
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

    /// SYM-03: every inside is opaque, even on a faded stale track; a
    /// civilian's is plain ground, and a larger mix is nearer the colour.
    #[test]
    fn symbol_insides_are_opaque_mixes_into_the_ground() {
        use super::super::symbols::FrameShape;
        let look = |fill_mix: f32, alpha: f32| SymbolLook {
            frame: FrameShape::Circle,
            color: colors::OWN,
            fill_mix,
            icon: Icon::None,
            dashed: false,
            alpha,
            echelon: None,
            installation_bar: false,
        };
        assert_eq!(fill_color(&look(0.0, 1.0)), colors::GROUND);
        for (mix, alpha) in [(0.16, 1.0), (0.30, 1.0), (0.30, 0.55)] {
            assert_eq!(fill_color(&look(mix, alpha)).alpha(), 1.0);
        }
        let blue = |c: Color| c.to_srgba().blue;
        assert!(blue(fill_color(&look(0.30, 1.0))) > blue(fill_color(&look(0.16, 1.0))));
    }
}
