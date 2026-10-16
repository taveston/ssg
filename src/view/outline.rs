//! The sidebar's "In orbit" outline (INS-05): the whole system as a tree
//! from the star down, in orbit order from the inside out, one indent per
//! generation. Each entry shows its symbol (SYM-02, at `list_symbol_px`)
//! or body marker (MRK-13) and its name; an entry with satellites has a
//! toggle. The members of each named orbit (HOV-04), however few, sit
//! under one orbit row, "High orbit · N", which expands and collapses like
//! an entry. A named orbit with no members has no row.
//!
//! When the selection changes, the path from the star to it and its own
//! satellites are expanded; everything else keeps the state the player
//! left it in. The selection's row is highlighted and scrolled into view.
//! The outline is built once and refilled in place, so a click on an
//! entry selects at once and no row moves under the pointer; a
//! double-click behaves as ZOOM-16 and ZOOM-17.
//!
//! The rows are a pure function of the bodies, the expansion state and
//! the selection (`outline_rows`); the systems gather, fill and scroll.

use std::collections::{HashMap, HashSet};

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::camera::requests::{FrameRequest, PlanetaryViewRequest};
use super::colors;
use super::fonts::Fonts;
use super::frame::Frame;
use super::hover::double_click;
use super::markers::{BodyLook, MarkerAssets, spawn_list_icon};
use super::planner_state::{PickTarget, PlannerState};
use super::selection::Selection;
use super::tunables::Tunables;
use crate::Stage;
use crate::sim::body::{Affiliation, BodyClass, DisplayName};
use crate::sim::group::ShipGroup;
use crate::sim::named_orbit::{InNamedOrbit, NamedOrbit};
use crate::sim::orbit::Orbit;
use crate::sim::track::ShipRecord;

const ENTRY_GAP_PX: f32 = 6.0;
/// One generation's indent.
const INDENT_PX: f32 = 14.0;
/// The width of the toggle's slot, so names line up whether or not an
/// entry has one.
const TOGGLE_PX: f32 = 14.0;
const ROW_PADDING_Y_PX: f32 = 2.0;
/// The toggle glyphs: collapsed and expanded.
const COLLAPSED_GLYPH: &str = ">";
const EXPANDED_GLYPH: &str = "v";
/// The selected row's highlight: the secondary colour, faint.
const HIGHLIGHT: Color = Color::srgba(
    0x9F as f32 / 255.0,
    0xB6 as f32 / 255.0,
    0xC4 as f32 / 255.0,
    0.18,
);

/// One body as the outline sees it.
#[derive(Debug, Clone, PartialEq)]
pub struct OutlineNode {
    pub body: Entity,
    pub name: String,
    pub class: BodyClass,
    pub affiliation: Affiliation,
    pub record: Option<ShipRecord>,
    /// What it orbits; `None` for the star.
    pub parent: Option<Entity>,
    /// Its orbit radius round its parent; zero for the star.
    pub orbit_radius_km: f64,
    /// The named orbit it sits in (SCL-07), and that orbit's name.
    pub named_orbit: Option<(Entity, String)>,
    /// A group's member count (GRP-09); `None` for anything else.
    pub members: Option<usize>,
}

/// GRP-09: what an entry reads: its name, and for a group its member
/// count, as "TF-1 · 4". A group is one entry, however many ships it has.
pub fn entry_text(node: &OutlineNode) -> String {
    match node.members {
        Some(count) => format!("{} · {count}", node.name),
        None => node.name.clone(),
    }
}

/// A row of the outline, in display order.
#[derive(Debug, Clone, PartialEq)]
pub enum Row {
    Entry {
        body: Entity,
        depth: usize,
        has_children: bool,
        expanded: bool,
        selected: bool,
    },
    Orbit {
        orbit: Entity,
        /// "High orbit · N": the orbit's name, "orbit", the member count
        /// (INS-05).
        title: String,
        depth: usize,
        expanded: bool,
    },
}

/// What is expanded: entries and shared-orbit rows. Kept across
/// selections (INS-05); a ring click adds its orbit (HOV-05).
#[derive(Resource, Default, Debug)]
pub struct Expanded {
    pub entries: HashSet<Entity>,
    pub orbits: HashSet<Entity>,
}

