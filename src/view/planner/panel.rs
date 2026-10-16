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

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::input;
use super::model::{
    DestKind, Line, OrderModel, Row, Tone, TrackModel, TuningFacts, TuningModel, hover_line,
    order_model, tuning_model,
};
use super::slider::mix;
use crate::sim::body::{BodyClass, DisplayName};
use crate::sim::clock::GameClock;
use crate::sim::named_orbit::{DockingOrbitOf, InNamedOrbit, NamedOrbit};
use crate::sim::orbit::Orbit;
use crate::sim::ship::{DeltaV, Drive, InFlight};
use crate::sim::transfer::phase::{gap_rad, separation_km, together};
use crate::sim::transfer::places::{Band, BandRadii, Place};
use crate::sim::transfer::plan::{Plan, ShipOrder, Status};
use crate::sim::transfer::presets::Preset;
use crate::sim::tunables::SimTunables;
use crate::view::buttons::{ButtonLook, ButtonStyle, ConsoleButton, spawn_button};
use crate::view::colors;
use crate::view::fonts::Fonts;
use crate::view::inspect::{InsetPanel, spawn_spaced_capitals};
use crate::view::planner_state::{PlannerState, SliderKind};
use crate::view::planner_words::{
    CANCEL, CHANGE, CHEAPEST, CHOOSE_PROMPT, DELTA_V, DELTA_V_TOOLTIP, FASTEST, HEADING, HIGH,
    INTERCEPT, INTERCEPT_TOOLTIP, LOW, LegWords, NEXT_WINDOW, RENDEZVOUS, RENDEZVOUS_TOOLTIP,
    SCHEDULE, at_place, band_place, coasting_place, place_and_drive,
};
use crate::view::selection::Selection;
use crate::view::tunables::Tunables;

use super::tooltip::Tooltip;

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

/// What the panel reads of a ship: where it is parked, what it orbits,
/// its drive, its delta-v, its plan, and whether it is in flight.
type ShipView = (
    Option<&'static InNamedOrbit>,
    Option<&'static Orbit>,
    Option<&'static ChildOf>,
    Option<&'static Drive>,
    Option<&'static DeltaV>,
    Option<&'static Plan>,
    Has<InFlight>,
);

/// What the panel reads of the world besides the planner's state.
#[derive(SystemParam)]
pub struct PanelFacts<'w, 's> {
    selection: Res<'w, Selection>,
    state: Res<'w, PlannerState>,
    clock: Res<'w, GameClock>,
    sim_tunables: Res<'w, SimTunables>,
    names: Query<'w, 's, &'static DisplayName>,
    ships: Query<'w, 's, ShipView>,
    orbits: Query<
        'w,
        's,
        (
            &'static NamedOrbit,
            Option<&'static Band>,
            Option<&'static DockingOrbitOf>,
        ),
    >,
    stations: Query<'w, 's, (Entity, &'static BodyClass, &'static InNamedOrbit)>,
    radii: Query<'w, 's, &'static BandRadii>,
}

impl PanelFacts<'_, '_> {
    fn name(&self, entity: Entity) -> String {
        self.names
            .get(entity)
            .map(|n| n.0.clone())
            .unwrap_or_default()
    }

    /// The name a place goes by: its body's, or the station's, gate's,
    /// site's or ship's own.
    pub fn place_name(&self, place: Place) -> String {
        match place {
            Place::Body { body, .. } => self.name(body),
            Place::Station { station } => self.name(station),
            Place::Gate { gate } => self.name(gate),
            Place::Site { site } => self.name(site),
            Place::Ship { ship } => self.name(ship),
            Place::Hold => String::new(),
        }
    }

    fn dest_kind(&self, place: Place) -> DestKind {
        match place {
            Place::Body { body, .. } => DestKind::Body {
                both_bands: self.radii.get(body).is_ok_and(|r| r.high_km.is_some()),
            },
            Place::Station { .. } => DestKind::Station,
            Place::Gate { .. } => DestKind::Gate,
            Place::Site { .. } | Place::Hold => DestKind::Site,
            Place::Ship { .. } => DestKind::Ship,
        }
    }

    /// Where a parked ship is and what its orbit and phase are.
    fn parked(&self, ship: Entity) -> Option<(Entity, Orbit)> {
        let (in_orbit, orbit, ..) = self.ships.get(ship).ok()?;
        Some((in_orbit?.0, *orbit?))
    }

