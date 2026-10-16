//! What the planner's panel says, as plain values (PLNR-04, ARCH-20):
//! every row, line, track and button state, worked out from the search's
//! and the solver's results and the planner's words. `panel.rs` lays the
//! model out and writes it into the nodes; nothing here touches Bevy's
//! world.

use bevy::prelude::Color;

use crate::sim::transfer::grid::{Sample, Setting};
use crate::sim::transfer::order::{Arrival, Infeasible, SolvedLeg};
use crate::sim::transfer::places::Band;
use crate::sim::transfer::presets::Preset;
use crate::sim::transfer::ranges::{Slider, Sliders};
use crate::sim::transfer::search::{Request, Search};
use crate::view::planner_state::{HoverOutcome, HoverPreview};
use crate::view::planner_words::{
    DELTA_V, DEPART, DEPARTED, DEPARTS, LegWords, NOW, TRAVEL_FIXED, TRAVEL_TIME, cant_reach_line,
    date, dated, delta_v_readout, fastest_line, gap_line, leave_readout, reason, travel_with_laps,
    trim_line,
};
use crate::view::readout::{format_duration, format_speed};
use crate::view::tunables::Tunables;

use super::slider::{fraction_of, track_stops};

/// The colour a line of text is drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Neutral,
    Secondary,
    Warning,
}

/// A line of text and its colour.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub text: String,
    pub tone: Tone,
}

impl Line {
    fn new(text: impl Into<String>, tone: Tone) -> Self {
        Self {
            text: text.into(),
            tone,
        }
    }
}

/// A label on the left and its figure on the right.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub label: String,
    pub value: String,
    pub tone: Tone,
}

impl Row {
    fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            tone: Tone::Neutral,
        }
    }
}

/// A slider's track: where its thumb sits (0 to 1) and its colours.
#[derive(Debug, Clone, PartialEq)]
pub struct TrackModel {
    pub thumb: f32,
    pub stops: Vec<(f32, Color)>,
}

/// A slider's row: its readout, its track (none for a setting the drive
/// or the leg fixes) and the line beneath it.
#[derive(Debug, Clone, PartialEq)]
pub struct SliderModel {
    pub row: Row,
    pub track: Option<TrackModel>,
    pub note: Option<Line>,
}

/// PLNR-27: the Delta-v readout, its bar and the line beneath.
#[derive(Debug, Clone, PartialEq)]
pub struct DeltaVModel {
    pub row: Row,
    /// How much of the bar is filled, 0 to 1.
    pub fill: f32,
    pub over: bool,
    pub line: Line,
}

/// What kind of place the destination is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestKind {
    /// A body's band; `both_bands` when it has High and Low (PLNR-14).
    Body {
        both_bands: bool,
    },
    Station,
    Gate,
    Site,
    Ship,
}

/// The tuning panel (PLNR-20).
#[derive(Debug, Clone, PartialEq)]
pub struct TuningModel {
    pub route: String,
    pub place: String,
    /// PLNR-14: the selected band, when there is a choice.
    pub bands: Option<Band>,
    pub depart: Option<SliderModel>,
    pub travel: Option<SliderModel>,
    /// PLNR-41: the gap to a target in the ship's own orbit.
    pub gap: Option<String>,
    pub arrival: Option<Row>,
    pub speed: Option<Row>,
    pub delta_v: Option<DeltaVModel>,
    /// PLNR-29: the presets shown, in order, and whether each is enabled.
    pub presets: Vec<(Preset, bool)>,
    /// PLNR-30: whether Intercept is selected, when the choice is shown.
    pub arrival_choice: Option<bool>,
    /// PLNR-31: whether Schedule is enabled, and why not.
    pub schedule: bool,
    pub reason: String,
}

