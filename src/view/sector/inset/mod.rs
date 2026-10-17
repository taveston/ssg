//! The sector view's inspection inset (SECT-20 to SECT-24): one panel
//! for the one selected entry, in the sidebar the system view's inset
//! uses (INS-07) and in its look (INS-09). It slides with the sidebar and
//! is rebuilt whenever the selection changes: a header with the name, the
//! Enter system button for a gated system and Close; the class line; the
//! key facts; and the Planets, Belts and Gates sections. A click on a
//! gate row selects the destination's node and leaves the camera alone.
//!
//! What it says is worked out in `content.rs`.

use bevy::input::mouse::AccumulatedMouseScroll;
use bevy::prelude::*;

use content::{InsetRow, SectorInset, sector_inset};

use super::EnterRequest;
use crate::content::start::LoadedSector;
use crate::view::buttons::{ButtonStyle, ConsoleButton, spawn_button};
use crate::view::colors;
use crate::view::fonts::Fonts;
use crate::view::inset::inspect::{
    PanelAssets, panel_column, sidebar_panel, sidebar_right_px, spawn_facts, spawn_section_heading,
    spawn_text, wheel_scroll,
};
use crate::view::map_layer::{MapLayer, SectorSelection};
use crate::view::sidebar::SidebarState;
use crate::view::targets::PointerTargets;
use crate::view::tunables::Tunables;

pub mod content;

/// The space between a header's buttons.
const BUTTON_GAP_PX: f32 = 8.0;
/// The space between rows in a section.
const ROW_GAP_PX: f32 = 6.0;

/// The sector view's sidebar panel.
#[derive(Component)]
pub struct SectorInsetPanel;

/// The part of the panel rebuilt for each selection.
#[derive(Component)]
pub struct SectorInsetDetails;

/// SECT-21: the Enter system button, for the gated system it enters.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnterButton(pub usize);

/// SECT-24: a gate row, for the entry it leads to.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct GateRowFor(pub usize);

/// A row's first line, as the tests read it.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct RowTitle(pub String);

/// SECT-20: the panel, off the right edge and hidden until it slides in.
pub fn spawn_panel(mut commands: Commands, tunables: Res<Tunables>) {
    commands
        .spawn((SectorInsetPanel, sidebar_panel(&tunables)))
        .with_children(|panel| {
            panel.spawn((SectorInsetDetails, panel_column()));
        });
}

/// SECT-20 / INS-07: the panel follows the sidebar's slide, and shows only
/// with the sector view.
pub fn slide_panel(
    (sidebar, layer): (Res<SidebarState>, Res<MapLayer>),
    tunables: Res<Tunables>,
    mut panels: Query<&mut Node, With<SectorInsetPanel>>,
) {
    for mut node in &mut panels {
        let right = px(sidebar_right_px(sidebar.slide, tunables.inset_width_px));
        let display = if sidebar.visible() && layer.is_sector() {
            Display::Flex
        } else {
            Display::None
        };
        if node.right != right || node.display != display {
            node.right = right;
            node.display = display;
        }
    }
}

/// INS-07: the wheel over the panel scrolls it, within its content.
pub fn scroll_panel(
    scroll: Res<AccumulatedMouseScroll>,
    targets: PointerTargets,
    mut panels: Query<(Entity, &mut ScrollPosition, &ComputedNode), With<SectorInsetPanel>>,
) {
    let Ok((panel, mut position, computed)) = panels.single_mut() else {
        return;
    };
    if scroll.delta.y != 0.0 && targets.panel() == Some(panel) {
        wheel_scroll(&scroll, &mut position, computed);
    }
}

/// SECT-20: whenever the selection changes the details are replaced in
/// place; with nothing selected they are emptied, and the sidebar slides
/// out.
pub fn rebuild_inset(
    mut commands: Commands,
    selection: Res<SectorSelection>,
    sector: Option<Res<LoadedSector>>,
    (fonts, tunables): (Res<Fonts>, Res<Tunables>),
    details: Query<Entity, With<SectorInsetDetails>>,
) {
    if !selection.is_changed() {
        return;
    }
    let Ok(details) = details.single() else {
        return;
    };
    let threshold = tunables.distance_au_threshold_au;
    let content = sector
        .as_ref()
        .zip(selection.0)
        .filter(|(loaded, id)| *id < loaded.sector.entries.len())
        .map(|(loaded, id)| sector_inset(&loaded.sector, id, threshold));
    let mut details = commands.entity(details);
    details.despawn_related::<Children>();
    if let Some(content) = content {
        details.with_children(|panel| fill_panel(panel, (&fonts, &tunables), &content));
    }
}

