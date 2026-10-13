//! Ship and station symbols as data (SYM-02 to SYM-08, SYM-13, SYM-15):
//! from what the state knows of an object (`ShipRecord`), which frame,
//! colour, fill, icon and amplifiers its symbol has, what its label's
//! second line says, and the words used for its ownership. Adapted from
//! NATO APP-6: the frame shape shows affiliation, the icon what it is, and
//! amplifiers round the frame its state.
//!
//! Plain functions on plain values, so every rule is tested apart from
//! drawing; `symbol_draw.rs` turns a `SymbolLook` into nodes.

use bevy::prelude::*;

use super::colors;
use crate::sim::track::{Affiliation, Echelon, ShipClass, ShipRecord, Track, TroopLoad};

/// SYM-02: the frame's shape, by affiliation.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum FrameShape {
    Circle,
    Square,
    Diamond,
    Quatrefoil,
}

/// SYM-05: the icon inside the frame.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Icon {
    /// Filled triangle: a warship, or anything known to be armed.
    Warship,
    /// Outline drop: fleet supply or tanker.
    Drop,
    /// Outline crate: a box with a horizontal centre line.
    Crate,
    /// Outline unit box; `crossed` draws both diagonals (embarked troops).
    UnitBox { crossed: bool },
    /// "?" in JetBrains Mono: class unknown.
    Question,
    /// A station has no icon.
    None,
}

/// What a symbol looks like, as values.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SymbolLook {
    pub frame: FrameShape,
    pub color: Color,
    /// The fill's alpha: a tint of `color`, or 0 for no fill (SYM-03).
    pub fill_alpha: f32,
    pub icon: Icon,
    /// A weak or stale track has a dashed frame (SYM-08).
    pub dashed: bool,
    /// The whole symbol's opacity: lower for a stale track (SYM-08).
    pub alpha: f32,
    /// The echelon text above the frame, if any (SYM-06).
    pub echelon: Option<&'static str>,
    /// The NATO installation bar on top of the frame (SYM-07).
    pub installation_bar: bool,
}

/// The tunables a look depends on.
#[derive(Clone, Copy, Debug)]
pub struct SymbolSettings {
    pub fill_state: f32,
    pub fill_other: f32,
    pub stale_alpha: f32,
}

/// Whether a record is an own-flag civilian: own, but not commanded.
pub fn own_flag_civilian(affiliation: Affiliation, record: &ShipRecord) -> bool {
    affiliation == Affiliation::Own && !record.commanded
}

/// SYM-02: frame shape and colour by affiliation; an own-flag civilian is
/// paler blue. A requisitioned civilian is commanded, so state blue.
pub fn frame_and_color(affiliation: Affiliation, record: &ShipRecord) -> (FrameShape, Color) {
    let shape = match affiliation {
        Affiliation::Own => FrameShape::Circle,
        Affiliation::Allied | Affiliation::Neutral => FrameShape::Square,
        Affiliation::Hostile => FrameShape::Diamond,
        Affiliation::Unknown => FrameShape::Quatrefoil,
    };
    let color = if own_flag_civilian(affiliation, record) {
        colors::CIVILIAN
    } else {
        colors::symbol_color(affiliation)
    };
    (shape, color)
}

/// SYM-03: fill shows command. A commanded own object has the state tint,
/// an own-flag civilian none, any other affiliation the lighter tint.
pub fn fill_alpha(affiliation: Affiliation, record: &ShipRecord, settings: SymbolSettings) -> f32 {
    match affiliation {
        Affiliation::Own if record.commanded => settings.fill_state,
        Affiliation::Own => 0.0,
        _ => settings.fill_other,
    }
}

/// SYM-05: the icon by class. Known to be armed shows the warship icon
/// whatever the class, so filled always means armed; a station has none.
pub fn icon(record: &ShipRecord) -> Icon {
    if record.class == ShipClass::Station {
        return Icon::None;
    }
    if record.armed {
        return Icon::Warship;
    }
    match record.class {
        ShipClass::Warship => Icon::Warship,
        ShipClass::FleetSupply => Icon::Drop,
        ShipClass::Cargo => Icon::Crate,
        ShipClass::TroopTransport => Icon::UnitBox {
            crossed: matches!(record.load, TroopLoad::Embarked(_)),
        },
        ShipClass::Unknown => Icon::Question,
        ShipClass::Station => Icon::None,
    }
}

