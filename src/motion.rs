//! The motion rule (`docs/spec/system-view.md`, MOT-01 to MOT-07).
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
use crate::orbits::Orbit;
use crate::scale::km_to_render;
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
/// Alpha of the newest trail dot. The trail is a hint of where the body has
/// just been, so even its brightest sample sits well under the marker's.
const TRAIL_PEAK_ALPHA: f32 = 0.5;
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
                .observe(hover_inspect::on_marker_over)
                .observe(hover_inspect::on_marker_out)
                .observe(hover_inspect::on_marker_click);
        }
    }
}

/// MOT-02 / MOT-01 / MOT-03: below `t_ring_secs` an object is an exposure
/// ring, at or above `t_dot_secs` it is a dot, and between the two it keeps
/// whichever mode it is already in.
///
/// MOT-04 falls out rather than being special-cased: pause gives an
/// infinite apparent period, which lands in the dot branch on its own.
pub fn next_ring_mode(
    apparent_secs: f64,
    currently_ring: bool,
    t_ring_secs: f32,
    t_dot_secs: f32,
) -> bool {
    if apparent_secs < t_ring_secs as f64 {
        true
    } else if apparent_secs >= t_dot_secs as f64 {
        false
    } else {
        currently_ring
    }
}

/// MOT-01 / MOT-04: the apparent period in real seconds - how long one
/// orbit takes to watch - is the orbital period divided by the time rate,
/// and infinite when paused.
pub fn apparent_period_secs(period_days: f64, rate_days_per_sec: f64, paused: bool) -> f64 {
    if paused || rate_days_per_sec <= 0.0 {
        f64::INFINITY
    } else {
        period_days / rate_days_per_sec
    }
}

/// MOT-03: the cross-fade runs from 0 to 1 and stops there; `step` comes
/// from `motion_crossfade_ms`.
pub fn advance_crossfade(crossfade_t: f32, step: f32) -> f32 {
    (crossfade_t + step.abs()).min(1.0)
}

/// MOT-01: a dot's trail covers `arc_degrees` of its own orbit, so it reads
/// as the same shape at every time rate and frame rate. Sampling by
/// simulated days rather than by frames is what makes that true.
pub fn trail_span_days(period_days: f64, arc_degrees: f32) -> f64 {
    period_days * (arc_degrees as f64 / 360.0)
}

/// MOT-01: the gap between trail samples, for a trail of `samples` dots.
pub fn trail_sample_interval_days(period_days: f64, arc_degrees: f32, samples: usize) -> f64 {
    if samples == 0 {
        return f64::INFINITY;
    }
    trail_span_days(period_days, arc_degrees) / samples as f64
}

/// MOT-01: whether enough simulated time has passed for the next sample.
/// Paused time never advances, so a paused trail simply stops growing.
pub fn should_sample(elapsed_days: f64, last_sample_day: f64, interval_days: f64) -> bool {
    elapsed_days - last_sample_day >= interval_days
}

/// MOT-01: how strongly a trail dot is drawn, oldest faintest. `age` is 0
/// for the newest sample.
pub fn trail_dot_fade(age: usize, length: usize) -> f32 {
    if length == 0 {
        return 0.0;
    }
    1.0 - age as f32 / length as f32
}

