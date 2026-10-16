//! Ship, group and station symbols as data (SYM-02 to SYM-09, SYM-12,
//! SYM-13, SYM-15): from what the faction knows of an object
//! (`ShipRecord`), which frame, colour, fill, icon and amplifiers (the
//! condition bar, and a group's count and task-force bracket) its symbol
//! has, what its label's second line says, and the words used for its
//! ownership. Adapted from
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
    /// "?" in the data face: class unknown.
    Question,
    /// A station has no icon.
    None,
}

/// What a symbol looks like, as values.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SymbolLook {
    pub frame: FrameShape,
    pub color: Color,
    /// How much of `color` is mixed into the ground colour for the frame's
    /// opaque inside: 0 is plain ground (SYM-03).
    pub fill_mix: f32,
    pub icon: Icon,
    /// A weak or stale track has a dashed frame (SYM-08).
    pub dashed: bool,
    /// The whole symbol's opacity: lower for a stale track (SYM-08).
    pub alpha: f32,
    /// The echelon text above the frame, if any (SYM-06).
    pub echelon: Option<&'static str>,
    /// The NATO installation bar on top of the frame (SYM-07).
    pub installation_bar: bool,
    /// The condition bar under the frame of an own ship (SYM-12).
    pub condition: Option<ConditionBar>,
    /// A group's member count, just outside the frame's upper right
    /// (SYM-09).
    pub count: Option<usize>,
    /// A task force's bracket above the frame, above any echelon (SYM-09).
    pub task_force: bool,
}

/// SYM-09: what marks a symbol as a group's: how many members it has and
/// whether it is a task force.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct GroupFace {
    pub count: usize,
    pub task_force: bool,
}

/// SYM-12: an own ship's or group's condition bar: how full it is and the
/// colour it is filled in.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ConditionBar {
    /// The filled share of the bar, 0 to 1.
    pub fill: f32,
    pub color: Color,
}

/// The tunables a look depends on.
#[derive(Clone, Copy, Debug)]
pub struct SymbolSettings {
    pub fill_faction: f32,
    pub fill_other: f32,
    pub stale_alpha: f32,
    /// Below this condition the bar turns orange (SYM-12).
    pub condition_warn: f32,
}

/// Whether a record is an own-flag civilian: own, but not commanded.
pub fn own_flag_civilian(affiliation: Affiliation, record: &ShipRecord) -> bool {
    affiliation == Affiliation::Own && !record.commanded
}

/// SYM-02: frame shape and colour by affiliation; an own-flag civilian is
/// paler blue. A requisitioned civilian is commanded, so faction blue.
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