/// INS-05: an entry or orbit row flips between expanded and collapsed.
pub fn toggle(set: &mut HashSet<Entity>, key: Entity) {
    if !set.remove(&key) {
        set.insert(key);
    }
}

/// INS-05: the ancestors of `body`, nearest first, from the nodes.
fn ancestors(nodes: &HashMap<Entity, &OutlineNode>, body: Entity) -> Vec<Entity> {
    let mut chain = Vec::new();
    let mut current = nodes.get(&body).and_then(|n| n.parent);
    while let Some(parent) = current {
        if chain.contains(&parent) {
            break;
        }
        chain.push(parent);
        current = nodes.get(&parent).and_then(|n| n.parent);
    }
    chain
}

/// INS-05: when the selection changes, the path from the star to it and
/// its own satellites open; other entries keep their state.
pub fn reveal_selection(expanded: &mut Expanded, nodes: &[OutlineNode], selected: Entity) {
    let by_body: HashMap<Entity, &OutlineNode> = nodes.iter().map(|n| (n.body, n)).collect();
    expanded.entries.insert(selected);
    expanded.entries.extend(ancestors(&by_body, selected));
    if let Some((orbit, _)) = by_body.get(&selected).and_then(|n| n.named_orbit.clone()) {
        expanded.orbits.insert(orbit);
    }
}

/// A parent's satellites grouped for the outline: lone entries and
/// shared orbits, each at its radius, inside out.
enum Child<'a> {
    Entry(&'a OutlineNode),
    Orbit {
        orbit: Entity,
        name: String,
        radius_km: f64,
        members: Vec<&'a OutlineNode>,
    },
}

impl Child<'_> {
    fn radius_km(&self) -> f64 {
        match self {
            Child::Entry(node) => node.orbit_radius_km,
            Child::Orbit { radius_km, .. } => *radius_km,
        }
    }
}

/// INS-05 / HOV-04: `parent`'s satellites inside out, with the members of
/// each named orbit under one orbit row, even a lone member. An orbit with
/// no members has no node pointing at it, so it gets no row.
fn children_of<'a>(nodes: &'a [OutlineNode], parent: Entity) -> Vec<Child<'a>> {
    let mut children: Vec<Child<'a>> = Vec::new();
    for node in nodes.iter().filter(|n| n.parent == Some(parent)) {
        let Some((orbit, name)) = node.named_orbit.clone() else {
            children.push(Child::Entry(node));
            continue;
        };
        let shared = children.iter_mut().find_map(|child| match child {
            Child::Orbit {
                orbit: o, members, ..
            } if *o == orbit => Some(members),
            _ => None,
        });
        match shared {
            Some(members) => members.push(node),
            None => children.push(Child::Orbit {
                orbit,
                name,
                radius_km: node.orbit_radius_km,
                members: vec![node],
            }),
        }
    }
    children.sort_by(|a, b| a.radius_km().total_cmp(&b.radius_km()));
    children
}

/// INS-05: an orbit row's title: the orbit's name, the word "orbit", then
/// how many it holds, as "High orbit · 14".
pub fn orbit_title(name: &str, members: usize) -> String {
    format!("{name} orbit · {members}")
}

