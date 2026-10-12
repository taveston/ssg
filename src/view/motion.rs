//! The motion rule on screen (MOT-01 to MOT-03, MOT-08): keeps each
//! orbiting body's motion mode current, and draws dot-mode trails and the
//! labels of exposure rings. The rings themselves are drawn in `rings.rs`,
//! hit-tested in `ring_picking.rs` (MOT-05); the rules are in
//! `motion_mode.rs`.
//!
//! MOT-08: applies to everything that orbits, not just satellites -
//! close-in planets get it too. Gates keep their edge-indicator treatment.

use bevy::prelude::*;

use super::motion_mode::{
    MotionModeState, advance_crossfade, apparent_period_secs, next_ring_mode, should_sample,
    trail_dot_fade, trail_sample_interval_days,
};
use crate::sim::body::{BodyClass, DisplayName, Ownership};
use crate::sim::clock::GameClock;
use crate::sim::orbit::Orbit;
use crate::sim::orbit::OrbitPositionKm;
use crate::view::anim::step_per_frame;
use crate::view::colors;
use crate::view::fold::SatelliteFoldState;
use crate::view::fonts::Fonts;
use crate::view::scale::{km_to_render, km_to_render_vec3};
use crate::view::screen::in_viewport;
use crate::view::selection::Selection;
use crate::view::tunables::Tunables;
use crate::view::world::ViewNow;
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

/// The label that sits on an exposure ring at a fixed spot (MOT-02).
#[derive(Component)]
pub struct RingLabelFor(pub Entity);

/// MOT-01: how many dots a trail is drawn with.
pub const TRAIL_LENGTH: usize = 16;
/// Alpha of the newest trail dot. The trail is a hint of where the body has
/// just been, so even its brightest sample sits well under the marker's.
const TRAIL_PEAK_ALPHA: f32 = 0.5;
const TRAIL_DOT_PX: f32 = 3.0;
/// Below this strength an exposure ring's label and hit targets are off.
const MIN_RING_STRENGTH: f32 = 0.05;
/// Where a ring's label sits relative to its fixed spot on the ring.
const RING_LABEL_OFFSET_PX: Vec2 = Vec2::new(6.0, -6.0);
const RING_LABEL_FONT_PX: f32 = 11.0;

/// MOT-01: which stored sample trail dot `dot_index` shows (dot 0 is the
/// oldest), when `stored` samples have been taken so far. A trail that is
/// still filling up leaves its oldest dots empty.
pub fn trail_sample_for_dot(dot_index: usize, stored: usize, length: usize) -> Option<usize> {
    let age_from_newest = length.checked_sub(dot_index + 1)?;
    stored.checked_sub(age_from_newest + 1)
}

/// MOT-01: add a sample, dropping the oldest once the trail is full.
pub fn push_trail_sample(samples: &mut Vec<Vec3>, sample: Vec3, length: usize) {
    samples.push(sample);
    while samples.len() > length {
        samples.remove(0);
    }
}

pub struct MotionPlugin;

impl Plugin for MotionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_motion_visuals.in_set(Boot::Overlay))
            .add_systems(
                Update,
                (
                    update_motion_modes,
                    sample_trails,
                    draw_trails,
                    place_ring_labels,
                )
                    .chain()
                    .in_set(Stage::Motion),
            );
    }
}

/// Every orbiting body gets motion state, a trail pool and a ring label
/// (its ring's hit target is `ring_picking.rs`'s). Gates keep their edge-indicator treatment instead.
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
        spawn_trail_dots(&mut commands, entity);
        spawn_ring_label(&mut commands, &fonts, entity, name);
    }
}

fn spawn_trail_dots(commands: &mut Commands, body: Entity) {
    for index in 0..TRAIL_LENGTH {
        commands.spawn((
            TrailDotFor(body, index),
            Node {
                position_type: PositionType::Absolute,
                width: px(TRAIL_DOT_PX),
                height: px(TRAIL_DOT_PX),
                display: Display::None,
                border_radius: BorderRadius::MAX,
                ..default()
            },
            BackgroundColor(Color::NONE),
        ));
    }
}

