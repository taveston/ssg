//! Hover and click on the map (HOV-01, INS-01, GRP-07): the tooltip, the
//! white hover ring, and the pointer handlers every pickable body node
//! (marker, pill, exposure-ring hit target) is observed by.
//!
//! A click selects (`selection.rs`); a double-click asks the camera to frame
//! the body (`camera/requests.rs`), or on an already selected body is the
//! planetary-view hook. Those double-click rules follow the withdrawn
//! ZOOM-08 and ZOOM-09 until the camera iteration (F4-13).

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::camera::requests::{FocusJumpRequest, PlanetaryViewRequest};
use super::colors;
use super::fold::FoldSummary;
use super::fonts::Fonts;
use super::gesture::{PressStart, was_click};
use super::selection::Selection;
use super::targets::{GroupPillFor, MarkerFor};
use super::tunables::Tunables;
use crate::Boot;
use crate::sim::body::{BodyClass, DisplayName, Ownership};

pub struct HoverPlugin;

impl Plugin for HoverPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_tooltip.in_set(Boot::Overlay));
    }
}

#[derive(Component)]
pub(crate) struct TooltipRoot;
#[derive(Component)]
pub(crate) struct TooltipLabel;

fn spawn_tooltip(mut commands: Commands, fonts: Res<Fonts>) {
    commands
        .spawn((
            TooltipRoot,
            Node {
                position_type: PositionType::Absolute,
                padding: UiRect::axes(px(8), px(4)),
                display: Display::None,
                ..default()
            },
            BackgroundColor(colors::PANEL),
            BorderColor::all(colors::PANEL_BORDER),
        ))
        .with_children(|p| {
            p.spawn((
                TooltipLabel,
                Text::new(""),
                fonts.label(12.0),
                TextColor(colors::NEUTRAL),
            ));
        });
}

fn child_counts(children: &Children, classes: &Query<&BodyClass>) -> (usize, usize) {
    let mut moons = 0;
    let mut stations = 0;
    for child in children.iter() {
        match classes.get(child) {
            Ok(BodyClass::Moon) => moons += 1,
            Ok(BodyClass::Station) => stations += 1,
            _ => {}
        }
    }
    (moons, stations)
}

pub fn ownership_label(ownership: Ownership) -> &'static str {
    match ownership {
        Ownership::Own => "own",
        Ownership::Foreign => "foreign",
        Ownership::Neutral => "neutral",
    }
}

pub fn on_marker_over(
    over: On<PointerOver>,
    markers: Query<&MarkerFor>,
    bodies: Query<(&DisplayName, &Ownership, Option<&Children>)>,
    classes: Query<&BodyClass>,
    mut outlines: Query<&mut Outline>,
    mut tooltip_nodes: Query<&mut Node, With<TooltipRoot>>,
    mut tooltip_labels: Query<&mut Text, With<TooltipLabel>>,
) {
    if let Ok(mut outline) = outlines.get_mut(over.entity) {
        outline.width = px(2);
        outline.color = colors::SELECTION_RING;
    }
    let Ok(marker_for) = markers.get(over.entity) else {
        return;
    };
    let Ok((name, ownership, children)) = bodies.get(marker_for.0) else {
        return;
    };
    let extra = match children {
        Some(children) => {
            let (moons, stations) = child_counts(children, &classes);
            match (moons, stations) {
                (0, 0) => String::new(),
                (m, 0) => format!(" · {m} moons"),
                (0, s) => format!(" · {s} stations"),
                (m, s) => format!(" · {m} moons, {s} stations"),
            }
        }
        None => String::new(),
    };

    for mut text in &mut tooltip_labels {
        *text = Text::new(format!(
            "{} · {}{} — Click: inspect · Double-click: frame its moons · Double-click again: planetary view",
            name.0,
            ownership_label(*ownership),
            extra
        ));
    }
    for mut node in &mut tooltip_nodes {
        node.display = Display::Flex;
    }
}

