//! The inset's "In orbit" list (INS-05, the flat list until the outline is
//! built): one row per satellite of the inspected body, inside out, each
//! with its class marker and its name in own or neutral colours (MRK-01),
//! and ships and stations as their symbols (SYM-02). The members of a
//! shared orbit (HOV-04) sit under one orbit row named by the parent and
//! the orbit ("Halden IV · High"), which expands and collapses on a click
//! and keeps the state the player left it in; a click on the orbit's ring
//! opens the inset with that row expanded (HOV-05).
//!
//! Clicking an entry selects that satellite; double-clicking it makes it
//! the frame and moves to it (ZOOM-16), or on the frame body calls the
//! planetary-view hook (ZOOM-17). Selecting a satellite rebuilds the inset
//! as the satellite's own, which would remove the list before a
//! double-click's second click arrived. So a single click on an entry takes
//! effect once `double_click_window_ms` has passed without a second one
//! (F5-07).

use std::collections::HashSet;

use bevy::prelude::*;

use super::camera::requests::{FrameRequest, PlanetaryViewRequest};
use super::colors;
use super::frame::Frame;
use super::hover::double_click;
use super::markers::{BodyLook, MarkerAssets, spawn_list_icon};
use super::selection::Selection;
use super::tunables::Tunables;
use crate::Stage;
use crate::sim::body::{Affiliation, BodyClass, DisplayName};
use crate::sim::named_orbit::{InNamedOrbit, NamedOrbit};
use crate::sim::orbit::Orbit;
use crate::sim::track::ShipRecord;
use crate::view::fonts::Fonts;

const ENTRY_FONT_PX: f32 = 11.0;
const HEADING_FONT_PX: f32 = 10.0;
/// Half the size of an entry's class marker.
const ICON_HALF_PX: f32 = 4.0;
const ENTRY_GAP_PX: f32 = 6.0;
/// How far a shared orbit's members are indented under their orbit row.
const INDENT_PX: f32 = 14.0;
/// The toggle glyphs on an orbit row: collapsed and expanded.
const COLLAPSED_GLYPH: &str = ">";
const EXPANDED_GLYPH: &str = "v";

/// One satellite in the list.
#[derive(Debug, Clone, PartialEq)]
pub struct OrbitEntry {
    pub body: Entity,
    pub name: String,
    pub class: BodyClass,
    pub affiliation: Affiliation,
    /// What the state knows of a ship or station, drawn as its symbol.
    pub record: Option<ShipRecord>,
    pub orbit_radius_km: f64,
    /// The named orbit it sits in (SCL-07), and that orbit's name.
    pub named_orbit: Option<(Entity, String)>,
}

/// A row of the list: a satellite on its own, or a shared orbit's row with
/// its members under it.
#[derive(Debug, Clone, PartialEq)]
pub enum Row {
    Entry(OrbitEntry),
    Orbit {
        orbit: Entity,
        /// "Parent · Orbit" (INS-05).
        title: String,
        radius_km: f64,
        expanded: bool,
        members: Vec<OrbitEntry>,
    },
}

impl Row {
    fn radius_km(&self) -> f64 {
        match self {
            Row::Entry(entry) => entry.orbit_radius_km,
            Row::Orbit { radius_km, .. } => *radius_km,
        }
    }
}

/// The satellites a list is built from.
pub type Satellites<'w, 's> = Query<
    'w,
    's,
    (
        &'static DisplayName,
        &'static BodyClass,
        &'static Affiliation,
        Option<&'static ShipRecord>,
        &'static Orbit,
        Option<&'static InNamedOrbit>,
    ),
>;

/// INS-05: the entries in orbit order, from the inside out.
pub fn in_orbit_order(mut entries: Vec<OrbitEntry>) -> Vec<OrbitEntry> {
    entries.sort_by(|a, b| a.orbit_radius_km.total_cmp(&b.orbit_radius_km));
    entries
}

