//! Groups of ships (SYM-01, SYM-09, PHAS-14). A group is one body on the
//! map: it has the orbit, the phase, the affiliation and the name, and the
//! view draws it as one symbol. Its ships are member entities holding only
//! a name and what the faction knows of each (`ShipRecord`). They have no
//! orbit or phase of their own: a member is wherever its group is, so the
//! ships of a group are together by construction (PHAS-14).
//!
//! A group's class, armed status, load, track and condition are not held
//! by it. Its `ShipRecord` is derived from its members' records
//! (`group_record`) at spawn and kept in step by `derive_group_records`,
//! so whatever reads a record reads a group the same way as a ship.

use bevy::prelude::*;

use super::body::DisplayName;
use super::system::GroupBlueprint;
use super::track::{ShipClass, ShipRecord, Track, TroopLoad};

/// On a group's body: its members, and whether it is a task force
/// (members under one commander, SYM-09).
#[derive(Component, Debug, Clone, PartialEq)]
pub struct ShipGroup {
    pub members: Vec<Entity>,
    pub task_force: bool,
}

/// On a member ship: the group it belongs to.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemberOf(pub Entity);

/// SYM-09: how significant a ship's icon is, lower first: a warship (or
/// anything known to be armed), then a troop transport, fleet supply,
/// cargo and unknown. A group shows the most significant of its members.
pub fn significance(record: &ShipRecord) -> u8 {
    if record.armed {
        return 0;
    }
    match record.class {
        ShipClass::Warship => 0,
        ShipClass::TroopTransport => 1,
        ShipClass::FleetSupply => 2,
        ShipClass::Cargo => 3,
        ShipClass::Unknown => 4,
        ShipClass::Station => 5,
    }
}

/// SYM-12: a group's condition, the mean of its members' conditions.
/// Members with no known condition are left out; with none known, the
/// group has none.
pub fn group_condition(members: impl IntoIterator<Item = Option<f64>>) -> Option<f64> {
    let (sum, count) = members
        .into_iter()
        .flatten()
        .fold((0.0, 0u32), |(sum, count), c| (sum + c, count + 1));
    (count > 0).then(|| sum / f64::from(count))
}

/// SYM-06 / SYM-09: the troop transports' loads read together: embarked
/// if any is known to be embarked (all their formations, so the echelon
/// is the largest's), else unknown if any load is unknown, else known
/// empty. A group with no troop transport is known empty.
pub fn group_load<'a>(members: impl IntoIterator<Item = &'a ShipRecord>) -> TroopLoad {
    let mut formations = Vec::new();
    let mut unknown = false;
    for member in members {
        if member.class != ShipClass::TroopTransport {
            continue;
        }
        match &member.load {
            TroopLoad::Embarked(embarked) => formations.extend(embarked.iter().cloned()),
            TroopLoad::Unknown => unknown = true,
            TroopLoad::Empty => {}
        }
    }
    if !formations.is_empty() {
        TroopLoad::Embarked(formations)
    } else if unknown {
        TroopLoad::Unknown
    } else {
        TroopLoad::Empty
    }
}

/// How firm a group's track is: as weak as its weakest member's. A stale
/// track is weaker than a weak one, and of two stale tracks the older.
fn group_track<'a>(members: impl IntoIterator<Item = &'a ShipRecord>) -> Track {
    members
        .into_iter()
        .map(|member| member.track)
        .fold(Track::Firm, |worst, track| match (worst, track) {
            (Track::Stale { last_observed_day: a }, Track::Stale { last_observed_day: b }) => {
                Track::Stale {
                    last_observed_day: a.min(b),
                }
            }
            (Track::Stale { .. }, _) => worst,
            (_, Track::Stale { .. }) | (_, Track::Weak) => track,
            (Track::Weak, Track::Firm) => worst,
            (Track::Firm, Track::Firm) => Track::Firm,
        })
}