fn update_motion_modes(
    time: Res<Time>,
    clock: Res<crate::time_control::GameClock>,
    tunables: Res<Tunables>,
    mut bodies: Query<(&Orbit, &mut MotionModeState)>,
) {
    let step = crate::anim::step_per_frame(time.delta_secs(), tunables.motion_crossfade_ms);

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
    clock: Res<crate::time_control::GameClock>,
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
/// drawn as a gizmo in `orbits.rs`, which reads `MotionModeState` directly.
#[allow(clippy::too_many_arguments)]
fn update_ring_visuals(
    camera_q: Query<(&Camera, &GlobalTransform)>,
    windows: Query<&Window>,
    inspections: Res<Inspections>,
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

#[cfg(test)]
mod tests {
    use super::*;

    const T_RING: f32 = 20.0;
    const T_DOT: f32 = 25.0;

    #[test]
    fn mot_03_ring_below_t_ring_and_dot_above_t_dot() {
        assert!(next_ring_mode(19.9, false, T_RING, T_DOT));
        assert!(next_ring_mode(0.1, false, T_RING, T_DOT));
        assert!(!next_ring_mode(25.0, true, T_RING, T_DOT));
        assert!(!next_ring_mode(68.0, true, T_RING, T_DOT));
    }

    /// Between the thresholds the mode is kept, so an object drifting
    /// across the band doesn't switch back and forth.
    #[test]
    fn mot_03_keeps_mode_between_thresholds() {
        for apparent in [20.0_f64, 22.5, 24.9] {
            assert!(next_ring_mode(apparent, true, T_RING, T_DOT));
            assert!(!next_ring_mode(apparent, false, T_RING, T_DOT));
        }
    }

    #[test]
    fn mot_03_no_flicker_on_a_round_trip_through_the_band() {
        let mut ring = false;
        for apparent in [30.0_f64, 25.0, 24.0, 21.0, 20.0] {
            ring = next_ring_mode(apparent, ring, T_RING, T_DOT);
            assert!(!ring, "still a dot at {apparent}s");
        }
        ring = next_ring_mode(19.5, ring, T_RING, T_DOT);
        assert!(ring);
        for apparent in [20.0_f64, 22.0, 24.9] {
            ring = next_ring_mode(apparent, ring, T_RING, T_DOT);
            assert!(ring, "still a ring at {apparent}s");
        }
        ring = next_ring_mode(25.0, ring, T_RING, T_DOT);
        assert!(!ring);
    }

    /// MOT-04: pause is an infinite apparent period, which must fall out of
    /// the same rule rather than needing a special case.
    #[test]
    fn mot_04_pause_gives_dot_mode() {
        assert!(!next_ring_mode(f64::INFINITY, true, T_RING, T_DOT));
    }

    /// MOT-03 against the real test content: IVc flips mode between the two
    /// rates the validation checklist calls for (31 s at 1 d/s, 6.2 s at 5).
    #[test]
    fn mot_03_ivc_changes_mode_between_rates() {
        let period_days = 31.0;
        assert!(!next_ring_mode(period_days / 1.0, false, T_RING, T_DOT));
        assert!(next_ring_mode(period_days / 5.0, false, T_RING, T_DOT));
    }

    /// MOT-01: the apparent period is the orbital period divided by the
    /// rate, so the same orbit reads differently at 1 d/s and 5 d/s.
    #[test]
    fn mot_01_apparent_period_divides_by_the_time_rate() {
        assert_eq!(apparent_period_secs(31.0, 1.0, false), 31.0);
        assert_eq!(apparent_period_secs(31.0, 5.0, false), 6.2);
        assert_eq!(apparent_period_secs(0.1, 1.0, false), 0.1);
    }

    /// MOT-04: pause (and a rate of zero, which is the same thing) gives an
    /// infinite apparent period - the dot branch, with no special case.
    #[test]
    fn mot_04_paused_time_gives_an_infinite_apparent_period() {
        assert!(apparent_period_secs(31.0, 1.0, true).is_infinite());
        assert!(apparent_period_secs(31.0, 0.0, false).is_infinite());
        assert!(!next_ring_mode(
            apparent_period_secs(0.1, 1.0, true),
            true,
            T_RING,
            T_DOT
        ));
    }

    /// MOT-03: the cross-fade climbs to 1 and stays there.
    #[test]
    fn mot_03_crossfade_runs_to_one_and_stops() {
        assert!((advance_crossfade(0.0, 0.25) - 0.25).abs() < 1e-6);
        assert_eq!(advance_crossfade(0.9, 0.25), 1.0);
        assert_eq!(advance_crossfade(1.0, 0.25), 1.0);
    }

    /// MOT-01: the trail covers `trail_arc_degrees` of arc whatever the
    /// period, which is what makes it read the same on a 0.1 d station orbit
    /// and a 68 d moon orbit.
    #[test]
    fn mot_01_trail_covers_a_fixed_arc_of_the_orbit() {
        for period in [0.1_f64, 1.5, 31.0, 68.0] {
            let span = trail_span_days(period, 30.0);
            assert!(
                (span / period - 30.0 / 360.0).abs() < 1e-9,
                "{period} d orbit sampled over {span} d"
            );
        }
    }

    /// MOT-01: the samples divide that arc evenly, and a longer orbit is
    /// sampled proportionally less often.
    #[test]
    fn mot_01_trail_samples_divide_the_arc_evenly() {
        let interval = trail_sample_interval_days(36.0, 30.0, 16);
        assert!((interval - 3.0 / 16.0).abs() < 1e-9);
        assert_eq!(
            trail_sample_interval_days(72.0, 30.0, 16),
            2.0 * interval,
            "twice the period, twice the gap"
        );
    }

    /// MOT-01: sampling is driven by simulated days, so a paused clock (no
    /// elapsed days) never takes another sample, and an empty trail pool
    /// can't divide by zero.
    #[test]
    fn mot_01_trail_sampling_is_driven_by_simulated_days() {
        let interval = trail_sample_interval_days(31.0, 30.0, 16);
        assert!(
            !should_sample(10.0, 10.0, interval),
            "paused: no new sample"
        );
        assert!(!should_sample(10.0 + interval * 0.5, 10.0, interval));
        assert!(should_sample(10.0 + interval, 10.0, interval));
        assert!(trail_sample_interval_days(31.0, 30.0, 0).is_infinite());
        assert!(!should_sample(1e9, 0.0, f64::INFINITY));
    }

    /// MOT-01: the trail fades from the body backwards.
    #[test]
    fn mot_01_trail_fades_from_newest_to_oldest() {
        assert_eq!(trail_dot_fade(0, 16), 1.0);
        assert!(trail_dot_fade(8, 16) < trail_dot_fade(4, 16));
        assert!(trail_dot_fade(15, 16) > 0.0, "the oldest dot still shows");
        assert_eq!(trail_dot_fade(0, 0), 0.0);
    }
}