    /// PLNR-20: a parked ship's place: "At *name*" at a station it is
    /// together with or a gate's or site's orbit, else the band, as
    /// "High orbit, Halden IV".
    fn ship_place(&self, ship: Entity) -> String {
        let Some((orbit, at)) = self.parked(ship) else {
            return String::new();
        };
        let Ok((named, band, docking)) = self.orbits.get(orbit) else {
            return String::new();
        };
        if let Some(docking) = docking {
            return at_place(&self.name(docking.0));
        }
        let docked = self.stations.iter().find(|(station, class, in_orbit)| {
            **class == BodyClass::Station
                && in_orbit.0 == orbit
                && self.parked(*station).is_some_and(|(_, there)| {
                    together(
                        at.radius_km,
                        gap_rad(at.phase0_rad, there.phase0_rad),
                        self.sim_tunables.together_km,
                    )
                })
        });
        match (docked, band) {
            (Some((station, ..)), _) => at_place(&self.name(station)),
            (None, Some(band)) => band_place(*band, &self.name(named.parent)),
            (None, None) => format!("{} orbit, {}", named.name, self.name(named.parent)),
        }
    }

    /// PLNR-41: for a station or ship in the ship's own orbit and not
    /// together with it, the distance to it and whether it is ahead.
    fn gap_km(&self, ship: Entity, place: Place) -> Option<(f64, bool)> {
        let target = match place {
            Place::Station { station } => station,
            Place::Ship { ship } => ship,
            _ => return None,
        };
        let (orbit, at) = self.parked(ship)?;
        let (target_orbit, there) = self.parked(target)?;
        if orbit != target_orbit {
            return None;
        }
        let gap = gap_rad(at.phase0_rad, there.phase0_rad);
        let km = separation_km(at.radius_km, gap);
        (km >= self.sim_tunables.together_km).then_some((km, gap <= std::f64::consts::PI))
    }

    fn available_kms(&self, ship: Entity) -> f64 {
        self.ships
            .get(ship)
            .ok()
            .and_then(|(.., account, _, _)| account)
            .map_or(0.0, DeltaV::available_kms)
    }

    /// What the panel shows this frame.
    pub fn view(&self, tunables: &Tunables) -> PanelView {
        if let Some(session) = &self.state.session {
            return match session.request {
                None => PanelView::Choosing(self.hover_line(session.ship, tunables)),
                Some(_) => PanelView::Tuning(Box::new(self.tuning(tunables))),
            };
        }
        let Some(ship) = self.selection.selected else {
            return PanelView::Empty;
        };
        self.order_view(ship)
    }

    /// PLNR-12: the hovered place's line.
    fn hover_line(&self, ship: Entity, tunables: &Tunables) -> Option<Line> {
        let preview = self.state.session.as_ref()?.hover.as_ref()?;
        let destination = self.place_name(preview.place);
        let words = LegWords {
            intercept: false,
            to_ship: matches!(preview.place, Place::Ship { .. }),
            destination: &destination,
        };
        let gap = self.gap_km(ship, preview.place).map(|(km, ahead)| {
            crate::view::planner_words::gap_line(km, ahead, tunables.distance_au_threshold_au)
        });
        Some(hover_line(
            preview,
            &words,
            &self.primary_name(ship, None),
            gap,
        ))
    }

    /// The crossing's primary's name, or the body the ship orbits.
    fn primary_name(&self, ship: Entity, primary: Option<Entity>) -> String {
        primary
            .or_else(|| {
                let (.., child_of, _, _, _, _) = self.ships.get(ship).ok()?;
                child_of.map(ChildOf::parent)
            })
            .map(|e| self.name(e))
            .unwrap_or_default()
    }

    /// PLNR-20 to PLNR-32, PLNR-40, PLNR-41: the tuning panel.
    fn tuning(&self, tunables: &Tunables) -> TuningModel {
        let session = self.state.session.as_ref().expect("an open planner");
        let request = session.request.expect("a request");
        let ship = session.ship;
        let destination = self.place_name(request.destination);
        let drive = self
            .ships
            .get(ship)
            .ok()
            .and_then(|(.., drive, _, _, _)| drive)
            .map_or("", |d| d.name);
        let place = place_and_drive(&self.ship_place(ship), drive);
        let found = session.found.as_ref();
        let available_kms = found
            .and_then(|f| f.context.as_ref())
            .map_or(self.available_kms(ship), |c| c.available_kms);
        let primary = self.primary_name(ship, found.and_then(|f| f.primary));
        let ship_name = self.name(ship);
        let facts = TuningFacts {
            ship: &ship_name,
            place: &place,
            destination: &destination,
            kind: self.dest_kind(request.destination),
            primary: &primary,
            request,
            setting: session.setting,
            found: found.map(|f| &f.result),
            leg: session.leg.as_ref(),
            travel_track: &session.travel_track,
            available_kms,
            today: self.clock.elapsed_days(),
            gap_km: self.gap_km(ship, request.destination),
        };
        tuning_model(&facts, tunables)
    }

