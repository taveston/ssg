//! Hover tooltip and click-to-inspect panels. Inspecting is separate from
//! the camera: the camera only follows a body via the panel's explicit
//! Focus action.
//!
//! Simplifications made for this pass (noted rather than silently
//! skipped): the leader "line" is a short straight connector rather than a
//! drawn diagonal line; an inspected planet's children un-group instead of
//! fanning out into a laid-out bubble; the rest of the map isn't dimmed
//! yet; "click empty space to close" isn't wired up (Esc closes the
//! unpinned inspection).

use bevy::prelude::*;

use crate::camera::CameraFocus;
use crate::colors;
use crate::fidelity::FidelityImages;
use crate::fonts::Fonts;
use crate::markers::MarkerFor;
use crate::system_data::{BodyClass, Description, DisplayName, KeyFacts, Knowledge, Ownership};

#[derive(Resource, Default)]
pub struct Inspections {
    pub primary: Option<Entity>,
    pub pinned: Vec<Entity>,
}

impl Inspections {
    pub fn is_open(&self, entity: Entity) -> bool {
        self.primary == Some(entity) || self.pinned.contains(&entity)
    }
}

#[derive(Component)]
pub(crate) struct TooltipRoot;
#[derive(Component)]
pub(crate) struct TooltipLabel;

#[derive(Component)]
struct InspectPanelFor {
    body: Entity,
}
#[derive(Component)]
struct LeaderFor(Entity);

const PANEL_WIDTH: f32 = 240.0;
const LEADER_LENGTH: f32 = 28.0;

pub struct HoverInspectPlugin;

impl Plugin for HoverInspectPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Inspections>()
            .add_systems(Startup, spawn_tooltip)
            .add_systems(
                Update,
                (handle_escape, rebuild_panels, reposition_panels).chain(),
            );
    }
}

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
            p.spawn((TooltipLabel, Text::new(""), fonts.label(12.0), TextColor(colors::NEUTRAL)));
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

fn ownership_label(ownership: Ownership) -> &'static str {
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
            "{} · {}{} — Click: inspect",
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

pub fn on_marker_click(
    click: On<PointerClick>,
    markers: Query<&MarkerFor>,
    mut inspections: ResMut<Inspections>,
) {
    let Ok(marker_for) = markers.get(click.entity) else {
        return;
    };
    let body = marker_for.0;
    if inspections.is_open(body) {
        return;
    }
    inspections.primary = Some(body);
}

