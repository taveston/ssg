//! The map's and the top bar's type sizes, set in one place (MRK-19).
//! Each text on the map or in the top bar carries a `TextSize` naming its
//! role, and the role alone picks its size from `Tunables`: names on the
//! map at `map_label_px`, second lines and small plot text at
//! `map_second_line_px`, the breadcrumb and the date at `bar_text_px`. No
//! module sets a size of its own, so a name on an exposure ring is the
//! same size as a marker's, and changing one tunable resizes every text
//! of its role, even after it is drawn.

use bevy::prelude::*;

use super::fonts::{Face, Fonts};
use super::tunables::Tunables;

/// The role of a text on the map or in the top bar, which sets its size.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextSize {
    /// A name on the map: a marker's, an exposure ring's, an edge tab's,
    /// and a path's or ghost's label.
    MapLabel,
    /// A second line under a name (a marker's, an exposure ring's
    /// period), the scale bar, the north dial's and the frame cue's "N".
    SecondLine,
    /// The breadcrumb and the date in the top bar.
    Bar,
}

impl TextSize {
    /// This role's size in pixels.
    pub fn px(self, tunables: &Tunables) -> f32 {
        match self {
            TextSize::MapLabel => tunables.map_label_px,
            TextSize::SecondLine => tunables.map_second_line_px,
            TextSize::Bar => tunables.bar_text_px,
        }
    }

    /// The font for this role in `face`, with the role, so the text
    /// follows later changes to its tunable.
    pub fn font(self, fonts: &Fonts, face: Face, tunables: &Tunables) -> (TextFont, TextSize) {
        (fonts.face(face, self.px(tunables)), self)
    }
}

/// Keeps every text with a role at its role's size (MRK-19).
pub struct TextSizePlugin;

impl Plugin for TextSizePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, apply_text_sizes);
    }
}

/// Resizes every text with a role when the tunables change, so a size
/// tuned while the game runs reaches what is already drawn.
pub fn apply_text_sizes(tunables: Res<Tunables>, mut texts: Query<(&TextSize, &mut TextFont)>) {
    if !tunables.is_changed() {
        return;
    }
    for (role, mut font) in &mut texts {
        let size = FontSize::Px(role.px(&tunables));
        if font.font_size != size {
            font.font_size = size;
        }
    }
}