/// A body's satellites, as list entries in orbit order.
pub fn orbit_entries(
    children: Option<&Children>,
    satellites: &Satellites,
    named: &Query<&NamedOrbit>,
) -> Vec<OrbitEntry> {
    let entries = children
        .into_iter()
        .flat_map(|children| children.iter())
        .filter_map(|child| {
            let (name, class, affiliation, record, orbit, in_orbit) = satellites.get(child).ok()?;
            let named_orbit = in_orbit.and_then(|in_orbit| {
                let orbit = named.get(in_orbit.0).ok()?;
                Some((in_orbit.0, orbit.name.clone()))
            });
            Some(OrbitEntry {
                body: child,
                name: name.0.clone(),
                class: *class,
                affiliation: *affiliation,
                record: record.cloned(),
                orbit_radius_km: orbit.radius_km,
                named_orbit,
            })
        })
        .collect();
    in_orbit_order(entries)
}

/// INS-05 / HOV-04: the rows of the list, inside out: each satellite on
/// its own, except that the members of a named orbit with several of them
/// sit under one orbit row, "Parent · Orbit", shown expanded if the orbit
/// is in `expanded`.
pub fn list_rows(entries: Vec<OrbitEntry>, parent: &str, expanded: &HashSet<Entity>) -> Vec<Row> {
    let mut rows: Vec<Row> = Vec::new();
    for entry in entries {
        let Some((orbit, name)) = entry.named_orbit.clone() else {
            rows.push(Row::Entry(entry));
            continue;
        };
        let members = rows.iter_mut().find_map(|row| match row {
            Row::Orbit {
                orbit: o, members, ..
            } if *o == orbit => Some(members),
            _ => None,
        });
        match members {
            Some(members) => members.push(entry),
            None => rows.push(Row::Orbit {
                orbit,
                title: format!("{parent} · {name}"),
                radius_km: entry.orbit_radius_km,
                expanded: expanded.contains(&orbit),
                members: vec![entry],
            }),
        }
    }
    // An orbit with one member is that member's own row.
    let mut rows: Vec<Row> = rows
        .into_iter()
        .map(|row| match row {
            Row::Orbit { mut members, .. } if members.len() == 1 => Row::Entry(members.remove(0)),
            row => row,
        })
        .collect();
    rows.sort_by(|a, b| a.radius_km().total_cmp(&b.radius_km()));
    rows
}

/// HOV-05 / INS-05: the orbit rows the player has expanded. They keep
/// their state across insets; a click on a shared ring adds its orbit.
#[derive(Resource, Default)]
pub struct ExpandedOrbits(pub HashSet<Entity>);

/// The list's container in an inset, for the inspected body. The inset
/// spawns it empty; `fill_lists` fills it and refills it as rows expand.
#[derive(Component)]
pub struct InOrbitList(pub Entity);

/// A single click on an entry, waiting out the double-click window.
#[derive(Resource, Default)]
pub struct PendingEntryClick(Option<(Entity, f32)>);

pub struct InOrbitPlugin;

impl Plugin for InOrbitPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PendingEntryClick>()
            .init_resource::<ExpandedOrbits>()
            .add_systems(
                Update,
                (reveal_orbit, fill_lists, commit_entry_click)
                    .chain()
                    .in_set(Stage::Overlay),
            );
    }
}

/// HOV-05: a selection made through a shared orbit's ring opens that
/// orbit's row.
fn reveal_orbit(selection: Res<Selection>, mut expanded: ResMut<ExpandedOrbits>) {
    if !selection.is_changed() {
        return;
    }
    if let Some(orbit) = selection.revealed_orbit
        && !expanded.0.contains(&orbit)
    {
        expanded.0.insert(orbit);
    }
}

/// INS-05: a single click selects the entry once no second click came.
fn commit_entry_click(
    time: Res<Time>,
    mut pending: ResMut<PendingEntryClick>,
    mut selection: ResMut<Selection>,
) {
    let Some((body, remaining)) = pending.0 else {
        return;
    };
    let remaining = remaining - time.delta_secs();
    if remaining > 0.0 {
        pending.0 = Some((body, remaining));
        return;
    }
    pending.0 = None;
    selection.select(body);
}

/// What the list reads of the inspected body.
type Parents<'w, 's> = Query<'w, 's, (&'static DisplayName, Option<&'static Children>)>;

