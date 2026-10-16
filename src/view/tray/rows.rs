//! What a tray says (MUL-08 to MUL-10, GRP-09): its rows, its header, the
//! tag at the right of each row, the key hint in its footer, and which
//! row the keys move to. Plain functions of what the marker holds, which
//! rows are open and which is selected; `contents.rs` gathers those from
//! the world.

use std::collections::HashSet;

use bevy::prelude::*;

use crate::view::bindings::Bindings;

/// Rows open in place at most this deep, so a loop in the data can't
/// list forever.
const MAX_DEPTH: usize = 4;

/// One row of a tray, in display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrayRow {
    pub entity: Entity,
    /// How far in it is listed: 0 for a thing on the marker, one more for
    /// each row it is opened from (a group's members, or the things docked
    /// at a station in a larger marker).
    pub depth: usize,
    /// `Some(open)` for a row with members it can open in place, `None`
    /// for one with none (MUL-09).
    pub open: Option<bool>,
    /// The row it is listed under, when it is opened from one.
    pub parent: Option<Entity>,
}

/// MUL-09: the rows for the things `top`, in that order. A thing with
/// members (`members`, in their order) can open; when it is in `open`,
/// its members follow it one level in.
pub fn tray_rows(
    top: &[Entity],
    members: &impl Fn(Entity) -> Vec<Entity>,
    open: &HashSet<Entity>,
) -> Vec<TrayRow> {
    fn push(
        rows: &mut Vec<TrayRow>,
        (entity, depth, parent): (Entity, usize, Option<Entity>),
        members: &impl Fn(Entity) -> Vec<Entity>,
        open: &HashSet<Entity>,
    ) {
        let inner = members(entity);
        let is_open = open.contains(&entity) && depth < MAX_DEPTH;
        rows.push(TrayRow {
            entity,
            depth,
            open: (!inner.is_empty()).then_some(is_open),
            parent,
        });
        if is_open {
            for member in inner {
                push(rows, (member, depth + 1, Some(entity)), members, open);
            }
        }
    }
    let mut rows = Vec::new();
    for &thing in top {
        push(&mut rows, (thing, 0, None), members, open);
    }
    rows
}

