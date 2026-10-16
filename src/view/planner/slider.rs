//! The planner's sliders (PLNR-21 to PLNR-23, PLNR-28, PLNR-40): where a
//! thumb sits on its track, what setting a point on the track or a key
//! press gives, and the colour of each setting on the track. Plain
//! functions (ARCH-20); `panel.rs` draws the slider and `input.rs` moves
//! it.
//!
//! A range is laid out by value along the track. A phasing leg's
//! meetings are laid out evenly, one position each, in order (PLNR-40).

use bevy::prelude::*;

use crate::sim::transfer::grid::Sample;
use crate::sim::transfer::ranges::Slider;
use crate::view::colors;
use crate::view::tunables::Tunables;

/// PLNR-23: a slider's settings move ten steps at a time with Shift.
pub const FAST_STEPS: i32 = 10;

/// Where along its track (0 at the start, 1 at the end) `value` sits.
pub fn fraction_of(slider: &Slider, value: f64) -> f32 {
    match slider {
        Slider::Range { start, end } if end > start => {
            ((value - start) / (end - start)).clamp(0.0, 1.0) as f32
        }
        Slider::Positions(positions) if positions.len() > 1 => {
            nearest_index(positions, value) as f32 / (positions.len() - 1) as f32
        }
        _ => 0.0,
    }
}

/// PLNR-23: the setting a press or drag at `fraction` of the track gives:
/// the nearest step of a range, or the nearest position.
pub fn value_at(slider: &Slider, fraction: f32, max_steps: u32) -> f64 {
    let fraction = f64::from(fraction.clamp(0.0, 1.0));
    match slider {
        Slider::Range { start, end } => slider.snap(start + fraction * (end - start), max_steps),
        Slider::Positions(positions) if !positions.is_empty() => {
            let index = (fraction * (positions.len() - 1) as f64).round() as usize;
            positions[index.min(positions.len() - 1)]
        }
        Slider::Positions(_) => 0.0,
        Slider::Fixed(value) => *value,
    }
}

/// PLNR-23: the setting `steps` steps on from `value` (back when
/// negative), kept on the slider: whole steps of a range, or neighbouring
/// positions.
pub fn stepped(slider: &Slider, value: f64, steps: i32, max_steps: u32) -> f64 {
    match slider {
        Slider::Range { start, end } => {
            let step = slider.step(max_steps).unwrap_or(0.0);
            let moved = (value + step * f64::from(steps)).clamp(*start, *end);
            slider.snap(moved, max_steps)
        }
        Slider::Positions(positions) if !positions.is_empty() => {
            let index = nearest_index(positions, value) as i64 + i64::from(steps);
            positions[index.clamp(0, positions.len() as i64 - 1) as usize]
        }
        Slider::Positions(_) => value,
        Slider::Fixed(value) => *value,
    }
}

/// The index of the position nearest `value`.
fn nearest_index(positions: &[f64], value: f64) -> usize {
    positions
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| (*a - value).abs().total_cmp(&(*b - value).abs()))
        .map_or(0, |(i, _)| i)
}

/// PLNR-28: the colour of one setting on a track. A setting the solver
/// rejects is the empty colour; one over budget is the warning colour at
/// `track_over_alpha`; an affordable one is own blue mixed into the empty
/// colour, brightest for the cheapest: m = `track_dim_mix` + (1 −
/// `track_dim_mix`)(1 − t), with t = (v − v_min) ÷ (available − v_min)
/// held between 0 and 1 (0 when available equals v_min).
pub fn track_colour(
    sample: Option<&Sample>,
    least_kms: f64,
    available_kms: f64,
    tunables: &Tunables,
) -> Color {
    let empty = colors::PANEL_BORDER;
    let Some(cost) = sample.and_then(Sample::cost) else {
        return empty;
    };
    if cost.delta_v_kms > available_kms {
        return colors::WARNING.with_alpha(tunables.track_over_alpha);
    }
    let span = available_kms - least_kms;
    let t = if span > 0.0 {
        ((cost.delta_v_kms - least_kms) / span).clamp(0.0, 1.0) as f32
    } else {
        0.0
    };
    let dim = tunables.track_dim_mix;
    mix(empty, colors::OWN, dim + (1.0 - dim) * (1.0 - t))
}

/// `a` mixed with `b` at `m` (0 is all `a`, 1 all `b`), in sRGB.
pub fn mix(a: Color, b: Color, m: f32) -> Color {
    let (a, b) = (a.to_srgba(), b.to_srgba());
    let lerp = |x: f32, y: f32| x + (y - x) * m;
    Color::srgba(
        lerp(a.red, b.red),
        lerp(a.green, b.green),
        lerp(a.blue, b.blue),
        lerp(a.alpha, b.alpha),
    )
}

