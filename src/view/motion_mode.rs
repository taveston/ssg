//! The motion rule (MOT-01, MOT-02, MOT-04, MOT-10 to MOT-12): how an
//! orbiting body reads on screen, as a dot with a tail on its ring, or -
//! when it moves too fast to read as a position - as an exposure ring.
//! Shared state, read by markers, rings, the inset and the camera
//! (ARCH-03); `motion.rs` is the one writer.
//!
//! The apparent period of an orbit is its period divided by the selected
//! time rate, in real seconds; pausing keeps the selected rate, so it
//! doesn't change the apparent period. The tail covers the arc the body
//! swept in the last `tail_secs` real seconds. As it grows past
//! `dot_fade_from_degrees` the dot fades and the ring thickens, until at
//! 360° the tail has closed into the exposure ring.

use bevy::prelude::*;

/// Degrees in a full orbit.
const FULL_TURN_DEGREES: f32 = 360.0;

/// How an orbiting body is drawn this frame.
#[derive(Component, Default, Debug, Clone, Copy, PartialEq)]
pub struct MotionModeState {
    /// The tail's length along the ring, in degrees (0 to 360).
    pub tail_degrees: f32,
    /// How far the tail has closed into the exposure ring: 0 while it is
    /// shorter than `dot_fade_from_degrees`, 1 at a full circle.
    pub closing: f32,
    /// MOT-02: running, with the tail closed into the exposure ring.
    pub ring_mode: bool,
}

impl MotionModeState {
    /// MOT-11: the dot fades out as the tail closes.
    pub fn dot_alpha(&self) -> f32 {
        1.0 - self.closing
    }

    /// MOT-11: the ring grows from `normal_px` to `exposure_px` wide as the
    /// tail closes.
    pub fn ring_width(&self, normal_px: f32, exposure_px: f32) -> f32 {
        normal_px + (exposure_px - normal_px) * self.closing
    }
}

/// The motion rule's settings, from the tunables.
#[derive(Debug, Clone, Copy)]
pub struct TailSettings {
    pub tail_secs: f32,
    pub dot_fade_from_degrees: f32,
}

/// The apparent period in real seconds (how long one orbit takes to
/// watch) is the orbital period divided by the selected time rate. Pausing
/// keeps the selected rate, so it is not an input.
pub fn apparent_period_secs(period_days: f64, rate_days_per_sec: f64) -> f64 {
    if rate_days_per_sec <= 0.0 {
        f64::INFINITY
    } else {
        period_days / rate_days_per_sec
    }
}

/// MOT-10: the tail covers the arc swept in the last `tail_secs` real
/// seconds, 360° × `tail_secs` ÷ apparent period, up to a full circle. It
/// follows the time rate continuously, with no steps.
pub fn running_tail_degrees(apparent_secs: f64, tail_secs: f32) -> f32 {
    if apparent_secs <= 0.0 {
        return FULL_TURN_DEGREES;
    }
    if !apparent_secs.is_finite() {
        return 0.0;
    }
    let degrees = FULL_TURN_DEGREES as f64 * tail_secs as f64 / apparent_secs;
    (degrees as f32).min(FULL_TURN_DEGREES)
}

/// MOT-11: how far a tail of `tail_degrees` has closed into the exposure
/// ring: 0 up to `fade_from_degrees`, rising linearly to 1 at 360°.
pub fn closing(tail_degrees: f32, fade_from_degrees: f32) -> f32 {
    let span = FULL_TURN_DEGREES - fade_from_degrees;
    if span <= 0.0 {
        return if tail_degrees >= FULL_TURN_DEGREES {
            1.0
        } else {
            0.0
        };
    }
    ((tail_degrees - fade_from_degrees) / span).clamp(0.0, 1.0)
}

/// MOT-01, MOT-02, MOT-04, MOT-10 to MOT-12: how a body with this period
/// is drawn at this rate. Running, the tail follows the rate and closes
/// into the exposure ring at a full circle. Paused, each tail keeps its
/// running length but is capped at `dot_fade_from_degrees`, so every body
/// shows as a full-brightness dot with its tail; unpausing restores the
/// running state.
pub fn motion_state(
    period_days: f64,
    rate_days_per_sec: f64,
    paused: bool,
    settings: TailSettings,
) -> MotionModeState {
    let apparent = apparent_period_secs(period_days, rate_days_per_sec);
    let running = running_tail_degrees(apparent, settings.tail_secs);
    let tail = if paused {
        running.min(settings.dot_fade_from_degrees)
    } else {
        running
    };
    state_for_tail(tail, settings.dot_fade_from_degrees)
}

/// MOT-11: the state a tail of `tail_degrees` shows: how far it has
/// closed, and whether it has closed into the exposure ring. Paused tails
/// are capped short of a full circle (MOT-12), so they never reach it.
pub fn state_for_tail(tail_degrees: f32, fade_from_degrees: f32) -> MotionModeState {
    MotionModeState {
        tail_degrees,
        closing: closing(tail_degrees, fade_from_degrees),
        ring_mode: tail_degrees >= FULL_TURN_DEGREES,
    }
}