/// INS-05: the outline's rows from the star down. An expanded entry is
/// followed by its satellites one level deeper; an expanded orbit row by
/// its members.
pub fn outline_rows(
    nodes: &[OutlineNode],
    expanded: &Expanded,
    selected: Option<Entity>,
) -> Vec<Row> {
    fn push_body(
        rows: &mut Vec<Row>,
        nodes: &[OutlineNode],
        node: &OutlineNode,
        depth: usize,
        expanded: &Expanded,
        selected: Option<Entity>,
    ) {
        if depth > nodes.len() {
            return;
        }
        let children = children_of(nodes, node.body);
        let open = expanded.entries.contains(&node.body);
        rows.push(Row::Entry {
            body: node.body,
            depth,
            has_children: !children.is_empty(),
            expanded: open,
            selected: selected == Some(node.body),
        });
        if !open {
            return;
        }
        for child in children {
            match child {
                Child::Entry(child) => push_body(rows, nodes, child, depth + 1, expanded, selected),
                Child::Orbit {
                    orbit,
                    name,
                    members,
                    ..
                } => {
                    let open = expanded.orbits.contains(&orbit);
                    rows.push(Row::Orbit {
                        orbit,
                        title: orbit_title(&name, members.len()),
                        depth: depth + 1,
                        expanded: open,
                    });
                    if open {
                        for member in members {
                            push_body(rows, nodes, member, depth + 2, expanded, selected);
                        }
                    }
                }
            }
        }
    }
    let mut rows = Vec::new();
    for star in nodes.iter().filter(|n| n.parent.is_none()) {
        push_body(&mut rows, nodes, star, 0, expanded, selected);
    }
    rows
}

/// INS-05: how far the panel must scroll so a row from `row_top` to
/// `row_bottom` (relative to the panel's visible top) is in view: up if
/// it starts above, down if it ends below, nothing if it is in view.
pub fn scroll_to_reveal(row_top: f32, row_bottom: f32, panel_height: f32) -> f32 {
    if row_top < 0.0 {
        row_top
    } else if row_bottom > panel_height {
        (row_bottom - panel_height).min(row_top)
    } else {
        0.0
    }
}

/// The outline's container in the sidebar. `fill_outline` fills it and
/// refills it as the selection or the expansion changes.
#[derive(Component)]
pub struct OutlineRoot;

/// A row of the outline, for the body or orbit it stands for.
#[derive(Component)]
pub struct OutlineRow(pub Entity);

/// The selected row waits to be scrolled into view (INS-05).
#[derive(Resource, Default)]
pub struct ScrollToSelection(pub bool);

/// The bodies the outline is built from.
type Bodies<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static DisplayName,
        &'static BodyClass,
        &'static Affiliation,
        Option<&'static ShipRecord>,
        Option<&'static Orbit>,
        Option<&'static ChildOf>,
        Option<&'static InNamedOrbit>,
        Option<&'static ShipGroup>,
    ),
>;

/// The bodies the outline is built from, and the named orbits they sit
/// in.
#[derive(SystemParam)]
struct OutlineBodies<'w, 's> {
    bodies: Bodies<'w, 's>,
    named: Query<'w, 's, &'static NamedOrbit>,
}

impl OutlineBodies<'_, '_> {
    /// INS-05: every body as an outline node. The star is the root;
    /// anything else with no parent orbits the star.
    fn nodes(&self) -> Vec<OutlineNode> {
        let star = self
            .bodies
            .iter()
            .find(|(_, _, class, ..)| **class == BodyClass::Star)
            .map(|(entity, ..)| entity);
        self.bodies
            .iter()
            .map(
                |(entity, name, class, affiliation, record, orbit, child_of, in_orbit, group)| {
                    let parent = match child_of {
                        Some(child_of) => Some(child_of.parent()),
                        None if Some(entity) == star => None,
                        None => star,
                    };
                    OutlineNode {
                        body: entity,
                        name: name.0.clone(),
                        class: *class,
                        affiliation: *affiliation,
                        record: record.cloned(),
                        parent,
                        orbit_radius_km: orbit.map_or(0.0, |o| o.radius_km),
                        named_orbit: in_orbit.and_then(|in_orbit| {
                            Some((in_orbit.0, self.named.get(in_orbit.0).ok()?.name.clone()))
                        }),
                        members: group.map(|group| group.members.len()),
                    }
                },
            )
            .collect()
    }
}

pub struct OutlinePlugin;

impl Plugin for OutlinePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Expanded>()
            .init_resource::<ScrollToSelection>()
            .add_systems(
                Update,
                (
                    reveal_on_selection,
                    fill_outline,
                    scroll_selection_into_view,
                )
                    .chain()
                    .in_set(Stage::Overlay),
            );
    }
}