/// What the tuning panel is built from: names, the request and setting,
/// and what the search and the solver found for them.
#[derive(Debug, Clone, Copy)]
pub struct TuningFacts<'a> {
    pub ship: &'a str,
    /// The ship's place and drive, as PLNR-20 words them.
    pub place: &'a str,
    pub destination: &'a str,
    pub kind: DestKind,
    /// The crossing's primary, for the reasons that name it (PLNR-92).
    pub primary: &'a str,
    pub request: Request,
    pub setting: Option<Setting>,
    pub found: Option<&'a Result<Search, Infeasible>>,
    pub leg: Option<&'a Result<SolvedLeg, Infeasible>>,
    pub travel_track: &'a [Sample],
    pub available_kms: f64,
    pub today: f64,
    /// PLNR-41: the gap to a target in the ship's own orbit, in km, and
    /// whether it is ahead.
    pub gap_km: Option<(f64, bool)>,
}

impl TuningFacts<'_> {
    fn words(&self) -> LegWords<'_> {
        LegWords {
            intercept: matches!(self.request.arrival, Arrival::Intercept { .. }),
            to_ship: self.kind == DestKind::Ship,
            destination: self.destination,
        }
    }

    fn leg(&self) -> Option<&SolvedLeg> {
        self.leg?.as_ref().ok()
    }
}

/// PLNR-40: a phasing leg leaves now and its travel times are meetings.
pub fn is_phasing(sliders: &Sliders) -> bool {
    matches!(sliders.travel, Slider::Positions(_))
}

/// PLNR-20 to PLNR-32, PLNR-40 to PLNR-42: the tuning panel for a
/// request, from its search, its setting and the setting's leg.
pub fn tuning_model(facts: &TuningFacts, tunables: &Tunables) -> TuningModel {
    let search = facts.found.and_then(|found| found.as_ref().ok());
    let bands = match (facts.kind, facts.request.destination) {
        (
            DestKind::Body { both_bands: true },
            crate::sim::transfer::places::Place::Body { band, .. },
        ) => Some(band),
        _ => None,
    };
    let (schedule, reason) = schedule_state(facts);
    let mut model = TuningModel {
        route: crate::view::planner_words::route(facts.ship, facts.destination),
        place: facts.place.to_string(),
        bands,
        depart: None,
        travel: None,
        gap: facts
            .gap_km
            .map(|(km, ahead)| gap_line(km, ahead, tunables.distance_au_threshold_au)),
        arrival: None,
        speed: None,
        delta_v: None,
        presets: Vec::new(),
        arrival_choice: None,
        schedule,
        reason,
    };
    let (Some(search), Some(setting)) = (search, facts.setting) else {
        return model;
    };
    let phasing = is_phasing(&search.sliders);
    model.depart = Some(depart_model(facts, search, setting, tunables));
    model.travel = Some(travel_model(facts, search, setting, tunables));
    if let Some(leg) = facts.leg() {
        model.arrival = Some(arrival_row(facts, leg));
        model.speed = leg.arrival_speed_kms.filter(|_| !phasing).map(|kms| {
            let label = facts.words().speed_label(facts.kind == DestKind::Station);
            Row::new(label, format_speed(kms))
        });
        model.delta_v = Some(delta_v_model(facts, leg));
    }
    model.presets = preset_buttons(search);
    let offers_intercept = matches!(
        facts.kind,
        DestKind::Body { .. } | DestKind::Station | DestKind::Ship
    );
    model.arrival_choice = (offers_intercept && !phasing).then(|| facts.words().intercept);
    model
}

/// PLNR-31 / PLNR-42 / PLNR-92: Schedule is enabled only for a leg that
/// can be flown within the budget; otherwise the reason says why.
fn schedule_state(facts: &TuningFacts) -> (bool, String) {
    let words = |why: Infeasible| reason(why, facts.primary, facts.destination);
    match (facts.found, facts.leg) {
        (Some(Err(why)), _) => (false, words(*why)),
        (_, Some(Err(why))) => (false, words(*why)),
        (_, Some(Ok(leg))) if leg.delta_v_kms > facts.available_kms => (
            false,
            words(Infeasible::DeltaV {
                needed_kms: leg.delta_v_kms,
                available_kms: facts.available_kms,
            }),
        ),
        (_, Some(Ok(_))) => (true, String::new()),
        _ => (false, String::new()),
    }
}

