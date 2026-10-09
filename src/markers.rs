//! The vector-display overlay: one fixed-size UI marker per body, projected
//! from its true-scale 3D position each frame, plus clutter grouping into
//! count pills and fading trails for fast bodies.

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;

use crate::colors;
use crate::fonts::Fonts;
use crate::hover_inspect::{self, Inspections};
use crate::orbits::OrbitalBody;
use crate::system_data::{BodyClass, DisplayName, Ownership};
use crate::time_control::GameClock;

/// Marker UI node -> the body entity it represents.
#[derive(Component)]
pub struct MarkerFor(pub Entity);

/// Half-size in pixels, so the update system can centre the node on its
/// projected screen position without re-deriving size from class.
#[derive(Component)]
pub struct MarkerHalfSize(pub Vec2);

/// The group-count pill for a planet's clutter-eligible children.
#[derive(Component)]
pub struct GroupPillFor(pub Entity);

#[derive(Component)]
pub struct PillCountText;

/// Trail of recent *world*-space positions for fast bodies (Pell, inner
/// moons). Kept in world space, not screen space, and re-projected through
/// the current camera every frame - otherwise a static camera makes the
/// trail collapse to the body's own (often sub-pixel) motion, while any pan
/// or zoom shows up as a spurious "trail" that's really camera movement.
///
/// Samples are spaced by simulated days (a fraction of the body's own
/// orbital period), not render frames - a fixed frame count covers only a
/// fraction of a real-time second at 1 day/s, far too little orbital motion
/// to produce a visible arc. Spacing by a fraction of the period instead
/// keeps the trail a sensible, visible length for both fast moons and
/// slower planets, and scales with playback speed automatically since it's
/// keyed off simulated days rather than wall-clock time.
#[derive(Component)]
pub struct FadingTrail {
    pub positions: Vec<Vec3>,
    pub last_sample_day: f64,
}

impl Default for FadingTrail {
    fn default() -> Self {
        Self {
            positions: Vec::new(),
            // So the first sample is taken immediately rather than after a
            // full interval has passed.
            last_sample_day: f64::NEG_INFINITY,
        }
    }
}

/// One of a fixed pool of small dot nodes used to draw a body's fading
/// trail: index 0 is the oldest sample, `TRAIL_LENGTH - 1` the newest.
#[derive(Component)]
pub struct TrailDotFor(pub Entity, pub usize);

/// Edge-of-screen arrowhead for a gate that's off-screen, rotated each
/// frame to point toward it.
#[derive(Component)]
pub struct EdgeIndicatorFor(pub Entity);

/// The label next to an edge indicator - a sibling, not a child, so it
/// doesn't rotate along with the arrowhead.
#[derive(Component)]
pub struct EdgeLabelFor(pub Entity);

const PILL_OVERLAP_PX: f32 = 26.0;
const TRAIL_LENGTH: usize = 16;
const EDGE_MARGIN_PX: f32 = 28.0;
/// The trail covers this fraction of a body's own orbital period, so both
/// fast moons and slow planets get a trail that's a sensible, visible
/// length rather than a fixed (and often sub-pixel) number of frames.
const TRAIL_SPAN_FRACTION_OF_PERIOD: f64 = 0.08;

fn marker_half_size(class: BodyClass) -> Vec2 {
    match class {
        BodyClass::Star => Vec2::splat(9.0),
        BodyClass::Planet => Vec2::splat(6.0),
        BodyClass::Moon => Vec2::splat(3.0),
        BodyClass::Station => Vec2::splat(5.0),
        BodyClass::Gate => Vec2::splat(5.5),
        BodyClass::SalvageSite => Vec2::splat(7.0),
    }
}

pub struct MarkersPlugin;

impl Plugin for MarkersPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Startup,
            spawn_markers.after(crate::scene::spawn_system),
        )
        .add_systems(Update, (update_markers, update_trails).chain());
    }
}

