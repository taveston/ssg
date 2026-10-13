//! Turns a `SymbolLook` (`symbols.rs`) into UI nodes (SYM-02 to SYM-07):
//! a square node `size` pixels across, holding the tinted fill, frame and
//! icon masks (`symbol_images.rs`), the "?" for an unknown class, the
//! echelon above the frame and a station's installation bar on top.
//! Amplifiers sit outside the square (SYM-04).
//!
//! Every coloured part carries a `Tint` with its full-strength colour, so
//! whoever draws the symbol can fade it as one by scaling each tint's alpha
//! (`markers.rs` does, for folding and the motion rule).

use bevy::prelude::*;

use super::fonts::Fonts;
use super::symbol_images::{SymbolImages, SymbolMask};
use super::symbols::{Icon, SymbolLook};

/// A part's full-strength colour. Fading writes `tint × alpha` to the
/// part's image, text or background colour.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Tint(pub Color);

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
    (images, fonts): (&SymbolImages, &Fonts),
    look: &SymbolLook,
    size: f32,
) {
    let color = look.color.with_alpha(look.alpha);
    let mut image = |mask: SymbolMask, tint: Color| {
        if let Some(handle) = images.get(mask, size) {
            parent.spawn((
                full(),
                ImageNode::new(handle).with_color(tint),
                Tint(tint),
                Pickable::IGNORE,
            ));
        }
    };
    if look.fill_alpha > 0.0 {
        image(
            SymbolMask::Fill(look.frame),
            look.color.with_alpha(look.fill_alpha * look.alpha),
        );
    }
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
            Tint(color),
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
                Tint(color),
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
}
