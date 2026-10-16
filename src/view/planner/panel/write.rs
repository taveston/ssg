//! Writes a view into the planner's panel in place: its texts, the
//! sliders' tracks and thumbs, the Delta-v bar, the buttons' enabled
//! state and the choices' selected options; and the notice line.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::super::model::{TrackModel, TuningModel};
use super::super::state::{PlannerState, SliderKind};
use super::super::tooltip::{Stretches, Tooltip};
use super::super::words::preset_tooltip;
use super::{
    Choice, DeltaFill, Field, NoticeLine, PanelView, PresetButton, ScheduleButton, Segment,
    SliderLabel, Thumb, Track, segment_look, tone_colour,
};
use crate::view::buttons::ConsoleButton;
use crate::view::colors;
use crate::view::tunables::Tunables;

/// The nodes `update_panel` writes into.
#[derive(SystemParam)]
pub struct PanelWrites<'w, 's> {
    texts: Query<'w, 's, (&'static Field, &'static mut Text, &'static mut TextColor)>,
    tracks: Query<
        'w,
        's,
        (
            &'static Track,
            &'static mut BackgroundGradient,
            &'static mut Stretches,
        ),
    >,
    thumbs: Query<'w, 's, (&'static Thumb, &'static mut Node), Without<DeltaFill>>,
    fills: Query<'w, 's, (&'static mut Node, &'static mut BackgroundColor), With<DeltaFill>>,
    presets: Query<
        'w,
        's,
        (
            &'static PresetButton,
            &'static mut ConsoleButton,
            &'static mut Tooltip,
        ),
        Without<ScheduleButton>,
    >,
    slider_labels:
        Query<'w, 's, (&'static SliderLabel, &'static mut Tooltip), Without<PresetButton>>,
    schedule: Query<'w, 's, &'static mut ConsoleButton, With<ScheduleButton>>,
    segments: Query<'w, 's, (&'static Choice, &'static mut Segment)>,
}

impl PanelWrites<'_, '_> {
    pub(super) fn write(&mut self, view: &PanelView, tunables: &Tunables) {
        let texts = view.texts();
        for (field, mut text, mut colour) in &mut self.texts {
            if let Some((_, value, tone)) = texts.iter().find(|(f, ..)| f == field) {
                if text.0 != *value {
                    text.0.clone_from(value);
                }
                colour.set_if_neq(TextColor(tone_colour(*tone)));
            }
        }
        let PanelView::Tuning(model) = view else {
            return;
        };
        self.write_sliders(model);
        if let Some(delta_v) = &model.delta_v {
            for (mut node, mut background) in &mut self.fills {
                node.width = percent(delta_v.fill * 100.0);
                let colour = if delta_v.over {
                    colors::WARNING
                } else {
                    colors::OWN
                };
                background.set_if_neq(BackgroundColor(colour));
            }
        }
        for (preset, mut button, mut tooltip) in &mut self.presets {
            let enabled = model
                .presets
                .iter()
                .any(|(p, enabled)| *p == preset.0 && *enabled);
            if button.disabled == enabled {
                button.disabled = !enabled;
            }
            // PLNR-103: a disabled Fastest says why instead.
            tooltip.set_if_neq(Tooltip(preset_tooltip(preset.0, enabled)));
            // PLNR-29: the preset the sliders sit on is drawn selected.
            let selected = model.selected_preset == Some(preset.0);
            if button.selected != selected {
                button.selected = selected;
            }
        }
        for mut button in &mut self.schedule {
            if button.disabled == model.schedule {
                button.disabled = !model.schedule;
            }
        }
        for (choice, mut segment) in &mut self.segments {
            let selected = match choice {
                Choice::Band(band) => model.bands == Some(*band),
                Choice::Arrival { intercept } => model.arrival_choice == Some(*intercept),
            };
            segment.set_if_neq(Segment { selected });
        }
        let _ = tunables;
    }

    fn write_sliders(&mut self, model: &TuningModel) {
        let track_of = |kind: SliderKind| -> Option<&TrackModel> {
            match kind {
                SliderKind::Depart => model.depart.as_ref()?.track.as_ref(),
                SliderKind::Travel => model.travel.as_ref()?.track.as_ref(),
            }
        };
        // PLNR-104 to PLNR-106: the labels' tooltips follow the leg.
        for (label, mut tooltip) in &mut self.slider_labels {
            let slider = match label.0 {
                SliderKind::Depart => model.depart.as_ref(),
                SliderKind::Travel => model.travel.as_ref(),
            };
            if let Some(slider) = slider {
                tooltip.set_if_neq(Tooltip(slider.tooltip));
            }
        }
        for (track, mut gradient, mut stretches) in &mut self.tracks {
            if let Some(model) = track_of(track.0) {
                let stops = model
                    .stops
                    .iter()
                    .map(|(at, colour)| ColorStop::percent(*colour, at * 100.0))
                    .collect();
                let new = BackgroundGradient(vec![LinearGradient::to_right(stops).into()]);
                if *gradient != new {
                    *gradient = new;
                }
                // PLNR-35: the rejected stretches carry their reasons.
                if stretches.0 != model.stretches {
                    stretches.0.clone_from(&model.stretches);
                }
            }
        }
        for (thumb, mut node) in &mut self.thumbs {
            if let Some(model) = track_of(thumb.0) {
                let left = percent(model.thumb * 100.0);
                if node.left != left {
                    node.left = left;
                }
            }
        }
    }
}

/// PLNR-14: paints each option of a two-way choice.
pub fn paint_segments(
    tunables: Res<Tunables>,
    mut segments: Query<
        (&Segment, &Children, &mut BackgroundColor, &mut BorderColor),
        Changed<Segment>,
    >,
    mut texts: Query<&mut TextColor>,
) {
    for (segment, children, mut background, mut border) in &mut segments {
        let look = segment_look(segment.selected, &tunables);
        background.set_if_neq(BackgroundColor(look.background));
        border.set_if_neq(BorderColor::all(look.border));
        for child in children {
            if let Ok(mut text) = texts.get_mut(*child) {
                text.set_if_neq(TextColor(look.text));
            }
        }
    }
}

/// PLNR-53: the notice line shows its text while there is one.
pub fn update_notice(
    state: Res<PlannerState>,
    mut lines: Query<(&mut Text, &mut Node), With<NoticeLine>>,
) {
    for (mut text, mut node) in &mut lines {
        let notice = state.notice.unwrap_or("");
        if text.0 != notice {
            notice.clone_into(&mut text.0);
        }
        let display = if state.notice.is_some() {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
    }
}
