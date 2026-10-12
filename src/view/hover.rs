//! Hover and click on the map (HOV-01, INS-01, GRP-07): the tooltip, the
//! white hover ring, and the pointer handlers every pickable body node
//! (marker, pill, exposure-ring hit target) is observed by.
//!
//! A click selects (`selection.rs`); a double-click also focuses the body
//! and frames its satellites (ZOOM-16), or on the focused body is the
//! planetary-view hook (ZOOM-17), through `camera/requests.rs`.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::camera::requests::{FocusRequest, PlanetaryViewRequest};
use super::colors;
use super::focus::Focus;
use super::fold::FoldSummary;
use super::fonts::Fonts;
use super::gesture::{PressStart, was_click};
use super::selection::Selection;
use super::targets::{GroupPillFor, Hovered, MarkerFor};
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

/// Ownership as the word the tooltip and inset show.
pub fn ownership_label(ownership: Ownership) -> &'static str {
    match ownership {
        Ownership::Own => "own",
        Ownership::Foreign => "foreign",
        Ownership::Neutral => "neutral",
    }
}

/// A marker's tooltip: name, ownership and what orbits it, then how to
/// click it (HOV-01).
pub fn marker_tooltip(name: &str, ownership: Ownership, moons: usize, stations: usize) -> String {
    let contents = match (moons, stations) {
        (0, 0) => String::new(),
        (m, 0) => format!(" · {m} moons"),
        (0, s) => format!(" · {s} stations"),
        (m, s) => format!(" · {m} moons, {s} stations"),
    };
    format!(
        "{name} · {}{contents} — Click: inspect · Double-click: frame its moons · Double-click again: planetary view",
        ownership_label(ownership)
    )
}

pub fn on_marker_over(
    over: On<PointerOver>,
    markers: Query<&MarkerFor>,
    bodies: Query<(&DisplayName, &Ownership, Option<&Children>)>,
    classes: Query<&BodyClass>,
    mut commands: Commands,
    mut tooltip_nodes: Query<&mut Node, With<TooltipRoot>>,
    mut tooltip_labels: Query<&mut Text, With<TooltipLabel>>,
) {
    // HOV-01: the white ring, drawn by the marker overlay.
    commands.entity(over.entity).insert(Hovered);
    let Ok(marker_for) = markers.get(over.entity) else {
        return;
    };
    let Ok((name, ownership, children)) = bodies.get(marker_for.0) else {
        return;
    };
    let (moons, stations) = children.map_or((0, 0), |children| child_counts(children, &classes));
    let tooltip = marker_tooltip(&name.0, *ownership, moons, stations);
    for mut text in &mut tooltip_labels {
        *text = Text::new(tooltip.clone());
    }
    for mut node in &mut tooltip_nodes {
        node.display = Display::Flex;
    }
}

pub fn on_marker_out(
    out: On<PointerOut>,
    mut commands: Commands,
    mut tooltip_nodes: Query<&mut Node, With<TooltipRoot>>,
) {
    if let Ok(mut node) = commands.get_entity(out.entity) {
        node.remove::<Hovered>();
    }
    for mut node in &mut tooltip_nodes {
        node.display = Display::None;
    }
}

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

/// What a click on a body does, from its marker or its pill alike. A
/// click selects (INS-01); a double-click also focuses the body, or, on
/// the body already focused, is the planetary-view trigger (ZOOM-16,
/// ZOOM-17). Focus, not selection, decides.
#[derive(SystemParam)]
pub struct BodyClick<'w> {
    focus: Res<'w, Focus>,
    selection: ResMut<'w, Selection>,
    focus_request: ResMut<'w, FocusRequest>,
    planetary: ResMut<'w, PlanetaryViewRequest>,
}

impl BodyClick<'_> {
    fn click(&mut self, body: Entity, count: u8) {
        self.selection.select(body);
        if count >= 2 {
            if self.focus.is_on(body) {
                self.planetary.0 = Some(body);
            } else {
                self.focus_request.0 = Some(body);
            }
        }
    }
}

pub fn on_marker_click(
    click: On<PointerClick>,
    markers: Query<&MarkerFor>,
    check: ClickCheck,
    mut body_click: BodyClick,
) {
    if !check.selects(&click) {
        return;
    }
    if let Ok(marker_for) = markers.get(click.entity) {
        body_click.click(marker_for.0, click.count);
    }
}

/// Clicking a "+N" pill does exactly what clicking its parent does
/// (GRP-07).
pub fn on_pill_click(
    click: On<PointerClick>,
    pills: Query<&GroupPillFor>,
    check: ClickCheck,
    mut body_click: BodyClick,
) {
    if !check.selects(&click) {
        return;
    }
    if let Ok(pill) = pills.get(click.entity) {
        body_click.click(pill.0, click.count);
    }
}