    /// PLNR-51 / PLNR-54: the selected ship's order, or its coasting
    /// place, while the planner is closed.
    fn order_view(&self, ship: Entity) -> PanelView {
        let Ok((.., Some(plan), in_flight)) = self.ships.get(ship) else {
            return PanelView::Empty;
        };
        let leg = plan.leg();
        let destination = self.place_name(leg.destination);
        let intercept = leg.arrival.is_intercept();
        match plan.status {
            Status::Scheduled | Status::InFlight => {
                let words = LegWords {
                    intercept,
                    to_ship: matches!(leg.destination, Place::Ship { .. }),
                    destination: &destination,
                };
                let model = order_model(
                    &words,
                    leg,
                    plan.status == Status::Scheduled,
                    self.clock.elapsed_days(),
                );
                PanelView::Order(model, ship)
            }
            Status::Done if intercept && in_flight => {
                let origin = leg.origin.map(|o| self.place_name(o)).unwrap_or_default();
                PanelView::Coasting(coasting_place(&destination, &origin))
            }
            _ => PanelView::Empty,
        }
    }
}

/// The panel's root and the notice line, put into the sidebar once it
/// exists: the root after the details, the notice at the very top.
pub fn spawn_root(
    mut commands: Commands,
    panels: Query<Entity, With<InsetPanel>>,
    roots: Query<(), With<PlannerRoot>>,
    fonts: Res<Fonts>,
    tunables: Res<Tunables>,
) {
    let Ok(panel) = panels.single() else {
        return;
    };
    if !roots.is_empty() {
        return;
    }
    let root = commands
        .spawn((
            PlannerRoot,
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(ROW_GAP_PX),
                ..default()
            },
        ))
        .id();
    let notice = commands
        .spawn((
            NoticeLine,
            Node {
                display: Display::None,
                ..default()
            },
            Text::new(""),
            fonts.label(tunables.sidebar_text_px),
            TextColor(colors::SECONDARY),
        ))
        .id();
    commands.entity(panel).insert_children(0, &[notice]);
    commands.entity(panel).insert_children(2, &[root]);
}

const ROW_GAP_PX: f32 = 6.0;
/// The space between buttons in a row.
const BUTTON_GAP_PX: f32 = 6.0;
/// Room above and below a track for the taller thumb.
fn track_margin_px(tunables: &Tunables) -> f32 {
    ((tunables.slider_thumb_h_px - tunables.slider_track_px) / 2.0).max(0.0)
}

/// The fonts and tunables the panel is built from.
type Assets<'a> = (&'a Fonts, &'a Tunables);

/// Rebuilds the panel when its shape changes, then writes this frame's
/// view into it.
#[allow(clippy::too_many_arguments)]
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
        root.with_children(|panel| spawn_view(panel, (&fonts, &tunables), &shape));
        *shown = Some(shape);
        // The new nodes exist from the next frame; they are written then.
        return;
    }
    writes.write(&view, &tunables);
}

