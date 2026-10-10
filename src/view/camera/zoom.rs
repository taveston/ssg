//! The zoom stepping rules, as plain functions (`docs/spec/system-view.md`,
//! ZOOM-01, ZOOM-02, ZOOM-05).
//!
//! The camera works in *distance* from the pivot, and a distance is
//! inversely proportional to the view scale: one notch in multiplies the
//! scale by `zoom_step_factor`, which is the same as dividing the distance
//! by it. Everything here is therefore phrased in distances, and the
//! conversion to and from a framed radius lives in `stops.rs`.

use crate::view::anim::ease_out_cubic;

/// ZOOM-01: each wheel notch multiplies the view scale by
/// `step_factor`, i.e. divides the camera distance by it. Positive
/// `notches` zoom in.
pub fn stepped_distance(current_distance: f32, notches: f32, step_factor: f32) -> f32 {
    if step_factor <= 0.0 {
        return current_distance;
    }
    current_distance * step_factor.powf(-notches)
}

/// ZOOM-01: how many notches a scroll event is worth. Line-unit deltas are
/// notches already; pixel-unit deltas (trackpads, some mice) are converted
/// at a fixed rate rather than being used raw, which would zoom by many
/// steps per event.
///
/// Continuous pinch input (ZOOM-07) is not built; until it is, a pixel-unit
/// device gets whole steps like any other.
pub fn notches_from_scroll(delta_y: f32, pixel_units: bool) -> f32 {
    if !pixel_units {
        return delta_y;
    }
    // One notch per `PIXELS_PER_NOTCH` of scroll, keeping the sign and
    // dropping anything short of a whole notch so a slow drag doesn't
    // creep.
    let notches = delta_y / PIXELS_PER_NOTCH;
    notches.trunc()
}

/// A wheel notch on a pixel-reporting device reports roughly this much
/// scroll; the same figure Bevy's own examples use for the conversion.
const PIXELS_PER_NOTCH: f32 = 20.0;