fn handle_escape(keys: Res<ButtonInput<KeyCode>>, mut inspections: ResMut<Inspections>) {
    if keys.just_pressed(KeyCode::Escape) {
        inspections.primary = None;
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn rebuild_panels(
    inspections: Res<Inspections>,
    mut commands: Commands,
    fonts: Res<Fonts>,
    fidelity: Res<FidelityImages>,
    existing_panels: Query<Entity, With<InspectPanelFor>>,
    existing_leaders: Query<Entity, With<LeaderFor>>,
    bodies: Query<(
        &DisplayName,
        &Description,
        &KeyFacts,
        &BodyClass,
        &Ownership,
        &Knowledge,
        Option<&Children>,
    )>,
    names: Query<&DisplayName>,
) {
    if !inspections.is_changed() {
        return;
    }

    for entity in &existing_panels {
        commands.entity(entity).despawn();
    }
    for entity in &existing_leaders {
        commands.entity(entity).despawn();
    }

    let mut open: Vec<(Entity, bool)> = inspections.pinned.iter().map(|&e| (e, true)).collect();
    if let Some(primary) = inspections.primary {
        open.push((primary, false));
    }

    for (body, pinned) in open {
        spawn_panel(&mut commands, &fonts, &fidelity, body, pinned, &bodies, &names);
        commands.spawn((
            LeaderFor(body),
            Node {
                position_type: PositionType::Absolute,
                width: px(LEADER_LENGTH),
                height: px(2),
                display: Display::None,
                ..default()
            },
            BackgroundColor(colors::PANEL_BORDER.with_alpha(if pinned { 0.4 } else { 0.8 })),
        ));
    }
}

#[allow(clippy::type_complexity)]
fn spawn_panel(
    commands: &mut Commands,
    fonts: &Fonts,
    fidelity: &FidelityImages,
    body: Entity,
    pinned: bool,
    bodies: &Query<(
        &DisplayName,
        &Description,
        &KeyFacts,
        &BodyClass,
        &Ownership,
        &Knowledge,
        Option<&Children>,
    )>,
    names: &Query<&DisplayName>,
) {
    let Ok((name, description, facts, class, ownership, knowledge, children)) = bodies.get(body) else {
        return;
    };

    let class_line = format!("{:?} · {}", class, ownership_label(*ownership));
    let (status_text, status_color, image_handle, show_corner_brackets) = match knowledge {
        Knowledge::Surveyed { imaged_day } => (
            format!(
                "SURVEYED · {}",
                crate::date::format_date(&crate::date::date_from_elapsed_days(*imaged_day))
            ),
            colors::OWN,
            fidelity.surveyed.clone(),
            false,
        ),
        Knowledge::Unsurveyed => (
            "TELESCOPE FRAME · UNSURVEYED".to_string(),
            colors::FOREIGN,
            fidelity.unsurveyed.clone(),
            true,
        ),
        Knowledge::Stale { last_imaged_day } => (
            format!(
                "LAST IMAGED · {}",
                crate::date::format_time_ago(-*last_imaged_day)
            ),
            Color::srgb(0.65, 0.55, 0.4),
            fidelity.stale.clone(),
            false,
        ),
    };

    let name_text = name.0.clone();
    let description_text = description.0.clone();
    let facts_list = facts.0.clone();
    let child_list: Vec<Entity> = children.map(|c| c.iter().collect()).unwrap_or_default();

    commands
        .spawn((
            InspectPanelFor { body },
            Node {
                position_type: PositionType::Absolute,
                width: px(PANEL_WIDTH),
                padding: UiRect::all(px(10)),
                flex_direction: FlexDirection::Column,
                row_gap: px(5),
                border: UiRect::all(px(1)),
                display: Display::None,
                ..default()
            },
            BackgroundColor(colors::PANEL),
            BorderColor::all(if pinned {
                colors::PANEL_BORDER.with_alpha(0.5)
            } else {
                colors::PANEL_BORDER
            }),
        ))
        .with_children(|p| {
            p.spawn(Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|header| {
                header.spawn((
                    Text::new(name_text),
                    fonts.label_semibold(15.0),
                    TextColor(colors::NEUTRAL),
                ));
                header
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: px(8),
                        ..default()
                    })
                    .with_children(|actions| {
                        actions
                            .spawn((Text::new("Focus"), fonts.label(11.0), TextColor(colors::OWN)))
                            .observe(move |_click: On<PointerClick>, mut focus: ResMut<CameraFocus>| {
                                *focus = CameraFocus::Following(body);
                            });
                        actions
                            .spawn((
                                Text::new(if pinned { "Unpin" } else { "Pin" }),
                                fonts.label(11.0),
                                TextColor(colors::NEUTRAL),
                            ))
                            .observe(move |_click: On<PointerClick>, mut inspections: ResMut<Inspections>| {
                                if pinned {
                                    inspections.pinned.retain(|&e| e != body);
                                } else {
                                    if inspections.primary == Some(body) {
                                        inspections.primary = None;
                                    }
                                    if !inspections.pinned.contains(&body) {
                                        inspections.pinned.push(body);
                                    }
                                }
                            });
                        actions
                            .spawn((Text::new("Close"), fonts.label(11.0), TextColor(colors::FOREIGN)))
                            .observe(move |_click: On<PointerClick>, mut inspections: ResMut<Inspections>| {
                                if inspections.primary == Some(body) {
                                    inspections.primary = None;
                                }
                                inspections.pinned.retain(|&e| e != body);
                            });
                    });
            });

            p.spawn((
                Text::new(class_line),
                fonts.label(11.0),
                TextColor(colors::PANEL_BORDER),
            ));

            // Fidelity image: procedurally generated per knowledge state.
            p.spawn(Node {
                width: percent(100),
                height: px(64),
                ..default()
            })
            .with_children(|frame| {
                frame.spawn((
                    ImageNode::new(image_handle),
                    Node {
                        width: percent(100),
                        height: percent(100),
                        ..default()
                    },
                ));
                if show_corner_brackets {
                    for (top, right) in [(true, true), (true, false), (false, true), (false, false)] {
                        frame.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                width: px(10),
                                height: px(10),
                                top: if top { px(0) } else { Val::Auto },
                                bottom: if top { Val::Auto } else { px(0) },
                                left: if right { Val::Auto } else { px(0) },
                                right: if right { px(0) } else { Val::Auto },
                                border: UiRect {
                                    top: if top { px(2) } else { px(0) },
                                    bottom: if top { px(0) } else { px(2) },
                                    left: if right { px(0) } else { px(2) },
                                    right: if right { px(2) } else { px(0) },
                                },
                                ..default()
                            },
                            BorderColor::all(colors::FOREIGN),
                        ));
                    }
                }
            });
            p.spawn((
                Text::new(status_text),
                fonts.mono(10.0),
                TextColor(status_color),
            ));

            p.spawn((
                Text::new(description_text),
                fonts.label(11.0),
                TextColor(colors::NEUTRAL),
            ));

            for (key, value) in facts_list {
                p.spawn((
                    Text::new(format!("{key}: {value}")),
                    fonts.mono(11.0),
                    TextColor(colors::NEUTRAL),
                ));
            }

            if !child_list.is_empty() {
                p.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: px(4),
                    row_gap: px(4),
                    ..default()
                })
                .with_children(|chips| {
                    for child in child_list {
                        let label = names.get(child).map(|n| n.0.clone()).unwrap_or_default();
                        chips
                            .spawn((
                                Node {
                                    padding: UiRect::axes(px(6), px(2)),
                                    border: UiRect::all(px(1)),
                                    ..default()
                                },
                                BorderColor::all(colors::PANEL_BORDER),
                            ))
                            .with_children(|c| {
                                c.spawn((
                                    Text::new(label),
                                    fonts.label(10.0),
                                    TextColor(colors::NEUTRAL),
                                ));
                            })
                            .observe(move |_click: On<PointerClick>, mut inspections: ResMut<Inspections>| {
                                inspections.primary = Some(child);
                            });
                    }
                });
            }

            p.spawn((
                Node {
                    padding: UiRect::all(px(6)),
                    border: UiRect::all(px(1)),
                    ..default()
                },
                BorderColor::all(colors::PANEL_BORDER.with_alpha(0.3)),
            ))
            .with_children(|b| {
                b.spawn((
                    Text::new("Open planetary view"),
                    fonts.label(11.0),
                    TextColor(colors::PANEL_BORDER.with_alpha(0.4)),
                ));
            });
        });
}

