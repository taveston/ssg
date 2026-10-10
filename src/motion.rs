//! The motion rule (`docs/presentation.md` section 2, "Satellites").
//!
//! Apparent period = orbital period / time rate, in real seconds, and
//! infinite when paused. Above T_dot an orbiting body reads as a dot at its
//! true position with a short trail; below T_ring it becomes an "exposure
//! ring" - no dot, a brighter and thicker orbit ring carrying the label,
//! because the body is moving too fast on screen to read as a position.
//! Between the two thresholds, whichever mode is already active is kept.
//!
//! Applies to everything that orbits, not just satellites: close-in planets
//! get it too.

use std::collections::HashMap;

use bevy::prelude::*;

use crate::Stage;
use crate::colors;
use crate::fonts::Fonts;
use crate::grouping::SatelliteFoldState;
use crate::hover_inspect::{self, Inspections};
use crate::markers::MarkerFor;
use crate::orbits::OrbitalBody;
use crate::system_data::{BodyClass, DisplayName};
use crate::tunables::Tunables;

/// Which way an orbiting body is currently drawn, plus the cross-fade
/// between modes.
#[derive(Component, Default)]
pub struct MotionModeState {
    pub ring_mode: bool,
    /// 0.0 = fully the previous mode, 1.0 = fully `ring_mode`.
    pub crossfade_t: f32,
}

impl MotionModeState {
    /// How strongly the ring should be drawn, 0.0 to 1.0.
    pub fn ring_strength(&self) -> f32 {
        if self.ring_mode {
            self.crossfade_t
        } else {
            1.0 - self.crossfade_t
        }
    }

    /// How strongly the dot and trail should be drawn.
    pub fn dot_strength(&self) -> f32 {
        1.0 - self.ring_strength()
    }
}

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
const RING_HIT_SEGMENTS: usize = 16;
const RING_HIT_SIZE_PX: f32 = 14.0;

pub struct MotionPlugin;

impl Plugin for MotionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Startup,
            spawn_motion_visuals.after(crate::scene::spawn_system),
        )
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
    bodies: Query<(Entity, &BodyClass, &DisplayName), With<OrbitalBody>>,
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
                .observe(hover_inspect::on_marker_over)
                .observe(hover_inspect::on_marker_out)
                .observe(hover_inspect::on_marker_click);
        }
    }
}

/// Apparent period in real seconds: how long one orbit takes to watch.
fn apparent_period_secs(period_days: f64, clock: &crate::time_control::GameClock) -> f64 {
    if clock.paused || clock.speed_days_per_sec <= 0.0 {
        f64::INFINITY
    } else {
        period_days / clock.speed_days_per_sec
    }
}

fn update_motion_modes(
    time: Res<Time>,
    clock: Res<crate::time_control::GameClock>,
    tunables: Res<Tunables>,
    mut bodies: Query<(&OrbitalBody, &mut MotionModeState)>,
) {
    let step = if tunables.motion_crossfade_ms > 0.0 {
        time.delta_secs() / (tunables.motion_crossfade_ms / 1000.0)
    } else {
        1.0
    };

    for (orbital, mut state) in &mut bodies {
        let apparent = apparent_period_secs(orbital.period_days, &clock);

        // Paused means an infinite apparent period, which lands in the dot
        // branch on its own - no special case needed.
        let target_ring = if apparent < tunables.t_ring_secs as f64 {
            true
        } else if apparent >= tunables.t_dot_secs as f64 {
            false
        } else {
            state.ring_mode
        };

        if target_ring != state.ring_mode {
            state.ring_mode = target_ring;
            state.crossfade_t = 0.0;
        }
        state.crossfade_t = (state.crossfade_t + step).min(1.0);
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn update_trails(
    clock: Res<crate::time_control::GameClock>,
    tunables: Res<Tunables>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut bodies: Query<(
        Entity,
        &GlobalTransform,
        &OrbitalBody,
        &MotionModeState,
        Option<&SatelliteFoldState>,
        Option<&ChildOf>,
        &mut FadingTrail,
    )>,
    transforms: Query<&GlobalTransform>,
    mut dots: Query<(&TrailDotFor, &mut Node, &mut BackgroundColor)>,
    colors_q: Query<&crate::system_data::Ownership>,
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
        let span_days = orbital.period_days * (tunables.trail_arc_degrees as f64 / 360.0);
        let interval_days = span_days / TRAIL_LENGTH as f64;
        if clock.elapsed_days - trail.last_sample_day >= interval_days {
            trail.last_sample_day = clock.elapsed_days;
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
        let fade = 1.0 - age_from_newest as f32 / TRAIL_LENGTH as f32;
        background.0 = view.color.with_alpha(fade * 0.5 * view.strength);
    }
}

/// Places the on-ring label and the ring's hit targets. The ring itself is
/// drawn as a gizmo in `orbits.rs`, which reads `MotionModeState` directly.
#[allow(clippy::too_many_arguments)]
fn update_ring_visuals(
    camera_q: Query<(&Camera, &GlobalTransform)>,
    windows: Query<&Window>,
    inspections: Res<Inspections>,
    bodies: Query<(
        &OrbitalBody,
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
                orbital.orbit_radius * angle.cos(),
                0.0,
                orbital.orbit_radius * angle.sin(),
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
        let base = if inspections.is_open(entity) {
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