/// INS-05 / HOV-05: a new selection opens the path to it and its own
/// satellites, and the orbit a ring click came through.
fn reveal_on_selection(
    selection: Res<Selection>,
    bodies: OutlineBodies,
    mut expanded: ResMut<Expanded>,
    mut scroll: ResMut<ScrollToSelection>,
) {
    if !selection.is_changed() {
        return;
    }
    let Some(selected) = selection.selected else {
        return;
    };
    let nodes = bodies.nodes();
    reveal_selection(&mut expanded, &nodes, selected);
    if let Some(orbit) = selection.revealed_orbit {
        expanded.orbits.insert(orbit);
    }
    scroll.0 = true;
}

/// INS-05: fill a new outline, and refill it when the selection or the
/// expansion changes, in place.
fn fill_outline(
    mut commands: Commands,
    (fonts, tunables): (Res<Fonts>, Res<Tunables>),
    selection: Res<Selection>,
    expanded: Res<Expanded>,
    roots: Query<Entity, With<OutlineRoot>>,
    new_roots: Query<(), Added<OutlineRoot>>,
    bodies: OutlineBodies,
) {
    for root in &roots {
        if !selection.is_changed() && !expanded.is_changed() && !new_roots.contains(root) {
            continue;
        }
        let nodes = bodies.nodes();
        let rows = outline_rows(&nodes, &expanded, selection.selected);
        let by_body: HashMap<Entity, &OutlineNode> = nodes.iter().map(|n| (n.body, n)).collect();
        let mut root = commands.entity(root);
        root.despawn_related::<Children>();
        root.with_children(|list| {
            for row in &rows {
                spawn_row(list, (&fonts, &tunables), &by_body, row);
            }
        });
    }
}

/// INS-05: the row's indent and its toggle slot.
fn row_node(depth: usize, selected: bool) -> (Node, BackgroundColor) {
    (
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(ENTRY_GAP_PX),
            padding: UiRect::axes(px(0), px(ROW_PADDING_Y_PX)),
            margin: UiRect::left(px(depth as f32 * INDENT_PX)),
            ..default()
        },
        BackgroundColor(if selected { HIGHLIGHT } else { Color::NONE }),
    )
}

/// The toggle glyph of a row with children, or an empty slot.
fn spawn_toggle(
    row: &mut ChildSpawnerCommands,
    (fonts, tunables): MarkerAssets,
    state: Option<bool>,
) -> Entity {
    let glyph = match state {
        Some(true) => EXPANDED_GLYPH,
        Some(false) => COLLAPSED_GLYPH,
        None => "",
    };
    row.spawn((
        Node {
            width: px(TOGGLE_PX),
            ..default()
        },
        Text::new(glyph),
        fonts.data(tunables.sidebar_text_px),
        TextColor(colors::PANEL_BORDER),
    ))
    .id()
}

fn spawn_row(
    list: &mut ChildSpawnerCommands,
    assets: MarkerAssets,
    nodes: &HashMap<Entity, &OutlineNode>,
    row: &Row,
) {
    match *row {
        Row::Entry {
            body,
            depth,
            has_children,
            expanded,
            selected,
        } => {
            if let Some(node) = nodes.get(&body) {
                spawn_entry(
                    list,
                    assets,
                    node,
                    (depth, has_children.then_some(expanded), selected),
                );
            }
        }
        Row::Orbit {
            orbit,
            ref title,
            depth,
            expanded,
        } => spawn_orbit_row(list, assets, orbit, title, depth, expanded),
    }
}