/// MOT-13: the tail length on screen, easing towards the length the rate
/// and pause give, so a change animates rather than snaps.
#[derive(Component, Default, Debug, Clone, Copy, PartialEq)]
pub struct TailEase {
    /// The length shown when the current ease began.
    from: f32,
    /// The length being eased towards.
    target: f32,
    /// Real seconds since the current ease began.
    elapsed_secs: f32,
    /// False until the first length is known; that one is shown at once.
    started: bool,
}

impl TailEase {
    /// Advance by `dt` real seconds towards `target` and return the length
    /// to show. A new target starts a fresh ease from what is shown now,
    /// which reaches it after `ease_secs`.
    pub fn step(&mut self, target: f32, dt: f32, ease_secs: f32) -> f32 {
        if !self.started {
            *self = Self {
                from: target,
                target,
                elapsed_secs: 0.0,
                started: true,
            };
            return target;
        }
        if target != self.target {
            self.from = self.shown(ease_secs);
            self.target = target;
            self.elapsed_secs = 0.0;
        }
        self.elapsed_secs = (self.elapsed_secs + dt).min(ease_secs.max(0.0));
        self.shown(ease_secs)
    }

    /// The length shown now: from the start towards the target, eased in
    /// and out.
    fn shown(&self, ease_secs: f32) -> f32 {
        if ease_secs <= 0.0 {
            return self.target;
        }
        let t = (self.elapsed_secs / ease_secs).clamp(0.0, 1.0);
        let eased = t * t * (3.0 - 2.0 * t);
        self.from + (self.target - self.from) * eased
    }
}

/// MOT-01: how far behind the body a point on its ring is, in degrees
/// along the direction of motion (0 at the body, rising backwards round
/// the orbit). Angles are the orbit's own, in radians.
pub fn degrees_behind(body_angle: f32, point_angle: f32) -> f32 {
    (body_angle - point_angle)
        .rem_euclid(std::f32::consts::TAU)
        .to_degrees()
}