/// SECT-21 to SECT-24: header, class line, key facts, then the planets,
/// belts and gates.
fn fill_panel(panel: &mut ChildSpawnerCommands, assets: PanelAssets, content: &SectorInset) {
    let (fonts, tunables) = assets;
    spawn_header(panel, assets, content);
    spawn_text(
        panel,
        &content.class_line,
        fonts.label(tunables.sidebar_text_px),
        colors::PANEL_BORDER,
    );
    spawn_facts(panel, assets, &content.facts);
    if !content.planets.is_empty() {
        spawn_section_heading(panel, assets, "Planets");
        spawn_rows(panel, assets, &content.planets);
    }
    if !content.belts.is_empty() {
        spawn_section_heading(panel, assets, "Belts");
        spawn_rows(panel, assets, &content.belts);
    }
    if !content.gates.is_empty() {
        spawn_section_heading(panel, assets, "Gates");
        for gate in &content.gates {
            spawn_row(panel, assets, &gate.row)
                .insert(GateRowFor(gate.destination))
                .observe(select_destination);
        }
    }
}

/// SECT-21: the name, then the Enter system button for a gated system
/// and the Close button. There is no Focus button.
fn spawn_header(panel: &mut ChildSpawnerCommands, assets: PanelAssets, content: &SectorInset) {
    let (fonts, tunables) = assets;
    panel
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|header| {
            header.spawn((
                Text::new(content.name.clone()),
                fonts.label_semibold(tunables.sidebar_header_px),
                TextColor(colors::NEUTRAL),
            ));
            header
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: px(BUTTON_GAP_PX),
                    ..default()
                })
                .with_children(|actions| spawn_actions(actions, assets, content));
        });
}

/// SECT-21: Enter system (secondary) asks for the system to be entered;
/// Close (quiet) clears the selection.
fn spawn_actions(actions: &mut ChildSpawnerCommands, assets: PanelAssets, content: &SectorInset) {
    let (fonts, tunables) = assets;
    if content.enterable {
        let id = content.id;
        spawn_button(
            actions,
            fonts,
            "Enter system",
            tunables.sidebar_text_px,
            ConsoleButton::new(ButtonStyle::Secondary),
        )
        .insert(EnterButton(id))
        .observe(
            move |click: On<PointerClick>, mut enter: ResMut<EnterRequest>| {
                if click.button == PointerButton::Primary {
                    enter.0 = Some(id);
                }
            },
        );
    }
    spawn_button(
        actions,
        fonts,
        "Close",
        tunables.sidebar_text_px,
        ConsoleButton::new(ButtonStyle::Quiet),
    )
    .observe(
        |_click: On<PointerClick>, mut selection: ResMut<SectorSelection>| {
            selection.0 = None;
        },
    );
}

/// SECT-23: a section's rows.
fn spawn_rows(panel: &mut ChildSpawnerCommands, assets: PanelAssets, rows: &[InsetRow]) {
    for row in rows {
        spawn_row(panel, assets, row);
    }
}

/// One row: its first line in the label face in the neutral colour, its
/// second in the data face in the secondary colour.
fn spawn_row<'a>(
    panel: &'a mut ChildSpawnerCommands,
    (fonts, tunables): PanelAssets,
    row: &InsetRow,
) -> EntityCommands<'a> {
    let mut entity = panel.spawn((
        RowTitle(row.title.clone()),
        Node {
            flex_direction: FlexDirection::Column,
            margin: UiRect::bottom(px(ROW_GAP_PX)),
            ..default()
        },
    ));
    entity.with_children(|lines| {
        lines.spawn((
            Text::new(row.title.clone()),
            fonts.label(tunables.sidebar_text_px),
            TextColor(colors::NEUTRAL),
            Pickable::IGNORE,
        ));
        lines.spawn((
            Text::new(row.detail.clone()),
            fonts.data(tunables.sidebar_caps_px),
            TextColor(colors::SECONDARY),
            Pickable::IGNORE,
        ));
    });
    entity
}

/// SECT-24: a click on a gate row selects the destination's node; the
/// camera stays where it is.
fn select_destination(
    click: On<PointerClick>,
    rows: Query<&GateRowFor>,
    mut selection: ResMut<SectorSelection>,
) {
    if click.button != PointerButton::Primary {
        return;
    }
    if let Ok(row) = rows.get(click.entity) {
        selection.0 = Some(row.0);
    }
}
