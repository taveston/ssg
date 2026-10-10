//! Frame-rate-independent animation helpers, shared by every animated
//! presentation rule: the fold slide (GRP-03), the motion-mode cross-fade
//! (MOT-03) and the zoom step's ease-out (ZOOM-01).
//!
//! These are deliberately plain functions on plain values (ARCH-20): the
//! systems that own the state read the clock, call these, and store the
//! result.

/// How much of a `duration_ms` animation one frame of `delta_secs` covers,
/// as a fraction of its full travel. A zero or negative duration means
/// "instant", which is what a tunable set to 0 should do rather than
/// dividing by zero.
pub fn step_per_frame(delta_secs: f32, duration_ms: f32) -> f32 {
    if duration_ms > 0.0 {
        delta_secs / (duration_ms / 1000.0)
    } else {
        1.0
    }
}

/// Move `current` towards `target` by at most `step`, without overshooting
/// in either direction.
pub fn advance_towards(current: f32, target: f32, step: f32) -> f32 {
    let step = step.abs();
    if current < target {
        (current + step).min(target)
    } else {
        (current - step).max(target)
    }
}

/// ZOOM-01: the ease-out a zoom step is animated with. Cubic, so it leaves
/// quickly and settles gently onto the target scale.
pub fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_per_frame_covers_the_whole_animation_in_its_duration() {
        // 200 ms of animation in 200 ms of frames, in any number of steps.
        for frames in [1_u32, 4, 60] {
            let delta = 0.2 / frames as f32;
            let total: f32 = (0..frames).map(|_| step_per_frame(delta, 200.0)).sum();
            assert!(
                (total - 1.0).abs() < 1e-5,
                "{frames} frames summed to {total}"
            );
        }
    }

    /// A tunable set to zero means instant, not a division by zero.
    #[test]
    fn step_per_frame_treats_a_zero_duration_as_instant() {
        assert_eq!(step_per_frame(0.016, 0.0), 1.0);
        assert_eq!(step_per_frame(0.016, -5.0), 1.0);
    }

    #[test]
    fn advance_towards_never_overshoots_in_either_direction() {
        assert_eq!(advance_towards(0.0, 1.0, 10.0), 1.0);
        assert_eq!(advance_towards(1.0, 0.0, 10.0), 0.0);
        assert!((advance_towards(0.25, 1.0, 0.5) - 0.75).abs() < 1e-6);
        assert!((advance_towards(0.75, 0.0, 0.5) - 0.25).abs() < 1e-6);
    }

    #[test]
    fn advance_towards_holds_still_when_already_there() {
        assert_eq!(advance_towards(0.4, 0.4, 0.5), 0.4);
    }

    /// ZOOM-01: ease-out means most of the travel happens early, and the
    /// ends are exact so a step settles precisely on its target.
    #[test]
    fn zoom_01_ease_out_is_front_loaded_and_exact_at_the_ends() {
        assert_eq!(ease_out_cubic(0.0), 0.0);
        assert_eq!(ease_out_cubic(1.0), 1.0);
        assert!(ease_out_cubic(0.5) > 0.5, "ease-out leads the linear ramp");
        // Monotonic, so nothing ever travels backwards mid-step.
        let mut previous = 0.0;
        for i in 0..=20 {
            let value = ease_out_cubic(i as f32 / 20.0);
            assert!(value >= previous);
            previous = value;
        }
    }

    #[test]
    fn zoom_01_ease_out_clamps_outside_the_unit_range() {
        assert_eq!(ease_out_cubic(-1.0), 0.0);
        assert_eq!(ease_out_cubic(2.0), 1.0);
    }
}
