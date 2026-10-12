//! The shared palette from `docs/spec/system-view.md` (MRK-01).

use bevy::prelude::Color;

use crate::sim::body::Ownership;

pub const GROUND: Color = Color::srgb(
    0x07 as f32 / 255.0,
    0x0A as f32 / 255.0,
    0x0E as f32 / 255.0,
);
pub const PANEL: Color = Color::srgba(12.0 / 255.0, 17.0 / 255.0, 23.0 / 255.0, 0.97);
pub const PANEL_BORDER: Color = Color::srgb(
    0x9F as f32 / 255.0,
    0xB6 as f32 / 255.0,
    0xC4 as f32 / 255.0,
);

pub const NEUTRAL: Color = Color::srgb(
    0xDC as f32 / 255.0,
    0xE6 as f32 / 255.0,
    0xEC as f32 / 255.0,
);
pub const ORBIT_LINE: Color = Color::srgb(
    0x9F as f32 / 255.0,
    0xB6 as f32 / 255.0,
    0xC4 as f32 / 255.0,
);
pub const OWN: Color = Color::srgb(
    0x5A as f32 / 255.0,
    0xA9 as f32 / 255.0,
    0xE6 as f32 / 255.0,
);
pub const FOREIGN: Color = Color::srgb(
    0xF0 as f32 / 255.0,
    0xA0 as f32 / 255.0,
    0x40 as f32 / 255.0,
);
pub const SELECTION_RING: Color = Color::WHITE;

/// A marker's colour says whose it is: own blue, foreign orange,
/// neutral the pale display white.
pub fn ownership_color(ownership: Ownership) -> Color {
    match ownership {
        Ownership::Own => OWN,
        Ownership::Foreign => FOREIGN,
        Ownership::Neutral => NEUTRAL,
    }
}