/// SYM-06: the NATO echelon mark.
pub fn echelon_mark(echelon: Echelon) -> &'static str {
    match echelon {
        Echelon::Battalion => "II",
        Echelon::Regiment => "III",
        Echelon::Brigade => "X",
    }
}

/// SYM-06: what sits above a troop transport's frame: "?" for an unknown
/// load, nothing when known empty, and the largest embarked formation's
/// echelon (or "?" if its size isn't known) when troops are embarked.
pub fn echelon(record: &ShipRecord) -> Option<&'static str> {
    if !matches!(icon(record), Icon::UnitBox { .. }) {
        return None;
    }
    match &record.load {
        TroopLoad::Unknown => Some("?"),
        TroopLoad::Empty => None,
        TroopLoad::Embarked(formations) => {
            // With any size unknown, the largest isn't known either.
            let sizes: Option<Vec<Echelon>> = formations.iter().map(|f| f.echelon).collect();
            Some(
                sizes
                    .and_then(|s| s.into_iter().max())
                    .map_or("?", echelon_mark),
            )
        }
    }
}

/// SYM-02 to SYM-08: the whole look of a symbol from what the state knows.
pub fn symbol_look(
    affiliation: Affiliation,
    record: &ShipRecord,
    settings: SymbolSettings,
) -> SymbolLook {
    let (frame, color) = frame_and_color(affiliation, record);
    SymbolLook {
        frame,
        color,
        fill_alpha: fill_alpha(affiliation, record, settings),
        icon: icon(record),
        dashed: !matches!(record.track, Track::Firm),
        alpha: match record.track {
            Track::Stale { .. } => settings.stale_alpha,
            _ => 1.0,
        },
        echelon: echelon(record),
        installation_bar: record.class == ShipClass::Station,
    }
}

/// SYM-08: a stale track's age, "last seen *N* d ago", in whole days of
/// game time.
pub fn last_seen_text(last_observed_day: f64, now_day: f64) -> String {
    let days = (now_day - last_observed_day).max(0.0).floor() as i64;
    format!("last seen {days} d ago")
}

/// SYM-13: the label's second line, in the parts that apply joined by
/// " · ": embarked formations, "own flag · civilian" or "requisitioned",
/// and a stale track's age. `None` when nothing applies.
pub fn label_second_line(
    affiliation: Affiliation,
    record: &ShipRecord,
    now_day: f64,
) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    if let TroopLoad::Embarked(formations) = &record.load {
        parts.extend(formations.iter().map(|f| f.name.to_string()));
    }
    if own_flag_civilian(affiliation, record) {
        parts.push("own flag · civilian".to_string());
    } else if record.requisitioned {
        parts.push("requisitioned".to_string());
    }
    if let Track::Stale { last_observed_day } = record.track {
        parts.push(last_seen_text(last_observed_day, now_day));
    }
    (!parts.is_empty()).then(|| parts.join(" · "))
}