/// A pill's tooltip lists what is folded into it (GRP-07).
pub fn pill_tooltip(parent: &str, folded: &[String]) -> String {
    format!("In orbit of {parent}: {}", folded.join(", "))
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
    let listed: Vec<String> = group
        .members
        .iter()
        .filter_map(|entity| names.get(*entity).ok().map(|name| name.0.clone()))
        .collect();
    for mut text in &mut tooltip_labels {
        *text = Text::new(pill_tooltip(&parent_name, &listed));
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::camera::NormalizedRenderTarget;
    use bevy::picking::backend::HitData;
    use bevy::picking::pointer::{Location, PointerId};

    use super::*;

    /// HOV-01: name, ownership and contents, then the click hints.
    #[test]
    fn marker_tooltip_names_the_body_its_ownership_and_contents() {
        let hints = " — Click: inspect · Double-click: frame its moons · Double-click again: planetary view";
        assert_eq!(
            marker_tooltip("Halden IV", Ownership::Own, 4, 1),
            format!("Halden IV · own · 4 moons, 1 stations{hints}")
        );
        assert_eq!(
            marker_tooltip("Pell", Ownership::Neutral, 0, 0),
            format!("Pell · neutral{hints}")
        );
        assert!(
            marker_tooltip("K-3", Ownership::Foreign, 0, 2)
                .starts_with("K-3 · foreign · 2 stations —")
        );
    }

    /// GRP-07: the pill's tooltip lists the folded satellites.
    #[test]
    fn pill_tooltip_lists_the_folded_satellites() {
        let folded = ["Halden IVa".to_string(), "K-3".to_string()];
        assert_eq!(
            pill_tooltip("Halden IV", &folded),
            "In orbit of Halden IV: Halden IVa, K-3"
        );
    }

    /// A world with the click handlers' resources, a body, and a marker
    /// and a pill that both stand for it.
    fn click_world() -> (World, Entity, Entity, Entity) {
        let mut world = World::new();
        world.init_resource::<Focus>();
        world.init_resource::<Selection>();
        world.init_resource::<FocusRequest>();
        world.init_resource::<PlanetaryViewRequest>();
        world.init_resource::<PressStart>();
        world.init_resource::<Tunables>();
        let body = world.spawn_empty().id();
        let marker = world.spawn(MarkerFor(body)).observe(on_marker_click).id();
        let pill = world.spawn(GroupPillFor(body)).observe(on_pill_click).id();
        (world, body, marker, pill)
    }

    fn mouse() -> bevy::picking::events::Pointer {
        bevy::picking::events::Pointer::new(
            PointerId::Mouse,
            Location {
                target: NormalizedRenderTarget::None {
                    width: 1600,
                    height: 900,
                },
                position: Vec2::ZERO,
            },
        )
    }

    fn click(world: &mut World, entity: Entity, count: u8) {
        let camera = world.spawn_empty().id();
        world.trigger(PointerClick {
            entity,
            pointer: mouse(),
            button: PointerButton::Primary,
            hit: HitData::new(camera, 0.0, None, None),
            duration: Duration::from_millis(80),
            count,
        });
    }

    /// What a run of clicks left behind: selection, focus request and
    /// planetary-view request.
    fn outcome(world: &World) -> (Option<Entity>, Option<Entity>, Option<Entity>) {
        (
            world.resource::<Selection>().primary,
            world.resource::<FocusRequest>().0,
            world.resource::<PlanetaryViewRequest>().0,
        )
    }

    /// GRP-07: clicking a pill does exactly what clicking its parent's
    /// marker does - select on one click, ask for focus on a double-click,
    /// and the planetary view on a double-click of the focused body.
    #[test]
    fn clicking_a_pill_is_clicking_its_parent() {
        for (count, focused) in [(1, false), (2, false), (2, true)] {
            let mut results = Vec::new();
            for on_pill in [false, true] {
                let (mut world, body, marker, pill) = click_world();
                if focused {
                    world.resource_mut::<Focus>().body = Some(body);
                }
                click(&mut world, if on_pill { pill } else { marker }, count);
                results.push((outcome(&world), body));
            }
            assert_eq!(results[0], results[1], "{count} clicks, focused {focused}");
            let ((selected, focus_request, planetary), body) = results[0];
            assert_eq!(selected, Some(body));
            assert_eq!(focus_request.is_some(), count == 2 && !focused);
            assert_eq!(planetary.is_some(), count == 2 && focused);
        }
    }

    /// HOV-01: hovering a marker marks it for the white ring and shows the
    /// tooltip with the body's name; leaving it undoes both.
    #[test]
    fn hovering_a_marker_rings_it_and_shows_the_tooltip() {
        let mut world = World::new();
        let tooltip = world
            .spawn((
                TooltipRoot,
                Node {
                    display: Display::None,
                    ..default()
                },
            ))
            .id();
        let label = world.spawn((TooltipLabel, Text::new(""))).id();
        let body = world
            .spawn((DisplayName("Morrow".into()), Ownership::Neutral))
            .id();
        let marker = world
            .spawn(MarkerFor(body))
            .observe(on_marker_over)
            .observe(on_marker_out)
            .id();
        let camera = world.spawn_empty().id();
        let hit = HitData::new(camera, 0.0, None, None);

        world.trigger(PointerOver {
            entity: marker,
            pointer: mouse(),
            hit: hit.clone(),
        });
        world.flush();
        assert!(world.get::<Hovered>(marker).is_some());
        assert_eq!(world.get::<Node>(tooltip).unwrap().display, Display::Flex);
        assert!(
            world
                .get::<Text>(label)
                .unwrap()
                .0
                .starts_with("Morrow · neutral —")
        );

        world.trigger(PointerOut {
            entity: marker,
            pointer: mouse(),
            hit,
        });
        world.flush();
        assert!(world.get::<Hovered>(marker).is_none());
        assert_eq!(world.get::<Node>(tooltip).unwrap().display, Display::None);
    }
}
