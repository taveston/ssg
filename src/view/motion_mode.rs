//! Motion mode: whether an orbiting body reads as a dot or as an exposure
//! ring, and the rules that decide it (MOT-01 to MOT-04). Shared state,
//! read by markers, rings and trails (ARCH-03); `motion.rs` is the one
//! writer.
//!
//! The apparent period of an orbit is its period divided by the time rate,
//! in real seconds, and infinite when paused. Above T_dot a body is a dot at
//! its true position with a short trail; below T_ring it becomes an exposure
//! ring - no dot, a brighter and thicker ring carrying the label, because
//! the body moves too fast on screen to read as a position. Between the two
//! thresholds the mode already active is kept.

use bevy::prelude::*;

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
