//! The zoom stepping rules, as plain functions (`docs/spec/system-view.md`,
//! ZOOM-01, ZOOM-02, ZOOM-05).
//!
//! The camera works in *distance* from the pivot, and a distance is
//! inversely proportional to the view scale: one notch in multiplies the
//! scale by `zoom_step_factor`, which is the same as dividing the distance
//! by it. Everything here is therefore phrased in distances, and the
//! conversion to and from a framed radius lives in `stops.rs`.

use bevy::prelude::{Entity, Vec3};

use crate::view::anim::ease_out_cubic;

/// One planned zoom step: where it ends and how long it takes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZoomStep {
    pub target: f32,
    pub duration_ms: f32,
}

/// The settings a zoom step is planned with.
#[derive(Debug, Clone, Copy)]
pub struct StepSettings {
    pub step_factor: f32,
    pub anim_ms: f32,
    pub stop_resistance: f32,
}

/// ZOOM-01 / ZOOM-05: plan a step of `notches` from `current`: one step
/// factor per notch, settling on the first stop in the way (and taking
/// longer to do it), clamped to the limits `inner..=outer` (ZOOM-14):
/// scrolling past one does nothing.
pub fn plan_zoom_step(
    current: f32,
    notches: f32,
    stops: &[f32],
    inner: f32,
    outer: f32,
    settings: StepSettings,
) -> ZoomStep {
    let naive_target = stepped_distance(current, notches, settings.step_factor);
    let crossed = first_stop_crossed(current, naive_target, stops);
    ZoomStep {
        target: crossed
            .unwrap_or(naive_target)
            .clamp(inner.min(outer), outer),
        duration_ms: step_duration_ms(
            settings.anim_ms,
            settings.stop_resistance,
            crossed.is_some(),
        ),
    }
}

/// ZOOM-13: the pivot that keeps `anchor` at the same place on screen when
/// the camera distance goes from `current` to `target` - the whole view
/// scaled about the anchor.
pub fn anchored_centre(anchor: Vec3, centre: Vec3, current: f32, target: f32) -> Vec3 {
    if current <= f32::EPSILON {
        return centre;
    }
    anchor + (centre - anchor) * (target / current)
}

/// ZOOM-13: what a zoom step is anchored on - the focused object when there
/// is one, else the point where the cursor's ray meets the orbital plane,
/// else the pivot (the screen centre, for keyboard zoom or a cursor ray
/// that misses the plane).
pub fn zoom_anchor(focused: Option<Vec3>, cursor_point: Option<Vec3>, pivot: Vec3) -> Vec3 {
    focused.or(cursor_point).unwrap_or(pivot)
}

/// ZOOM-14 / ZOOM-15: the body nearest the anchor, whose satellite frame is
/// the inner limit and the satellite stop when nothing is focused.
pub fn nearest_body(anchor: Vec3, candidates: &[(Entity, Vec3)]) -> Option<Entity> {
    candidates
        .iter()
        .map(|&(body, position)| (body, position.distance(anchor)))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(body, _)| body)
}

/// What a zoom step does to the focus (CAM-08, CAM-21).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusChange {
    Keep,
    /// CAM-08: a zoom that reaches the system frame ends focus.
    End,
    /// CAM-21: with nothing focused, a step in past the region frame
    /// focuses the body whose satellite frame is the inner limit.
    Start(Entity),
}

/// The distances a zoom step is judged against.
#[derive(Debug, Clone, Copy)]
pub struct ZoomFrames {
    pub system: f32,
    pub region: f32,
}

/// CAM-08 / CAM-21: whether a zoom step from `current` to `target` keeps,
/// ends or starts focus. `limit_body` is the body whose satellite frame is
/// the inner limit (ZOOM-14).
pub fn zoom_focus_change(
    focused: bool,
    current: f32,
    target: f32,
    frames: ZoomFrames,
    limit_body: Option<Entity>,
) -> FocusChange {
    // Distances are compared with a little slack, so a step that settles
    // exactly on a frame counts as reaching it.
    let reaches = |frame: f32| target >= frame * (1.0 - FRAME_SLACK);
    if focused {
        return if reaches(frames.system) {
            FocusChange::End
        } else {
            FocusChange::Keep
        };
    }
    let zooming_in = target < current;
    let past_region = target < frames.region * (1.0 - FRAME_SLACK);
    match limit_body {
        Some(body) if zooming_in && past_region => FocusChange::Start(body),
        _ => FocusChange::Keep,
    }
}