/// INS-05: fill each new list, and refill every list when a row expands or
/// collapses. Nothing for a body with no satellites.
#[allow(clippy::too_many_arguments)]
fn fill_lists(
    mut commands: Commands,
    fonts: Res<Fonts>,
    tunables: Res<Tunables>,
    expanded: Res<ExpandedOrbits>,
    lists: Query<(Entity, &InOrbitList)>,
    new_lists: Query<(), Added<InOrbitList>>,
    parents: Parents,
    satellites: Satellites,
    named: Query<&NamedOrbit>,
) {
    for (list, in_orbit) in &lists {
        if !expanded.is_changed() && !new_lists.contains(list) {
            continue;
        }
        let Ok((parent_name, children)) = parents.get(in_orbit.0) else {
            continue;
        };
        let rows = list_rows(
            orbit_entries(children, &satellites, &named),
            &parent_name.0,
            &expanded.0,
        );
        let mut list = commands.entity(list);
        list.despawn_related::<Children>();
        list.with_children(|list| spawn_rows(list, (&fonts, &tunables), &rows));
    }
}

/// INS-05: the heading and the rows.
fn spawn_rows(list: &mut ChildSpawnerCommands, assets: MarkerAssets, rows: &[Row]) {
    let (fonts, ..) = assets;
    if rows.is_empty() {
        return;
    }
    list.spawn((
        Text::new("In orbit"),
        fonts.label(HEADING_FONT_PX),
        TextColor(colors::PANEL_BORDER),
    ));
    for row in rows {
        match row {
            Row::Entry(entry) => spawn_entry(list, assets, entry, 0.0),
            Row::Orbit {
                orbit,
                title,
                expanded,
                members,
                ..
            } => {
                spawn_orbit_row(list, fonts, *orbit, title, *expanded);
                if *expanded {
                    for member in members {
                        spawn_entry(list, assets, member, INDENT_PX);
                    }
                }
            }
        }
    }
}

/// INS-05: one entry: the satellite's symbol at `list_symbol_px` (or its
/// body disc) and its name, in its colour, indented under an orbit row if
/// it is a member of one.
fn spawn_entry(
    list: &mut ChildSpawnerCommands,
    assets: MarkerAssets,
    entry: &OrbitEntry,
    indent_px: f32,
) {
    let (fonts, tunables) = assets;
    let body = entry.body;
    let look = BodyLook {
        class: entry.class,
        affiliation: entry.affiliation,
        record: entry.record.as_ref(),
    };
    list.spawn(Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: px(ENTRY_GAP_PX),
        margin: UiRect::left(px(indent_px)),
        ..default()
    })
    .with_children(|row| {
        spawn_list_icon(row, assets, look, ICON_HALF_PX);
        row.spawn((
            Text::new(entry.name.clone()),
            fonts.label(ENTRY_FONT_PX),
            TextColor(look.color(tunables)),
        ));
    })
    .observe(move |click: On<PointerClick>, mut actions: EntryActions| {
        actions.clicked(body, &click)
    });
}

/// INS-05 / HOV-04: a shared orbit's row: a toggle and "Parent · Orbit";
/// clicking it expands or collapses the members under it.
fn spawn_orbit_row(
    list: &mut ChildSpawnerCommands,
    fonts: &Fonts,
    orbit: Entity,
    title: &str,
    expanded: bool,
) {
    let glyph = if expanded {
        EXPANDED_GLYPH
    } else {
        COLLAPSED_GLYPH
    };
    list.spawn(Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: px(ENTRY_GAP_PX),
        ..default()
    })
    .with_children(|row| {
        row.spawn((
            Text::new(glyph),
            fonts.data(ENTRY_FONT_PX),
            TextColor(colors::PANEL_BORDER),
        ));
        row.spawn((
            Text::new(title),
            fonts.label(ENTRY_FONT_PX),
            TextColor(colors::NEUTRAL),
        ));
    })
    .observe(
        move |click: On<PointerClick>, mut expanded: ResMut<ExpandedOrbits>| {
            if click.button == PointerButton::Primary {
                toggle(&mut expanded.0, orbit);
            }
        },
    );
}

/// INS-05: an orbit row flips between expanded and collapsed.
pub fn toggle(expanded: &mut HashSet<Entity>, orbit: Entity) {
    if !expanded.remove(&orbit) {
        expanded.insert(orbit);
    }
}

