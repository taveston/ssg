//! The planner's panel in the inset (PLNR-02, PLNR-10, PLNR-12, PLNR-14,
//! PLNR-20 to PLNR-32, PLNR-40, PLNR-41, PLNR-51, PLNR-53, PLNR-54): what
//! sits between the inset's details and its outline. While the planner
//! is open it is the choosing prompt or the tuning panel; while it is
//! closed it shows the selected ship's order, or the place of a ship
//! coasting after an Intercept.
//!
//! Each frame the panel's view is worked out from the planner's state
//! (`model.rs`); its layout is rebuilt only when the view's shape changes
//! (a row appears or goes), and otherwise the texts, tracks, bar and
//! buttons are written in place, so a slider being dragged is never
//! despawned under the pointer.
//!
//! - `facts.rs`: what the panel reads of the world, and the view it
//!   works out from it.
//! - `spawn.rs`: the panel's nodes, one helper per section.
//! - `write.rs`: writing a view into the nodes in place.

use bevy::prelude::*;

use super::model::{Line, OrderModel, Row, Tone, TuningModel};
use super::slider::mix;
use crate::sim::transfer::places::Band;
use crate::sim::transfer::presets::Preset;
use crate::view::buttons::ButtonLook;
use crate::view::colors;
use crate::view::fonts::Fonts;
use crate::view::planner_state::SliderKind;
use crate::view::tunables::Tunables;

mod facts;
mod spawn;
mod write;

pub use facts::PanelFacts;
pub use spawn::spawn_root;
pub use write::{PanelWrites, paint_segments, update_notice};

/// The node the panel is built in, between the inset's details and its
/// outline.
#[derive(Component)]
pub struct PlannerRoot;
/// PLNR-53: the line at the top of the inset after an order departed or
/// failed while it was being changed.
#[derive(Component)]
pub struct NoticeLine;

/// A text the panel writes each frame.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Field {
    Hover,
    Route,
    Place,
    DepartLabel,
    DepartValue,
    TravelValue,
    TravelNote,
    Gap,
    ArrivalLabel,
    ArrivalValue,
    SpeedLabel,
    SpeedValue,
    DeltaVValue,
    DeltaVLine,
    Reason,
    OrderHeading,
    OrderDepartsLabel,
    OrderDepartsValue,
    OrderArrivalLabel,
    OrderArrivalValue,
    OrderDeltaV,
    Coasting,
}

/// A slider's track, coloured by cost (PLNR-28).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Track(pub SliderKind);
/// A slider's thumb.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Thumb(pub SliderKind);
/// PLNR-27: the filled part of the Delta-v bar.
#[derive(Component)]
pub struct DeltaFill;
/// PLNR-29: a preset's button.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresetButton(pub Preset);
/// PLNR-31: the panel's one primary button.
#[derive(Component)]
pub struct ScheduleButton;

/// An option of a two-way choice (PLNR-14, PLNR-30).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    Band(Band),
    Arrival { intercept: bool },
}

/// Whether a choice's option is the selected one, which `paint_segments`
/// draws.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment {
    pub selected: bool,
}

/// What the panel shows this frame.
#[derive(Debug, Clone, PartialEq)]
pub enum PanelView {
    /// Nothing: no ship selected, or nothing to show for it.
    Empty,
    /// PLNR-10 / PLNR-12: the prompt, and the hovered place's line.
    Choosing(Option<Line>),
    Tuning(Box<TuningModel>),
    /// PLNR-51: the selected ship's order, and its ship.
    Order(OrderModel, Entity),
    /// PLNR-54: a ship coasting after an Intercept.
    Coasting(String),
}

/// Which rows and buttons the panel has: a change rebuilds it.
#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    Empty,
    Choosing,
    Tuning {
        bands: bool,
        /// A Depart row, and whether it has a track.
        depart: Option<bool>,
        travel: Option<bool>,
        travel_note: bool,
        gap: bool,
        arrival: bool,
        speed: bool,
        delta_v: bool,
        presets: Vec<Preset>,
        arrival_choice: bool,
    },
    Order {
        ship: Entity,
        scheduled: bool,
    },
    Coasting,
}

impl PanelView {
    /// The shape this view is laid out in.
    pub fn shape(&self) -> Shape {
        match self {
            PanelView::Empty => Shape::Empty,
            PanelView::Choosing(_) => Shape::Choosing,
            PanelView::Tuning(model) => Shape::Tuning {
                bands: model.bands.is_some(),
                depart: model.depart.as_ref().map(|s| s.track.is_some()),
                travel: model.travel.as_ref().map(|s| s.track.is_some()),
                travel_note: model.travel.as_ref().is_some_and(|s| s.note.is_some()),
                gap: model.gap.is_some(),
                arrival: model.arrival.is_some(),
                speed: model.speed.is_some(),
                delta_v: model.delta_v.is_some(),
                presets: model.presets.iter().map(|(p, _)| *p).collect(),
                arrival_choice: model.arrival_choice.is_some(),
            },
            PanelView::Order(order, ship) => Shape::Order {
                ship: *ship,
                scheduled: order.scheduled,
            },
            PanelView::Coasting(_) => Shape::Coasting,
        }
    }

