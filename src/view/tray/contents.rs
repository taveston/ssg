//! What the open tray holds this frame (MUL-08, MUL-09, MUL-12): its
//! header, its rows with each one's name, look and tag, the key hint, and
//! the box of the marker it is beside. Gathered from the multi-marker (or
//! a shared orbit's identity mark) every frame, so the rows follow the
//! marker's things; when the marker stops existing the tray closes and
//! the selection is kept (MUL-10).

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::TrayState;
use super::rows::{
    SideKey, TagFacts, TrayKind, TrayRow, key_hint, row_tag, tray_header, tray_rows,
};
use crate::sim::body::{Affiliation, BodyClass, DisplayName};
use crate::sim::group::{MemberOf, ShipGroup};
use crate::sim::named_orbit::{InNamedOrbit, NamedOrbit};
use crate::sim::track::ShipRecord;
use crate::sim::tunables::SimTunables;
use crate::view::bindings::Bindings;
use crate::view::markers::MarkerDrawn;
use crate::view::multi::{
    BackFrame, MultiKey, MultiMarker, MultiMarkers, OpenTray, TrayFor, back_offset, frames_rect,
};
use crate::view::multi_markers::draw::StationMarker;
use crate::view::multi_markers::order::TrayOrder;
use crate::view::planner::state::PlannerState;
use crate::view::rings::marks::{RingMarkFor, ring_things};
use crate::view::shared_orbit::OrbitRings;
use crate::view::ship_groups::GroupFaces;
use crate::view::symbols::member_count_text;
use crate::view::targets::{MarkerFor, OrbitFor};
use crate::view::tunables::Tunables;

/// How a row's icon is drawn: as its body's marker or symbol (MUL-09).
#[derive(Debug, Clone, PartialEq)]
pub struct RowLook {
    pub class: BodyClass,
    pub affiliation: Affiliation,
    pub record: Option<ShipRecord>,
}

/// One row as drawn.
#[derive(Debug, Clone, PartialEq)]
pub struct TrayLine {
    pub row: TrayRow,
    pub name: String,
    pub tag: String,
    pub look: RowLook,
    /// The selected row, highlighted as in the outline (MUL-09).
    pub selected: bool,
    /// MUL-12: while the planner is open, a row that can't be picked is
    /// dimmed and a click on it does nothing.
    pub dimmed: bool,
}

/// The open tray's contents this frame; empty while none is open.
#[derive(Resource, Default, Debug, Clone, PartialEq)]
pub struct TrayContents {
    /// The tray these contents are for.
    pub tray: Option<TrayFor>,
    pub header: String,
    pub lines: Vec<TrayLine>,
    pub hint: String,
    /// The marker's box on screen (HOV-07), while it is drawn.
    pub anchor: Option<Rect>,
}

impl TrayContents {
    /// The rows, in display order.
    pub fn rows(&self) -> Vec<TrayRow> {
        self.lines.iter().map(|line| line.row).collect()
    }
}

/// Things docked at stations, by station (MUL-04).
type Docked = Vec<(Entity, Vec<Entity>)>;

/// What a marker holds, ready to list: the things at the top in order,
/// what each opens to, and the header.
struct Held {
    top: Vec<Entity>,
    /// Things docked at a station, listed under its row (MUL-04).
    docked: Docked,
    header: String,
}

/// The parts of a body a row is drawn from.
type RowBodies<'w, 's> = Query<
    'w,
    's,
    (
        &'static DisplayName,
        &'static BodyClass,
        &'static Affiliation,
        Option<&'static ShipRecord>,
        Option<&'static InNamedOrbit>,
    ),
>;