/// What clicking an entry can do.
#[derive(bevy::ecs::system::SystemParam)]
pub struct EntryActions<'w> {
    tunables: Res<'w, Tunables>,
    frame: Res<'w, Frame>,
    pending: ResMut<'w, PendingEntryClick>,
    selection: ResMut<'w, Selection>,
    frame_request: ResMut<'w, FrameRequest>,
    planetary: ResMut<'w, PlanetaryViewRequest>,
}

impl EntryActions<'_> {
    /// INS-05: a click waits out the double-click window; a double-click
    /// selects and makes it the frame (ZOOM-16), or on the frame body
    /// selects and calls the planetary-view hook (ZOOM-17).
    fn clicked(&mut self, body: Entity, click: &On<PointerClick>) {
        if click.button != PointerButton::Primary {
            return;
        }
        if click.count < 2 {
            let window = self.tunables.double_click_window_ms / crate::view::anim::MS_PER_SEC;
            self.pending.0 = Some((body, window));
            return;
        }
        self.pending.0 = None;
        self.selection.select(body);
        double_click(
            &self.frame,
            body,
            &mut self.frame_request,
            &mut self.planetary,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(
        name: &str,
        radius: f64,
        orbit: Option<(Entity, &str)>,
        world: &mut World,
    ) -> OrbitEntry {
        OrbitEntry {
            body: world.spawn_empty().id(),
            name: name.to_string(),
            class: BodyClass::Moon,
            affiliation: Affiliation::Own,
            record: None,
            orbit_radius_km: radius,
            named_orbit: orbit.map(|(o, n)| (o, n.to_string())),
        }
    }

    /// INS-05: inside out, whatever order the satellites were spawned in.
    #[test]
    fn entries_run_from_the_inside_out() {
        let mut world = World::new();
        let entries = vec![
            entry("IVd", 726_432.0, None, &mut world),
            entry("Arden Highport", 9_394.0, None, &mut world),
            entry("IVa", 57_137.0, None, &mut world),
        ];
        let names: Vec<String> = in_orbit_order(entries)
            .into_iter()
            .map(|e| e.name)
            .collect();
        assert_eq!(names, ["Arden Highport", "IVa", "IVd"]);
        assert!(in_orbit_order(Vec::new()).is_empty());
    }

    /// INS-05 / HOV-04: a shared orbit's members sit under one row named
    /// "Parent · Orbit" at the orbit's place in the order, expanded only if
    /// the player (or a ring click) opened it; a named orbit with one
    /// member is a plain entry.
    #[test]
    fn shared_orbit_members_sit_under_one_orbit_row() {
        let mut world = World::new();
        let high = world.spawn_empty().id();
        let low = world.spawn_empty().id();
        let entries = in_orbit_order(vec![
            entry("IVa", 57_000.0, None, &mut world),
            entry("Orison", 30_000.0, Some((high, "High")), &mut world),
            entry("Ardent", 30_000.0, Some((high, "High")), &mut world),
            entry("Arden Highport", 9_000.0, Some((low, "Low")), &mut world),
        ]);
        let rows = list_rows(entries.clone(), "Halden IV", &HashSet::new());
        assert_eq!(rows.len(), 3);
        assert!(matches!(&rows[0], Row::Entry(e) if e.name == "Arden Highport"));
        let Row::Orbit {
            orbit,
            title,
            expanded,
            members,
            ..
        } = &rows[1]
        else {
            panic!("the shared orbit should be an orbit row");
        };
        assert_eq!(
            (*orbit, title.as_str(), *expanded),
            (high, "Halden IV · High", false)
        );
        let names: Vec<&str> = members.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, ["Orison", "Ardent"]);
        assert!(matches!(&rows[2], Row::Entry(e) if e.name == "IVa"));

        let mut open = HashSet::new();
        toggle(&mut open, high);
        let rows = list_rows(entries, "Halden IV", &open);
        assert!(matches!(&rows[1], Row::Orbit { expanded: true, .. }));
        toggle(&mut open, high);
        assert!(open.is_empty());
    }
}