pub fn on_marker_out(
    out: On<PointerOut>,
    mut outlines: Query<&mut Outline>,
    mut tooltip_nodes: Query<&mut Node, With<TooltipRoot>>,
) {
    if let Ok(mut outline) = outlines.get_mut(out.entity) {
        outline.width = px(0);
        outline.color = Color::NONE;
    }
    for mut node in &mut tooltip_nodes {
        node.display = Display::None;
    }
}

#[allow(clippy::too_many_arguments)]
/// What a click handler needs to tell a selecting click from the release
/// of a drag or a reserved button.
#[derive(SystemParam)]
pub struct ClickCheck<'w, 's> {
    press: Res<'w, PressStart>,
    tunables: Res<'w, Tunables>,
    windows: Query<'w, 's, &'static Window>,
}

impl ClickCheck<'_, '_> {
    /// A click that selects: the primary button, and not the release of a
    /// drag (CAM-10). Right-clicks are reserved for context menus (CAM-13).
    fn selects(&self, click: &On<PointerClick>) -> bool {
        let release = self.windows.single().ok().and_then(Window::cursor_position);
        click.button == PointerButton::Primary
            && release.is_none_or(|at| was_click(&self.press, at, self.tunables.drag_threshold_px))
    }
}

pub fn on_marker_click(
    click: On<PointerClick>,
    markers: Query<&MarkerFor>,
    check: ClickCheck,
    mut selection: ResMut<Selection>,
    mut focus_jump: ResMut<FocusJumpRequest>,
    mut planetary: ResMut<PlanetaryViewRequest>,
) {
    if !check.selects(&click) {
        return;
    }
    let Ok(marker_for) = markers.get(click.entity) else {
        return;
    };
    let body = marker_for.0;

    if click.count >= 2 {
        if selection.is_open(body) {
            // Second double-click on an already-inspected body is the
            // planetary-view trigger. That view doesn't exist yet, so this
            // just records the request as the hook for it.
            planetary.0 = Some(body);
        } else {
            selection.select(body);
            focus_jump.0 = Some(body);
        }
        return;
    }

    selection.select(body);
}

pub fn on_pill_click(
    click: On<PointerClick>,
    pills: Query<&GroupPillFor>,
    check: ClickCheck,
    mut selection: ResMut<Selection>,
) {
    if !check.selects(&click) {
        return;
    }
    let Ok(pill) = pills.get(click.entity) else {
        return;
    };
    // GRP-07: clicking a pill is clicking its parent.
    selection.select(pill.0);
}

/// Hovering a count pill lists what's folded into it.
pub fn on_pill_over(
    over: On<PointerOver>,
    pills: Query<&GroupPillFor>,
    names: Query<&DisplayName>,
    folds: Res<FoldSummary>,
    mut tooltip_nodes: Query<&mut Node, With<TooltipRoot>>,
    mut tooltip_labels: Query<&mut Text, With<TooltipLabel>>,
) {
    let Ok(pill) = pills.get(over.entity) else {
        return;
    };
    let Some(group) = folds.0.get(&pill.0) else {
        return;
    };
    let parent_name = names
        .get(pill.0)
        .map(|name| name.0.clone())
        .unwrap_or_default();
    let listed = group
        .members
        .iter()
        .filter_map(|entity| names.get(*entity).ok().map(|name| name.0.clone()))
        .collect::<Vec<_>>()
        .join(", ");

    for mut text in &mut tooltip_labels {
        *text = Text::new(format!("In orbit of {parent_name}: {listed}"));
    }
    for mut node in &mut tooltip_nodes {
        node.display = Display::Flex;
    }
}

pub fn on_pill_out(_out: On<PointerOut>, mut tooltip_nodes: Query<&mut Node, With<TooltipRoot>>) {
    for mut node in &mut tooltip_nodes {
        node.display = Display::None;
    }
}