/// What a tray's marker is, for its header (MUL-08).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrayKind<'a> {
    /// A station with things docked (MUL-03).
    Station { name: &'a str, docked: usize },
    /// Things together at one point of a named orbit (MUL-05).
    Together { orbit: &'a str, count: usize },
    /// An overlap cluster whose things are all in one named orbit, which
    /// holds `of` things in all.
    OverlapInOrbit {
        orbit: &'a str,
        count: usize,
        of: usize,
    },
    /// A parent's fold marker (GRP-05).
    Fold { parent: &'a str, count: usize },
    /// Any other: an overlap across orbits, or a shared orbit's identity
    /// mark.
    Objects { count: usize },
}

/// MUL-08: the header, in capitals: "NAME · N DOCKED", "HIGH ORBIT · N
/// THINGS AT ONE POINT", "HIGH ORBIT · N OF M", "PARENT · N IN ORBIT" or
/// "N OBJECTS".
pub fn tray_header(kind: TrayKind) -> String {
    let text = match kind {
        TrayKind::Station { name, docked } => format!("{name} · {docked} docked"),
        TrayKind::Together { orbit, count } => {
            format!("{orbit} orbit · {count} things at one point")
        }
        TrayKind::OverlapInOrbit { orbit, count, of } => format!("{orbit} orbit · {count} of {of}"),
        TrayKind::Fold { parent, count } => format!("{parent} · {count} in orbit"),
        TrayKind::Objects { count } => format!("{count} objects"),
    };
    text.to_uppercase()
}

/// What a row's tag is read from (MUL-09).
#[derive(Debug, Clone, PartialEq)]
pub enum TagFacts {
    Station,
    /// A group: its member count and its members' classes in words
    /// (SYM-13), such as "4 warships".
    Group {
        members: usize,
        classes: Option<String>,
    },
    /// A member of an open group, with its condition when the group is
    /// own and the condition known.
    Member {
        condition: Option<f64>,
    },
    /// Anything else: no tag.
    Plain,
}

/// MUL-09 / GRP-09: the tag at the right of a row: "station"; "×N ·
/// class summary" for a group; a member's condition to one decimal; or
/// nothing.
pub fn row_tag(facts: &TagFacts) -> String {
    match facts {
        TagFacts::Station => "station".to_string(),
        TagFacts::Group {
            members,
            classes: Some(classes),
        } => format!("×{members} · {classes}"),
        TagFacts::Group {
            members,
            classes: None,
        } => format!("×{members}"),
        TagFacts::Member {
            condition: Some(condition),
        } => format!("{condition:.1}"),
        TagFacts::Member { condition: None } | TagFacts::Plain => String::new(),
    }
}

/// A key as the footer names it: arrows as arrows, Esc as "Esc".
fn key_name(keys: &[KeyCode]) -> String {
    match keys.first() {
        Some(KeyCode::ArrowUp) => "↑".to_string(),
        Some(KeyCode::ArrowDown) => "↓".to_string(),
        Some(KeyCode::ArrowLeft) => "←".to_string(),
        Some(KeyCode::ArrowRight) => "→".to_string(),
        Some(KeyCode::Escape) => "Esc".to_string(),
        Some(key) => format!("{key:?}")
            .trim_start_matches("Key")
            .trim_start_matches("Digit")
            .to_string(),
        None => String::new(),
    }
}

/// What the selected row lets the side keys do (MUL-10), for the hint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SideKey<'a> {
    /// The selected row can open; it is a group or a station (`noun`).
    Open(&'a str),
    /// The selected row, or the row it is listed under, can close.
    Close(&'a str),
    None,
}

/// MUL-08 / MUL-10: the footer's key hint, from the bindings: step with
/// Up and Down, open or close with Right or Left when the selected row
/// allows it, close the tray with Esc. For example "↑↓ step · → open
/// group · Esc close".
pub fn key_hint(bindings: &Bindings, side: SideKey) -> String {
    let mut parts = vec![format!(
        "{}{} step",
        key_name(&bindings.tray_previous),
        key_name(&bindings.tray_next)
    )];
    match side {
        SideKey::Open(noun) => {
            parts.push(format!("{} open {noun}", key_name(&bindings.tray_open_row)))
        }
        SideKey::Close(noun) => parts.push(format!(
            "{} close {noun}",
            key_name(&bindings.tray_close_row)
        )),
        SideKey::None => {}
    }
    parts.push(format!("{} close", key_name(&bindings.close_inset)));
    parts.join(" · ")
}

/// MUL-10: the row Up (`forward` false) or Down moves the selection to:
/// the one before or after the selected row, stopping at the ends. With
/// no row selected, the first.
pub fn step_row(rows: &[TrayRow], cursor: Option<Entity>, forward: bool) -> Option<Entity> {
    let at = cursor.and_then(|c| rows.iter().position(|r| r.entity == c));
    let index = match at {
        None => 0,
        Some(i) if forward => (i + 1).min(rows.len().saturating_sub(1)),
        Some(i) => i.saturating_sub(1),
    };
    rows.get(index).map(|r| r.entity)
}

/// MUL-10: the row Left closes: the selected row if it is open, or else
/// the open row it is listed under, which then takes the selection so it
/// doesn't vanish with the rows it closes.
pub fn row_to_close(rows: &[TrayRow], cursor: Option<Entity>) -> Option<Entity> {
    let row = rows.iter().find(|r| Some(r.entity) == cursor)?;
    if row.open == Some(true) {
        Some(row.entity)
    } else {
        row.parent
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn entities<const N: usize>() -> [Entity; N] {
        let mut world = World::new();
        [(); N].map(|_| world.spawn_empty().id())
    }

    /// MUL-09 / GRP-09: a group is one row with a toggle; opened, its
    /// members follow it one level in, under it; a thing with no members
    /// has no toggle.
    #[test]
    fn an_open_group_lists_its_members_one_level_in() {
        let [station, group, ship, a, b] = entities();
        let members: HashMap<Entity, Vec<Entity>> = [(group, vec![a, b])].into();
        let members = |e: Entity| members.get(&e).cloned().unwrap_or_default();
        let closed = tray_rows(&[station, group, ship], &members, &HashSet::new());
        assert_eq!(
            closed
                .iter()
                .map(|r| (r.entity, r.open))
                .collect::<Vec<_>>(),
            [(station, None), (group, Some(false)), (ship, None)]
        );
        let open = tray_rows(&[station, group, ship], &members, &[group].into());
        let listed: Vec<(Entity, usize, Option<Entity>)> =
            open.iter().map(|r| (r.entity, r.depth, r.parent)).collect();
        assert_eq!(
            listed,
            [
                (station, 0, None),
                (group, 0, None),
                (a, 1, Some(group)),
                (b, 1, Some(group)),
                (ship, 0, None)
            ]
        );
        assert_eq!(open[1].open, Some(true));
    }

    /// MUL-08: each kind of marker's header.
    #[test]
    fn the_header_says_what_the_marker_holds() {
        let cases = [
            (
                TrayKind::Station {
                    name: "Arden Highport",
                    docked: 3,
                },
                "ARDEN HIGHPORT · 3 DOCKED",
            ),
            (
                TrayKind::Together {
                    orbit: "High",
                    count: 4,
                },
                "HIGH ORBIT · 4 THINGS AT ONE POINT",
            ),
            (
                TrayKind::OverlapInOrbit {
                    orbit: "High",
                    count: 4,
                    of: 14,
                },
                "HIGH ORBIT · 4 OF 14",
            ),
            (
                TrayKind::Fold {
                    parent: "Halden IV",
                    count: 7,
                },
                "HALDEN IV · 7 IN ORBIT",
            ),
            (TrayKind::Objects { count: 5 }, "5 OBJECTS"),
        ];
        for (kind, want) in cases {
            assert_eq!(tray_header(kind), want);
        }
    }

    /// MUL-09 / GRP-09: "station" for a station, "×N · class summary" for
    /// a group, a member's condition to one decimal when known, else
    /// nothing.
    #[test]
    fn row_tags_name_stations_groups_and_members_condition() {
        assert_eq!(row_tag(&TagFacts::Station), "station");
        let group = TagFacts::Group {
            members: 4,
            classes: Some("4 warships".to_string()),
        };
        assert_eq!(row_tag(&group), "×4 · 4 warships");
        let member = TagFacts::Member {
            condition: Some(0.86),
        };
        assert_eq!(row_tag(&member), "0.9");
        assert_eq!(row_tag(&TagFacts::Member { condition: None }), "");
        assert_eq!(row_tag(&TagFacts::Plain), "");
    }

    /// MUL-10 / CAM-18: the hint names the bound keys, and Right or Left
    /// only when the selected row can open or close.
    #[test]
    fn the_key_hint_follows_the_bindings_and_the_selected_row() {
        let mut bindings = Bindings::default();
        assert_eq!(key_hint(&bindings, SideKey::None), "↑↓ step · Esc close");
        assert_eq!(
            key_hint(&bindings, SideKey::Open("group")),
            "↑↓ step · → open group · Esc close"
        );
        assert_eq!(
            key_hint(&bindings, SideKey::Close("station")),
            "↑↓ step · ← close station · Esc close"
        );
        bindings.tray_next = vec![KeyCode::KeyJ];
        assert_eq!(key_hint(&bindings, SideKey::None), "↑J step · Esc close");
    }

    /// MUL-10: Up and Down step one row, stopping at the ends; with no
    /// row selected, either selects the first.
    #[test]
    fn stepping_stops_at_the_ends() {
        let [a, b, c, outside] = entities();
        let rows = tray_rows(&[a, b, c], &|_| Vec::new(), &HashSet::new());
        assert_eq!(step_row(&rows, Some(a), true), Some(b));
        assert_eq!(step_row(&rows, Some(c), true), Some(c));
        assert_eq!(step_row(&rows, Some(b), false), Some(a));
        assert_eq!(step_row(&rows, Some(a), false), Some(a));
        assert_eq!(step_row(&rows, None, true), Some(a));
        assert_eq!(step_row(&rows, Some(outside), false), Some(a));
        assert_eq!(step_row(&[], None, true), None);
    }

    /// MUL-10: Left closes the selected open row, or from one of its
    /// members, the row they are listed under.
    #[test]
    fn left_closes_the_open_row_or_the_one_above() {
        let [group, member, ship] = entities();
        let rows = tray_rows(
            &[group, ship],
            &|e| if e == group { vec![member] } else { vec![] },
            &[group].into(),
        );
        assert_eq!(row_to_close(&rows, Some(group)), Some(group));
        assert_eq!(row_to_close(&rows, Some(member)), Some(group));
        assert_eq!(row_to_close(&rows, Some(ship)), None);
        assert_eq!(row_to_close(&rows, None), None);
    }
}