/// ZOOM-05: the first stop a step would skip over, if any - the one nearest
/// the current distance in the direction of travel. A step that lands on it
/// settles exactly there instead of going on.
pub fn first_stop_crossed(current: f32, naive_target: f32, stops: &[f32]) -> Option<f32> {
    let (low, high) = if naive_target < current {
        (naive_target, current)
    } else {
        (current, naive_target)
    };
    stops
        .iter()
        .copied()
        .filter(|stop| *stop > low && *stop < high)
        .min_by(|a, b| {
            let da = (a - current).abs();
            let db = (b - current).abs();
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
}

/// ZOOM-05: a step that lands on a stop takes longer by
/// `resistance`, so the stop reads as slight resistance rather than a snag.
pub fn step_duration_ms(anim_ms: f32, resistance: f32, lands_on_stop: bool) -> f32 {
    if lands_on_stop {
        anim_ms * (1.0 + resistance)
    } else {
        anim_ms
    }
}

/// ZOOM-01: where an in-flight step is at `elapsed` seconds, eased.
pub fn animated_distance(start: f32, target: f32, elapsed: f32, duration: f32) -> f32 {
    if duration <= 0.0 {
        return target;
    }
    let t = (elapsed / duration).clamp(0.0, 1.0);
    start + (target - start) * ease_out_cubic(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    const STEP_FACTOR: f32 = 2.0;
    const ANIM_MS: f32 = 200.0;
    const RESISTANCE: f32 = 0.35;

    /// ZOOM-01: one notch is exactly one factor of two, in either
    /// direction, and notches compose.
    #[test]
    fn zoom_01_one_notch_is_one_step_factor() {
        assert_eq!(stepped_distance(100.0, 1.0, STEP_FACTOR), 50.0);
        assert_eq!(stepped_distance(100.0, -1.0, STEP_FACTOR), 200.0);
        assert_eq!(stepped_distance(100.0, 3.0, STEP_FACTOR), 12.5);
        assert_eq!(stepped_distance(100.0, 0.0, STEP_FACTOR), 100.0);
    }

    /// ZOOM-01: a step in and a step out return to where they started, so
    /// stepping can't drift.
    #[test]
    fn zoom_01_stepping_in_and_out_round_trips() {
        let start = 37.5_f32;
        let there = stepped_distance(start, 1.0, STEP_FACTOR);
        let back = stepped_distance(there, -1.0, STEP_FACTOR);
        assert!((back - start).abs() < 1e-4);
    }

    /// ZOOM-01: raw wheel deltas are never used. A device reporting pixels
    /// must not turn one event into dozens of steps (F4-11).
    #[test]
    fn zoom_01_pixel_scroll_is_converted_not_used_raw() {
        // A line-unit device reports notches directly.
        assert_eq!(notches_from_scroll(1.0, false), 1.0);
        assert_eq!(notches_from_scroll(-2.0, false), -2.0);
        // A pixel-unit device reporting a wheel notch gives one step, not 20.
        assert_eq!(notches_from_scroll(20.0, true), 1.0);
        assert_eq!(notches_from_scroll(-20.0, true), -1.0);
        // A big pixel delta is still only a few steps.
        assert_eq!(notches_from_scroll(120.0, true), 6.0);
        // And a fractional scroll doesn't creep.
        assert_eq!(notches_from_scroll(9.0, true), 0.0);
    }

    /// ZOOM-05: zooming in from the system frame towards a satellite frame
    /// stops at the region frame on the way.
    #[test]
    fn zoom_05_step_settles_on_a_stop_it_would_skip() {
        let stops = [1.0_f32, 40.0, 180.0];
        // 200 -> 100 would skip the 180 stop.
        assert_eq!(first_stop_crossed(200.0, 100.0, &stops), Some(180.0));
        // Zooming out from 20 towards 80 meets 40 first.
        assert_eq!(first_stop_crossed(20.0, 80.0, &stops), Some(40.0));
    }

    /// ZOOM-05: a step that crosses two stops takes the nearer one, so no
    /// stop is ever skipped.
    #[test]
    fn zoom_05_takes_the_nearest_stop_in_the_direction_of_travel() {
        let stops = [50.0_f32, 90.0];
        assert_eq!(first_stop_crossed(100.0, 10.0, &stops), Some(90.0));
        assert_eq!(first_stop_crossed(10.0, 100.0, &stops), Some(50.0));
    }

    #[test]
    fn zoom_05_no_stop_in_the_way_means_a_plain_step() {
        let stops = [1.0_f32, 180.0];
        assert_eq!(first_stop_crossed(100.0, 50.0, &stops), None);
        assert_eq!(first_stop_crossed(100.0, 100.0, &stops), None);
        assert_eq!(first_stop_crossed(100.0, 50.0, &[]), None);
    }

    /// ZOOM-05: a stop the step lands exactly on, or starts from, is not
    /// "crossed" - otherwise sitting on a stop would snag every step.
    #[test]
    fn zoom_05_endpoints_are_not_crossings() {
        let stops = [50.0_f32, 100.0];
        assert_eq!(first_stop_crossed(100.0, 50.0, &stops), None);
        assert_eq!(first_stop_crossed(100.0, 25.0, &stops), Some(50.0));
    }

    #[test]
    fn zoom_05_landing_on_a_stop_takes_longer() {
        assert_eq!(step_duration_ms(ANIM_MS, RESISTANCE, false), 200.0);
        assert!((step_duration_ms(ANIM_MS, RESISTANCE, true) - 270.0).abs() < 1e-4);
    }

    /// ZOOM-01: the step animates with ease-out and ends exactly on target.
    #[test]
    fn zoom_01_animation_eases_out_onto_the_target() {
        let (start, target, duration) = (200.0_f32, 100.0, 0.2);
        assert_eq!(animated_distance(start, target, 0.0, duration), start);
        assert_eq!(animated_distance(start, target, duration, duration), target);
        assert_eq!(animated_distance(start, target, 10.0, duration), target);
        let half = animated_distance(start, target, duration * 0.5, duration);
        assert!(half < 150.0, "ease-out covers more than half by halfway");
    }

    /// ZOOM-02: a notch arriving mid-animation retargets from the *current*
    /// distance rather than queueing behind the step in flight - so the new
    /// step starts exactly where the view is, with no jump and no stutter.
    #[test]
    fn zoom_02_new_input_retargets_from_the_current_scale() {
        let duration = 0.2;
        let first_target = stepped_distance(200.0, 1.0, STEP_FACTOR);
        let interrupted_at = animated_distance(200.0, first_target, 0.08, duration);
        assert!(interrupted_at < 200.0 && interrupted_at > first_target);

        // The second notch starts from there, not from 200, and not from
        // the first step's target.
        let second_target = stepped_distance(interrupted_at, 1.0, STEP_FACTOR);
        assert_eq!(
            animated_distance(interrupted_at, second_target, 0.0, duration),
            interrupted_at
        );
        assert!(second_target < first_target);
        // Queueing would instead have landed a full step past the first
        // target; retargeting lands a step past where we actually were.
        assert!(second_target > stepped_distance(first_target, 1.0, STEP_FACTOR));
    }
}
