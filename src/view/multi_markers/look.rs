//! What a multi-marker looks like, as values (MUL-01 to MUL-03): its
//! front and back frames' shapes, colours and insides, and its count's
//! text. Plain functions, tested apart from drawing; `draw.rs` turns a
//! `MultiLook` into nodes.

use bevy::color::Mix;
use bevy::prelude::*;

use crate::sim::body::{Affiliation, BodyClass};
use crate::view::colors;
use crate::view::symbols::{FrameShape, SymbolLook, SymbolSettings};

/// The largest count shown in figures; above it the count reads "99+"
/// (MUL-01).
const MAX_SHOWN_COUNT: usize = 99;

/// One frame of a multi-marker: its outline's shape and colour, and how
/// much of the colour is mixed into the ground for its opaque inside
/// (SYM-02, SYM-03).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameLook {
    pub shape: FrameShape,
    pub color: Color,
    pub fill_mix: f32,
}

impl FrameLook {
    /// SYM-03 / MUL-02: the opaque inside, the colour mixed into the
    /// ground.
    pub fn fill(&self) -> Color {
        colors::GROUND.mix(&self.color.with_alpha(1.0), self.fill_mix)
    }
}

/// What a multi-marker shows: its two frames and its count. A station
/// marker's front frame is the station's own symbol, drawn by its marker,
/// so for one only the back frame and the count are drawn from the look;
/// `front` gives the count its colour (MUL-03).
#[derive(Debug, Clone, PartialEq)]
pub struct MultiLook {
    pub front: FrameLook,
    pub back: FrameLook,
    pub count: String,
}

impl MultiLook {
    /// MUL-01 / MUL-02: the look of a marker holding things of these
    /// classes and affiliations, its count reading `count`.
    pub fn of_things(
        things: &[(BodyClass, Affiliation)],
        count: usize,
        settings: SymbolSettings,
    ) -> Self {
        let (front, back) = multi_frames(things, settings);
        Self {
            front,
            back,
            count: count_text(count),
        }
    }

    /// MUL-03: a station marker's look: the station's own frame in front,
    /// the back frame over what is docked, and the count of the `count`
    /// things docked.
    pub fn of_station(
        station: &SymbolLook,
        docked: &[(BodyClass, Affiliation)],
        count: usize,
        settings: SymbolSettings,
    ) -> Self {
        let (front, back) = station_frames(station, docked, settings);
        Self {
            front,
            back,
            count: count_text(count),
        }
    }
}

/// MUL-01: the count's text: the figure, or "99+" above 99.
fn count_text(count: usize) -> String {
    if count > MAX_SHOWN_COUNT {
        format!("{MAX_SHOWN_COUNT}+")
    } else {
        count.to_string()
    }
}

/// MUL-02: how pressing an affiliation is for the front frame: hostile,
/// then unknown, then neutral or allied, then own.
fn urgency(affiliation: Affiliation) -> u8 {
    match affiliation {
        Affiliation::Hostile => 3,
        Affiliation::Unknown => 2,
        Affiliation::Neutral | Affiliation::Allied => 1,
        Affiliation::Own => 0,
    }
}

/// MUL-02: the frame an affiliation is drawn with: its shape and colour
/// (SYM-02), an own-flag civilian counting as own, and the SYM-03 inside
/// for it.
pub fn affiliation_frame(affiliation: Affiliation, settings: SymbolSettings) -> FrameLook {
    let shape = match affiliation {
        Affiliation::Own => FrameShape::Circle,
        Affiliation::Allied | Affiliation::Neutral => FrameShape::Square,
        Affiliation::Hostile => FrameShape::Diamond,
        Affiliation::Unknown => FrameShape::Quatrefoil,
    };
    let fill_mix = match affiliation {
        Affiliation::Own => settings.fill_faction,
        _ => settings.fill_other,
    };
    FrameLook {
        shape,
        color: colors::symbol_color(affiliation),
        fill_mix,
    }
}

/// Whether a thing shows affiliation on a multi-marker: ships, groups
/// and stations do; natural bodies don't (MUL-02).
fn has_symbol(class: BodyClass) -> bool {
    matches!(class, BodyClass::Ship | BodyClass::Station)
}

/// MUL-02: the back frame: the own frame when any of the things is own,
/// else a copy of `front`.
fn back_frame(
    things: &[(BodyClass, Affiliation)],
    front: FrameLook,
    settings: SymbolSettings,
) -> FrameLook {
    let any_own = things
        .iter()
        .any(|&(class, affiliation)| has_symbol(class) && affiliation == Affiliation::Own);
    if any_own {
        affiliation_frame(Affiliation::Own, settings)
    } else {
        front
    }
}

/// MUL-02: a multi-marker's front and back frames from the class and
/// affiliation of each thing it holds. Natural bodies are left out. A
/// multi-marker always holds a ship, group or station; were it to hold
/// none, it would show as unknown.
fn multi_frames(
    things: &[(BodyClass, Affiliation)],
    settings: SymbolSettings,
) -> (FrameLook, FrameLook) {
    let pressing = things
        .iter()
        .filter(|(class, _)| has_symbol(*class))
        .map(|&(_, affiliation)| affiliation)
        .max_by_key(|&affiliation| urgency(affiliation))
        .unwrap_or(Affiliation::Unknown);
    let front = affiliation_frame(pressing, settings);
    (front, back_frame(things, front, settings))
}

