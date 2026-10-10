//! The fan-out magnifier (`docs/spec/system-view.md`, MAG-01 to MAG-07).
//!
//! Selecting a body that has folded satellites opens a circular magnifier
//! centred on it: a dashed frame over the dimmed map, showing the folded
//! satellites at *true* phase and true relative radii, scaled up so the
//! outermost included orbit fills the frame. It closes when the selection
//! changes, on Esc, or once the satellites unfold normally.

use bevy::prelude::*;

use crate::Stage;
use crate::colors;
use crate::fonts::Fonts;
use crate::grouping::FoldSummary;
use crate::hover_inspect::{self, Inspections};
use crate::markers::MarkerFor;
use crate::motion::MotionModeState;
use crate::orbits::OrbitalBody;
use crate::system_data::{BodyClass, BodyRadius, DisplayName, Ownership};
use crate::tunables::Tunables;

/// What the magnifier is currently showing. Only structural changes land
/// here, so the UI is rebuilt when the contents change and not every frame.
#[derive(Resource, Default, PartialEq)]
pub struct MagnifierState {
    pub open_for: Option<Entity>,
    /// Satellites shown inside the frame, innermost first.
    pub included: Vec<Entity>,
    /// Outermost small moons left out because the innermost orbit would
    /// otherwise crowd the centre.
    pub dropped_outer: usize,
}

/// How much bigger than true scale the contents are drawn. Recomputed
/// every frame as the camera moves, so it's kept apart from
/// `MagnifierState` to avoid triggering a rebuild.
#[derive(Resource, Default)]
pub struct MagnifierView {
    pub magnification: f32,
}

#[derive(Component)]
struct MagnifierRoot;
#[derive(Component)]
struct MagnifierFrame;
#[derive(Component)]
struct MagnificationLabel;
#[derive(Component)]
struct OuterCountLabel;
/// A marker drawn inside the magnifier. Deliberately lacks the components
/// `markers.rs` keys on, so the ordinary overlay leaves these alone.
#[derive(Component)]
struct MagnifierMarker(Entity);

/// A satellite's label inside the magnifier, kept separate from its marker
/// so it can move onto the ring when the marker is hidden.
#[derive(Component)]
struct MagnifierLabel(Entity);

const DASH_COUNT: usize = 28;
const DASH_LENGTH_PX: f32 = 7.0;
const RING_SEGMENTS: usize = 96;
const NEAR_EPSILON: f32 = 0.01;

pub struct MagnifierPlugin;

impl Plugin for MagnifierPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MagnifierState>()
            .init_resource::<MagnifierView>()
            .add_systems(
                Update,
                (
                    update_magnifier_state,
                    rebuild_magnifier_ui,
                    position_magnifier,
                    draw_magnifier_rings,
                )
                    .chain()
                    .in_set(Stage::Magnifier),
            );
    }
}

/// Screen pixels per render unit, measured at a body's depth.
fn pixels_per_unit(camera: &Camera, camera_transform: &GlobalTransform, at: Vec3) -> Option<f32> {
    let right = camera_transform.right().as_vec3();
    let right_flat = Vec3::new(right.x, 0.0, right.z).normalize_or_zero();
    let a = camera.world_to_viewport(camera_transform, at).ok()?;
    let b = camera
        .world_to_viewport(camera_transform, at + right_flat)
        .ok()?;
    let scale = a.distance(b);
    (scale > f32::EPSILON).then_some(scale)
}