/// Relative slack when comparing a distance with a frame's.
const FRAME_SLACK: f32 = 1e-3;

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
    fn one_notch_is_one_step_factor() {
        assert_eq!(stepped_distance(100.0, 1.0, STEP_FACTOR), 50.0);
        assert_eq!(stepped_distance(100.0, -1.0, STEP_FACTOR), 200.0);
        assert_eq!(stepped_distance(100.0, 3.0, STEP_FACTOR), 12.5);
        assert_eq!(stepped_distance(100.0, 0.0, STEP_FACTOR), 100.0);
    }

    /// ZOOM-01: a step in and a step out return to where they started, so
    /// stepping can't drift.
    #[test]
    fn stepping_in_and_out_round_trips() {
        let start = 37.5_f32;
        let there = stepped_distance(start, 1.0, STEP_FACTOR);
        let back = stepped_distance(there, -1.0, STEP_FACTOR);
        assert!((back - start).abs() < 1e-4);
    }

    /// ZOOM-01: raw wheel deltas are never used. A device reporting pixels
    /// must not turn one event into dozens of steps.
    #[test]
    fn pixel_scroll_is_converted_not_used_raw() {
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
    fn step_settles_on_a_stop_it_would_skip() {
        let stops = [1.0_f32, 40.0, 180.0];
        // 200 -> 100 would skip the 180 stop.
        assert_eq!(first_stop_crossed(200.0, 100.0, &stops), Some(180.0));
        // Zooming out from 20 towards 80 meets 40 first.
        assert_eq!(first_stop_crossed(20.0, 80.0, &stops), Some(40.0));
    }

    /// ZOOM-05: a step that crosses two stops takes the nearer one, so no
    /// stop is ever skipped.
    #[test]
    fn takes_the_nearest_stop_in_the_direction_of_travel() {
        let stops = [50.0_f32, 90.0];
        assert_eq!(first_stop_crossed(100.0, 10.0, &stops), Some(90.0));
        assert_eq!(first_stop_crossed(10.0, 100.0, &stops), Some(50.0));
    }

    /// Covers ZOOM-05.
    #[test]
    fn no_stop_in_the_way_means_a_plain_step() {
        let stops = [1.0_f32, 180.0];
        assert_eq!(first_stop_crossed(100.0, 50.0, &stops), None);
        assert_eq!(first_stop_crossed(100.0, 100.0, &stops), None);
        assert_eq!(first_stop_crossed(100.0, 50.0, &[]), None);
    }

    /// ZOOM-05: a stop the step lands exactly on, or starts from, is not
    /// "crossed" - otherwise sitting on a stop would snag every step.
    #[test]
    fn endpoints_are_not_crossings() {
        let stops = [50.0_f32, 100.0];
        assert_eq!(first_stop_crossed(100.0, 50.0, &stops), None);
        assert_eq!(first_stop_crossed(100.0, 25.0, &stops), Some(50.0));
    }

    /// Covers ZOOM-05.
    #[test]
    fn landing_on_a_stop_takes_longer() {
        assert_eq!(step_duration_ms(ANIM_MS, RESISTANCE, false), 200.0);
        assert!((step_duration_ms(ANIM_MS, RESISTANCE, true) - 270.0).abs() < 1e-4);
    }

    /// ZOOM-01: the step animates with ease-out and ends exactly on target.
    #[test]
    fn animation_eases_out_onto_the_target() {
        let (start, target, duration) = (200.0_f32, 100.0, 0.2);
        assert_eq!(animated_distance(start, target, 0.0, duration), start);
        assert_eq!(animated_distance(start, target, duration, duration), target);
        assert_eq!(animated_distance(start, target, 10.0, duration), target);
        let half = animated_distance(start, target, duration * 0.5, duration);
        assert!(half < 150.0, "ease-out covers more than half by halfway");
    }

    const SETTINGS: StepSettings = StepSettings {
        step_factor: STEP_FACTOR,
        anim_ms: ANIM_MS,
        stop_resistance: RESISTANCE,
    };

    /// ZOOM-01 / ZOOM-05: a plain step is one factor; a step over a stop
    /// lands on it and takes longer.
    #[test]
    fn planned_step_settles_on_stops() {
        let plain = plan_zoom_step(100.0, 1.0, &[], 0.0, 1000.0, SETTINGS);
        assert_eq!(
            plain,
            ZoomStep {
                target: 50.0,
                duration_ms: ANIM_MS
            }
        );
        let resisted = plan_zoom_step(100.0, 1.0, &[70.0], 0.0, 1000.0, SETTINGS);
        assert_eq!(resisted.target, 70.0);
        assert!(resisted.duration_ms > ANIM_MS);
    }

    /// Scrolling past a limit does nothing: the step stops at the limit.
    /// Covers ZOOM-01.
    #[test]
    fn planned_step_stops_at_the_limits() {
        assert_eq!(
            plan_zoom_step(100.0, -1.0, &[], 1.0, 150.0, SETTINGS).target,
            150.0
        );
        assert_eq!(
            plan_zoom_step(100.0, -1.0, &[], 1.0, 100.0, SETTINGS).target,
            100.0
        );
        assert_eq!(
            plan_zoom_step(2.0, 1.0, &[], 1.5, 150.0, SETTINGS).target,
            1.5
        );
    }

    /// The anchor keeps its place: scaling the distance scales the anchor's
    /// offset from the centre by the same factor.
    /// Covers ZOOM-13.
    #[test]
    fn anchor_keeps_its_place_on_screen() {
        let anchor = Vec3::new(10.0, 0.0, 0.0);
        let centre = Vec3::ZERO;
        assert_eq!(
            anchored_centre(anchor, centre, 100.0, 50.0),
            Vec3::new(5.0, 0.0, 0.0)
        );
        assert_eq!(anchored_centre(anchor, centre, 100.0, 100.0), centre);
        assert_eq!(anchored_centre(anchor, centre, 0.0, 50.0), centre);
    }

    /// ZOOM-13: focus first, then the cursor's plane point, then the pivot;
    /// selection never comes into it.
    #[test]
    fn zoom_anchor_prefers_focus_then_cursor_then_pivot() {
        let (focus, cursor, pivot) = (Vec3::X, Vec3::Y, Vec3::Z);
        assert_eq!(zoom_anchor(Some(focus), Some(cursor), pivot), focus);
        assert_eq!(zoom_anchor(None, Some(cursor), pivot), cursor);
        assert_eq!(zoom_anchor(None, None, pivot), pivot);
    }

    fn entities() -> [Entity; 2] {
        let mut world = bevy::prelude::World::new();
        [world.spawn_empty().id(), world.spawn_empty().id()]
    }

    /// Covers ZOOM-14.
    #[test]
    fn nearest_body_to_the_anchor() {
        let [near, far] = entities();
        let candidates = [
            (far, Vec3::new(50.0, 0.0, 0.0)),
            (near, Vec3::new(2.0, 0.0, 0.0)),
        ];
        assert_eq!(nearest_body(Vec3::ZERO, &candidates), Some(near));
        assert_eq!(nearest_body(Vec3::ZERO, &[]), None);
    }

    const FRAMES: ZoomFrames = ZoomFrames {
        system: 1000.0,
        region: 400.0,
    };

    /// CAM-08: focus survives zooming that stays inside the system frame,
    /// and ends on a step that reaches it.
    #[test]
    fn reaching_the_system_frame_ends_focus() {
        let [body, _] = entities();
        let some = Some(body);
        assert_eq!(
            zoom_focus_change(true, 100.0, 200.0, FRAMES, some),
            FocusChange::Keep
        );
        assert_eq!(
            zoom_focus_change(true, 100.0, 50.0, FRAMES, some),
            FocusChange::Keep
        );
        assert_eq!(
            zoom_focus_change(true, 600.0, 1000.0, FRAMES, some),
            FocusChange::End
        );
    }

    /// CAM-21: with nothing focused, only a step in past the region frame
    /// focuses - not one that lands on it, nor one zooming out.
    #[test]
    fn zooming_in_past_the_region_frame_focuses() {
        let [body, _] = entities();
        let some = Some(body);
        assert_eq!(
            zoom_focus_change(false, 800.0, 400.0, FRAMES, some),
            FocusChange::Keep
        );
        assert_eq!(
            zoom_focus_change(false, 400.0, 200.0, FRAMES, some),
            FocusChange::Start(body)
        );
        assert_eq!(
            zoom_focus_change(false, 200.0, 300.0, FRAMES, some),
            FocusChange::Keep
        );
        assert_eq!(
            zoom_focus_change(false, 400.0, 200.0, FRAMES, None),
            FocusChange::Keep
        );
    }

    /// ZOOM-02: a notch arriving mid-animation retargets from the *current*
    /// distance rather than queueing behind the step in flight - so the new
    /// step starts exactly where the view is, with no jump and no stutter.
    #[test]
    fn new_input_retargets_from_the_current_scale() {
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