/// PLNR-21 / PLNR-40: the Depart row: the leave day, with a track over
/// the Depart range, or "Departs now" for a phasing leg.
fn depart_model(
    facts: &TuningFacts,
    search: &Search,
    setting: Setting,
    tunables: &Tunables,
) -> SliderModel {
    let wait = facts.leg().map_or(0.0, |leg| leg.departure_wait_days);
    if is_phasing(&search.sliders) {
        return SliderModel {
            row: Row::new(DEPARTS, NOW),
            track: None,
            note: None,
        };
    }
    let slider = &search.sliders.depart;
    let least = search.grid.least_delta_v_kms().unwrap_or(0.0);
    let cheapest = search.grid.cheapest_per_day();
    let samples: Vec<(f64, Option<&Sample>)> = search
        .grid
        .rows
        .iter()
        .zip(&cheapest)
        .filter_map(|(row, best)| Some((row.first()?.setting.depart_day, best.as_ref())))
        .collect();
    SliderModel {
        row: Row::new(
            DEPART,
            leave_readout(setting.depart_day + wait, facts.today),
        ),
        track: matches!(slider, Slider::Range { .. }).then(|| TrackModel {
            thumb: fraction_of(slider, setting.depart_day),
            stops: track_stops(slider, &samples, least, facts.available_kms, tunables),
        }),
        note: None,
    }
}

/// PLNR-22 / PLNR-32 / PLNR-40: the Travel time row: the requested
/// travel time over its track, a phasing leg's meetings with their laps,
/// or a low-thrust drive's fixed time; with the trim line or the fixed
/// line beneath.
fn travel_model(
    facts: &TuningFacts,
    search: &Search,
    setting: Setting,
    tunables: &Tunables,
) -> SliderModel {
    let slider = &search.sliders.travel;
    let leg = facts.leg();
    let readout = match (slider, leg.and_then(|leg| leg.phasing_laps)) {
        (Slider::Positions(_), Some(laps)) => travel_with_laps(setting.travel_days, laps),
        _ => format_duration(setting.travel_days),
    };
    let least = search.grid.least_delta_v_kms().unwrap_or(0.0);
    let samples: Vec<(f64, Option<&Sample>)> = facts
        .travel_track
        .iter()
        .map(|sample| (sample.setting.travel_days, Some(sample)))
        .collect();
    let track = (!matches!(slider, Slider::Fixed(_))).then(|| TrackModel {
        thumb: fraction_of(slider, setting.travel_days),
        stops: track_stops(slider, &samples, least, facts.available_kms, tunables),
    });
    let note = if matches!(slider, Slider::Fixed(_)) {
        Some(Line::new(TRAVEL_FIXED, Tone::Secondary))
    } else {
        leg.and_then(|leg| leg.trim_days)
            .map(|trim| Line::new(trim_line(trim, facts.destination), Tone::Secondary))
    };
    SliderModel {
        row: Row::new(TRAVEL_TIME, readout),
        track,
        note,
    }
}

/// PLNR-25: the day the leg ends: the arrival for a Rendezvous, the
/// crossing's end for an Intercept.
pub fn leg_end_day(leg: &SolvedLeg, intercept: bool) -> f64 {
    if intercept {
        leg.crossing_end_day
    } else {
        leg.arrival_day
    }
}

/// PLNR-25: Arrives, Intercepts or Meeting, with the date and how long
/// until it.
fn arrival_row(facts: &TuningFacts, leg: &SolvedLeg) -> Row {
    let words = facts.words();
    Row::new(
        words.arrival_label(),
        dated(leg_end_day(leg, words.intercept), facts.today),
    )
}

/// PLNR-27: "leg of available", the bar filled to their ratio, and what
/// is left after, all in the warning colour and the bar full when the
/// leg is over budget.
fn delta_v_model(facts: &TuningFacts, leg: &SolvedLeg) -> DeltaVModel {
    let available = facts.available_kms;
    let over = leg.delta_v_kms > available;
    let fill = if over || available <= 0.0 {
        1.0
    } else {
        (leg.delta_v_kms / available) as f32
    };
    let tone = if over { Tone::Warning } else { Tone::Neutral };
    DeltaVModel {
        row: Row {
            tone,
            ..Row::new(DELTA_V, delta_v_readout(leg.delta_v_kms, available))
        },
        fill,
        over,
        line: Line::new(
            facts.words().left_after(leg.delta_v_kms, available),
            if over { Tone::Warning } else { Tone::Secondary },
        ),
    }
}