/// INS-05: one entry: its toggle, its symbol at `list_symbol_px` (or its
/// body marker) and its name, in its colour. The toggle expands or
/// collapses its satellites; the rest selects.
fn spawn_entry(
    list: &mut ChildSpawnerCommands,
    assets: MarkerAssets,
    node: &OutlineNode,
    (depth, toggle_state, selected): (usize, Option<bool>, bool),
) {
    let (fonts, tunables) = assets;
    let body = node.body;
    let look = BodyLook {
        class: node.class,
        affiliation: node.affiliation,
        record: node.record.as_ref(),
        // The entry's text gives a group's count (GRP-09); the small
        // symbol carries no amplifiers of a group.
        group: None,
    };
    let (row_node, background) = row_node(depth, selected);
    list.spawn((OutlineRow(body), row_node, background))
        .with_children(|row| {
            let toggle_id = spawn_toggle(row, assets, toggle_state);
            if toggle_state.is_some() {
                row.commands().entity(toggle_id).observe(
                    move |click: On<PointerClick>, mut expanded: ResMut<Expanded>| {
                        if click.button == PointerButton::Primary {
                            toggle(&mut expanded.entries, body);
                        }
                    },
                );
            }
            row.spawn(Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(ENTRY_GAP_PX),
                ..default()
            })
            .with_children(|label| {
                spawn_list_icon(label, assets, look, tunables.list_marker_px / 2.0);
                label.spawn((
                    Text::new(entry_text(node)),
                    fonts.label(tunables.sidebar_text_px),
                    TextColor(look.color(tunables)),
                ));
            })
            .observe(move |click: On<PointerClick>, mut actions: EntryActions| {
                actions.clicked(body, &click)
            });
        });
}

/// INS-05 / HOV-04: a named orbit's row, "High orbit · N", with its
/// toggle; clicking it expands or collapses the members under it. While
/// the transfer planner is open it also picks that orbit's band as the
/// destination (PLNR-10, PLNR-11), and still opens, so the ships in the
/// orbit stay within reach.
fn spawn_orbit_row(
    list: &mut ChildSpawnerCommands,
    assets: MarkerAssets,
    orbit: Entity,
    title: &str,
    depth: usize,
    expanded: bool,
) {
    let (fonts, tunables) = assets;
    let (row_node, background) = row_node(depth, false);
    list.spawn((OutlineRow(orbit), row_node, background))
        .with_children(|row| {
            spawn_toggle(row, assets, Some(expanded));
            row.spawn((
                Text::new(title),
                fonts.label(tunables.sidebar_text_px),
                TextColor(colors::NEUTRAL),
            ));
        })
        .observe(
            move |click: On<PointerClick>,
                  mut expanded: ResMut<Expanded>,
                  mut planner: ResMut<PlannerState>| {
                if click.button == PointerButton::Primary {
                    toggle(&mut expanded.orbits, orbit);
                    planner.pick(PickTarget::Orbit(orbit));
                }
            },
        );
}

/// What clicking an entry can do.
#[derive(bevy::ecs::system::SystemParam)]
pub struct EntryActions<'w> {
    frame: Res<'w, Frame>,
    selection: ResMut<'w, Selection>,
    planner: ResMut<'w, PlannerState>,
    frame_request: ResMut<'w, FrameRequest>,
    planetary: ResMut<'w, PlanetaryViewRequest>,
}

impl EntryActions<'_> {
    /// INS-05: a click selects at once, or, while the transfer planner is
    /// open, picks the entry as the destination (PLNR-10); a double-click
    /// also makes the body the frame (ZOOM-16), or on the frame body calls
    /// the planetary-view hook (ZOOM-17).
    fn clicked(&mut self, body: Entity, click: &On<PointerClick>) {
        if click.button != PointerButton::Primary {
            return;
        }
        if !self.planner.pick(PickTarget::Body(body)) {
            self.selection.select(body);
        }
        if click.count >= 2 {
            double_click(
                &self.frame,
                body,
                &mut self.frame_request,
                &mut self.planetary,
            );
        }
    }
}

/// The scrolling panel the outline sits in.
type Panel<'w, 's> = Query<
    'w,
    's,
    (
        &'static ComputedNode,
        &'static UiGlobalTransform,
        &'static mut ScrollPosition,
    ),
    With<super::inspect::InsetPanel>,
>;