fn reposition_panels(
    camera_q: Query<(&Camera, &GlobalTransform)>,
    bodies: Query<&GlobalTransform>,
    mut panels: Query<(&InspectPanelFor, &mut Node)>,
    mut leaders: Query<(&LeaderFor, &mut Node), Without<InspectPanelFor>>,
) {
    let Ok((camera, camera_transform)) = camera_q.single() else {
        return;
    };

    for (panel_for, mut node) in &mut panels {
        let Ok(body_transform) = bodies.get(panel_for.body) else {
            node.display = Display::None;
            continue;
        };
        let Ok(pos) = camera.world_to_viewport(camera_transform, body_transform.translation()) else {
            node.display = Display::None;
            continue;
        };
        node.display = Display::Flex;
        node.left = px(pos.x + LEADER_LENGTH);
        node.top = px(pos.y - 60.0);
    }

    for (leader_for, mut node) in &mut leaders {
        let Ok(body_transform) = bodies.get(leader_for.0) else {
            node.display = Display::None;
            continue;
        };
        let Ok(pos) = camera.world_to_viewport(camera_transform, body_transform.translation()) else {
            node.display = Display::None;
            continue;
        };
        node.display = Display::Flex;
        node.left = px(pos.x);
        node.top = px(pos.y - 1.0);
    }
}
