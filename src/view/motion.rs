//! The motion rule on screen (MOT-01 to MOT-03, MOT-05, MOT-08): keeps each
//! orbiting body's motion mode current, and draws dot-mode trails and the
//! labels and hit targets of exposure rings. The rings themselves are drawn
//! in `rings.rs`; the rules are in `motion_mode.rs`.
//!
//! MOT-08: applies to everything that orbits, not just satellites -
//! close-in planets get it too. Gates keep their edge-indicator treatment.

use std::collections::HashMap;

use bevy::prelude::*;

use super::motion_mode::{
    MotionModeState, advance_crossfade, apparent_period_secs, next_ring_mode, should_sample,
    trail_dot_fade, trail_sample_interval_days,
};
use crate::sim::body::{BodyClass, DisplayName};
use crate::sim::orbit::Orbit;
use crate::view::colors;
use crate::view::fold::SatelliteFoldState;
use crate::view::fonts::Fonts;
use crate::view::hover;
use crate::view::scale::km_to_render;
use crate::view::selection::Selection;
use crate::view::targets::MarkerFor;
use crate::view::tunables::Tunables;
use crate::{Boot, Stage};

/// Trail of recent positions, re-projected every frame so it shows the
/// body's own motion rather than the camera's.
///
/// Samples are stored *relative to the parent* for satellites, and in world
/// space for everything else. A moon's trail would otherwise smear along
/// its planet's travel through the system instead of tracing its orbit -
/// at satellite zoom the planet covers the whole view in about a second,
/// which swamps the moon's own motion entirely.
#[derive(Component, Default)]
pub struct FadingTrail {
    pub positions: Vec<Vec3>,
    pub last_sample_day: f64,
}

/// One of a fixed pool of dot nodes drawing a body's trail. Index 0 is the
/// oldest sample.
#[derive(Component)]
pub struct TrailDotFor(pub Entity, pub usize);

/// The label that sits on an exposure ring at a fixed spot.
#[derive(Component)]
pub struct RingLabelFor(pub Entity);

/// One of a necklace of invisible hit targets strung around an exposure
/// ring, so hovering or clicking the ring itself hits the satellite.
/// Gizmo-drawn rings aren't pickable on their own.
#[derive(Component)]
pub struct RingHitFor(pub Entity, pub usize);

pub const TRAIL_LENGTH: usize = 16;
/// Alpha of the newest trail dot. The trail is a hint of where the body has
/// just been, so even its brightest sample sits well under the marker's.
const TRAIL_PEAK_ALPHA: f32 = 0.5;
const RING_HIT_SEGMENTS: usize = 16;
const RING_HIT_SIZE_PX: f32 = 14.0;

pub struct MotionPlugin;

impl Plugin for MotionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_motion_visuals.in_set(Boot::Overlay))
            .add_systems(
                Update,
                (update_motion_modes, update_trails, update_ring_visuals)
                    .chain()
                    .in_set(Stage::Motion),
            );
    }
}