/// PLNR-29: the presets that exist, in order, without repeats; Fastest
/// shows disabled when nothing is affordable.
fn preset_buttons(search: &Search) -> Vec<(Preset, bool)> {
    let shown = search.presets.shown();
    let mut buttons: Vec<(Preset, bool)> = Vec::new();
    if search.presets.fastest.is_none() {
        buttons.push((Preset::Fastest, false));
    }
    buttons.extend(shown.into_iter().map(|(preset, _)| (preset, true)));
    buttons
}

/// PLNR-12: the line under the prompt while a place is hovered, with
/// the gap for a target in the ship's own orbit (PLNR-41).
pub fn hover_line(
    preview: &HoverPreview,
    words: &LegWords,
    primary: &str,
    gap: Option<String>,
) -> Line {
    let line = match &preview.outcome {
        HoverOutcome::Fastest(leg) => Line::new(
            fastest_line(words, leg_end_day(leg, words.intercept), leg.delta_v_kms),
            Tone::Neutral,
        ),
        HoverOutcome::CantReach { cheapest_kms } => Line::new(
            cant_reach_line(*cheapest_kms, preview.available_kms),
            Tone::Warning,
        ),
        HoverOutcome::Unplannable(why) => {
            Line::new(reason(*why, primary, words.destination), Tone::Warning)
        }
    };
    match gap {
        Some(gap) => Line::new(format!("{} · {gap}", line.text), line.tone),
        None => line,
    }
}

/// PLNR-51: an order as its inset shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct OrderModel {
    pub heading: String,
    pub departs: Row,
    pub arrival: Row,
    pub delta_v: Row,
    /// Scheduled, so it has Cancel and Change.
    pub scheduled: bool,
}

