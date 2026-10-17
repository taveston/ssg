//! The shared palette from `docs/spec/system-view.md` (MRK-01).

use bevy::prelude::Color;

use crate::sim::track::Affiliation;

pub const GROUND: Color = Color::srgb(
    0x07 as f32 / 255.0,
    0x0A as f32 / 255.0,
    0x0E as f32 / 255.0,
);
pub const PANEL: Color = Color::srgba(12.0 / 255.0, 17.0 / 255.0, 23.0 / 255.0, 0.97);
/// MUL-08: the tray's fill: the panel colour fully opaque, so labels and
/// belts on the map never show through a tray drawn over them.
pub const TRAY_FILL: Color = Color::srgb(12.0 / 255.0, 17.0 / 255.0, 23.0 / 255.0);
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
/// MRK-01: secondary text, and secondary buttons' outlines (BTN-01).
pub const SECONDARY: Color = ORBIT_LINE;
pub const OWN: Color = Color::srgb(
    0x5A as f32 / 255.0,
    0xA9 as f32 / 255.0,
    0xE6 as f32 / 255.0,
);
pub const WARNING: Color = Color::srgb(
    0xF0 as f32 / 255.0,
    0xA0 as f32 / 255.0,
    0x40 as f32 / 255.0,
);
/// MRK-01: the violet of the current frame, used for nothing else: the
/// frame cue (REF-01) and the breadcrumb entry for the frame (CAM-25).
pub const REFERENCE: Color = Color::srgb(
    0x9B as f32 / 255.0,
    0x8C as f32 / 255.0,
    0xE8 as f32 / 255.0,
);
/// MRK-01 / HOV-03 / HOV-06: the white of the selection and hover
/// brackets and of a highlighted orbit ring.
pub const SELECTION: Color = Color::WHITE;

/// INS-05 / MUL-09: the selected row's highlight in a list (the outline,
/// a tray): the secondary colour, faint.
pub const LIST_HIGHLIGHT: Color = Color::srgba(
    0x9F as f32 / 255.0,
    0xB6 as f32 / 255.0,
    0xC4 as f32 / 255.0,
    0.18,
);

/// SYM-02: the paler blue of an own-flag civilian ship.
pub const CIVILIAN: Color = Color::srgb(
    0xA9 as f32 / 255.0,
    0xCB as f32 / 255.0,
    0xE3 as f32 / 255.0,
);
/// SYM-02: allied and neutral ships and stations.
pub const NEUTRAL_GREEN: Color = Color::srgb(
    0x8C as f32 / 255.0,
    0xCB as f32 / 255.0,
    0x8A as f32 / 255.0,
);
/// SYM-02: hostile ships and stations.
pub const HOSTILE: Color = Color::srgb(
    0xEE as f32 / 255.0,
    0x6A as f32 / 255.0,
    0x5F as f32 / 255.0,
);
/// SYM-02: ships and stations of unknown affiliation, in the warning
/// orange.
pub const UNKNOWN: Color = WARNING;

/// SYM-02: a symbol's colour by affiliation, as frames and multi-markers
/// (MUL-02) use it. Own-flag civilians are paler (`CIVILIAN`).
pub fn symbol_color(affiliation: Affiliation) -> Color {
    match affiliation {
        Affiliation::Own => OWN,
        Affiliation::Allied | Affiliation::Neutral => NEUTRAL_GREEN,
        Affiliation::Hostile => HOSTILE,
        Affiliation::Unknown => UNKNOWN,
    }
}

/// `a` mixed with `b` at `m` (0 is all `a`, 1 all `b`), in sRGB.
pub fn mix(a: Color, b: Color, m: f32) -> Color {
    let (a, b) = (a.to_srgba(), b.to_srgba());
    let lerp = |x: f32, y: f32| x + (y - x) * m;
    Color::srgba(
        lerp(a.red, b.red),
        lerp(a.green, b.green),
        lerp(a.blue, b.blue),
        lerp(a.alpha, b.alpha),
    )
}