/// MUL-03: a station marker's frames: the station's own frame in front,
/// whatever is docked, and the back frame by MUL-02 over the docked
/// things.
fn station_frames(
    station: &SymbolLook,
    docked: &[(BodyClass, Affiliation)],
    settings: SymbolSettings,
) -> (FrameLook, FrameLook) {
    let front = FrameLook {
        shape: station.frame,
        color: station.color,
        fill_mix: station.fill_mix,
    };
    (front, back_frame(docked, front, settings))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::track::{ShipClass, ShipRecord};
    use crate::view::symbols::symbol_look;

    const SETTINGS: SymbolSettings = SymbolSettings {
        fill_faction: 0.30,
        fill_other: 0.16,
        stale_alpha: 0.55,
        condition_warn: 0.5,
    };

    /// MUL-01: the count is the figure up to 99, then "99+".
    #[test]
    fn counts_above_ninety_nine_read_99_plus() {
        assert_eq!(count_text(1), "1");
        assert_eq!(count_text(99), "99");
        assert_eq!(count_text(100), "99+");
    }

    /// MUL-02: the front frame takes the most pressing affiliation, in
    /// the order hostile, unknown, neutral or allied, own; the back frame
    /// is own when any thing is own, otherwise a copy of the front; each
    /// inside is the SYM-03 mix for its affiliation.
    #[test]
    fn the_frames_follow_each_mix_of_affiliations() {
        use Affiliation::*;
        let ship = |a| (BodyClass::Ship, a);
        let own = affiliation_frame(Own, SETTINGS);
        let cases = [
            (vec![ship(Own), ship(Own)], Own, Some(Own)),
            (vec![ship(Own), ship(Neutral)], Neutral, Some(Own)),
            (vec![ship(Allied), ship(Unknown)], Unknown, None),
            (
                vec![ship(Unknown), ship(Hostile), ship(Own)],
                Hostile,
                Some(Own),
            ),
            (vec![ship(Neutral), ship(Hostile)], Hostile, None),
        ];
        for (things, front, back) in cases {
            let (f, b) = multi_frames(&things, SETTINGS);
            assert_eq!(f, affiliation_frame(front, SETTINGS), "{things:?}");
            match back {
                Some(_) => assert_eq!(b, own, "{things:?}"),
                None => assert_eq!(b, f, "{things:?}"),
            }
        }
        assert_eq!(own.fill_mix, SETTINGS.fill_faction);
        assert_eq!(own.shape, FrameShape::Circle);
        let hostile = affiliation_frame(Hostile, SETTINGS);
        assert_eq!(
            (hostile.shape, hostile.color, hostile.fill_mix),
            (FrameShape::Diamond, colors::HOSTILE, SETTINGS.fill_other)
        );
        assert_eq!(
            affiliation_frame(Allied, SETTINGS).shape,
            FrameShape::Square
        );
        assert_eq!(
            affiliation_frame(Unknown, SETTINGS).shape,
            FrameShape::Quatrefoil
        );
    }

    /// MUL-02: natural bodies don't count for affiliation.
    #[test]
    fn natural_bodies_never_colour_the_frames() {
        // A hostile moon doesn't turn a marker of own ships red.
        let mixed = [
            (BodyClass::Moon, Affiliation::Hostile),
            (BodyClass::Ship, Affiliation::Own),
        ];
        assert_eq!(
            multi_frames(&mixed, SETTINGS).0,
            affiliation_frame(Affiliation::Own, SETTINGS)
        );
    }

    /// MUL-03: a station marker keeps the station's own frame whatever is
    /// docked; its back frame is own when an own thing is docked, else a
    /// copy of the station's.
    #[test]
    fn a_station_marker_keeps_its_own_frame() {
        let station = symbol_look(
            Affiliation::Neutral,
            &ShipRecord::new(ShipClass::Station, false),
            SETTINGS,
        );
        let hostile = [(BodyClass::Ship, Affiliation::Hostile)];
        let (front, back) = station_frames(&station, &hostile, SETTINGS);
        assert_eq!((front.shape, front.color), (station.frame, station.color));
        assert_eq!(back, front);
        let mixed = [
            (BodyClass::Ship, Affiliation::Hostile),
            (BodyClass::Ship, Affiliation::Own),
        ];
        let (_, back) = station_frames(&station, &mixed, SETTINGS);
        assert_eq!(back, affiliation_frame(Affiliation::Own, SETTINGS));
    }

    /// SYM-03: an inside is opaque, its colour mixed into the ground.
    #[test]
    fn insides_are_opaque() {
        let frame = affiliation_frame(Affiliation::Hostile, SETTINGS);
        assert_eq!(frame.fill().alpha(), 1.0);
        assert_ne!(frame.fill(), colors::GROUND);
    }
}
