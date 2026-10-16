//! The planner's sliders (PLNR-21 to PLNR-23, PLNR-28, PLNR-33,
//! PLNR-35, PLNR-40): where a thumb sits on its track, what setting a
//! point on the track or a key press gives, where a thumb may come to
//! rest, the colour of each setting on the track and the stretches the
//! solver rejects. Plain functions (ARCH-20); `panel/` draws the slider
//! and `input.rs` moves it.
//!
//! A range and the Depart slider's days are laid out by value along the
//! track. A phasing leg's meetings are laid out evenly, one position
//! each, in order (PLNR-40).

use bevy::prelude::*;

use crate::sim::transfer::grid::Sample;
use crate::sim::transfer::ranges::Slider;
use crate::view::colors;
use crate::view::colors::mix;
use crate::view::tunables::Tunables;

/// PLNR-23: a slider's settings move ten steps at a time with Shift.
pub const FAST_STEPS: i32 = 10;

/// Where along its track (0 at the start, 1 at the end) `value` sits.
pub fn fraction_of(slider: &Slider, value: f64) -> f32 {
    match slider {
        Slider::Range { start, end }
        | Slider::Days {
            soonest: start,
            end,
            ..
        } if end > start => ((value - start) / (end - start)).clamp(0.0, 1.0) as f32,
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
        Slider::Range { start, end }
        | Slider::Days {
            soonest: start,
            end,
            ..
        } => slider.snap(start + fraction * (end - start), max_steps),
        Slider::Positions(positions) if !positions.is_empty() => {
            let index = (fraction * (positions.len() - 1) as f64).round() as usize;
            positions[index.min(positions.len() - 1)]
        }
        Slider::Positions(_) => 0.0,
        Slider::Fixed(value) => *value,
    }
}

/// The index of the setting nearest `value`; the first on a tie.
pub fn nearest_index(settings: &[f64], value: f64) -> usize {
    settings
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| (*a - value).abs().total_cmp(&(*b - value).abs()))
        .map_or(0, |(i, _)| i)
}

/// PLNR-33: where a thumb sent to setting `index` of `count` comes to
/// rest: there if it is accepted, else the nearest accepted setting, the
/// earlier on a tie. `None` when no setting is accepted.
pub fn nearest_accepted(
    count: usize,
    index: usize,
    mut accepted: impl FnMut(usize) -> bool,
) -> Option<usize> {
    let index = index.min(count.checked_sub(1)?);
    for distance in 0..count {
        let below = index.checked_sub(distance);
        let above = Some(index + distance).filter(|&i| i < count && distance > 0);
        if below.is_none() && above.is_none() {
            break;
        }
        for candidate in [below, above].into_iter().flatten() {
            if accepted(candidate) {
                return Some(candidate);
            }
        }
    }
    None
}