/// MOT-01 / MOT-02: how much of the exposure brightness a point on the
/// ring has, 0 to 1: 1 at the dot, falling linearly to 0 (the ring's
/// normal brightness) at the tail's end, 0 beyond it. In ring mode the
/// whole ring is at full exposure brightness.
///
/// MOT-11: as the tail closes, its fade flattens, so the far end brightens
/// with it and the tail is already even all round when it becomes the
/// exposure ring, with no jump.
pub fn tail_brightness(behind_degrees: f32, state: &MotionModeState) -> f32 {
    if state.ring_mode {
        return 1.0;
    }
    if state.tail_degrees <= 0.0 || behind_degrees > state.tail_degrees {
        return 0.0;
    }
    1.0 - behind_degrees / state.tail_degrees * (1.0 - state.closing)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SETTINGS: TailSettings = TailSettings {
        tail_secs: 8.0,
        dot_fade_from_degrees: 300.0,
    };

    /// The apparent period is the orbital period over the selected rate;
    /// pausing doesn't change it.
    #[test]
    fn apparent_period_divides_by_the_time_rate() {
        assert_eq!(apparent_period_secs(31.0, 1.0), 31.0);
        assert_eq!(apparent_period_secs(31.0, 5.0), 6.2);
        assert!(apparent_period_secs(31.0, 0.0).is_infinite());
    }

    /// MOT-10: the tail is the arc swept in `tail_secs`: a 16 s apparent
    /// period gives half a circle, an 80 s one 36°, and anything at or
    /// under 8 s a full circle.
    #[test]
    fn the_tail_is_the_arc_swept_in_tail_secs() {
        assert!((running_tail_degrees(16.0, 8.0) - 180.0).abs() < 1e-4);
        assert!((running_tail_degrees(80.0, 8.0) - 36.0).abs() < 1e-4);
        assert_eq!(running_tail_degrees(8.0, 8.0), 360.0);
        assert_eq!(running_tail_degrees(0.5, 8.0), 360.0);
        assert_eq!(running_tail_degrees(f64::INFINITY, 8.0), 0.0);
    }

    /// MOT-10: the tail's length follows the rate continuously: a small
    /// change of rate makes a small change of length, with no steps.
    #[test]
    fn the_tail_follows_the_rate_without_steps() {
        let mut last = running_tail_degrees(apparent_period_secs(31.0, 0.5), 8.0);
        for step in 1..=200 {
            let rate = 0.5 + step as f64 * 0.01;
            let now = running_tail_degrees(apparent_period_secs(31.0, rate), 8.0);
            assert!(now >= last && now - last < 2.0, "step at {rate} d/s");
            last = now;
        }
    }

    /// MOT-11: from 300° to 360° the dot fades out and the ring widens to
    /// the exposure width, linearly.
    #[test]
    fn the_dot_fades_and_the_ring_widens_as_the_tail_closes() {
        let at = |tail: f32| MotionModeState {
            tail_degrees: tail,
            closing: closing(tail, 300.0),
            ring_mode: false,
        };
        assert_eq!(at(200.0).dot_alpha(), 1.0);
        assert_eq!(at(300.0).dot_alpha(), 1.0);
        assert!((at(330.0).dot_alpha() - 0.5).abs() < 1e-5);
        assert_eq!(at(360.0).dot_alpha(), 0.0);
        assert_eq!(at(300.0).ring_width(2.0, 3.0), 2.0);
        assert!((at(330.0).ring_width(2.0, 3.0) - 2.5).abs() < 1e-5);
        assert_eq!(at(360.0).ring_width(2.0, 3.0), 3.0);
    }

    /// MOT-02: running with an apparent period under `tail_secs`, the tail
    /// has closed: ring mode, no dot, the whole ring bright and wide.
    #[test]
    fn a_fast_orbit_is_an_exposure_ring() {
        let state = motion_state(0.1, 1.0, false, SETTINGS);
        assert!(state.ring_mode);
        assert_eq!(state.dot_alpha(), 0.0);
        assert_eq!(tail_brightness(180.0, &state), 1.0);
        let slow = motion_state(31.0, 1.0, false, SETTINGS);
        assert!(!slow.ring_mode && slow.dot_alpha() == 1.0);
    }

    /// MOT-04 / MOT-12: paused, a tail keeps its running length but is
    /// capped at `dot_fade_from_degrees`, so even an exposure-ring body is
    /// a full-brightness dot with its tail; unpausing restores it.
    #[test]
    fn pausing_caps_the_tail_and_shows_every_dot() {
        let slow_paused = motion_state(31.0, 1.0, true, SETTINGS);
        let slow_running = motion_state(31.0, 1.0, false, SETTINGS);
        assert_eq!(slow_paused, slow_running, "a short tail is kept as is");
        let fast_paused = motion_state(0.1, 1.0, true, SETTINGS);
        assert_eq!(fast_paused.tail_degrees, 300.0);
        assert!(!fast_paused.ring_mode);
        assert_eq!(fast_paused.dot_alpha(), 1.0);
        assert!(motion_state(0.1, 1.0, false, SETTINGS).ring_mode);
    }

    /// MOT-01: the tail runs from exposure brightness at the dot down to
    /// the ring's normal brightness at its end, behind the body.
    #[test]
    fn the_tail_fades_behind_the_body() {
        let state = MotionModeState {
            tail_degrees: 90.0,
            closing: 0.0,
            ring_mode: false,
        };
        assert_eq!(tail_brightness(0.0, &state), 1.0);
        assert!((tail_brightness(45.0, &state) - 0.5).abs() < 1e-5);
        assert_eq!(tail_brightness(90.0, &state), 0.0);
        assert_eq!(tail_brightness(120.0, &state), 0.0);
        // Ahead of the body (just under a full turn behind) is untouched.
        let ahead = degrees_behind(1.0, 1.1);
        assert!(ahead > 350.0);
        assert_eq!(tail_brightness(ahead, &state), 0.0);
        assert!((degrees_behind(1.0, 0.9) - 0.1_f32.to_degrees()).abs() < 1e-3);
    }

    /// MOT-13: a new length is reached within the ease time, passing
    /// through the lengths between; the first length shows at once.
    #[test]
    fn the_tail_eases_to_a_new_length_within_the_ease_time() {
        let mut ease = TailEase::default();
        assert_eq!(ease.step(360.0, 0.016, 0.3), 360.0);
        let midway = ease.step(100.0, 0.15, 0.3);
        assert!(midway > 100.0 && midway < 360.0, "{midway}");
        assert_eq!(ease.step(100.0, 0.15, 0.3), 100.0);
        assert_eq!(ease.step(100.0, 0.15, 0.3), 100.0);
    }

    /// MOT-11 / MOT-13: the dot fade and ring mode follow the length shown.
    #[test]
    fn the_state_follows_the_shown_length() {
        assert!(state_for_tail(360.0, 300.0).ring_mode);
        let closing = state_for_tail(330.0, 300.0);
        assert!(!closing.ring_mode);
        assert!((closing.dot_alpha() - 0.5).abs() < 1e-6);
    }

    /// MOT-11 / MOT-13: a closing tail's far end brightens with it, so it
    /// meets the even exposure ring without a jump at 360°.
    #[test]
    fn a_closing_tail_evens_out_into_the_exposure_ring() {
        let far_end = |tail: f32| tail_brightness(tail, &state_for_tail(tail, 300.0));
        assert_eq!(far_end(300.0), 0.0);
        assert!((far_end(330.0) - 0.5).abs() < 1e-5);
        assert!(far_end(359.9) > 0.99);
        assert_eq!(far_end(360.0), 1.0);
    }
}