/// A group member's parts.
type Members<'w, 's> =
    Query<'w, 's, (&'static DisplayName, &'static ShipRecord, &'static MemberOf)>;

/// Where the markers a tray can open beside are on screen.
#[derive(SystemParam)]
pub struct Anchors<'w, 's> {
    multis: Res<'w, MultiMarkers>,
    stations: Query<
        'w,
        's,
        (&'static MarkerFor, &'static MarkerDrawn, &'static BackFrame),
        With<StationMarker>,
    >,
    ring_marks: Query<'w, 's, (&'static RingMarkFor, &'static Node, &'static OrbitFor)>,
}

impl Anchors<'_, '_> {
    /// MUL-08 / HOV-07: the box of the marker a tray is for: `None` once
    /// the marker no longer exists; `Some(None)` while it exists but isn't
    /// drawn.
    fn anchor(&self, tray: TrayFor, tunables: &Tunables) -> Option<Option<Rect>> {
        let margin = tunables.bracket_margin_px;
        let inflate = |rect: Rect| rect.inflate(margin);
        match tray {
            TrayFor::Marker(MultiKey::Station(station)) => {
                self.multis.get(MultiKey::Station(station))?;
                let drawn = self
                    .stations
                    .iter()
                    .find(|(marker_for, ..)| marker_for.0 == station)
                    .and_then(|(_, drawn, back)| {
                        let (centre, half) = drawn.0?;
                        Some(inflate(frames_rect(centre, half, back.0)))
                    });
                Some(drawn)
            }
            TrayFor::Marker(key) => {
                let marker = self.multis.get(key)?;
                let back = back_offset(tunables.back_frame_offset_px);
                let half = tunables.ship_symbol_px / 2.0;
                Some(
                    marker
                        .drawn
                        .then(|| inflate(frames_rect(marker.centre, half, Some(back)))),
                )
            }
            TrayFor::Orbit(orbit) => self
                .ring_marks
                .iter()
                .find(|(_, node, orbit_for)| orbit_for.0 == orbit && node.display != Display::None)
                .and_then(|(mark, ..)| mark.mark)
                .map(|rect| Some(inflate(rect))),
        }
    }
}

/// Everything a tray's contents are read from.
#[derive(SystemParam)]
pub struct TrayInputs<'w, 's> {
    anchors: Anchors<'w, 's>,
    order: TrayOrder<'w, 's>,
    bodies: RowBodies<'w, 's>,
    groups: Query<'w, 's, &'static ShipGroup>,
    members: Members<'w, 's>,
    faces: GroupFaces<'w, 's>,
    named: Query<'w, 's, &'static NamedOrbit>,
    rings: OrbitRings<'w, 's>,
    sim: Res<'w, SimTunables>,
    planner: Res<'w, PlannerState>,
    bindings: Res<'w, Bindings>,
    tunables: Res<'w, Tunables>,
}

impl TrayInputs<'_, '_> {
    fn name(&self, entity: Entity) -> String {
        self.bodies
            .get(entity)
            .map(|(name, ..)| name.0.clone())
            .or_else(|_| self.members.get(entity).map(|(name, ..)| name.0.clone()))
            .unwrap_or_default()
    }

    /// The named orbit `thing` sits in, if any.
    fn orbit_of(&self, thing: Entity) -> Option<Entity> {
        self.bodies.get(thing).ok()?.4.map(|o| o.0)
    }

    fn orbit_name(&self, orbit: Entity) -> String {
        self.named
            .get(orbit)
            .map_or_else(|_| String::new(), |o| o.name.clone())
    }

    /// MUL-04: the things a shared orbit holds, groups once and docked
    /// things under their station, and those docked.
    fn orbit_things(&self, orbit: Entity) -> Option<(Vec<Entity>, Docked)> {
        let ring = self.rings.ring(orbit)?;
        Some(ring_things(&ring, self.sim.together_km))
    }

    /// MUL-08: the header of a multi-marker's tray.
    fn marker_header(&self, marker: &MultiMarker) -> String {
        let count = marker.count();
        let one_orbit = {
            let mut orbits = marker.things.iter().map(|&t| self.orbit_of(t));
            let first = orbits.next().flatten();
            first.filter(|&o| orbits.all(|other| other == Some(o)))
        };
        let kind_name = |entity| self.name(entity);
        match (marker.key, one_orbit) {
            (MultiKey::Station(station), _) => tray_header(TrayKind::Station {
                name: &kind_name(station),
                docked: count,
            }),
            (MultiKey::Fold(parent), _) => tray_header(TrayKind::Fold {
                parent: &kind_name(parent),
                count,
            }),
            (MultiKey::Together(_), Some(orbit)) => tray_header(TrayKind::Together {
                orbit: &self.orbit_name(orbit),
                count,
            }),
            (MultiKey::Overlap(_), Some(orbit)) => tray_header(TrayKind::OverlapInOrbit {
                orbit: &self.orbit_name(orbit),
                count,
                of: self.orbit_things(orbit).map_or(count, |(t, _)| t.len()),
            }),
            _ => tray_header(TrayKind::Objects { count }),
        }
    }

    /// MUL-08 / MUL-09: what a tray lists, in order, and its header. In
    /// a station marker the docked things follow the station at the top;
    /// in a larger marker they are listed under its row.
    fn held(&self, tray: TrayFor) -> Option<Held> {
        match tray {
            TrayFor::Marker(key) => {
                let marker = self.anchors.multis.get(key)?;
                let header = self.marker_header(marker);
                let mut top = self.order.things(marker);
                let mut docked = marker.docked.clone();
                if let MultiKey::Station(station) = key {
                    let at: Vec<Entity> = docked
                        .iter()
                        .filter(|(s, _)| *s == station)
                        .flat_map(|(_, d)| d.iter().copied())
                        .collect();
                    top.extend(self.order.sorted(&at));
                    docked.clear();
                }
                Some(Held {
                    top,
                    docked,
                    header,
                })
            }
            TrayFor::Orbit(orbit) => {
                let (things, docked) = self.orbit_things(orbit)?;
                Some(Held {
                    top: self.order.sorted(&things),
                    header: tray_header(TrayKind::Objects {
                        count: things.len(),
                    }),
                    docked,
                })
            }
        }
    }

    /// MUL-09: what a row opens to: a group's members, or what is docked
    /// at a station in a larger marker.
    fn members_of(&self, entity: Entity, docked: &[(Entity, Vec<Entity>)]) -> Vec<Entity> {
        if let Ok(group) = self.groups.get(entity) {
            return group.members.clone();
        }
        docked
            .iter()
            .find(|(station, _)| *station == entity)
            .map(|(_, things)| self.order.sorted(things))
            .unwrap_or_default()
    }

    /// One row's name, look and tag (MUL-09).
    fn line(&self, row: TrayRow, state: &TrayState) -> TrayLine {
        let selected = state.cursor == Some(row.entity);
        let (look, tag, member) = match self.members.get(row.entity) {
            Ok((_, record, member_of)) => {
                let affiliation = self
                    .bodies
                    .get(member_of.0)
                    .map_or(Affiliation::Unknown, |b| *b.2);
                let condition = record.condition.filter(|_| affiliation == Affiliation::Own);
                let look = RowLook {
                    class: BodyClass::Ship,
                    affiliation,
                    record: Some(record.clone()),
                };
                (look, row_tag(&TagFacts::Member { condition }), true)
            }
            Err(_) => {
                let (class, affiliation, record) = self.bodies.get(row.entity).map_or(
                    (BodyClass::Ship, Affiliation::Unknown, None),
                    |(_, class, affiliation, record, _)| (*class, *affiliation, record.cloned()),
                );
                let facts = match (class, self.groups.get(row.entity)) {
                    (BodyClass::Station, _) => TagFacts::Station,
                    (_, Ok(group)) => TagFacts::Group {
                        members: group.members.len(),
                        classes: member_count_text(&self.faces.member_classes(row.entity)),
                    },
                    _ => TagFacts::Plain,
                };
                let look = RowLook {
                    class,
                    affiliation,
                    record,
                };
                (look, row_tag(&facts), false)
            }
        };
        // MUL-12 / PLNR-11: a group's member is wherever its group is,
        // and a group can't be picked, so neither can its members.
        let dimmed =
            self.planner.is_open() && (member || self.planner.unpickable.contains(&row.entity));
        TrayLine {
            row,
            name: self.name(row.entity),
            tag,
            look,
            selected,
            dimmed,
        }
    }

    /// MUL-10: the hint for the selected row: Right when it can open,
    /// Left when it, or the row it is under, can close.
    fn hint(&self, rows: &[TrayRow], cursor: Option<Entity>) -> String {
        let noun = |entity: Entity| {
            if self.groups.contains(entity) {
                "group"
            } else {
                "station"
            }
        };
        let row = rows.iter().find(|r| Some(r.entity) == cursor);
        let side = match row {
            Some(row) if row.open == Some(false) => SideKey::Open(noun(row.entity)),
            Some(row) if row.open == Some(true) => SideKey::Close(noun(row.entity)),
            Some(TrayRow {
                parent: Some(parent),
                ..
            }) => SideKey::Close(noun(*parent)),
            _ => SideKey::None,
        };
        key_hint(&self.bindings, side)
    }
}

/// MUL-08 to MUL-10: gather the open tray's contents, and close it once
/// its marker no longer exists, keeping the selection.
pub fn gather_tray(
    mut open: ResMut<OpenTray>,
    state: Res<TrayState>,
    inputs: TrayInputs,
    mut contents: ResMut<TrayContents>,
) {
    let found = open.0.and_then(|tray| {
        let anchor = inputs.anchors.anchor(tray, &inputs.tunables)?;
        let held = inputs.held(tray)?;
        Some((tray, anchor, held))
    });
    let Some((tray, anchor, held)) = found else {
        if open.0.is_some() {
            open.0 = None;
        }
        contents.set_if_neq(TrayContents::default());
        return;
    };
    let rows = tray_rows(
        &held.top,
        &|entity| inputs.members_of(entity, &held.docked),
        &state.open_rows,
    );
    let gathered = TrayContents {
        tray: Some(tray),
        header: held.header,
        hint: inputs.hint(&rows, state.cursor),
        lines: rows.iter().map(|&row| inputs.line(row, &state)).collect(),
        anchor,
    };
    contents.set_if_neq(gathered);
}