/// PLNR-33: a key step: `steps` settings on from `index` (back when
/// negative), kept on the slider, then on in the same direction to the
/// next accepted setting. `None` when there is none that way, so the
/// thumb stays.
pub fn step_accepted(
    count: usize,
    index: usize,
    steps: i32,
    mut accepted: impl FnMut(usize) -> bool,
) -> Option<usize> {
    let last = count.checked_sub(1)? as i64;
    let target = (index as i64 + i64::from(steps)).clamp(0, last);
    if target == index as i64 {
        return None;
    }
    let direction = if steps < 0 { -1 } else { 1 };
    let mut candidate = target;
    while (0..=last).contains(&candidate) {
        if accepted(candidate as usize) {
            return Some(candidate as usize);
        }
        candidate += direction;
    }
    None
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

/// PLNR-28: a track as colour stops, each at its fraction of the track:
/// one per sample of `slider`, with its result.
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

/// PLNR-35: a stretch of a track, from one fraction of it to another,
/// where the solver rejects the setting, and the sentence that says why.
#[derive(Debug, Clone, PartialEq)]
pub struct Stretch {
    pub from: f32,
    pub to: f32,
    pub text: String,
}

/// PLNR-35: the rejected stretches of a track whose samples sit at the
/// given fractions with the given reasons (`None` for an accepted
/// sample). Each rejected sample covers the track from halfway to its
/// neighbours, the ends to the ends of the track; neighbours with the
/// same sentence make one stretch.
pub fn rejected_stretches(samples: &[(f32, Option<String>)]) -> Vec<Stretch> {
    let mut stretches: Vec<Stretch> = Vec::new();
    for (i, (at, reason)) in samples.iter().enumerate() {
        let Some(text) = reason else {
            continue;
        };
        let from = match i.checked_sub(1) {
            Some(before) => (samples[before].0 + at) / 2.0,
            None => 0.0,
        };
        let to = samples
            .get(i + 1)
            .map_or(1.0, |(next, _)| (at + next) / 2.0);
        match stretches.last_mut() {
            Some(last) if last.to == from && last.text == *text => last.to = to,
            _ => stretches.push(Stretch {
                from,
                to,
                text: text.clone(),
            }),
        }
    }
    stretches
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

    /// The setting `steps` on from `value` with every setting accepted,
    /// or `value` where the thumb stays.
    fn stepped(slider: &Slider, value: f64, steps: i32) -> f64 {
        let settings = slider.settings(400);
        let index = nearest_index(&settings, value);
        step_accepted(settings.len(), index, steps, |_| true).map_or(value, |i| settings[i])
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
        assert_eq!(stepped(&slider, 50.0, 1), 50.25);
        assert_eq!(stepped(&slider, 50.0, -FAST_STEPS), 47.5);
        assert_eq!(stepped(&slider, 99.8, FAST_STEPS), 100.0);
        assert_eq!(stepped(&slider, 100.0, FAST_STEPS), 100.0);
        assert_eq!(fraction_of(&slider, 25.0), 0.25);
        assert_eq!(fraction_of(&slider, -5.0), 0.0);
    }

    /// PLNR-21 / PLNR-23: the Depart slider is laid out by value from
    /// Soonest to its end; a press takes the nearest of Soonest and the
    /// whole days, and keys step from one position to the next.
    #[test]
    fn the_depart_slider_takes_soonest_or_a_whole_day() {
        let slider = Slider::Days {
            soonest: 10.4,
            end: 20.4,
            step: 1.0,
        };
        assert_eq!(fraction_of(&slider, 15.4), 0.5);
        assert_eq!(value_at(&slider, 0.0, 400), 10.4);
        assert_eq!(value_at(&slider, 0.02, 400), 10.4);
        assert_eq!(value_at(&slider, 0.07, 400), 11.0);
        assert_eq!(value_at(&slider, 1.0, 400), 20.0);
        assert_eq!(stepped(&slider, 10.4, 1), 11.0);
        assert_eq!(stepped(&slider, 11.0, -1), 10.4);
        assert_eq!(stepped(&slider, 10.4, FAST_STEPS), 20.0);
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
        assert_eq!(stepped(&slider, 2.0, 1), 5.0);
        assert_eq!(stepped(&slider, 5.0, FAST_STEPS), 11.0);
        assert_eq!(stepped(&slider, 5.0, -FAST_STEPS), 2.0);
    }

    /// PLNR-33: a thumb sent onto a rejected setting rests on the nearest
    /// accepted one, the earlier on a tie; a key step goes on past
    /// rejected settings in its direction, and stays when there is none
    /// that way; with nothing accepted there is nowhere to rest.
    #[test]
    fn a_thumb_never_rests_on_a_rejected_setting() {
        let accepted = [true, false, false, false, true, true, false];
        let ok = |i: usize| accepted[i];
        assert_eq!(nearest_accepted(7, 0, ok), Some(0));
        assert_eq!(nearest_accepted(7, 1, ok), Some(0));
        assert_eq!(nearest_accepted(7, 2, ok), Some(0), "a tie goes earlier");
        assert_eq!(nearest_accepted(7, 3, ok), Some(4));
        assert_eq!(nearest_accepted(7, 6, ok), Some(5));
        assert_eq!(nearest_accepted(7, 3, |_| false), None);
        assert_eq!(step_accepted(7, 0, 1, ok), Some(4));
        assert_eq!(step_accepted(7, 5, 1, ok), None, "nothing further on");
        assert_eq!(step_accepted(7, 5, -1, ok), Some(4));
        assert_eq!(step_accepted(7, 4, -2, ok), Some(0));
        assert_eq!(step_accepted(7, 6, 1, ok), None, "at the end");
    }

    /// PLNR-35: each rejected sample covers the track halfway to its
    /// neighbours, and neighbours with the same reason make one stretch.
    #[test]
    fn rejected_samples_make_stretches_with_their_reasons() {
        let why = |text: &str| Some(text.to_string());
        let samples = [
            (0.0, why("a")),
            (0.2, why("a")),
            (0.4, None),
            (0.6, why("b")),
            (1.0, why("c")),
        ];
        let stretches = rejected_stretches(&samples);
        let spans: Vec<(f32, f32, &str)> = stretches
            .iter()
            .map(|s| (s.from, s.to, s.text.as_str()))
            .collect();
        assert_eq!(
            spans,
            vec![(0.0, 0.3, "a"), (0.5, 0.8, "b"), (0.8, 1.0, "c")]
        );
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
