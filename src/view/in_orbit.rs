//! The inset's "In orbit" list (INS-05): one entry per satellite of the
//! inspected body, inside out, each with its class marker and its name in
//! own or foreign colours (MRK-01). Clicking an entry selects that
//! satellite; double-clicking it makes it the frame and moves to it
//! (ZOOM-16), or on the frame body calls the planetary-view hook (ZOOM-17).
//!
//! Selecting a satellite rebuilds the inset as the satellite's own, which
//! would remove the list before a double-click's second click arrived. So a
//! single click on an entry takes effect once `double_click_window_ms` has
//! passed without a second one (F5-07).

use bevy::prelude::*;

use super::camera::requests::{FrameRequest, PlanetaryViewRequest};
use super::colors;
use super::fonts::Fonts;
use super::frame::Frame;
use super::hover::double_click;
use super::marker_images::MarkerImages;
use super::markers::spawn_marker_icon;
use super::selection::Selection;
use super::tunables::Tunables;
use crate::Stage;
use crate::sim::body::{BodyClass, DisplayName, Ownership};
use crate::sim::orbit::Orbit;

const ENTRY_FONT_PX: f32 = 11.0;
const HEADING_FONT_PX: f32 = 10.0;
/// Half the size of an entry's class marker.
const ICON_HALF_PX: f32 = 4.0;
const ENTRY_GAP_PX: f32 = 6.0;

/// One satellite in the list.
#[derive(Debug, Clone, PartialEq)]
pub struct OrbitEntry {
    pub body: Entity,
    pub name: String,
    pub class: BodyClass,
    pub ownership: Ownership,
    pub orbit_radius_km: f64,
}

/// INS-05: the entries in orbit order, from the inside out.
pub fn in_orbit_order(mut entries: Vec<OrbitEntry>) -> Vec<OrbitEntry> {
    entries.sort_by(|a, b| a.orbit_radius_km.total_cmp(&b.orbit_radius_km));
    entries
}

/// A body's satellites, as list entries in orbit order.
pub fn orbit_entries(
    children: Option<&Children>,
    satellites: &Query<(&DisplayName, &BodyClass, &Ownership, &Orbit)>,
) -> Vec<OrbitEntry> {
    let entries = children
        .into_iter()
        .flat_map(|children| children.iter())
        .filter_map(|child| {
            let (name, class, ownership, orbit) = satellites.get(child).ok()?;
            Some(OrbitEntry {
                body: child,
                name: name.0.clone(),
                class: *class,
                ownership: *ownership,
                orbit_radius_km: orbit.radius_km,
            })
        })
        .collect();
    in_orbit_order(entries)
}

/// A single click on an entry, waiting out the double-click window.
#[derive(Resource, Default)]
pub struct PendingEntryClick(Option<(Entity, f32)>);

pub struct InOrbitPlugin;

impl Plugin for InOrbitPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PendingEntryClick>()
            .add_systems(Update, commit_entry_click.in_set(Stage::Overlay));
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

/// INS-05: the list, under the inset's facts. Nothing for a body with no
/// satellites.
pub fn spawn_in_orbit_list(
    panel: &mut ChildSpawnerCommands,
    fonts: &Fonts,
    images: &MarkerImages,
    entries: &[OrbitEntry],
) {
    if entries.is_empty() {
        return;
    }
    panel.spawn((
        Text::new("In orbit"),
        fonts.label(HEADING_FONT_PX),
        TextColor(colors::PANEL_BORDER),
    ));
    for entry in entries {
        spawn_entry(panel, fonts, images, entry);
    }
}

fn spawn_entry(
    panel: &mut ChildSpawnerCommands,
    fonts: &Fonts,
    images: &MarkerImages,
    entry: &OrbitEntry,
) {
    let body = entry.body;
    panel
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(ENTRY_GAP_PX),
            ..default()
        })
        .with_children(|row| {
            spawn_marker_icon(
                row,
                fonts,
                images,
                (entry.class, entry.ownership),
                ICON_HALF_PX,
            );
            row.spawn((
                Text::new(entry.name.clone()),
                fonts.label(ENTRY_FONT_PX),
                TextColor(colors::ownership_color(entry.ownership)),
            ));
        })
        .observe(move |click: On<PointerClick>, mut actions: EntryActions| {
            actions.clicked(body, &click)
        });
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

    /// INS-05: inside out, whatever order the satellites were spawned in.
    #[test]
    fn entries_run_from_the_inside_out() {
        let mut world = World::new();
        let entry = |name: &str, radius: f64, world: &mut World| OrbitEntry {
            body: world.spawn_empty().id(),
            name: name.to_string(),
            class: BodyClass::Moon,
            ownership: Ownership::Own,
            orbit_radius_km: radius,
        };
        let entries = vec![
            entry("IVd", 726_432.0, &mut world),
            entry("Arden Highport", 9_394.0, &mut world),
            entry("IVa", 57_137.0, &mut world),
        ];
        let names: Vec<String> = in_orbit_order(entries)
            .into_iter()
            .map(|e| e.name)
            .collect();
        assert_eq!(names, ["Arden Highport", "IVa", "IVd"]);
        assert!(in_orbit_order(Vec::new()).is_empty());
    }
}