/// INS-05: once the selected row has been laid out, scroll the panel so it
/// is in view.
fn scroll_selection_into_view(
    selection: Res<Selection>,
    mut scroll: ResMut<ScrollToSelection>,
    rows: Query<(&OutlineRow, &ComputedNode, &UiGlobalTransform)>,
    mut panels: Panel,
) {
    if !scroll.0 {
        return;
    }
    let Some(selected) = selection.selected else {
        scroll.0 = false;
        return;
    };
    let Ok((panel, panel_transform, mut position)) = panels.single_mut() else {
        return;
    };
    let Some((_, row, row_transform)) = rows.iter().find(|(row, ..)| row.0 == selected) else {
        return;
    };
    if row.size().y <= 0.0 || panel.size().y <= 0.0 {
        return;
    }
    let scale = panel.inverse_scale_factor();
    let panel_top = (panel_transform.translation.y - panel.size().y / 2.0) * scale;
    let row_top = (row_transform.translation.y - row.size().y / 2.0) * scale - panel_top;
    let row_bottom = row_top + row.size().y * scale;
    position.y += scroll_to_reveal(row_top, row_bottom, panel.size().y * scale);
    scroll.0 = false;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(
        world: &mut World,
        name: &str,
        parent: Option<Entity>,
        radius: f64,
        orbit: Option<(Entity, &str)>,
    ) -> OutlineNode {
        OutlineNode {
            body: world.spawn_empty().id(),
            name: name.to_string(),
            class: BodyClass::Moon,
            affiliation: Affiliation::Own,
            record: None,
            parent,
            orbit_radius_km: radius,
            named_orbit: orbit.map(|(o, n)| (o, n.to_string())),
            members: None,
        }
    }

    /// GRP-09: a group's entry reads its name and its member count; any
    /// other entry its name alone.
    #[test]
    fn a_group_is_listed_by_name_with_its_member_count() {
        let mut world = World::new();
        let ship = node(&mut world, "Orison", None, 0.0, None);
        assert_eq!(entry_text(&ship), "Orison");
        let group = OutlineNode {
            members: Some(4),
            ..node(&mut world, "TF-1", None, 0.0, None)
        };
        assert_eq!(entry_text(&group), "TF-1 · 4");
    }

    /// A star with two planets; the first has empty bands, the second a
    /// station alone in its Low orbit, two moons and a High orbit with two
    /// ships.
    fn system(world: &mut World) -> (Vec<OutlineNode>, Entity) {
        let star = node(world, "Vey", None, 0.0, None);
        let inner = node(world, "Pell", Some(star.body), 1.0e8, None);
        let outer = node(world, "Halden", Some(star.body), 3.0e8, None);
        let high = world.spawn_empty().id();
        let low = world.spawn_empty().id();
        let moon_far = node(world, "IVd", Some(outer.body), 700_000.0, None);
        let highport = node(
            world,
            "Highport",
            Some(outer.body),
            9_000.0,
            Some((low, "Low")),
        );
        let moon_near = node(world, "IVa", Some(outer.body), 57_000.0, None);
        let ship_a = node(
            world,
            "Orison",
            Some(outer.body),
            30_000.0,
            Some((high, "High")),
        );
        let ship_b = node(
            world,
            "Ardent",
            Some(outer.body),
            30_000.0,
            Some((high, "High")),
        );
        let nodes = vec![
            moon_far, star, outer, highport, ship_a, inner, moon_near, ship_b,
        ];
        (nodes, high)
    }

    fn names(rows: &[Row], nodes: &[OutlineNode]) -> Vec<(String, usize)> {
        rows.iter()
            .map(|row| match row {
                Row::Entry { body, depth, .. } => (
                    nodes.iter().find(|n| n.body == *body).unwrap().name.clone(),
                    *depth,
                ),
                Row::Orbit { title, depth, .. } => (title.clone(), *depth),
            })
            .collect()
    }

    /// INS-05: with nothing expanded the outline is the star alone; the
    /// path to a selection and its satellites open, inside out and one
    /// indent per generation, with each named orbit as one row, a lone
    /// member's too, and none for a body whose bands are empty.
    #[test]
    fn the_outline_opens_the_path_to_the_selection() {
        let mut world = World::new();
        let (nodes, high) = system(&mut world);
        let by_name = |name: &str| nodes.iter().find(|n| n.name == name).unwrap().body;
        let mut expanded = Expanded::default();
        let rows = outline_rows(&nodes, &expanded, None);
        assert_eq!(names(&rows, &nodes), [("Vey".to_string(), 0)]);
        assert!(matches!(
            rows[0],
            Row::Entry {
                has_children: true,
                expanded: false,
                ..
            }
        ));

        reveal_selection(&mut expanded, &nodes, by_name("Halden"));
        let rows = outline_rows(&nodes, &expanded, Some(by_name("Halden")));
        let listed = names(&rows, &nodes);
        assert_eq!(
            listed,
            [
                ("Vey".to_string(), 0),
                ("Pell".to_string(), 1),
                ("Halden".to_string(), 1),
                ("Low orbit · 1".to_string(), 2),
                ("High orbit · 2".to_string(), 2),
                ("IVa".to_string(), 2),
                ("IVd".to_string(), 2),
            ]
        );
        assert!(matches!(rows[2], Row::Entry { selected: true, .. }));
        assert!(matches!(
            rows[1],
            Row::Entry {
                has_children: false,
                selected: false,
                ..
            }
        ));
        assert!(matches!(rows[4], Row::Orbit { orbit, expanded: false, .. } if orbit == high));

        // Pell's bands are empty: it has no satellites and no orbit rows.
        let pell = by_name("Pell");
        reveal_selection(&mut expanded, &nodes, pell);
        let rows = outline_rows(&nodes, &expanded, Some(pell));
        let after_pell = &names(&rows, &nodes)[2..];
        assert_eq!(after_pell[0], ("Halden".to_string(), 1));
        toggle(&mut expanded.entries, pell);

        // Selecting a ship in the shared orbit opens that orbit too, and
        // Halden stays open as left.
        reveal_selection(&mut expanded, &nodes, by_name("Ardent"));
        let rows = outline_rows(&nodes, &expanded, Some(by_name("Ardent")));
        let listed = names(&rows, &nodes);
        assert_eq!(listed[5], ("Orison".to_string(), 3));
        assert_eq!(listed[6], ("Ardent".to_string(), 3));
        assert!(matches!(rows[6], Row::Entry { selected: true, .. }));

        // Collapsing Pell's sibling by hand is kept across a reselection.
        toggle(&mut expanded.orbits, high);
        toggle(&mut expanded.entries, by_name("Vey"));
        assert!(!expanded.entries.contains(&by_name("Vey")));
        reveal_selection(&mut expanded, &nodes, by_name("Pell"));
        let rows = outline_rows(&nodes, &expanded, Some(by_name("Pell")));
        let listed = names(&rows, &nodes);
        assert!(
            listed.contains(&("Pell".to_string(), 1)),
            "the path reopens"
        );
        assert!(
            !listed.contains(&("Orison".to_string(), 3)),
            "the orbit stays closed"
        );
    }

    /// INS-05: selecting a lone member of a named orbit opens its row and
    /// lists it there, never outside the row.
    #[test]
    fn selecting_a_lone_member_opens_its_row() {
        let mut world = World::new();
        let (nodes, _) = system(&mut world);
        let highport = nodes.iter().find(|n| n.name == "Highport").unwrap().body;
        let mut expanded = Expanded::default();
        reveal_selection(&mut expanded, &nodes, highport);
        let rows = outline_rows(&nodes, &expanded, Some(highport));
        let listed = names(&rows, &nodes);
        let at = listed.iter().position(|r| r.0 == "Low orbit · 1").unwrap();
        assert_eq!(listed[at + 1], ("Highport".to_string(), 3));
        assert_eq!(listed.iter().filter(|r| r.0 == "Highport").count(), 1);
        assert!(matches!(rows[at], Row::Orbit { expanded: true, .. }));
    }

    /// INS-05: the panel scrolls up for a row above the view, down for one
    /// below, and not at all for one in view.
    #[test]
    fn scrolling_brings_the_row_into_view() {
        assert_eq!(scroll_to_reveal(-30.0, -10.0, 400.0), -30.0);
        assert_eq!(scroll_to_reveal(420.0, 440.0, 400.0), 40.0);
        assert_eq!(scroll_to_reveal(100.0, 120.0, 400.0), 0.0);
        assert_eq!(scroll_to_reveal(0.0, 20.0, 400.0), 0.0);
    }
}