/// PLNR-51: "→ destination", Departs (Departed once the leave day has
/// passed) with the leave day, the arrival row and the leg's delta-v.
pub fn order_model(
    words: &LegWords,
    leg: &crate::sim::transfer::plan::Leg,
    scheduled: bool,
    today: f64,
) -> OrderModel {
    let leave = leg.depart_day + leg.departure_wait_days;
    let departs = if leave > today {
        Row::new(DEPARTS, leave_readout(leave, today))
    } else {
        Row::new(DEPARTED, date(leave))
    };
    let end = if words.intercept {
        leg.crossing_end_day
    } else {
        leg.arrival_day
    };
    OrderModel {
        heading: crate::view::planner_words::order_heading(words.destination),
        departs,
        arrival: Row::new(words.arrival_label(), dated(end, today)),
        delta_v: Row::new(DELTA_V, format_speed(leg.delta_v_kms)),
        scheduled,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::transfer::grid::{Cost, Grid};
    use crate::sim::transfer::order::{Landing, LegBurns};
    use crate::sim::transfer::places::Place;
    use crate::sim::transfer::presets::Presets;
    use bevy::prelude::*;

    fn sample(depart: f64, travel: f64, kms: f64) -> Sample {
        Sample {
            setting: Setting {
                depart_day: depart,
                travel_days: travel,
            },
            result: Ok(Cost {
                delta_v_kms: kms,
                arrival_day: depart + travel,
            }),
        }
    }

    /// A crossing search: depart 10 to 20, travel 50 to 150.
    fn search(phasing: bool) -> Search {
        let rows = (0..3)
            .map(|d| {
                (0..3)
                    .map(|t| {
                        sample(
                            10.0 + 5.0 * d as f64,
                            50.0 + 50.0 * t as f64,
                            4.0 + t as f64,
                        )
                    })
                    .collect()
            })
            .collect();
        let fastest = Setting {
            depart_day: 10.0,
            travel_days: 50.0,
        };
        Search {
            sliders: Sliders {
                depart: if phasing {
                    Slider::Fixed(10.0)
                } else {
                    Slider::Range {
                        start: 10.0,
                        end: 20.0,
                    }
                },
                travel: if phasing {
                    Slider::Positions(vec![50.0, 100.0, 150.0])
                } else {
                    Slider::Range {
                        start: 50.0,
                        end: 150.0,
                    }
                },
                windows: !phasing,
            },
            grid: Grid { rows },
            presets: Presets {
                fastest: Some(fastest),
                cheapest: Some(fastest),
                next_window: None,
            },
        }
    }

    fn leg(kms: f64) -> SolvedLeg {
        let mut world = World::new();
        let primary = world.spawn_empty().id();
        SolvedLeg {
            origin: None,
            primary,
            crossing_start_day: 12.0,
            crossing_end_day: 112.0,
            arrival_day: 113.0,
            travel_days: 100.0,
            trajectory: crate::sim::rail::ShipTrajectory::new(vec![orbits::Segment {
                start_day: 0.0,
                end_day: None,
                kind: crate::sim::rail::SegmentKind::Parked {
                    orbit: primary,
                    primary,
                    path: orbits::CircularOrbit::new(1.0, 1.0, 0.0),
                },
            }])
            .unwrap(),
            delta_v_kms: kms,
            burns: LegBurns::default(),
            landing: Landing::Intercept,
            intercept_speed_kms: None,
            closing_speed_kms: None,
            arrival_speed_kms: Some(3.25),
            turn_rad: None,
            departure_wait_days: 2.0,
            arrival_wait_days: 0.0,
            closest_approach_rad: None,
            trim_days: None,
            target_lock: None,
            phasing_laps: None,
        }
    }

    fn facts<'a>(
        found: &'a Result<Search, Infeasible>,
        leg: &'a Result<SolvedLeg, Infeasible>,
        track: &'a [Sample],
        arrival: Arrival,
    ) -> TuningFacts<'a> {
        let mut world = World::new();
        let body = world.spawn_empty().id();
        TuningFacts {
            ship: "Orison",
            place: "High orbit, Halden IV · hydrogen NTR",
            destination: "Iver",
            kind: DestKind::Body { both_bands: true },
            primary: "Corvane",
            request: Request {
                destination: Place::Body {
                    body,
                    band: Band::High,
                },
                arrival,
            },
            setting: Some(Setting {
                depart_day: 10.0,
                travel_days: 100.0,
            }),
            found: Some(found),
            leg: Some(leg),
            travel_track: track,
            available_kms: 9.9,
            today: 10.0,
            gap_km: None,
        }
    }

    /// PLNR-20 / PLNR-21 / PLNR-25 to PLNR-27 / PLNR-29 / PLNR-30: a
    /// crossing's panel: the route and place, the leave day (after the
    /// wait), the travel time, the arrival row and speed, the delta-v
    /// against the budget, the presets and the arrival choice; Schedule
    /// enabled with no reason.
    #[test]
    fn a_crossing_fills_every_row_of_the_panel() {
        let found = Ok(search(false));
        let leg = Ok(leg(7.5));
        let track: Vec<Sample> = (0..3)
            .map(|t| sample(10.0, 50.0 + 50.0 * t as f64, 5.0))
            .collect();
        let model = tuning_model(
            &facts(&found, &leg, &track, Arrival::Rendezvous),
            &Tunables::default(),
        );
        assert_eq!(model.route, "Orison → Iver");
        assert_eq!(model.bands, Some(Band::High));
        let depart = model.depart.unwrap();
        assert_eq!(depart.row.label, "Depart");
        assert!(
            depart.row.value.contains("(in 2.0 d)"),
            "{}",
            depart.row.value
        );
        assert_eq!(depart.track.as_ref().unwrap().stops.len(), 3);
        assert_eq!(depart.track.unwrap().thumb, 0.0);
        let travel = model.travel.unwrap();
        assert_eq!(travel.row.value, "100.0 d");
        assert_eq!(travel.track.unwrap().thumb, 0.5);
        assert_eq!(model.arrival.unwrap().label, "Arrives");
        assert_eq!(model.speed.unwrap().label, "Arrival speed");
        let delta_v = model.delta_v.unwrap();
        assert_eq!(delta_v.row.value, "7.5 of 9.9 km/s");
        assert!((delta_v.fill - 7.5 / 9.9).abs() < 1e-6 && !delta_v.over);
        assert_eq!(delta_v.line.text, "2.4 km/s left after arrival");
        assert_eq!(model.presets, vec![(Preset::Fastest, true)]);
        assert_eq!(model.arrival_choice, Some(false));
        assert!(model.schedule && model.reason.is_empty());
    }

    /// PLNR-27 / PLNR-31: a leg over budget fills the bar in the warning
    /// colour, says by how much, and disables Schedule with the reason.
    #[test]
    fn a_leg_over_budget_disables_schedule_and_says_why() {
        let found = Ok(search(false));
        let leg = Ok(leg(12.0));
        let model = tuning_model(
            &facts(&found, &leg, &[], Arrival::Intercept { band: Band::High }),
            &Tunables::default(),
        );
        let delta_v = model.delta_v.unwrap();
        assert!(delta_v.over && delta_v.fill == 1.0);
        assert_eq!(delta_v.row.tone, Tone::Warning);
        assert_eq!(delta_v.line.text, "Over budget by 2.1 km/s");
        assert!(!model.schedule);
        assert_eq!(model.reason, "Needs 12.0 km/s, has 9.9");
        assert_eq!(model.arrival.unwrap().label, "Intercepts");
    }

    /// PLNR-42 / PLNR-92: a destination the ship is already at has no
    /// sliders, and Schedule is disabled with "Already there".
    #[test]
    fn already_there_disables_schedule() {
        let found = Err(Infeasible::AlreadyThere);
        let leg = Err(Infeasible::AlreadyThere);
        let model = tuning_model(
            &facts(&found, &leg, &[], Arrival::Rendezvous),
            &Tunables::default(),
        );
        assert!(model.depart.is_none() && model.travel.is_none());
        assert!(!model.schedule);
        assert_eq!(model.reason, "Already there");
    }

    /// PLNR-40: a phasing leg departs now, its travel readout gives the
    /// laps, it has no Rendezvous / Intercept choice and no speed row.
    #[test]
    fn a_phasing_leg_departs_now_and_counts_its_laps() {
        let found = Ok(search(true));
        let leg = Ok(SolvedLeg {
            phasing_laps: Some(3),
            arrival_speed_kms: None,
            ..leg(1.0)
        });
        let model = tuning_model(
            &facts(&found, &leg, &[], Arrival::Rendezvous),
            &Tunables::default(),
        );
        let depart = model.depart.unwrap();
        assert_eq!(
            (depart.row.label.as_str(), depart.row.value.as_str()),
            ("Departs", "now")
        );
        assert!(depart.track.is_none());
        assert_eq!(model.travel.unwrap().row.value, "100.0 d · 3 laps");
        assert_eq!(model.arrival_choice, None);
        assert_eq!(model.speed, None);
    }

    /// PLNR-22 / PLNR-32: a fixed travel time has no track and says why;
    /// a trimmed Intercept says what the trim is for.
    #[test]
    fn fixed_and_trimmed_travel_times_have_their_lines() {
        let mut fixed = search(false);
        fixed.sliders.travel = Slider::Fixed(80.0);
        let found = Ok(fixed);
        let leg_ok = Ok(leg(3.0));
        let model = tuning_model(
            &facts(&found, &leg_ok, &[], Arrival::Rendezvous),
            &Tunables::default(),
        );
        let travel = model.travel.unwrap();
        assert!(travel.track.is_none());
        assert_eq!(travel.note.unwrap().text, TRAVEL_FIXED);

        let found = Ok(search(false));
        let trimmed = Ok(SolvedLeg {
            trim_days: Some(0.125),
            ..leg(3.0)
        });
        let model = tuning_model(
            &facts(
                &found,
                &trimmed,
                &[],
                Arrival::Intercept { band: Band::Low },
            ),
            &Tunables::default(),
        );
        assert_eq!(
            model.travel.unwrap().note.unwrap().text,
            "+ 3.0 h to meet Iver"
        );
    }

    /// PLNR-29: with nothing affordable Fastest shows disabled and the
    /// rest follow it.
    #[test]
    fn fastest_is_disabled_when_nothing_is_affordable() {
        let mut found = search(false);
        found.presets.fastest = None;
        assert_eq!(
            preset_buttons(&found),
            vec![(Preset::Fastest, false), (Preset::Cheapest, true)]
        );
    }
}