#[allow(clippy::too_many_arguments)]
fn update_magnifier_state(
    tunables: Res<Tunables>,
    inspections: Res<Inspections>,
    folds: Res<FoldSummary>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    transforms: Query<&GlobalTransform>,
    bodies: Query<(&OrbitalBody, &BodyClass, Option<&BodyRadius>)>,
    mut state: ResMut<MagnifierState>,
    mut view: ResMut<MagnifierView>,
) {
    let Ok((camera, camera_transform)) = camera_q.single() else {
        return;
    };

    // Opens with the selection, and only while that body still has
    // something folded into it: zoom in far enough that everything
    // unfolds and the magnifier hands over to the real view.
    let candidate = inspections.primary.filter(|body| {
        folds
            .0
            .get(body)
            .is_some_and(|group| !group.members.is_empty())
    });

    let Some(parent) = candidate else {
        if state.open_for.is_some() {
            *state = MagnifierState::default();
        }
        return;
    };

    let Some(group) = folds.0.get(&parent) else {
        return;
    };
    let Ok(parent_transform) = transforms.get(parent) else {
        return;
    };
    let Some(px_per_unit) =
        pixels_per_unit(camera, camera_transform, parent_transform.translation())
    else {
        return;
    };

    // Stations and parked ships always make the cut; moons have to clear
    // the size threshold.
    let mut kept: Vec<(Entity, f32)> = group
        .members
        .iter()
        .filter_map(|member| {
            let (orbital, class, radius) = bodies.get(*member).ok()?;
            let always = matches!(class, BodyClass::Station | BodyClass::Ship);
            let big_enough = radius
                .map(|r| r.0 >= tunables.magnifier_moon_radius_threshold_km)
                .unwrap_or(true);
            (always || big_enough).then_some((*member, orbital.orbit_radius))
        })
        .collect();
    kept.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    // True relative radii are kept, so if the innermost orbit would crowd
    // the centre the only remedy is to drop the outermost and rescale.
    let mut dropped = 0;
    while kept.len() > 1 {
        let innermost = kept[0].1;
        let outermost = kept[kept.len() - 1].1;
        if outermost <= f32::EPSILON {
            break;
        }
        let innermost_px = innermost / outermost * tunables.magnifier_radius_px;
        if innermost_px < tunables.magnifier_inner_min_px {
            kept.pop();
            dropped += 1;
        } else {
            break;
        }
    }

    let outermost = kept.last().map(|(_, radius)| *radius).unwrap_or(0.0);
    view.magnification = if outermost > f32::EPSILON {
        tunables.magnifier_radius_px / (outermost * px_per_unit)
    } else {
        1.0
    };

    // Only write when the contents actually change, so the UI isn't torn
    // down and rebuilt every frame.
    let next = MagnifierState {
        open_for: Some(parent),
        included: kept.into_iter().map(|(entity, _)| entity).collect(),
        dropped_outer: dropped,
    };
    if *state != next {
        *state = next;
    }
}

/// Rebuilt whole whenever what's shown changes - infrequent enough that
/// pooling isn't worth the complexity, mirroring the inspection panels.
fn rebuild_magnifier_ui(
    state: Res<MagnifierState>,
    mut commands: Commands,
    fonts: Res<Fonts>,
    existing: Query<Entity, With<MagnifierRoot>>,
    bodies: Query<(&BodyClass, &Ownership, &DisplayName)>,
) {
    if !state.is_changed() {
        return;
    }
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    let Some(_parent) = state.open_for else {
        return;
    };

    // Dim the rest of the map. Spawned before the frame and its contents,
    // so it sits underneath them.
    commands.spawn((
        MagnifierRoot,
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.45)),
    ));

    // Dashed circular frame, built from short rotated segments.
    for index in 0..DASH_COUNT {
        let angle = index as f32 / DASH_COUNT as f32 * std::f32::consts::TAU;
        commands.spawn((
            MagnifierRoot,
            MagnifierFrame,
            Node {
                position_type: PositionType::Absolute,
                width: px(DASH_LENGTH_PX),
                height: px(1.5),
                display: Display::None,
                ..default()
            },
            BackgroundColor(colors::PANEL_BORDER.with_alpha(0.8)),
            UiTransform::from_rotation(Rot2::radians(angle)),
        ));
    }

    commands.spawn((
        MagnifierRoot,
        MagnificationLabel,
        Node {
            position_type: PositionType::Absolute,
            display: Display::None,
            ..default()
        },
        Text::new(""),
        fonts.mono(11.0),
        TextColor(colors::PANEL_BORDER),
    ));

    if state.dropped_outer > 0 {
        commands.spawn((
            MagnifierRoot,
            OuterCountLabel,
            Node {
                position_type: PositionType::Absolute,
                display: Display::None,
                ..default()
            },
            Text::new(format!("+{} outer", state.dropped_outer)),
            fonts.mono(11.0),
            TextColor(colors::PANEL_BORDER),
        ));
    }

    // Interactive markers for the satellites inside, wired to the same
    // hover/click observers as everything else.
    for member in &state.included {
        let Ok((class, ownership, name)) = bodies.get(*member) else {
            continue;
        };
        let color = colors::ownership_color(*ownership);
        let size = match class {
            BodyClass::Station => 7.0,
            BodyClass::Ship => 9.0,
            _ => 6.0,
        };

        let mut entity = commands.spawn((
            MagnifierRoot,
            MagnifierMarker(*member),
            MarkerFor(*member),
            Node {
                position_type: PositionType::Absolute,
                width: px(size),
                height: px(size),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                display: Display::None,
                ..default()
            },
            Outline {
                width: px(0),
                offset: px(2),
                color: Color::NONE,
            },
        ));

        match class {
            BodyClass::Station => {
                entity.insert((
                    BackgroundColor(Color::NONE),
                    BorderColor::all(color),
                    Node {
                        position_type: PositionType::Absolute,
                        width: px(size),
                        height: px(size),
                        border: UiRect::all(px(1.5)),
                        display: Display::None,
                        ..default()
                    },
                ));
            }
            BodyClass::Ship => {
                let glyph = if *ownership == Ownership::Foreign {
                    "<>"
                } else {
                    "^"
                };
                entity.with_children(|parent| {
                    parent.spawn((Text::new(glyph), fonts.label(11.0), TextColor(color)));
                });
            }
            _ => {
                entity.insert((
                    BackgroundColor(color),
                    Node {
                        position_type: PositionType::Absolute,
                        width: px(size),
                        height: px(size),
                        border_radius: BorderRadius::MAX,
                        display: Display::None,
                        ..default()
                    },
                ));
            }
        }

        let marker = entity.id();
        commands
            .entity(marker)
            .observe(hover_inspect::on_marker_over)
            .observe(hover_inspect::on_marker_out)
            .observe(hover_inspect::on_marker_click);

        // The label is a sibling, not a child: in exposure-ring mode the
        // dot is gone and the label has to sit on the ring instead.
        commands.spawn((
            MagnifierRoot,
            MagnifierLabel(*member),
            Node {
                position_type: PositionType::Absolute,
                display: Display::None,
                ..default()
            },
            Text::new(name.0.clone()),
            fonts.label(10.0),
            TextColor(color),
        ));
    }
}