fn spawn_ring_label(commands: &mut Commands, fonts: &Fonts, body: Entity, name: &DisplayName) {
    commands.spawn((
        RingLabelFor(body),
        Node {
            position_type: PositionType::Absolute,
            display: Display::None,
            ..default()
        },
        Text::new(name.0.clone()),
        fonts.label(RING_LABEL_FONT_PX),
        TextColor(colors::NEUTRAL),
    ));
}

/// MOT-01 to MOT-04: each body's mode follows its apparent period, with a
/// cross-fade on every switch.
fn update_motion_modes(
    time: Res<Time>,
    clock: Res<GameClock>,
    tunables: Res<Tunables>,
    mut bodies: Query<(&Orbit, &mut MotionModeState)>,
) {
    let step = step_per_frame(time.delta_secs(), tunables.motion_crossfade_ms);
    for (orbit, mut state) in &mut bodies {
        let apparent = apparent_period_secs(
            orbit.period_days,
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

/// The position of a body's parent in render space, or the origin (the
/// star) for a body with none.
fn parent_position(child_of: Option<&ChildOf>, view: &ViewNow) -> Vec3 {
    child_of
        .and_then(|child_of| view.position(child_of.parent()))
        .unwrap_or(Vec3::ZERO)
}

/// MOT-01: record trail samples by simulated days, so a trail covers a
/// fixed arc of its orbit, independent of frame rate and time rate.
fn sample_trails(
    clock: Res<GameClock>,
    tunables: Res<Tunables>,
    mut bodies: Query<(&OrbitPositionKm, &Orbit, &mut FadingTrail)>,
) {
    let now = clock.elapsed_days();
    for (local, orbit, mut trail) in &mut bodies {
        let interval_days =
            trail_sample_interval_days(orbit.period_days, tunables.trail_arc_degrees, TRAIL_LENGTH);
        if !should_sample(now, trail.last_sample_day, interval_days) {
            continue;
        }
        trail.last_sample_day = now;
        // Relative to the parent, so the trail traces the orbit rather than
        // the parent's path through the system.
        let sample = km_to_render_vec3(local.0);
        push_trail_sample(&mut trail.positions, sample, TRAIL_LENGTH);
    }
}

/// MOT-01: place each trail dot on its sample, replayed against where the
/// parent is *now* so the trail stays wrapped round it, fading with age.
#[allow(clippy::type_complexity)]
fn draw_trails(
    view: ViewNow,
    bodies: Query<(
        &FadingTrail,
        &MotionModeState,
        Option<&SatelliteFoldState>,
        Option<&ChildOf>,
        Option<&Ownership>,
    )>,
    mut dots: Query<(&TrailDotFor, &mut Node, &mut BackgroundColor)>,
) {
    for (dot_for, mut node, mut background) in &mut dots {
        let Ok((trail, mode, fold, child_of, ownership)) = bodies.get(dot_for.0) else {
            node.display = Display::None;
            continue;
        };
        // GRP-01: a folded satellite draws nothing at all - no marker,
        // label, ring or trail.
        let hidden = fold.is_some_and(|f| f.effective_hidden);
        let strength = if hidden { 0.0 } else { mode.dot_strength() };
        let sample = trail_sample_for_dot(dot_for.1, trail.positions.len(), TRAIL_LENGTH)
            .map(|index| trail.positions[index])
            .filter(|_| strength > 0.0);
        let world = sample.map(|sample| parent_position(child_of, &view) + sample);
        let Some(pos) = world.and_then(|world| view.project(world)) else {
            node.display = Display::None;
            continue;
        };
        node.display = Display::Flex;
        node.left = px(pos.x - TRAIL_DOT_PX / 2.0);
        node.top = px(pos.y - TRAIL_DOT_PX / 2.0);
        let age = TRAIL_LENGTH - 1 - dot_for.1;
        let color = ownership.map_or(colors::NEUTRAL, |o| colors::ownership_color(*o));
        background.0 =
            color.with_alpha(trail_dot_fade(age, TRAIL_LENGTH) * TRAIL_PEAK_ALPHA * strength);
    }
}

/// Orbiting bodies, with what decides whether their exposure ring shows.
pub type RingBodies<'w, 's> = Query<
    'w,
    's,
    (
        &'static Orbit,
        &'static MotionModeState,
        Option<&'static ChildOf>,
        Option<&'static SatelliteFoldState>,
    ),
>;

/// Where the point at `angle` on a body's orbit lands on screen.
pub fn ring_point(body: Entity, angle: f32, bodies: &RingBodies, view: &ViewNow) -> Option<Vec2> {
    let (orbit, _, child_of, _) = bodies.get(body).ok()?;
    let centre = match child_of {
        Some(child_of) => view.position(child_of.parent())?,
        None => Vec3::ZERO,
    };
    let radius = km_to_render(orbit.radius_km);
    let world = centre + Vec3::new(radius * angle.cos(), 0.0, radius * angle.sin());
    view.project(world)
}

/// MOT-02: how strongly a body's exposure ring shows, if at all.
pub fn ring_strength(body: Entity, bodies: &RingBodies) -> Option<f32> {
    let (_, mode, _, fold) = bodies.get(body).ok()?;
    let hidden = fold.is_some_and(|f| f.effective_hidden);
    let strength = mode.ring_strength();
    (!hidden && strength > MIN_RING_STRENGTH).then_some(strength)
}

/// MOT-02: the label sits on the ring at a fixed spot - the body's start
/// phase - so it doesn't appear to orbit while the body is a ring. MOT-07:
/// a selected body's label is white.
fn place_ring_labels(
    view: ViewNow,
    windows: Query<&Window>,
    selection: Res<Selection>,
    bodies: RingBodies,
    mut labels: Query<(&RingLabelFor, &mut Node, &mut TextColor)>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let viewport = Vec2::new(window.width(), window.height());
    for (label_for, mut node, mut color) in &mut labels {
        let body = label_for.0;
        let placed = ring_strength(body, &bodies).and_then(|strength| {
            let (orbit, ..) = bodies.get(body).ok()?;
            let pos = ring_point(body, orbit.phase0_rad as f32, &bodies, &view)?;
            in_viewport(pos, viewport).then_some((pos, strength))
        });
        let Some((pos, strength)) = placed else {
            node.display = Display::None;
            continue;
        };
        node.display = Display::Flex;
        node.left = px(pos.x + RING_LABEL_OFFSET_PX.x);
        node.top = px(pos.y + RING_LABEL_OFFSET_PX.y);
        let base = if selection.is_open(body) {
            colors::SELECTION_RING
        } else {
            colors::NEUTRAL
        };
        color.0 = base.with_alpha(strength);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// MOT-01: dot 15 is the newest sample, dot 0 the oldest, and a trail
    /// still filling up leaves its oldest dots empty.
    #[test]
    fn trail_dots_map_onto_samples_newest_last() {
        assert_eq!(trail_sample_for_dot(15, 16, 16), Some(15));
        assert_eq!(trail_sample_for_dot(0, 16, 16), Some(0));
        assert_eq!(trail_sample_for_dot(15, 3, 16), Some(2));
        assert_eq!(trail_sample_for_dot(13, 3, 16), Some(0));
        assert_eq!(trail_sample_for_dot(12, 3, 16), None);
        assert_eq!(trail_sample_for_dot(0, 0, 16), None);
    }

    /// MOT-01: the trail keeps the newest samples and drops the oldest.
    #[test]
    fn trail_keeps_only_the_newest_samples() {
        let mut samples = Vec::new();
        for i in 0..20 {
            push_trail_sample(&mut samples, Vec3::splat(i as f32), 16);
        }
        assert_eq!(samples.len(), 16);
        assert_eq!(samples[0], Vec3::splat(4.0));
        assert_eq!(samples[15], Vec3::splat(19.0));
    }
}
