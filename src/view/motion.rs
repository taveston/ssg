//! The motion rule on screen (MOT-01, MOT-02, MOT-04, MOT-08, MOT-10 to
//! MOT-13): keeps each orbiting body's tail and mode current. The rings and
//! their tails are drawn in `rings.rs`, hit-tested in `ring_picking.rs`
//! (HOV-02) and labelled and marked in `ring_marks.rs` (MOT-02, MOT-14);
//! the rules are in `motion_mode.rs`.
//!
//! MOT-08: applies to everything that orbits, not just satellites -
//! close-in planets get it too. Gates keep their edge-indicator treatment.
//! A ship in flight doesn't orbit, so it has no motion state and is always
//! a dot (PLNR-68); it gets one again when it joins an orbit.

use bevy::prelude::*;

use super::motion_mode::{MotionModeState, TailEase, TailSettings, motion_state, state_for_tail};
use crate::sim::body::BodyClass;
use crate::sim::clock::GameClock;
use crate::sim::orbit::Orbit;
use crate::view::tunables::Tunables;
use crate::{Boot, Stage};

pub struct MotionPlugin;

impl Plugin for MotionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, attach_motion_state.in_set(Boot::Derive))
            .add_systems(
                Update,
                (
                    attach_motion_state,
                    drop_motion_state_in_flight,
                    update_motion_modes,
                )
                    .chain()
                    .in_set(Stage::Motion),
            );
    }
}

/// Every orbiting body gets motion state, a ship that joins an orbit too.
/// Gates keep their edge-indicator treatment instead.
fn attach_motion_state(
    mut commands: Commands,
    bodies: Query<(Entity, &BodyClass), (With<Orbit>, Without<MotionModeState>)>,
) {
    for (entity, class) in &bodies {
        if *class == BodyClass::Gate {
            continue;
        }
        commands
            .entity(entity)
            .insert((MotionModeState::default(), TailEase::default()));
    }
}

/// MOT-08 / PLNR-68: a ship that leaves its orbit is in flight: the
/// motion rule ignores it, so it is drawn as a dot, never as an exposure
/// ring.
fn drop_motion_state_in_flight(
    mut commands: Commands,
    in_flight: Query<Entity, (With<MotionModeState>, Without<Orbit>)>,
) {
    for entity in &in_flight {
        commands
            .entity(entity)
            .remove::<(MotionModeState, TailEase)>();
    }
}

/// MOT-01, MOT-02, MOT-04, MOT-10 to MOT-12: each body's tail follows its
/// apparent period at the selected rate, every frame. MOT-13: the length
/// shown eases towards that within `tail_ease_ms`, and the dot fade and
/// ring width follow the length shown.
fn update_motion_modes(
    clock: Res<GameClock>,
    tunables: Res<Tunables>,
    time: Res<Time<Real>>,
    mut bodies: Query<(&Orbit, &mut MotionModeState, &mut TailEase)>,
) {
    let settings = TailSettings {
        tail_secs: tunables.tail_secs,
        dot_fade_from_degrees: tunables.dot_fade_from_degrees,
    };
    let ease_secs = tunables.tail_ease_ms / 1000.0;
    for (orbit, mut state, mut ease) in &mut bodies {
        let target = motion_state(
            orbit.period_days,
            clock.rate_days_per_sec(),
            clock.is_paused(),
            settings,
        );
        let shown = ease.step(target.tail_degrees, time.delta_secs(), ease_secs);
        let next = state_for_tail(shown, settings.dot_fade_from_degrees);
        if *state != next {
            *state = next;
        }
    }
}