fn spawn_markers(
    mut commands: Commands,
    fonts: Res<Fonts>,
    bodies: Query<(Entity, &BodyClass, &Ownership, &DisplayName)>,
) {
    for (entity, class, ownership, name) in &bodies {
        let half = marker_half_size(*class);
        let color = colors::ownership_color(*ownership);

        let base_node = Node {
            position_type: PositionType::Absolute,
            width: px(half.x * 2.0),
            height: px(half.y * 2.0),
            display: Display::None,
            ..default()
        };

        let marker_entity = match class {
            BodyClass::Star | BodyClass::Planet | BodyClass::Moon => commands
                .spawn((
                    MarkerFor(entity),
                    MarkerHalfSize(half),
                    Node {
                        border_radius: BorderRadius::MAX,
                        ..base_node.clone()
                    },
                    BackgroundColor(color),
                    Outline {
                        width: px(0),
                        offset: px(2),
                        color: Color::NONE,
                    },
                ))
                .id(),
            BodyClass::Station => commands
                .spawn((
                    MarkerFor(entity),
                    MarkerHalfSize(half),
                    Node {
                        border: UiRect::all(px(1.5)),
                        ..base_node.clone()
                    },
                    BackgroundColor(Color::NONE),
                    BorderColor::all(color),
                    Outline {
                        width: px(0),
                        offset: px(2),
                        color: Color::NONE,
                    },
                ))
                .id(),
            BodyClass::Gate => commands
                .spawn((
                    MarkerFor(entity),
                    MarkerHalfSize(half),
                    Node {
                        border: UiRect::all(px(1.5)),
                        border_radius: BorderRadius::MAX,
                        ..base_node.clone()
                    },
                    BackgroundColor(Color::NONE),
                    BorderColor::all(color),
                    Outline {
                        width: px(0),
                        offset: px(2),
                        color: Color::NONE,
                    },
                ))
                .id(),
            BodyClass::SalvageSite => {
                let marker_entity = commands
                    .spawn((
                        MarkerFor(entity),
                        MarkerHalfSize(half),
                        base_node.clone(),
                        Outline {
                            width: px(0),
                            offset: px(2),
                            color: Color::NONE,
                        },
                    ))
                    .id();
                commands.entity(marker_entity).with_children(|parent| {
                    parent.spawn((
                        Text::new("X"),
                        fonts.label(half.x * 1.8),
                        TextColor(color),
                    ));
                });
                marker_entity
            }
        };

        commands
            .entity(marker_entity)
            .observe(hover_inspect::on_marker_over)
            .observe(hover_inspect::on_marker_out)
            .observe(hover_inspect::on_marker_click);

        // Label, positioned just right of the marker.
        commands.entity(marker_entity).with_children(|parent| {
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(half.x * 2.0 + 4.0),
                    top: px(0),
                    ..default()
                },
                Text::new(name.0.clone()),
                fonts.label(12.0),
                TextColor(color),
            ));
        });

        if matches!(class, BodyClass::Gate) {
            commands
                .spawn((
                    EdgeIndicatorFor(entity),
                    MarkerHalfSize(half),
                    Node {
                        position_type: PositionType::Absolute,
                        width: px(half.x * 2.0),
                        height: px(half.y * 2.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        display: Display::None,
                        ..default()
                    },
                    UiTransform::IDENTITY,
                ))
                .with_children(|parent| {
                    // Points up by default; rotated per-frame to point
                    // toward the gate.
                    parent.spawn((Text::new("^"), fonts.label(half.x * 2.2), TextColor(color)));
                });

            commands.spawn((
                EdgeLabelFor(entity),
                Node {
                    position_type: PositionType::Absolute,
                    display: Display::None,
                    ..default()
                },
                Text::new(name.0.clone()),
                fonts.label(12.0),
                TextColor(color),
            ));
        }

        if matches!(class, BodyClass::Planet) {
            commands.spawn((
                GroupPillFor(entity),
                Node {
                    position_type: PositionType::Absolute,
                    display: Display::None,
                    padding: UiRect::axes(px(5), px(1)),
                    border_radius: BorderRadius::MAX,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.07, 0.09, 0.11, 0.9)),
                children![(PillCountText, Text::new(""), fonts.mono(11.0), TextColor(colors::NEUTRAL))],
            ));
        }

        if matches!(name.0.as_str(), "Pell" | "Halden IVa" | "Halden IVb") {
            commands.entity(entity).insert(FadingTrail::default());
            for i in 0..TRAIL_LENGTH {
                commands.spawn((
                    TrailDotFor(entity, i),
                    Node {
                        position_type: PositionType::Absolute,
                        width: px(3),
                        height: px(3),
                        display: Display::None,
                        border_radius: BorderRadius::MAX,
                        ..default()
                    },
                    BackgroundColor(color.with_alpha(0.0)),
                ));
            }
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn update_markers(
    camera_q: Query<(&Camera, &GlobalTransform)>,
    windows: Query<&Window>,
    bodies: Query<(Entity, &GlobalTransform, &BodyClass)>,
    parent_of: Query<&ChildOf>,
    inspections: Res<Inspections>,
    mut markers: Query<(&MarkerFor, &MarkerHalfSize, &mut Node)>,
    mut pills: Query<(&GroupPillFor, &mut Node, &Children), (Without<MarkerFor>, Without<EdgeIndicatorFor>)>,
    mut pill_texts: Query<&mut Text, With<PillCountText>>,
    mut edge_indicators: Query<
        (&EdgeIndicatorFor, &MarkerHalfSize, &mut Node, &mut UiTransform),
        (Without<MarkerFor>, Without<GroupPillFor>),
    >,
    mut edge_labels: Query<(&EdgeLabelFor, &mut Node), (Without<MarkerFor>, Without<GroupPillFor>, Without<EdgeIndicatorFor>)>,
) {
    let Ok((camera, camera_transform)) = camera_q.single() else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    let viewport = Vec2::new(window.width(), window.height());

    let mut screen_pos: HashMap<Entity, Vec2> = HashMap::new();
    for (entity, transform, _) in &bodies {
        if let Ok(pos) = camera.world_to_viewport(camera_transform, transform.translation()) {
            screen_pos.insert(entity, pos);
        }
    }

    // Group clutter-eligible children by parent when they overlap on screen.
    let mut candidates: HashMap<Entity, Vec<Entity>> = HashMap::new();
    for (entity, _, class) in &bodies {
        if !matches!(class, BodyClass::Moon | BodyClass::Station) {
            continue;
        }
        let Ok(child_of) = parent_of.get(entity) else {
            continue;
        };
        let parent = child_of.parent();
        let Ok((_, _, parent_class)) = bodies.get(parent) else {
            continue;
        };
        if *parent_class != BodyClass::Planet {
            continue;
        }
        if inspections.is_open(parent) {
            // An inspected planet's children un-group so they can be
            // clicked individually.
            continue;
        }
        let (Some(&child_pos), Some(&parent_pos)) = (screen_pos.get(&entity), screen_pos.get(&parent))
        else {
            continue;
        };
        if child_pos.distance(parent_pos) < PILL_OVERLAP_PX {
            candidates.entry(parent).or_default().push(entity);
        }
    }

    let mut hidden: HashSet<Entity> = HashSet::new();
    let mut pill_counts: HashMap<Entity, u32> = HashMap::new();
    for (parent, children) in &candidates {
        if children.len() >= 2 {
            for &child in children {
                hidden.insert(child);
            }
            pill_counts.insert(*parent, children.len() as u32);
        }
    }

    let in_viewport = |pos: Vec2| pos.x >= 0.0 && pos.y >= 0.0 && pos.x <= viewport.x && pos.y <= viewport.y;

    // Gates that fall outside a small margin get clamped to the viewport
    // edge instead of being hidden outright.
    let center = viewport / 2.0;
    let half_extent = viewport / 2.0 - Vec2::splat(EDGE_MARGIN_PX);
    let mut gate_edge_targets: HashMap<Entity, Vec2> = HashMap::new();
    for (entity, _, class) in &bodies {
        if *class != BodyClass::Gate {
            continue;
        }
        let Some(&pos) = screen_pos.get(&entity) else {
            continue;
        };
        let delta = pos - center;
        let sx = if delta.x.abs() > f32::EPSILON {
            half_extent.x / delta.x.abs()
        } else {
            f32::INFINITY
        };
        let sy = if delta.y.abs() > f32::EPSILON {
            half_extent.y / delta.y.abs()
        } else {
            f32::INFINITY
        };
        let scale = sx.min(sy);
        if scale < 1.0 {
            gate_edge_targets.insert(entity, center + delta * scale.max(0.0));
        }
    }

    for (marker_for, half, mut node) in &mut markers {
        let body = marker_for.0;
        let visible = !hidden.contains(&body)
            && !gate_edge_targets.contains_key(&body)
            && screen_pos.get(&body).is_some_and(|&pos| in_viewport(pos));
        if !visible {
            node.display = Display::None;
            continue;
        }
        let pos = screen_pos[&body];
        node.display = Display::Flex;
        node.left = px(pos.x - half.0.x);
        node.top = px(pos.y - half.0.y);
    }

    for (pill_for, mut node, children) in &mut pills {
        let parent = pill_for.0;
        let Some(&count) = pill_counts.get(&parent) else {
            node.display = Display::None;
            continue;
        };
        let Some(&pos) = screen_pos.get(&parent) else {
            node.display = Display::None;
            continue;
        };
        node.display = Display::Flex;
        node.left = px(pos.x + 10.0);
        node.top = px(pos.y + 10.0);
        if let Some(&text_entity) = children.first()
            && let Ok(mut text) = pill_texts.get_mut(text_entity)
        {
            *text = Text::new(format!("+{count}"));
        }
    }

    for (edge_for, half, mut node, mut transform) in &mut edge_indicators {
        let Some(&pos) = gate_edge_targets.get(&edge_for.0) else {
            node.display = Display::None;
            continue;
        };
        node.display = Display::Flex;
        node.left = px(pos.x - half.0.x);
        node.top = px(pos.y - half.0.y);
        // The glyph points up by default; rotate it (clockwise) to point
        // from the viewport centre toward the gate's true direction.
        let dir = pos - center;
        let angle = dir.x.atan2(-dir.y);
        transform.rotation = Rot2::radians(angle);
    }

    for (label_for, mut node) in &mut edge_labels {
        let Some(&pos) = gate_edge_targets.get(&label_for.0) else {
            node.display = Display::None;
            continue;
        };
        node.display = Display::Flex;
        node.left = px(pos.x + 10.0);
        node.top = px(pos.y + 10.0);
    }
}

fn update_trails(
    clock: Res<GameClock>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut bodies: Query<(Entity, &GlobalTransform, &OrbitalBody, &mut FadingTrail)>,
    mut dots: Query<(&TrailDotFor, &mut Node, &mut BackgroundColor)>,
) {
    let Ok((camera, camera_transform)) = camera_q.single() else {
        return;
    };

    let mut trails: HashMap<Entity, Vec<Vec3>> = HashMap::new();
    for (entity, transform, orbital_body, mut trail) in &mut bodies {
        let span_days = orbital_body.period_days * TRAIL_SPAN_FRACTION_OF_PERIOD;
        let interval_days = span_days / TRAIL_LENGTH as f64;
        if clock.elapsed_days - trail.last_sample_day >= interval_days {
            trail.last_sample_day = clock.elapsed_days;
            trail.positions.push(transform.translation());
            if trail.positions.len() > TRAIL_LENGTH {
                trail.positions.remove(0);
            }
        }
        trails.insert(entity, trail.positions.clone());
    }

    for (dot_for, mut node, mut background) in &mut dots {
        let Some(positions) = trails.get(&dot_for.0) else {
            node.display = Display::None;
            continue;
        };
        // Index 0 is the oldest slot; map it to the oldest *available* sample.
        let age_from_newest = TRAIL_LENGTH - 1 - dot_for.1;
        let Some(world_pos) = positions.len().checked_sub(age_from_newest + 1).map(|i| positions[i]) else {
            node.display = Display::None;
            continue;
        };
        // Re-project through the *current* camera every frame, so the trail
        // reflects the body's own motion rather than any camera movement.
        let Ok(pos) = camera.world_to_viewport(camera_transform, world_pos) else {
            node.display = Display::None;
            continue;
        };
        node.display = Display::Flex;
        node.left = px(pos.x - 1.5);
        node.top = px(pos.y - 1.5);
        let fade = 1.0 - age_from_newest as f32 / TRAIL_LENGTH as f32;
        background.0 = background.0.with_alpha(fade * 0.5);
    }
}