/// The nodes `update_panel` writes into.
#[derive(SystemParam)]
pub struct PanelWrites<'w, 's> {
    texts: Query<'w, 's, (&'static Field, &'static mut Text, &'static mut TextColor)>,
    tracks: Query<'w, 's, (&'static Track, &'static mut BackgroundGradient)>,
    thumbs: Query<'w, 's, (&'static Thumb, &'static mut Node), Without<DeltaFill>>,
    fills: Query<'w, 's, (&'static mut Node, &'static mut BackgroundColor), With<DeltaFill>>,
    presets:
        Query<'w, 's, (&'static PresetButton, &'static mut ConsoleButton), Without<ScheduleButton>>,
    schedule: Query<'w, 's, &'static mut ConsoleButton, With<ScheduleButton>>,
    segments: Query<'w, 's, (&'static Choice, &'static mut Segment)>,
}

impl PanelWrites<'_, '_> {
    fn write(&mut self, view: &PanelView, tunables: &Tunables) {
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
        for (preset, mut button) in &mut self.presets {
            let enabled = model
                .presets
                .iter()
                .any(|(p, enabled)| *p == preset.0 && *enabled);
            if button.disabled == enabled {
                button.disabled = !enabled;
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
        for (track, mut gradient) in &mut self.tracks {
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

/// Lays out a view's shape; the texts are written afterwards.
fn spawn_view(panel: &mut ChildSpawnerCommands, assets: Assets, shape: &Shape) {
    match shape {
        Shape::Empty => {}
        Shape::Choosing => spawn_choosing(panel, assets),
        Shape::Tuning { .. } => spawn_tuning(panel, assets, shape),
        Shape::Order { ship, scheduled } => spawn_order(panel, assets, *ship, *scheduled),
        Shape::Coasting => {
            spawn_field(panel, assets, Field::Coasting, false);
        }
    }
}

/// The section's hairline and its heading, "TRANSFER" (PLNR-20).
fn spawn_heading(panel: &mut ChildSpawnerCommands, (fonts, tunables): Assets) {
    panel.spawn((
        Node {
            width: percent(100),
            height: px(1),
            margin: UiRect::vertical(px(4)),
            ..default()
        },
        BackgroundColor(colors::PANEL_BORDER),
    ));
    spawn_spaced_capitals(panel, (fonts, tunables), HEADING, ());
}

/// A text written each frame: data face for figures, label face else.
fn spawn_field(
    panel: &mut ChildSpawnerCommands,
    (fonts, tunables): Assets,
    field: Field,
    data: bool,
) -> Entity {
    let font = if data {
        fonts.data(tunables.sidebar_text_px)
    } else {
        fonts.label(tunables.sidebar_text_px)
    };
    panel
        .spawn((field, Text::new(""), font, TextColor(colors::NEUTRAL)))
        .id()
}

/// A label on the left and a figure on the right.
fn spawn_row(
    panel: &mut ChildSpawnerCommands,
    assets: Assets,
    label: Result<&'static str, Field>,
    value: Field,
    tooltip: Option<&'static str>,
) {
    // Only a fixed label carries a tooltip (PLNR-91).
    let (fonts, tunables) = assets;
    panel
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Baseline,
            ..default()
        })
        .with_children(|row| {
            match label {
                Ok(text) => {
                    let mut label = row.spawn((
                        Text::new(text),
                        fonts.label(tunables.sidebar_text_px),
                        TextColor(colors::SECONDARY),
                    ));
                    if let Some(tip) = tooltip {
                        label.insert(Tooltip(tip));
                    }
                }
                Err(field) => {
                    spawn_field(row, assets, field, false);
                }
            }
            spawn_field(row, assets, value, true);
        });
}

/// PLNR-10 / PLNR-12: the prompt, the hovered place's line and Cancel.
fn spawn_choosing(panel: &mut ChildSpawnerCommands, assets: Assets) {
    let (fonts, tunables) = assets;
    spawn_heading(panel, assets);
    panel.spawn((
        Text::new(CHOOSE_PROMPT),
        fonts.label(tunables.sidebar_text_px),
        TextColor(colors::NEUTRAL),
    ));
    spawn_field(panel, assets, Field::Hover, false);
    spawn_button_row(panel, |row| spawn_quiet_cancel(row, assets));
}

/// PLNR-02: the quiet Cancel that ends the planner.
fn spawn_quiet_cancel(row: &mut ChildSpawnerCommands, (fonts, tunables): Assets) {
    spawn_button(
        row,
        fonts,
        CANCEL,
        tunables.sidebar_text_px,
        ConsoleButton::new(ButtonStyle::Quiet),
    )
    .observe(input::on_planner_cancel);
}

fn spawn_button_row(
    panel: &mut ChildSpawnerCommands,
    buttons: impl FnOnce(&mut ChildSpawnerCommands),
) {
    panel
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(BUTTON_GAP_PX),
            row_gap: px(BUTTON_GAP_PX),
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(buttons);
}

/// PLNR-20: the tuning panel, top to bottom.
fn spawn_tuning(panel: &mut ChildSpawnerCommands, assets: Assets, shape: &Shape) {
    let Shape::Tuning {
        bands,
        depart,
        travel,
        travel_note,
        gap,
        arrival,
        speed,
        delta_v,
        presets,
        arrival_choice,
    } = shape
    else {
        return;
    };
    spawn_heading(panel, assets);
    spawn_field(panel, assets, Field::Route, false);
    spawn_field(panel, assets, Field::Place, false);
    if *bands {
        spawn_choice(
            panel,
            assets,
            [
                (Choice::Band(Band::High), HIGH, None),
                (Choice::Band(Band::Low), LOW, None),
            ],
        );
    }
    if let Some(track) = depart {
        spawn_row(
            panel,
            assets,
            Err(Field::DepartLabel),
            Field::DepartValue,
            None,
        );
        if *track {
            spawn_track(panel, assets, SliderKind::Depart);
        }
    }
    if let Some(track) = travel {
        spawn_row(
            panel,
            assets,
            Ok(crate::view::planner_words::TRAVEL_TIME),
            Field::TravelValue,
            None,
        );
        if *track {
            spawn_track(panel, assets, SliderKind::Travel);
        }
        if *travel_note {
            spawn_field(panel, assets, Field::TravelNote, false);
        }
    }
    if *gap {
        spawn_field(panel, assets, Field::Gap, false);
    }
    if *arrival {
        spawn_row(
            panel,
            assets,
            Err(Field::ArrivalLabel),
            Field::ArrivalValue,
            None,
        );
    }
    if *speed {
        spawn_row(
            panel,
            assets,
            Err(Field::SpeedLabel),
            Field::SpeedValue,
            None,
        );
    }
    if *delta_v {
        spawn_delta_v(panel, assets);
    }
    spawn_presets(panel, assets, presets);
    if *arrival_choice {
        spawn_choice(
            panel,
            assets,
            [
                (
                    Choice::Arrival { intercept: false },
                    RENDEZVOUS,
                    Some(RENDEZVOUS_TOOLTIP),
                ),
                (
                    Choice::Arrival { intercept: true },
                    INTERCEPT,
                    Some(INTERCEPT_TOOLTIP),
                ),
            ],
        );
    }
    spawn_schedule(panel, assets);
}

/// PLNR-14 / PLNR-30: a two-way choice, the first option first.
fn spawn_choice(
    panel: &mut ChildSpawnerCommands,
    (fonts, tunables): Assets,
    options: [(Choice, &'static str, Option<&'static str>); 2],
) {
    spawn_button_row(panel, |row| {
        for (choice, label, tooltip) in options {
            let look = segment_look(false, tunables);
            let mut option = row.spawn((
                choice,
                Segment { selected: false },
                Node {
                    padding: UiRect::axes(px(12), px(5)),
                    border: UiRect::all(px(tunables.button_stroke_px)),
                    ..default()
                },
                BackgroundColor(look.background),
                BorderColor::all(look.border),
            ));
            option.with_children(|b| {
                b.spawn((
                    Text::new(label),
                    fonts.label(tunables.sidebar_text_px),
                    TextColor(look.text),
                ));
            });
            if let Some(text) = tooltip {
                option.insert(Tooltip(text));
            }
            option.observe(
                move |click: On<PointerClick>, mut state: ResMut<PlannerState>| {
                    if click.button == PointerButton::Primary {
                        input::choose(&mut state, choice);
                    }
                },
            );
        }
    });
}

/// PLNR-23 / PLNR-28: a slider's track, with its thumb.
fn spawn_track(panel: &mut ChildSpawnerCommands, (_, tunables): Assets, kind: SliderKind) {
    let margin = track_margin_px(tunables);
    panel
        .spawn((
            Track(kind),
            Node {
                width: percent(100),
                height: px(tunables.slider_track_px),
                margin: UiRect::vertical(px(margin)),
                ..default()
            },
            BackgroundGradient(vec![]),
        ))
        .with_children(|track| {
            track.spawn((
                Thumb(kind),
                Node {
                    position_type: PositionType::Absolute,
                    width: px(tunables.slider_thumb_w_px),
                    height: px(tunables.slider_thumb_h_px),
                    top: px(-margin),
                    left: percent(0),
                    margin: UiRect::left(px(-tunables.slider_thumb_w_px / 2.0)),
                    ..default()
                },
                BackgroundColor(colors::NEUTRAL),
            ));
        })
        .observe(
            move |press: On<PointerPress>, mut slide: input::SliderInput| {
                if press.button == PointerButton::Primary {
                    slide.focus(kind);
                    slide.slide_to(kind, press.pointer.position.x);
                }
            },
        )
        .observe(
            move |drag: On<PointerDrag>, mut slide: input::SliderInput| {
                if drag.button == PointerButton::Primary {
                    slide.slide_to(kind, drag.pointer.position.x);
                }
            },
        );
}

/// PLNR-27: the Delta-v row with its tooltip, the bar and the line.
fn spawn_delta_v(panel: &mut ChildSpawnerCommands, assets: Assets) {
    let (_, tunables) = assets;
    spawn_row(
        panel,
        assets,
        Ok(DELTA_V),
        Field::DeltaVValue,
        Some(DELTA_V_TOOLTIP),
    );
    panel
        .spawn((
            Node {
                width: percent(100),
                height: px(tunables.delta_bar_px),
                ..default()
            },
            BackgroundColor(colors::PANEL_BORDER.with_alpha(HAIRLINE_ALPHA)),
        ))
        .with_children(|bar| {
            bar.spawn((
                DeltaFill,
                Node {
                    width: percent(0),
                    height: percent(100),
                    ..default()
                },
                BackgroundColor(colors::OWN),
            ));
        });
    spawn_field(panel, assets, Field::DeltaVLine, false);
}

/// The Delta-v bar's empty part is the inset's hairline colour, faint so
/// the fill reads against it.
const HAIRLINE_ALPHA: f32 = 0.25;

/// PLNR-29: the presets, as secondary buttons in a row.
fn spawn_presets(panel: &mut ChildSpawnerCommands, (fonts, tunables): Assets, presets: &[Preset]) {
    if presets.is_empty() {
        return;
    }
    spawn_button_row(panel, |row| {
        for &preset in presets {
            let label = match preset {
                Preset::Fastest => FASTEST,
                Preset::Cheapest => CHEAPEST,
                Preset::NextWindow => NEXT_WINDOW,
            };
            spawn_button(
                row,
                fonts,
                label,
                tunables.sidebar_text_px,
                ConsoleButton::new(ButtonStyle::Secondary),
            )
            .insert(PresetButton(preset))
            .observe(
                move |click: On<PointerClick>, mut state: ResMut<PlannerState>| {
                    if click.button == PointerButton::Primary {
                        input::apply_preset(&mut state, preset);
                    }
                },
            );
        }
    });
}

/// PLNR-31 / PLNR-02: Schedule, the quiet Cancel, and the reason line.
fn spawn_schedule(panel: &mut ChildSpawnerCommands, assets: Assets) {
    let (fonts, tunables) = assets;
    spawn_button_row(panel, |row| {
        spawn_button(
            row,
            fonts,
            SCHEDULE,
            tunables.sidebar_text_px,
            ConsoleButton::new(ButtonStyle::Primary).disabled(),
        )
        .insert(ScheduleButton)
        .observe(input::on_schedule);
        spawn_quiet_cancel(row, assets);
    });
    spawn_field(panel, assets, Field::Reason, false);
}

/// PLNR-51: an order: its heading, Departs, the arrival row, its delta-v
/// and, while Scheduled, Cancel and Change.
fn spawn_order(panel: &mut ChildSpawnerCommands, assets: Assets, ship: Entity, scheduled: bool) {
    let (fonts, tunables) = assets;
    panel.spawn((
        Node {
            width: percent(100),
            height: px(1),
            margin: UiRect::vertical(px(4)),
            ..default()
        },
        BackgroundColor(colors::PANEL_BORDER),
    ));
    spawn_field(panel, assets, Field::OrderHeading, false);
    spawn_row(
        panel,
        assets,
        Err(Field::OrderDepartsLabel),
        Field::OrderDepartsValue,
        None,
    );
    spawn_row(
        panel,
        assets,
        Err(Field::OrderArrivalLabel),
        Field::OrderArrivalValue,
        None,
    );
    spawn_row(panel, assets, Ok(DELTA_V), Field::OrderDeltaV, None);
    if !scheduled {
        return;
    }
    spawn_button_row(panel, |row| {
        spawn_button(
            row,
            fonts,
            CANCEL,
            tunables.sidebar_text_px,
            ConsoleButton::new(ButtonStyle::Secondary),
        )
        .observe(
            move |click: On<PointerClick>, mut orders: MessageWriter<ShipOrder>| {
                if click.button == PointerButton::Primary {
                    orders.write(ShipOrder::Cancel { ship });
                }
            },
        );
        spawn_button(
            row,
            fonts,
            CHANGE,
            tunables.sidebar_text_px,
            ConsoleButton::new(ButtonStyle::Secondary),
        )
        .observe(
            move |click: On<PointerClick>, plans: Query<&Plan>, mut state: ResMut<PlannerState>| {
                if click.button == PointerButton::Primary
                    && let Ok(plan) = plans.get(ship)
                {
                    input::change_order(&mut state, ship, plan);
                }
            },
        );
    });
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