/// SYM-15: ownership as the tooltip and inset say it: "own", "own flag ·
/// civilian", "requisitioned", "allied", "neutral", "hostile" or
/// "unknown". A body with no ship record is just its affiliation.
pub fn ownership_text(affiliation: Affiliation, record: Option<&ShipRecord>) -> &'static str {
    match (affiliation, record) {
        (Affiliation::Own, Some(record)) if record.requisitioned => "requisitioned",
        (Affiliation::Own, Some(record)) if !record.commanded => "own flag · civilian",
        (Affiliation::Own, _) => "own",
        (Affiliation::Allied, _) => "allied",
        (Affiliation::Neutral, _) => "neutral",
        (Affiliation::Hostile, _) => "hostile",
        (Affiliation::Unknown, _) => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::track::Formation;

    const SETTINGS: SymbolSettings = SymbolSettings {
        fill_state: 0.30,
        fill_other: 0.16,
        stale_alpha: 0.55,
    };

    fn ship(class: ShipClass, commanded: bool) -> ShipRecord {
        ShipRecord::new(class, commanded)
    }

    /// SYM-02: frame and colour by affiliation; own-flag civilians are
    /// paler blue.
    #[test]
    fn frames_and_colours_follow_affiliation() {
        let warship = ship(ShipClass::Warship, true);
        let cases = [
            (Affiliation::Own, FrameShape::Circle, colors::OWN),
            (
                Affiliation::Allied,
                FrameShape::Square,
                colors::NEUTRAL_GREEN,
            ),
            (
                Affiliation::Neutral,
                FrameShape::Square,
                colors::NEUTRAL_GREEN,
            ),
            (Affiliation::Hostile, FrameShape::Diamond, colors::HOSTILE),
            (
                Affiliation::Unknown,
                FrameShape::Quatrefoil,
                colors::UNKNOWN,
            ),
        ];
        for (affiliation, frame, color) in cases {
            assert_eq!(frame_and_color(affiliation, &warship), (frame, color));
        }
        let civilian = ship(ShipClass::Cargo, false);
        assert_eq!(
            frame_and_color(Affiliation::Own, &civilian),
            (FrameShape::Circle, colors::CIVILIAN)
        );
    }

    /// SYM-03: commanded own objects take the state fill, own-flag
    /// civilians none, and a requisitioned civilian the state blue and
    /// fill; every other affiliation the lighter tint.
    #[test]
    fn fill_shows_command() {
        let cargo = ship(ShipClass::Cargo, false);
        assert_eq!(fill_alpha(Affiliation::Own, &cargo, SETTINGS), 0.0);
        let requisitioned = ShipRecord {
            commanded: true,
            requisitioned: true,
            ..cargo.clone()
        };
        assert_eq!(fill_alpha(Affiliation::Own, &requisitioned, SETTINGS), 0.30);
        assert_eq!(
            frame_and_color(Affiliation::Own, &requisitioned).1,
            colors::OWN
        );
        assert_eq!(fill_alpha(Affiliation::Hostile, &cargo, SETTINGS), 0.16);
        assert_eq!(fill_alpha(Affiliation::Neutral, &cargo, SETTINGS), 0.16);
    }

    /// SYM-05: icons by class; known armed means the warship icon whatever
    /// the class; a station has none.
    #[test]
    fn icons_follow_class_and_armed_means_filled() {
        assert_eq!(icon(&ship(ShipClass::Warship, true)), Icon::Warship);
        assert_eq!(icon(&ship(ShipClass::FleetSupply, true)), Icon::Drop);
        assert_eq!(icon(&ship(ShipClass::Cargo, true)), Icon::Crate);
        assert_eq!(icon(&ship(ShipClass::Unknown, true)), Icon::Question);
        assert_eq!(icon(&ship(ShipClass::Station, true)), Icon::None);
        let armed_hauler = ShipRecord {
            armed: true,
            ..ship(ShipClass::Cargo, false)
        };
        assert_eq!(icon(&armed_hauler), Icon::Warship);
    }

    fn transport(load: TroopLoad) -> ShipRecord {
        ShipRecord {
            load,
            ..ship(ShipClass::TroopTransport, true)
        }
    }

    /// SYM-06: unknown load: empty box and "?"; known empty: empty box,
    /// nothing above; embarked: crossed box and the largest echelon, or
    /// "?" when a size is unknown.
    #[test]
    fn a_troop_transport_shows_its_load() {
        let unknown = transport(TroopLoad::Unknown);
        assert_eq!(icon(&unknown), Icon::UnitBox { crossed: false });
        assert_eq!(echelon(&unknown), Some("?"));
        let empty = transport(TroopLoad::Empty);
        assert_eq!(icon(&empty), Icon::UnitBox { crossed: false });
        assert_eq!(echelon(&empty), None);
        let guard = Formation {
            name: "1st Arden Guard",
            echelon: Some(Echelon::Battalion),
        };
        let embarked = transport(TroopLoad::Embarked(vec![guard.clone()]));
        assert_eq!(icon(&embarked), Icon::UnitBox { crossed: true });
        assert_eq!(echelon(&embarked), Some("II"));
        let bigger = Formation {
            name: "3rd Brigade",
            echelon: Some(Echelon::Brigade),
        };
        let both = transport(TroopLoad::Embarked(vec![guard.clone(), bigger]));
        assert_eq!(echelon(&both), Some("X"));
        let size_unknown = Formation {
            name: "Unknown unit",
            echelon: None,
        };
        let mixed = transport(TroopLoad::Embarked(vec![guard, size_unknown]));
        assert_eq!(echelon(&mixed), Some("?"));
        assert_eq!(echelon(&ship(ShipClass::Cargo, true)), None);
    }

    /// SYM-07: a station has the installation bar and no icon.
    #[test]
    fn a_station_has_the_installation_bar() {
        let look = symbol_look(Affiliation::Own, &ship(ShipClass::Station, true), SETTINGS);
        assert!(look.installation_bar);
        assert_eq!(look.icon, Icon::None);
        let ship_look = symbol_look(Affiliation::Own, &ship(ShipClass::Cargo, true), SETTINGS);
        assert!(!ship_look.installation_bar);
    }

    /// SYM-08: firm is solid; weak is dashed; stale is dashed and dimmed;
    /// track state changes nothing else.
    #[test]
    fn track_state_dashes_and_dims() {
        let base = ship(ShipClass::Warship, false);
        let with = |track| ShipRecord {
            track,
            ..base.clone()
        };
        let firm = symbol_look(Affiliation::Hostile, &with(Track::Firm), SETTINGS);
        let weak = symbol_look(Affiliation::Hostile, &with(Track::Weak), SETTINGS);
        let stale = symbol_look(
            Affiliation::Hostile,
            &with(Track::Stale {
                last_observed_day: -6.0,
            }),
            SETTINGS,
        );
        assert!(!firm.dashed && firm.alpha == 1.0);
        assert!(weak.dashed && weak.alpha == 1.0);
        assert!(stale.dashed && stale.alpha == 0.55);
        assert_eq!((weak.frame, weak.icon), (firm.frame, firm.icon));
    }

    /// SYM-08: the stale age is whole days of game time.
    #[test]
    fn the_stale_age_is_whole_days() {
        assert_eq!(last_seen_text(-6.0, 0.0), "last seen 6 d ago");
        assert_eq!(last_seen_text(-6.0, 0.9), "last seen 6 d ago");
        assert_eq!(last_seen_text(-6.0, 1.0), "last seen 7 d ago");
    }

    /// SYM-13: the second line joins what applies, and is absent when
    /// nothing does.
    #[test]
    fn the_second_line_joins_what_applies() {
        let embarked = transport(TroopLoad::Embarked(vec![Formation {
            name: "1st Arden Guard",
            echelon: Some(Echelon::Battalion),
        }]));
        assert_eq!(
            label_second_line(Affiliation::Own, &embarked, 0.0).as_deref(),
            Some("1st Arden Guard")
        );
        let civilian = ship(ShipClass::Cargo, false);
        assert_eq!(
            label_second_line(Affiliation::Own, &civilian, 0.0).as_deref(),
            Some("own flag · civilian")
        );
        let requisitioned = ShipRecord {
            commanded: true,
            requisitioned: true,
            ..civilian
        };
        assert_eq!(
            label_second_line(Affiliation::Own, &requisitioned, 0.0).as_deref(),
            Some("requisitioned")
        );
        let stale = ShipRecord {
            track: Track::Stale {
                last_observed_day: -6.0,
            },
            ..ship(ShipClass::Warship, false)
        };
        assert_eq!(
            label_second_line(Affiliation::Hostile, &stale, 0.0).as_deref(),
            Some("last seen 6 d ago")
        );
        let plain = ship(ShipClass::Warship, true);
        assert_eq!(label_second_line(Affiliation::Own, &plain, 0.0), None);
    }

    /// SYM-15: the ownership words.
    #[test]
    fn ownership_words() {
        let commanded = ship(ShipClass::Warship, true);
        let civilian = ship(ShipClass::Cargo, false);
        let requisitioned = ShipRecord {
            commanded: true,
            requisitioned: true,
            ..civilian.clone()
        };
        assert_eq!(ownership_text(Affiliation::Own, Some(&commanded)), "own");
        assert_eq!(
            ownership_text(Affiliation::Own, Some(&civilian)),
            "own flag · civilian"
        );
        assert_eq!(
            ownership_text(Affiliation::Own, Some(&requisitioned)),
            "requisitioned"
        );
        assert_eq!(ownership_text(Affiliation::Own, None), "own");
        for (affiliation, word) in [
            (Affiliation::Allied, "allied"),
            (Affiliation::Neutral, "neutral"),
            (Affiliation::Hostile, "hostile"),
            (Affiliation::Unknown, "unknown"),
        ] {
            assert_eq!(ownership_text(affiliation, None), word);
        }
    }
}