/// Where a satellite sits inside the magnifier: its true offset from the
/// parent, scaled up, then projected through the real camera so the
/// view's tilt carries over.
fn magnified_world(parent_world: Vec3, satellite_world: Vec3, magnification: f32) -> Vec3 {
    parent_world + (satellite_world - parent_world) * magnification
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn position_magnifier(
    state: Res<MagnifierState>,
    view: Res<MagnifierView>,
    tunables: Res<Tunables>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    transforms: Query<&GlobalTransform>,
    mut frames: Query<
        &mut Node,
        (
            With<MagnifierFrame>,
            Without<MagnificationLabel>,
            Without<OuterCountLabel>,
            Without<MagnifierMarker>,
            Without<MagnifierLabel>,
        ),
    >,
    mut magnification_labels: Query<
        (&mut Node, &mut Text),
        (
            With<MagnificationLabel>,
            Without<MagnifierFrame>,
            Without<OuterCountLabel>,
            Without<MagnifierMarker>,
            Without<MagnifierLabel>,
        ),
    >,
    mut outer_labels: Query<
        &mut Node,
        (
            With<OuterCountLabel>,
            Without<MagnifierFrame>,
            Without<MagnificationLabel>,
            Without<MagnifierMarker>,
            Without<MagnifierLabel>,
        ),
    >,
    mut markers: Query<
        (&MagnifierMarker, &mut Node),
        (
            Without<MagnifierFrame>,
            Without<MagnificationLabel>,
            Without<OuterCountLabel>,
            Without<MagnifierLabel>,
        ),
    >,
    mut member_labels: Query<
        (&MagnifierLabel, &mut Node),
        (
            Without<MagnifierFrame>,
            Without<MagnificationLabel>,
            Without<OuterCountLabel>,
            Without<MagnifierMarker>,
        ),
    >,
    motion: Query<(&OrbitalBody, Option<&MotionModeState>)>,
) {
    let Ok((camera, camera_transform)) = camera_q.single() else {
        return;
    };
    let Some(parent) = state.open_for else {
        return;
    };
    let Ok(parent_transform) = transforms.get(parent) else {
        return;
    };
    let parent_world = parent_transform.translation();
    // The magnifier follows the parent as it orbits.
    let Ok(centre) = camera.world_to_viewport(camera_transform, parent_world) else {
        return;
    };
    let radius = tunables.magnifier_radius_px;

    for (index, mut node) in frames.iter_mut().enumerate() {
        let angle = index as f32 / DASH_COUNT as f32 * std::f32::consts::TAU;
        let point = centre + Vec2::new(angle.cos(), angle.sin()) * radius;
        node.display = Display::Flex;
        node.left = px(point.x - DASH_LENGTH_PX * 0.5);
        node.top = px(point.y);
    }

    for (mut node, mut text) in &mut magnification_labels {
        node.display = Display::Flex;
        node.left = px(centre.x - radius);
        node.top = px(centre.y - radius - 14.0);
        *text = Text::new(format!("x{:.0}", view.magnification));
    }

    for mut node in &mut outer_labels {
        node.display = Display::Flex;
        node.left = px(centre.x + radius * 0.3);
        node.top = px(centre.y + radius + 4.0);
    }

    // Where a satellite's dot sits inside the frame, if it has one. In
    // exposure-ring mode it doesn't: the ring carries it instead.
    let dot_position = |member: Entity| -> Option<Vec2> {
        let satellite = transforms.get(member).ok()?;
        let world = magnified_world(parent_world, satellite.translation(), view.magnification);
        camera.world_to_viewport(camera_transform, world).ok()
    };
    let in_ring_mode = |member: Entity| -> bool {
        motion
            .get(member)
            .ok()
            .and_then(|(_, mode)| mode)
            .map(|mode| mode.ring_strength() > 0.5)
            .unwrap_or(false)
    };
    // A fixed spot on the magnified ring, for a label with no dot to sit by.
    let ring_label_position = |member: Entity| -> Option<Vec2> {
        let (orbital, _) = motion.get(member).ok()?;
        let radius = orbital.orbit_radius * view.magnification;
        let angle = orbital.phase0_rad as f32;
        let world = parent_world + Vec3::new(radius * angle.cos(), 0.0, radius * angle.sin());
        camera.world_to_viewport(camera_transform, world).ok()
    };

    for (marker, mut node) in &mut markers {
        // The motion rule applies inside the magnifier as it does
        // everywhere else: too fast to read as a position means no dot.
        match (in_ring_mode(marker.0), dot_position(marker.0)) {
            (false, Some(pos)) => {
                node.display = Display::Flex;
                node.left = px(pos.x - 3.0);
                node.top = px(pos.y - 3.0);
            }
            _ => node.display = Display::None,
        }
    }

    for (label, mut node) in &mut member_labels {
        let position = if in_ring_mode(label.0) {
            ring_label_position(label.0)
        } else {
            dot_position(label.0).map(|pos| pos + Vec2::new(6.0, -6.0))
        };
        match position {
            Some(pos) => {
                node.display = Display::Flex;
                node.left = px(pos.x);
                node.top = px(pos.y);
            }
            None => node.display = Display::None,
        }
    }
}

/// The orbits inside the frame, drawn as world-space gizmos at the
/// magnified radius so the motion rule's exposure rings apply here too.
#[allow(clippy::too_many_arguments)]
fn draw_magnifier_rings(
    state: Res<MagnifierState>,
    view: Res<MagnifierView>,
    mut gizmos: Gizmos,
    mut exposure: Gizmos<crate::orbits::ExposureRingGizmos>,
    inspections: Res<Inspections>,
    camera_q: Query<&GlobalTransform, With<Camera>>,
    transforms: Query<&GlobalTransform>,
    bodies: Query<(&OrbitalBody, Option<&MotionModeState>)>,
) {
    let Some(parent) = state.open_for else {
        return;
    };
    let Ok(camera_transform) = camera_q.single() else {
        return;
    };
    let Ok(parent_transform) = transforms.get(parent) else {
        return;
    };
    let centre = parent_transform.translation();
    let camera_pos = camera_transform.translation();
    let camera_forward = camera_transform.forward().as_vec3();

    for member in &state.included {
        let Ok((orbital, motion)) = bodies.get(*member) else {
            continue;
        };
        let radius = orbital.orbit_radius * view.magnification;
        let dim = colors::ORBIT_LINE.with_alpha(0.5);
        let strength = motion.map(MotionModeState::ring_strength).unwrap_or(0.0);
        let bright = (strength > 0.01).then(|| {
            let color = if inspections.is_open(*member) {
                colors::SELECTION_RING
            } else {
                colors::NEUTRAL
            };
            color.with_alpha(strength)
        });

        let point_at = |i: usize| {
            let angle = i as f32 / RING_SEGMENTS as f32 * std::f32::consts::TAU;
            centre + Vec3::new(radius * angle.cos(), 0.0, radius * angle.sin())
        };
        let in_front = |p: Vec3| (p - camera_pos).dot(camera_forward) > NEAR_EPSILON;

        let mut previous = point_at(0);
        for i in 1..=RING_SEGMENTS {
            let current = point_at(i % RING_SEGMENTS);
            if in_front(previous) && in_front(current) {
                gizmos.line(previous, current, dim);
                if let Some(bright) = bright {
                    exposure.line(previous, current, bright);
                }
            }
            previous = current;
        }
    }
}