/// PLNR-28: a track as colour stops, each at its fraction of the track:
/// one per sample of `slider`, with `sample_of` giving each its result.
pub fn track_stops(
    slider: &Slider,
    samples: &[(f64, Option<&Sample>)],
    least_kms: f64,
    available_kms: f64,
    tunables: &Tunables,
) -> Vec<(f32, Color)> {
    samples
        .iter()
        .map(|(value, sample)| {
            (
                fraction_of(slider, *value),
                track_colour(*sample, least_kms, available_kms, tunables),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::transfer::grid::{Cost, Setting};
    use crate::sim::transfer::order::Infeasible;

    fn sample(kms: f64) -> Sample {
        Sample {
            setting: Setting {
                depart_day: 0.0,
                travel_days: 1.0,
            },
            result: Ok(Cost {
                delta_v_kms: kms,
                arrival_day: 1.0,
            }),
        }
    }

    /// PLNR-23: a range takes values in steps between its ends; a press
    /// at a point of the track takes the nearest step; keys move one step
    /// (ten with Shift) and stop at the ends.
    #[test]
    fn a_range_slider_moves_in_whole_steps_between_its_ends() {
        let slider = Slider::Range {
            start: 0.0,
            end: 100.0,
        };
        // 100 days in at most 400 steps: 6-hour steps.
        assert_eq!(value_at(&slider, 0.5, 400), 50.0);
        assert_eq!(value_at(&slider, 0.10001, 400), 10.0);
        assert_eq!(value_at(&slider, 2.0, 400), 100.0);
        assert_eq!(stepped(&slider, 50.0, 1, 400), 50.25);
        assert_eq!(stepped(&slider, 50.0, -FAST_STEPS, 400), 47.5);
        assert_eq!(stepped(&slider, 99.9, FAST_STEPS, 400), 100.0);
        assert_eq!(fraction_of(&slider, 25.0), 0.25);
        assert_eq!(fraction_of(&slider, -5.0), 0.0);
    }

    /// PLNR-40: a phasing leg's slider has one position per meeting, laid
    /// out evenly in order; keys step from one to the next.
    #[test]
    fn a_positions_slider_steps_from_meeting_to_meeting() {
        let slider = Slider::Positions(vec![2.0, 5.0, 11.0]);
        assert_eq!(fraction_of(&slider, 5.0), 0.5);
        assert_eq!(fraction_of(&slider, 11.0), 1.0);
        assert_eq!(value_at(&slider, 0.3, 400), 5.0);
        assert_eq!(value_at(&slider, 0.9, 400), 11.0);
        assert_eq!(stepped(&slider, 2.0, 1, 400), 5.0);
        assert_eq!(stepped(&slider, 5.0, FAST_STEPS, 400), 11.0);
        assert_eq!(stepped(&slider, 5.0, -FAST_STEPS, 400), 2.0);
    }

    /// PLNR-28: rejected settings are the empty colour, settings over
    /// budget the faint warning colour, and affordable ones own blue,
    /// brightest at the least delta-v and dimmest at the budget.
    #[test]
    fn a_track_is_coloured_by_what_each_setting_costs() {
        let tunables = Tunables::default();
        let rejected = Sample {
            result: Err(Infeasible::NoPath),
            ..sample(0.0)
        };
        assert_eq!(
            track_colour(Some(&rejected), 2.0, 9.0, &tunables),
            colors::PANEL_BORDER
        );
        assert_eq!(
            track_colour(None, 2.0, 9.0, &tunables),
            colors::PANEL_BORDER
        );
        assert_eq!(
            track_colour(Some(&sample(9.5)), 2.0, 9.0, &tunables),
            colors::WARNING.with_alpha(tunables.track_over_alpha)
        );
        let cheapest = track_colour(Some(&sample(2.0)), 2.0, 9.0, &tunables);
        assert_eq!(cheapest, mix(colors::PANEL_BORDER, colors::OWN, 1.0));
        let dearest = track_colour(Some(&sample(9.0)), 2.0, 9.0, &tunables);
        assert_eq!(
            dearest,
            mix(colors::PANEL_BORDER, colors::OWN, tunables.track_dim_mix)
        );
        let tight = track_colour(Some(&sample(4.0)), 4.0, 4.0, &tunables);
        assert_eq!(tight, cheapest, "t is 0 when the budget is the least");
    }
}