/// Every orbiting body gets motion state, a trail pool, a ring label and a
/// ring hit necklace. Gates keep their edge-indicator treatment instead.
fn spawn_motion_visuals(
    mut commands: Commands,
    fonts: Res<Fonts>,
    bodies: Query<(Entity, &BodyClass, &DisplayName), With<Orbit>>,
) {
    for (entity, class, name) in &bodies {
        if *class == BodyClass::Gate {
            continue;
        }

        commands
            .entity(entity)
            .insert((MotionModeState::default(), FadingTrail::default()));

        for index in 0..TRAIL_LENGTH {
            commands.spawn((
                TrailDotFor(entity, index),
                Node {
                    position_type: PositionType::Absolute,
                    width: px(3),
                    height: px(3),
                    display: Display::None,
                    border_radius: BorderRadius::MAX,
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ));
        }

        commands.spawn((
            RingLabelFor(entity),
            Node {
                position_type: PositionType::Absolute,
                display: Display::None,
                ..default()
            },
            Text::new(name.0.clone()),
            fonts.label(11.0),
            TextColor(colors::NEUTRAL),
        ));

        for index in 0..RING_HIT_SEGMENTS {
            commands
                .spawn((
                    RingHitFor(entity, index),
                    MarkerFor(entity),
                    Node {
                        position_type: PositionType::Absolute,
                        width: px(RING_HIT_SIZE_PX),
                        height: px(RING_HIT_SIZE_PX),
                        display: Display::None,
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                ))
                .observe(hover::on_marker_over)
                .observe(hover::on_marker_out)
                .observe(hover::on_marker_click);
        }
    }
}

fn update_motion_modes(
    time: Res<Time>,
    clock: Res<crate::sim::clock::GameClock>,
    tunables: Res<Tunables>,
    mut bodies: Query<(&Orbit, &mut MotionModeState)>,
) {
    let step = crate::view::anim::step_per_frame(time.delta_secs(), tunables.motion_crossfade_ms);

    for (orbital, mut state) in &mut bodies {
        let apparent = apparent_period_secs(
            orbital.period_days,
            clock.rate_days_per_sec(),
            clock.is_paused(),
        );

        let target_ring = next_ring_mode(
            apparent,
            state.ring_mode,
            tunables.t_ring_secs,
            tunables.t_dot_secs,
        );

        if target_ring != state.ring_mode {
            state.ring_mode = target_ring;
            state.crossfade_t = 0.0;
        }
        state.crossfade_t = advance_crossfade(state.crossfade_t, step);
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn update_trails(
    clock: Res<crate::sim::clock::GameClock>,
    tunables: Res<Tunables>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut bodies: Query<(
        Entity,
        &GlobalTransform,
        &Orbit,
        &MotionModeState,
        Option<&SatelliteFoldState>,
        Option<&ChildOf>,
        &mut FadingTrail,
    )>,
    transforms: Query<&GlobalTransform>,
    mut dots: Query<(&TrailDotFor, &mut Node, &mut BackgroundColor)>,
    colors_q: Query<&crate::sim::body::Ownership>,
) {
    let Ok((camera, camera_transform)) = camera_q.single() else {
        return;
    };

    struct TrailView {
        positions: Vec<Vec3>,
        /// Samples are relative to this body's current position, if it has
        /// a parent; absolute otherwise.
        parent: Option<Entity>,
        strength: f32,
        color: Color,
    }
    let mut trails: HashMap<Entity, TrailView> = HashMap::new();

    for (entity, transform, orbital, mode, fold, child_of, mut trail) in &mut bodies {
        let hidden = fold.map(|f| f.effective_hidden).unwrap_or(false);
        // A folded satellite draws nothing at all - no marker, label, ring
        // or trail.
        let strength = if hidden { 0.0 } else { mode.dot_strength() };

        let parent = child_of.map(ChildOf::parent);
        let parent_pos = parent
            .and_then(|p| transforms.get(p).ok())
            .map(GlobalTransform::translation)
            .unwrap_or(Vec3::ZERO);

        // Sample by simulated days so the trail covers a fixed arc of the
        // orbit, independent of frame rate and time rate.
        let interval_days = trail_sample_interval_days(
            orbital.period_days,
            tunables.trail_arc_degrees,
            TRAIL_LENGTH,
        );
        if should_sample(clock.elapsed_days(), trail.last_sample_day, interval_days) {
            trail.last_sample_day = clock.elapsed_days();
            // Store relative to the parent, so the trail traces the orbit
            // rather than the parent's path through the system.
            trail.positions.push(transform.translation() - parent_pos);
            if trail.positions.len() > TRAIL_LENGTH {
                trail.positions.remove(0);
            }
        }

        let color = colors_q
            .get(entity)
            .map(|ownership| colors::ownership_color(*ownership))
            .unwrap_or(colors::NEUTRAL);
        trails.insert(
            entity,
            TrailView {
                positions: trail.positions.clone(),
                parent,
                strength,
                color,
            },
        );
    }

    for (dot_for, mut node, mut background) in &mut dots {
        let Some(view) = trails.get(&dot_for.0) else {
            node.display = Display::None;
            continue;
        };
        if view.strength <= 0.0 {
            node.display = Display::None;
            continue;
        }
        let age_from_newest = TRAIL_LENGTH - 1 - dot_for.1;
        let Some(sample) = view
            .positions
            .len()
            .checked_sub(age_from_newest + 1)
            .map(|i| view.positions[i])
        else {
            node.display = Display::None;
            continue;
        };
        // Replay the stored offset against where the parent is *now*, so
        // the trail stays wrapped around the parent as it moves.
        let parent_pos = view
            .parent
            .and_then(|p| transforms.get(p).ok())
            .map(GlobalTransform::translation)
            .unwrap_or(Vec3::ZERO);
        let world_pos = parent_pos + sample;
        let Ok(pos) = camera.world_to_viewport(camera_transform, world_pos) else {
            node.display = Display::None;
            continue;
        };
        node.display = Display::Flex;
        node.left = px(pos.x - 1.5);
        node.top = px(pos.y - 1.5);
        let fade = trail_dot_fade(age_from_newest, TRAIL_LENGTH);
        background.0 = view
            .color
            .with_alpha(fade * TRAIL_PEAK_ALPHA * view.strength);
    }
}

/// Places the on-ring label and the ring's hit targets. The ring itself is
/// drawn as a gizmo in `rings.rs`, which reads `MotionModeState` directly.
#[allow(clippy::too_many_arguments)]
fn update_ring_visuals(
    camera_q: Query<(&Camera, &GlobalTransform)>,
    windows: Query<&Window>,
    selection: Res<Selection>,
    bodies: Query<(
        &Orbit,
        &MotionModeState,
        Option<&ChildOf>,
        Option<&SatelliteFoldState>,
    )>,
    transforms: Query<&GlobalTransform>,
    mut labels: Query<(&RingLabelFor, &mut Node, &mut TextColor), Without<RingHitFor>>,
    mut hits: Query<(&RingHitFor, &mut Node), Without<RingLabelFor>>,
) {
    let Ok((camera, camera_transform)) = camera_q.single() else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    let viewport = Vec2::new(window.width(), window.height());

    // Where a point at `angle` on a body's orbit lands on screen.
    let ring_point = |entity: Entity, angle: f32| -> Option<Vec2> {
        let (orbital, _, child_of, _) = bodies.get(entity).ok()?;
        let centre = match child_of {
            Some(child_of) => transforms.get(child_of.parent()).ok()?.translation(),
            None => Vec3::ZERO,
        };
        let world = centre
            + Vec3::new(
                km_to_render(orbital.radius_km) * angle.cos(),
                0.0,
                km_to_render(orbital.radius_km) * angle.sin(),
            );
        camera.world_to_viewport(camera_transform, world).ok()
    };

    let ring_active = |entity: Entity| -> bool {
        let Ok((_, mode, _, fold)) = bodies.get(entity) else {
            return false;
        };
        let hidden = fold.map(|f| f.effective_hidden).unwrap_or(false);
        !hidden && mode.ring_strength() > 0.05
    };

    for (label_for, mut node, mut color) in &mut labels {
        let entity = label_for.0;
        if !ring_active(entity) {
            node.display = Display::None;
            continue;
        }
        // Fixed spot on the ring: the body's start phase, so the label
        // doesn't appear to orbit while the body itself is a ring.
        let Ok((orbital, mode, _, _)) = bodies.get(entity) else {
            node.display = Display::None;
            continue;
        };
        let Some(pos) = ring_point(entity, orbital.phase0_rad as f32) else {
            node.display = Display::None;
            continue;
        };
        if pos.x < 0.0 || pos.y < 0.0 || pos.x > viewport.x || pos.y > viewport.y {
            node.display = Display::None;
            continue;
        }
        node.display = Display::Flex;
        node.left = px(pos.x + 6.0);
        node.top = px(pos.y - 6.0);
        let base = if selection.is_open(entity) {
            colors::SELECTION_RING
        } else {
            colors::NEUTRAL
        };
        color.0 = base.with_alpha(mode.ring_strength());
    }

    for (hit_for, mut node) in &mut hits {
        let entity = hit_for.0;
        if !ring_active(entity) {
            node.display = Display::None;
            continue;
        }
        let angle = hit_for.1 as f32 / RING_HIT_SEGMENTS as f32 * std::f32::consts::TAU;
        let Some(pos) = ring_point(entity, angle) else {
            node.display = Display::None;
            continue;
        };
        node.display = Display::Flex;
        node.left = px(pos.x - RING_HIT_SIZE_PX * 0.5);
        node.top = px(pos.y - RING_HIT_SIZE_PX * 0.5);
    }
}