    /// Every text the view writes, with its colour.
    pub fn texts(&self) -> Vec<(Field, String, Tone)> {
        let mut texts = Vec::new();
        let mut row = |label: Field, value: Field, row: &Row| {
            texts.push((label, row.label.clone(), Tone::Secondary));
            texts.push((value, row.value.clone(), row.tone));
        };
        match self {
            PanelView::Empty => {}
            PanelView::Choosing(line) => {
                let line = line.clone().unwrap_or(Line {
                    text: String::new(),
                    tone: Tone::Neutral,
                });
                texts.push((Field::Hover, line.text, line.tone));
            }
            PanelView::Tuning(model) => return tuning_texts(model),
            PanelView::Order(order, _) => {
                row(
                    Field::OrderDepartsLabel,
                    Field::OrderDepartsValue,
                    &order.departs,
                );
                row(
                    Field::OrderArrivalLabel,
                    Field::OrderArrivalValue,
                    &order.arrival,
                );
                texts.push((
                    Field::OrderDeltaV,
                    order.delta_v.value.clone(),
                    Tone::Neutral,
                ));
                texts.push((Field::OrderHeading, order.heading.clone(), Tone::Neutral));
            }
            PanelView::Coasting(place) => {
                texts.push((Field::Coasting, place.clone(), Tone::Secondary));
            }
        }
        texts
    }
}

/// The tuning panel's texts.
fn tuning_texts(model: &TuningModel) -> Vec<(Field, String, Tone)> {
    let mut texts = vec![
        (Field::Route, model.route.clone(), Tone::Neutral),
        (Field::Place, model.place.clone(), Tone::Secondary),
        (Field::Reason, model.reason.clone(), Tone::Warning),
    ];
    let mut row = |label: Option<Field>, value: Field, row: &Row| {
        if let Some(label) = label {
            texts.push((label, row.label.clone(), Tone::Secondary));
        }
        texts.push((value, row.value.clone(), row.tone));
    };
    if let Some(depart) = &model.depart {
        row(Some(Field::DepartLabel), Field::DepartValue, &depart.row);
    }
    if let Some(travel) = &model.travel {
        row(None, Field::TravelValue, &travel.row);
    }
    if let Some(arrival) = &model.arrival {
        row(Some(Field::ArrivalLabel), Field::ArrivalValue, arrival);
    }
    if let Some(speed) = &model.speed {
        row(Some(Field::SpeedLabel), Field::SpeedValue, speed);
    }
    if let Some(delta_v) = &model.delta_v {
        row(None, Field::DeltaVValue, &delta_v.row);
        texts.push((
            Field::DeltaVLine,
            delta_v.line.text.clone(),
            delta_v.line.tone,
        ));
    }
    if let Some(note) = model.travel.as_ref().and_then(|t| t.note.as_ref()) {
        texts.push((Field::TravelNote, note.text.clone(), note.tone));
    }
    if let Some(gap) = &model.gap {
        texts.push((Field::Gap, gap.clone(), Tone::Secondary));
    }
    texts
}

/// A tone's colour.
pub fn tone_colour(tone: Tone) -> Color {
    match tone {
        Tone::Neutral => colors::NEUTRAL,
        Tone::Secondary => colors::SECONDARY,
        Tone::Warning => colors::WARNING,
    }
}

/// PLNR-14: a two-way choice's option: the selected one filled with the
/// neutral colour mixed into the ground at `segment_fill_mix`, with
/// neutral text; the other outlined like a secondary button (BTN-01).
pub fn segment_look(selected: bool, tunables: &Tunables) -> ButtonLook {
    if selected {
        let fill = mix(colors::GROUND, colors::NEUTRAL, tunables.segment_fill_mix);
        ButtonLook {
            background: fill,
            border: fill,
            text: colors::NEUTRAL,
        }
    } else {
        ButtonLook {
            background: Color::NONE,
            border: colors::SECONDARY,
            text: colors::NEUTRAL,
        }
    }
}

/// Rebuilds the panel when its shape changes, then writes this frame's
/// view into it.
pub fn update_panel(
    mut commands: Commands,
    facts: PanelFacts,
    fonts: Res<Fonts>,
    tunables: Res<Tunables>,
    roots: Query<Entity, With<PlannerRoot>>,
    mut shown: Local<Option<Shape>>,
    mut writes: PanelWrites,
) {
    let Ok(root) = roots.single() else {
        return;
    };
    let view = facts.view(&tunables);
    let shape = view.shape();
    if shown.as_ref() != Some(&shape) {
        let mut root = commands.entity(root);
        root.despawn_related::<Children>();
        root.with_children(|panel| spawn::spawn_view(panel, (&fonts, &tunables), &shape));
        *shown = Some(shape);
        // The new nodes exist from the next frame; they are written then.
        return;
    }
    writes.write(&view, &tunables);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// PLNR-14: the selected option is filled with the neutral colour
    /// mixed into the ground, with neutral text; the other is outlined
    /// like a secondary button.
    #[test]
    fn the_selected_option_is_filled_and_the_other_outlined() {
        let tunables = Tunables::default();
        let selected = segment_look(true, &tunables);
        assert_eq!(
            selected.background,
            mix(colors::GROUND, colors::NEUTRAL, tunables.segment_fill_mix)
        );
        assert_eq!(selected.text, colors::NEUTRAL);
        let other = segment_look(false, &tunables);
        assert_eq!(other.background, Color::NONE);
        assert_eq!(other.border, colors::SECONDARY);
        assert_eq!(other.text, colors::NEUTRAL);
    }
}
