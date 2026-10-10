//! Inspection insets (INS-01 to INS-04, MOT-07): one panel per open
//! selection, anchored to its marker by a leader line, rebuilt whenever the
//! selection changes.
//!
//! Simplifications carried from iteration 2: the leader "line" is a short
//! straight connector rather than a drawn diagonal; the rest of the map
//! isn't dimmed; Esc closes the unpinned inset (INS-03), and clicking empty
//! space does nothing.

use bevy::prelude::*;

use super::camera::state::CameraFocus;
use super::colors;
use super::fidelity::FidelityImages;
use super::fonts::Fonts;
use super::hover::ownership_label;
use super::selection::Selection;
use crate::Stage;
use crate::sim::body::{BodyClass, Description, DisplayName, KeyFacts, Knowledge, Ownership};

#[derive(Component)]
struct InspectPanelFor {
    body: Entity,
}
#[derive(Component)]
struct LeaderFor(Entity);

const PANEL_WIDTH: f32 = 240.0;
const LEADER_LENGTH: f32 = 28.0;
pub struct InspectPlugin;

impl Plugin for InspectPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (handle_escape, rebuild_panels, reposition_panels)
                .chain()
                .in_set(Stage::Overlay),
        );
    }
}

/// INS-03: Esc closes the unpinned inset.
fn handle_escape(keys: Res<ButtonInput<KeyCode>>, mut selection: ResMut<Selection>) {
    if keys.just_pressed(KeyCode::Escape) {
        selection.close_unpinned();
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn rebuild_panels(
    selection: Res<Selection>,
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
    if !selection.is_changed() {
        return;
    }

    for entity in &existing_panels {
        commands.entity(entity).despawn();
    }
    for entity in &existing_leaders {
        commands.entity(entity).despawn();
    }

    let mut open: Vec<(Entity, bool)> = selection.pinned.iter().map(|&e| (e, true)).collect();
    if let Some(primary) = selection.primary {
        open.push((primary, false));
    }

    for (body, pinned) in open {
        spawn_panel(
            &mut commands,
            &fonts,
            &fidelity,
            body,
            pinned,
            &bodies,
            &names,
        );
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
    let Ok((name, description, facts, class, ownership, knowledge, children)) = bodies.get(body)
    else {
        return;
    };

    let class_line = format!("{:?} · {}", class, ownership_label(*ownership));
    let (status_text, status_color, image_handle, show_corner_brackets) = match knowledge {
        Knowledge::Surveyed { imaged_day } => (
            format!(
                "SURVEYED · {}",
                crate::view::date::format_date(&crate::view::date::date_from_elapsed_days(
                    *imaged_day
                ))
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
                crate::view::date::format_time_ago(-*last_imaged_day)
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
                            .observe(move |_click: On<PointerClick>, mut selection: ResMut<Selection>| {
                                if pinned {
                                    selection.unpin(body);
                                } else {
                                    selection.pin(body);
                                }
                            });
                        actions
                            .spawn((Text::new("Close"), fonts.label(11.0), TextColor(colors::FOREIGN)))
                            .observe(move |_click: On<PointerClick>, mut selection: ResMut<Selection>| {
                                selection.close(body);
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
                            .observe(move |_click: On<PointerClick>, mut selection: ResMut<Selection>| {
                                selection.primary = Some(child);
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
        let Ok(pos) = camera.world_to_viewport(camera_transform, body_transform.translation())
        else {
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
        let Ok(pos) = camera.world_to_viewport(camera_transform, body_transform.translation())
        else {
            node.display = Display::None;
            continue;
        };
        node.display = Display::Flex;
        node.left = px(pos.x);
        node.top = px(pos.y - 1.0);
    }
}