/// SYM-01 / SYM-09 / SYM-12: what a group is, from its members: the class
/// of its most significant member, armed if any member is, the troop
/// transports' loads read together, the weakest track and the mean
/// condition. Command is shared by every member (SYM-09); a group is
/// requisitioned only if all its members are. `None` with no members.
pub fn group_record(members: &[&ShipRecord]) -> Option<ShipRecord> {
    let top = members.iter().min_by_key(|member| significance(member))?;
    Some(ShipRecord {
        commanded: members.iter().all(|member| member.commanded),
        requisitioned: members.iter().all(|member| member.requisitioned),
        class: top.class,
        armed: members.iter().any(|member| member.armed),
        load: group_load(members.iter().copied()),
        track: group_track(members.iter().copied()),
        condition: group_condition(members.iter().map(|member| member.condition)),
    })
}

/// Spawns a group's members, each pointing back at `group`, and puts the
/// group's members and its derived record on it (SYM-01).
pub fn spawn_members(commands: &mut Commands, group: Entity, blueprint: GroupBlueprint) {
    let records: Vec<ShipRecord> = blueprint
        .members
        .iter()
        .map(|member| member.record.clone())
        .collect();
    let members = blueprint
        .members
        .into_iter()
        .map(|member| {
            commands
                .spawn((
                    Name::new(member.name),
                    DisplayName(member.name.to_string()),
                    member.record,
                    MemberOf(group),
                ))
                .id()
        })
        .collect();
    let mut entity = commands.entity(group);
    entity.insert(ShipGroup {
        members,
        task_force: blueprint.task_force,
    });
    let refs: Vec<&ShipRecord> = records.iter().collect();
    if let Some(record) = group_record(&refs) {
        entity.insert(record);
    }
}