/// SYM-03: fill shows command. A commanded own object mixes its colour in
/// at the faction fraction, an own-flag civilian not at all, any other
/// affiliation at the lighter fraction.
pub fn fill_mix(affiliation: Affiliation, record: &ShipRecord, settings: SymbolSettings) -> f32 {
    match affiliation {
        Affiliation::Own if record.commanded => settings.fill_faction,
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

/// SYM-12: the condition bar of an own ship or group (commanded or
/// civilian) with a known condition, filled in the symbol's `color`, or in
/// the warning orange below the warning fraction. Other affiliations have
/// none.
pub fn condition_bar(
    affiliation: Affiliation,
    condition: Option<f64>,
    color: Color,
    settings: SymbolSettings,
) -> Option<ConditionBar> {
    if affiliation != Affiliation::Own {
        return None;
    }
    let fill = condition?.clamp(0.0, 1.0) as f32;
    let color = if fill < settings.condition_warn {
        colors::WARNING
    } else {
        color
    };
    Some(ConditionBar { fill, color })
}

/// SYM-02 to SYM-08, SYM-12: the whole look of a symbol from what the faction knows.
pub fn symbol_look(
    affiliation: Affiliation,
    record: &ShipRecord,
    settings: SymbolSettings,
) -> SymbolLook {
    let (frame, color) = frame_and_color(affiliation, record);
    SymbolLook {
        frame,
        color,
        fill_mix: fill_mix(affiliation, record, settings),
        icon: icon(record),
        dashed: !matches!(record.track, Track::Firm),
        alpha: match record.track {
            Track::Stale { .. } => settings.stale_alpha,
            _ => 1.0,
        },
        echelon: echelon(record),
        installation_bar: record.class == ShipClass::Station,
        condition: condition_bar(affiliation, record.condition, color, settings),
        count: None,
        task_force: false,
    }
}

/// SYM-09: a group's symbol is its record's look with the member count at
/// the upper right and, for a task force, the bracket above. A ship's look
/// (`group` is `None`) is unchanged.
pub fn with_group(look: SymbolLook, group: Option<GroupFace>) -> SymbolLook {
    match group {
        Some(group) => SymbolLook {
            count: Some(group.count),
            task_force: group.task_force,
            ..look
        },
        None => look,
    }
}

/// SYM-13: a class's name in a group's member count, singular and plural.
fn class_noun(class: ShipClass) -> (&'static str, &'static str) {
    match class {
        ShipClass::Warship => ("warship", "warships"),
        ShipClass::TroopTransport => ("troop transport", "troop transports"),
        ShipClass::FleetSupply => ("supply ship", "supply ships"),
        ShipClass::Cargo => ("cargo ship", "cargo ships"),
        ShipClass::Station => ("station", "stations"),
        ShipClass::Unknown => ("unknown ship", "unknown ships"),
    }
}

/// The order classes are listed in a member count: the order of their
/// icons' significance (SYM-09).
const CLASS_ORDER: [ShipClass; 6] = [
    ShipClass::Warship,
    ShipClass::TroopTransport,
    ShipClass::FleetSupply,
    ShipClass::Cargo,
    ShipClass::Unknown,
    ShipClass::Station,
];

/// SYM-13: a group's member count by class, most significant class first,
/// the classes joined by ", ": "4 warships", "2 warships, 1 supply ship".
/// `None` for no members.
pub fn member_count_text(classes: &[ShipClass]) -> Option<String> {
    let parts: Vec<String> = CLASS_ORDER
        .iter()
        .filter_map(|&class| {
            let n = classes.iter().filter(|&&c| c == class).count();
            let (one, many) = class_noun(class);
            (n > 0).then(|| format!("{n} {}", if n == 1 { one } else { many }))
        })
        .collect();
    (!parts.is_empty()).then(|| parts.join(", "))
}

/// SYM-08: a stale track's age, "last seen *N* d ago", in whole days of
/// game time.
pub fn last_seen_text(last_observed_day: f64, now_day: f64) -> String {
    let days = (now_day - last_observed_day).max(0.0).floor() as i64;
    format!("last seen {days} d ago")
}

/// SYM-13: the label's second line, in the parts that apply joined by
/// " · ": a group's member count by class (`members` holds its members'
/// classes, and is empty for a ship), embarked formations, "own flag ·
/// civilian" or "requisitioned", and a stale track's age. `None` when
/// nothing applies.
pub fn label_second_line(
    affiliation: Affiliation,
    record: &ShipRecord,
    members: &[ShipClass],
    now_day: f64,
) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    parts.extend(member_count_text(members));
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

/// SYM-15: ownership as the inset says it: "own", "own flag ·
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
        fill_faction: 0.30,
        fill_other: 0.16,
        stale_alpha: 0.55,
        condition_warn: 0.5,
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

    /// SYM-03: commanded own objects take the faction fill, own-flag
    /// civilians plain ground, and a requisitioned civilian the faction blue
    /// and fill; every other affiliation the lighter mix.
    #[test]
    fn fill_shows_command() {
        let cargo = ship(ShipClass::Cargo, false);
        assert_eq!(fill_mix(Affiliation::Own, &cargo, SETTINGS), 0.0);
        let requisitioned = ShipRecord {
            commanded: true,
            requisitioned: true,
            ..cargo.clone()
        };
        assert_eq!(fill_mix(Affiliation::Own, &requisitioned, SETTINGS), 0.30);
        assert_eq!(
            frame_and_color(Affiliation::Own, &requisitioned).1,
            colors::OWN
        );
        assert_eq!(fill_mix(Affiliation::Hostile, &cargo, SETTINGS), 0.16);
        assert_eq!(fill_mix(Affiliation::Neutral, &cargo, SETTINGS), 0.16);
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
            label_second_line(Affiliation::Own, &embarked, &[], 0.0).as_deref(),
            Some("1st Arden Guard")
        );
        let civilian = ship(ShipClass::Cargo, false);
        assert_eq!(
            label_second_line(Affiliation::Own, &civilian, &[], 0.0).as_deref(),
            Some("own flag · civilian")
        );
        let requisitioned = ShipRecord {
            commanded: true,
            requisitioned: true,
            ..civilian
        };
        assert_eq!(
            label_second_line(Affiliation::Own, &requisitioned, &[], 0.0).as_deref(),
            Some("requisitioned")
        );
        let stale = ShipRecord {
            track: Track::Stale {
                last_observed_day: -6.0,
            },
            ..ship(ShipClass::Warship, false)
        };
        assert_eq!(
            label_second_line(Affiliation::Hostile, &stale, &[], 0.0).as_deref(),
            Some("last seen 6 d ago")
        );
        let plain = ship(ShipClass::Warship, true);
        assert_eq!(label_second_line(Affiliation::Own, &plain, &[], 0.0), None);
    }

    /// SYM-12: own ships with a condition have a bar filled to it in the
    /// symbol's colour, orange below the warning fraction; civilians too,
    /// in their paler blue; other affiliations have none.
    #[test]
    fn own_ships_show_their_condition_orange_when_low() {
        let with = |condition, commanded| ShipRecord {
            condition: Some(condition),
            ..ship(ShipClass::Cargo, commanded)
        };
        let healthy = symbol_look(Affiliation::Own, &with(0.8, true), SETTINGS);
        assert_eq!(
            healthy.condition,
            Some(ConditionBar {
                fill: 0.8,
                color: colors::OWN
            })
        );
        let civilian = symbol_look(Affiliation::Own, &with(0.5, false), SETTINGS);
        assert_eq!(
            civilian.condition,
            Some(ConditionBar {
                fill: 0.5,
                color: colors::CIVILIAN
            })
        );
        let low = symbol_look(Affiliation::Own, &with(0.3, true), SETTINGS);
        assert_eq!(
            low.condition,
            Some(ConditionBar {
                fill: 0.3,
                color: colors::WARNING
            })
        );
        let hostile = symbol_look(Affiliation::Hostile, &with(0.8, false), SETTINGS);
        assert_eq!(hostile.condition, None);
        let unknown = symbol_look(Affiliation::Own, &ship(ShipClass::Cargo, true), SETTINGS);
        assert_eq!(unknown.condition, None);
    }

    /// SYM-09: a group's look is its record's, with its member count and,
    /// for a task force, the bracket; a ship's look has neither.
    #[test]
    fn a_group_adds_its_count_and_bracket() {
        let ship_look = symbol_look(Affiliation::Own, &ship(ShipClass::Warship, true), SETTINGS);
        assert_eq!((ship_look.count, ship_look.task_force), (None, false));
        assert_eq!(with_group(ship_look, None), ship_look);
        let face = GroupFace {
            count: 4,
            task_force: true,
        };
        let group = with_group(ship_look, Some(face));
        assert_eq!((group.count, group.task_force), (Some(4), true));
        assert_eq!((group.frame, group.icon), (ship_look.frame, ship_look.icon));
        let loose = with_group(
            ship_look,
            Some(GroupFace {
                count: 2,
                task_force: false,
            }),
        );
        assert_eq!((loose.count, loose.task_force), (Some(2), false));
    }

    /// SYM-13: a group's second line starts with its member count by
    /// class, most significant first, before the other parts.
    #[test]
    fn a_groups_second_line_counts_its_members_by_class() {
        use ShipClass::*;
        assert_eq!(
            member_count_text(&[Warship; 4]).as_deref(),
            Some("4 warships")
        );
        assert_eq!(
            member_count_text(&[Cargo, Warship, FleetSupply, Warship]).as_deref(),
            Some("2 warships, 1 supply ship, 1 cargo ship")
        );
        assert_eq!(member_count_text(&[]), None);
        let embarked = transport(TroopLoad::Embarked(vec![Formation {
            name: "1st Arden Guard",
            echelon: Some(Echelon::Battalion),
        }]));
        assert_eq!(
            label_second_line(
                Affiliation::Own,
                &embarked,
                &[Warship, TroopTransport],
                0.0
            )
            .as_deref(),
            Some("1 warship, 1 troop transport · 1st Arden Guard")
        );
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