/// SYM-09: keep each group's record in step with its members', so a
/// change to a member shows on the group's symbol.
pub fn derive_group_records(
    mut groups: Query<(&ShipGroup, &mut ShipRecord), Without<MemberOf>>,
    members: Query<&ShipRecord, With<MemberOf>>,
) {
    for (group, mut record) in &mut groups {
        let records: Vec<&ShipRecord> = group
            .members
            .iter()
            .filter_map(|&member| members.get(member).ok())
            .collect();
        if let Some(derived) = group_record(&records)
            && *record != derived
        {
            *record = derived;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::track::{Echelon, Formation};

    fn ship(class: ShipClass) -> ShipRecord {
        ShipRecord::new(class, true)
    }

    fn transport(load: TroopLoad) -> ShipRecord {
        ShipRecord {
            load,
            ..ship(ShipClass::TroopTransport)
        }
    }

    /// SYM-09: the group takes the class of its most significant member:
    /// warship, then troop transport, fleet supply, cargo, unknown; an
    /// armed member counts as a warship.
    #[test]
    fn a_group_shows_its_most_significant_member() {
        let class_of = |members: &[ShipRecord]| {
            let refs: Vec<&ShipRecord> = members.iter().collect();
            group_record(&refs).map(|r| (r.class, r.armed))
        };
        let tanker = ship(ShipClass::FleetSupply);
        let cargo = ship(ShipClass::Cargo);
        let unknown = ship(ShipClass::Unknown);
        let troops = transport(TroopLoad::Empty);
        let warship = ship(ShipClass::Warship);
        assert_eq!(
            class_of(&[cargo.clone(), unknown.clone()]),
            Some((ShipClass::Cargo, false))
        );
        assert_eq!(
            class_of(&[cargo.clone(), tanker.clone()]),
            Some((ShipClass::FleetSupply, false))
        );
        assert_eq!(
            class_of(&[tanker.clone(), troops.clone()]),
            Some((ShipClass::TroopTransport, false))
        );
        assert_eq!(
            class_of(&[troops, warship, tanker]),
            Some((ShipClass::Warship, true))
        );
        let armed_hauler = ShipRecord {
            armed: true,
            ..cargo.clone()
        };
        assert_eq!(
            class_of(&[transport(TroopLoad::Empty), armed_hauler]),
            Some((ShipClass::Cargo, true))
        );
        assert_eq!(class_of(&[]), None);
    }

    /// SYM-06 / SYM-09: troop transports are read together: embarked if
    /// any is, with every embarked formation; else unknown if any load is
    /// unknown; else known empty.
    #[test]
    fn a_groups_troop_transports_are_read_together() {
        let guard = Formation {
            name: "1st Arden Guard",
            echelon: Some(Echelon::Battalion),
        };
        let brigade = Formation {
            name: "Halden Field Brigade",
            echelon: Some(Echelon::Brigade),
        };
        let embarked = [
            transport(TroopLoad::Embarked(vec![guard.clone()])),
            transport(TroopLoad::Unknown),
            transport(TroopLoad::Embarked(vec![brigade.clone()])),
        ];
        assert_eq!(
            group_load(&embarked),
            TroopLoad::Embarked(vec![guard, brigade])
        );
        let unknown = [transport(TroopLoad::Empty), transport(TroopLoad::Unknown)];
        assert_eq!(group_load(&unknown), TroopLoad::Unknown);
        let empty = [transport(TroopLoad::Empty), ship(ShipClass::Warship)];
        assert_eq!(group_load(&empty), TroopLoad::Empty);
    }

    /// SYM-12: a group's condition is the mean of its members' known
    /// conditions, and unknown when none is known.
    #[test]
    fn a_groups_condition_is_its_members_mean() {
        let mean = group_condition([Some(0.9), Some(0.3), None, Some(0.6)]).unwrap();
        assert!((mean - 0.6).abs() < 1e-12);
        assert_eq!(group_condition([Some(0.4)]), Some(0.4));
        assert_eq!(group_condition([None, None]), None);
        assert_eq!(group_condition([]), None);
    }

    /// SYM-09: a group's track is its weakest member's; of two stale
    /// tracks, the older.
    #[test]
    fn a_groups_track_is_its_weakest_members() {
        let with = |track| ShipRecord {
            track,
            ..ship(ShipClass::Warship)
        };
        let stale = |day| {
            with(Track::Stale {
                last_observed_day: day,
            })
        };
        let track = |members: &[ShipRecord]| group_track(members);
        assert_eq!(track(&[with(Track::Firm), with(Track::Firm)]), Track::Firm);
        assert_eq!(track(&[with(Track::Firm), with(Track::Weak)]), Track::Weak);
        assert_eq!(
            track(&[stale(-2.0), with(Track::Weak), stale(-6.0)]),
            Track::Stale {
                last_observed_day: -6.0
            }
        );
    }

    /// SYM-01 / SYM-09: a group's record follows its members as they
    /// change: a member's new condition moves the group's mean.
    #[test]
    fn a_groups_record_follows_its_members() {
        let mut app = App::new();
        app.add_systems(Update, derive_group_records);
        let world = app.world_mut();
        let group = world.spawn_empty().id();
        let condition = |c| ShipRecord {
            condition: Some(c),
            ..ship(ShipClass::Warship)
        };
        let a = world.spawn((condition(0.9), MemberOf(group))).id();
        let b = world.spawn((condition(0.5), MemberOf(group))).id();
        world.entity_mut(group).insert((
            ShipGroup {
                members: vec![a, b],
                task_force: true,
            },
            ship(ShipClass::Unknown),
        ));
        app.update();
        let record = app.world().get::<ShipRecord>(group).unwrap().clone();
        assert_eq!(record.class, ShipClass::Warship);
        assert!((record.condition.unwrap() - 0.7).abs() < 1e-12);
        app.world_mut().get_mut::<ShipRecord>(b).unwrap().condition = Some(0.1);
        app.update();
        let record = app.world().get::<ShipRecord>(group).unwrap();
        assert!((record.condition.unwrap() - 0.5).abs() < 1e-12);
    }
}
