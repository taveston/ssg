//! Every line of text the transfer planner shows, as plain functions of
//! the search's results (PLNR-90 to PLNR-94, PLNR-04): labels, readouts,
//! reasons, tooltips and the map's labels. The panel and the map marks
//! call these and never write words of their own, so the planner speaks
//! one plain language: Depart, Travel time, Arrive, Delta-v and Window,
//! never the orbital mechanic's terms (PLNR-90).
//!
//! Figures go through the shared readout module (UNIT-01 to UNIT-04) and
//! dates through the top bar's calendar (CLK-02), as PLNR-93 asks.

use crate::sim::transfer::order::Infeasible;
use crate::sim::transfer::places::{Band, Place};
use crate::sim::transfer::plan::Leg;
use crate::view::date::{date_from_elapsed_days, format_date};
use crate::view::readout::{format_distance, format_duration, format_speed, format_speed_figure};

/// The panel's heading, in small spaced capitals (PLNR-20).
pub const HEADING: &str = "Transfer";
/// The inset's button that opens the planner (PLNR-01).
pub const TRANSFER: &str = HEADING;
/// The prompt while choosing a destination (PLNR-10).
pub const CHOOSE_PROMPT: &str =
    "Choose a destination: click a place on the map or in the list below";
/// The Depart slider's label (PLNR-21).
pub const DEPART: &str = "Depart";
/// The leave row of a phasing leg and of an order (PLNR-40, PLNR-51).
pub const DEPARTS: &str = "Departs";
/// An order's leave row once its leave day has passed (PLNR-51).
pub const DEPARTED: &str = "Departed";
/// The leave readout when the ship leaves today (PLNR-21, PLNR-40).
pub const NOW: &str = "now";
/// The Travel time slider's label (PLNR-22).
pub const TRAVEL_TIME: &str = "Travel time";
/// The line under a low-thrust drive's fixed travel time (PLNR-22).
pub const TRAVEL_FIXED: &str = "Travel time is fixed by this drive";
/// The Delta-v row's label (PLNR-27) and a key fact (PLNR-05).
pub const DELTA_V: &str = "Delta-v";
/// The drive's key fact (PLNR-05).
pub const DRIVE: &str = "Drive";
/// The presets (PLNR-29).
pub const FASTEST: &str = "Fastest";
pub const CHEAPEST: &str = "Cheapest";
pub const NEXT_WINDOW: &str = "Next window";
/// The arrival modes (PLNR-30).
pub const RENDEZVOUS: &str = "Rendezvous";
pub const INTERCEPT: &str = "Intercept";
/// The band choice (PLNR-14).
pub const HIGH: &str = "High";
pub const LOW: &str = "Low";
/// The panel's buttons (PLNR-02, PLNR-31, PLNR-51).
pub const SCHEDULE: &str = "Schedule";
pub const CANCEL: &str = "Cancel";
pub const CHANGE: &str = "Change";
/// The tooltip on the Delta-v label (PLNR-91).
pub const DELTA_V_TOOLTIP: &str = "Delta-v is how much a ship can change its speed before its tanks are dry. Every trip spends some.";
/// The tooltip on Rendezvous (PLNR-94).
pub const RENDEZVOUS_TOOLTIP: &str =
    "Match the target's orbit and stay with it. This costs the most delta-v.";
/// The tooltip on Intercept (PLNR-94).
pub const INTERCEPT_TOOLTIP: &str = "Fly through at speed without slowing down. This saves delta-v, and the ship keeps going afterwards.";
/// The line when an order departs while it is being changed (PLNR-53).
pub const DEPARTED_WHILE_CHANGING: &str = "Departed while you were changing it";
/// The line when an order fails while it is being changed (PLNR-53).
pub const ORDER_FAILED: &str = "The order failed";

/// A leave day this close to now, in days, reads as now: it is under the
/// half minute a duration would round away (UNIT-02).
const NOW_DAYS: f64 = 0.5 / 1440.0;

/// PLNR-93 / CLK-02: a day as the top bar shows it: "07 Mar 2341".
pub fn date(day: f64) -> String {
    format_date(&date_from_elapsed_days(day))
}

/// PLNR-21 / PLNR-25: a day with how long until it: "07 Mar 2341 (in
/// 3.0 d)".
pub fn dated(day: f64, today: f64) -> String {
    format!(
        "{} (in {})",
        date(day),
        format_duration((day - today).max(0.0))
    )
}

/// PLNR-21: the leave readout: "now" when the ship leaves today, else its
/// date and how long until it.
pub fn leave_readout(leave_day: f64, today: f64) -> String {
    if leave_day - today < NOW_DAYS {
        NOW.to_string()
    } else {
        dated(leave_day, today)
    }
}

/// What a leg is, as far as its words go: whether it is an Intercept,
/// whether its destination is a ship, and the destination's name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LegWords<'a> {
    pub intercept: bool,
    pub to_ship: bool,
    pub destination: &'a str,
}

impl<'a> LegWords<'a> {
    /// The words of an ordered leg, whose destination goes by
    /// `destination`.
    pub fn of_leg(leg: &Leg, destination: &'a str) -> Self {
        Self {
            intercept: leg.arrival.is_intercept(),
            to_ship: matches!(leg.destination, Place::Ship { .. }),
            destination,
        }
    }
}

impl LegWords<'_> {
    /// PLNR-25: the arrival row's label: Meeting for a ship, else
    /// Intercepts or Arrives. A station reads as a body does.
    pub fn arrival_label(&self) -> &'static str {
        match (self.to_ship, self.intercept) {
            (true, _) => "Meeting",
            (false, true) => "Intercepts",
            (false, false) => "Arrives",
        }
    }

    /// PLNR-25: the arrival row's verb, as the lines that refer to it use
    /// it: "meeting", "intercepts" or "arrives".
    pub fn arrival_verb(&self) -> String {
        self.arrival_label().to_lowercase()
    }

    /// PLNR-26: the arrival speed row's label: Closing speed at a ship,
    /// or at a station the leg intercepts; Arrival speed otherwise.
    /// `at_station` says the destination is a station.
    pub fn speed_label(&self, at_station: bool) -> &'static str {
        if self.to_ship || (at_station && self.intercept) {
            "Closing speed"
        } else {
            "Arrival speed"
        }
    }

    /// PLNR-66: the countdown on the ship's label: "departs in N" until
    /// the leave day, then "arrives in N" for a Rendezvous, "intercepts
    /// *place* in N" for an Intercept at a body or station, "rendezvous
    /// in N" or "intercept in N" at a ship. `days` is to the leave day,
    /// then to the leg's end.
    pub fn countdown(&self, left: bool, days: f64) -> String {
        let n = format_duration(days.max(0.0));
        if !left {
            return format!("departs in {n}");
        }
        match (self.to_ship, self.intercept) {
            (false, false) => format!("arrives in {n}"),
            (false, true) => format!("intercepts {} in {n}", self.destination),
            (true, false) => format!("rendezvous in {n}"),
            (true, true) => format!("intercept in {n}"),
        }
    }

    /// PLNR-65: the arrival ghost's label: "arrive *date*", "intercept
    /// *date* · *speed*" (with " · turn *N*°" after a pass through a Low
    /// band), or "meeting *date* · *speed* closing" at a ship.
    pub fn ghost_label(&self, day: f64, speed_kms: Option<f64>, turn_rad: Option<f64>) -> String {
        let speed = speed_kms.map(format_speed);
        match (self.to_ship, self.intercept, speed) {
            (true, _, Some(speed)) => format!("meeting {} · {speed} closing", date(day)),
            (true, _, None) => format!("meeting {}", date(day)),
            (false, true, speed) => {
                let mut label = format!("intercept {}", date(day));
                if let Some(speed) = speed {
                    label.push_str(&format!(" · {speed}"));
                }
                if let Some(turn) = turn_rad {
                    label.push_str(&format!(" · turn {:.0}°", turn.to_degrees()));
                }
                label
            }
            (false, false, _) => format!("arrive {}", date(day)),
        }
    }

    /// PLNR-27: the line under the Delta-v bar: what is left after
    /// arrival or after the intercept, or how far over budget the leg is.
    pub fn left_after(&self, leg_kms: f64, available_kms: f64) -> String {
        if leg_kms > available_kms {
            return format!("Over budget by {}", format_speed(leg_kms - available_kms));
        }
        let after = if self.intercept {
            "after the intercept"
        } else {
            "after arrival"
        };
        format!("{} left {after}", format_speed(available_kms - leg_kms))
    }
}

/// PLNR-12: the hover line for a place's Fastest setting: "Fastest:
/// arrives 07 Mar 2341, 3.2 km/s".
pub fn fastest_line(leg: &LegWords, day: f64, delta_v_kms: f64) -> String {
    format!(
        "{FASTEST}: {} {}, {}",
        leg.arrival_verb(),
        date(day),
        format_speed(delta_v_kms)
    )
}

/// PLNR-12: the hover line when nothing is affordable.
pub fn cant_reach_line(cheapest_kms: f64, available_kms: f64) -> String {
    format!(
        "Can't reach: the cheapest trip needs {}, has {}",
        format_speed(cheapest_kms),
        format_speed_figure(available_kms)
    )
}

/// PLNR-20: the route: "Orison → Iver".
pub fn route(ship: &str, destination: &str) -> String {
    format!("{ship} → {destination}")
}

/// PLNR-20: a ship's place in a band: "High orbit, Halden IV".
pub fn band_place(band: Band, body: &str) -> String {
    format!("{} orbit, {body}", band.name())
}

/// PLNR-20: a ship's place at a station, gate or site: "At Arden
/// Highport".
pub fn at_place(name: &str) -> String {
    format!("At {name}")
}

/// PLNR-20: the ship's place and its drive: "High orbit, Halden IV ·
/// hydrogen NTR".
pub fn place_and_drive(place: &str, drive: &str) -> String {
    format!("{place} · {drive}")
}

/// PLNR-40: a phasing leg's travel readout: "4.9 d · 3 laps".
pub fn travel_with_laps(travel_days: f64, laps: u32) -> String {
    let unit = if laps == 1 { "lap" } else { "laps" };
    format!("{} · {laps} {unit}", format_duration(travel_days))
}

/// PLNR-32: the trim line: "+ 3.2 h to meet Arden Highport".
pub fn trim_line(trim_days: f64, name: &str) -> String {
    format!("+ {} to meet {name}", format_duration(trim_days))
}

/// PLNR-27: the Delta-v readout: "3.2 of 9.9 km/s".
pub fn delta_v_readout(leg_kms: f64, available_kms: f64) -> String {
    format!(
        "{} of {}",
        format_speed_figure(leg_kms),
        format_speed(available_kms)
    )
}

/// PLNR-05: the Delta-v key fact: the ship's delta-v less what it has
/// spent, with " · N reserved" while some is reserved.
pub fn delta_v_fact(remaining_kms: f64, reserved_kms: f64) -> String {
    if reserved_kms > 0.0 {
        format!(
            "{} · {} reserved",
            format_speed(remaining_kms),
            format_speed_figure(reserved_kms)
        )
    } else {
        format_speed(remaining_kms)
    }
}

/// PLNR-41: the gap to a target in the ship's own orbit: "Gap 12,000 km
/// ahead".
pub fn gap_line(distance_km: f64, ahead: bool, au_threshold_au: f64) -> String {
    let side = if ahead { "ahead" } else { "behind" };
    format!(
        "Gap {} {side}",
        format_distance(distance_km, au_threshold_au)
    )
}

/// PLNR-51: an order's heading: "→ Iver".
pub fn order_heading(destination: &str) -> String {
    format!("→ {destination}")
}

/// PLNR-54: a coasting ship's place: "Coasting · intercepted Iver, from
/// Halden IV".
pub fn coasting_place(destination: &str, origin: &str) -> String {
    format!("Coasting · intercepted {destination}, from {origin}")
}

/// PLNR-67: the label at the coast's event, `days` from today: "then
/// coasting · crosses Iver's orbit in 41.0 d", or "then coasting · leaves
/// the system in 2.1 y" when `body` is `None`.
pub fn coast_label(body: Option<&str>, days: f64) -> String {
    let n = format_duration(days.max(0.0));
    match body {
        Some(body) => format!("then coasting · crosses {body}'s orbit in {n}"),
        None => format!("then coasting · leaves the system in {n}"),
    }
}

/// PLNR-92: the plain sentence for each reason the solver gives,
/// naming the crossing's `primary` and the `target` where it needs to.
pub fn reason(reason: Infeasible, primary: &str, target: &str) -> String {
    match reason {
        Infeasible::DeltaV {
            needed_kms,
            available_kms,
        } => format!(
            "Needs {}, has {}",
            format_speed(needed_kms),
            format_speed_figure(available_kms)
        ),
        Infeasible::Periapsis { .. } => format!("Path passes too close to {primary}"),
        Infeasible::CoastPeriapsis { .. } => {
            format!("The coast after the intercept goes too close to {primary}")
        }
        Infeasible::NoPhasingOrbit => "Can't close the gap in that time".to_string(),
        Infeasible::PhasingNeedsImpulsive => {
            "This drive can't close a gap in its orbit".to_string()
        }
        Infeasible::ShipPrimary => "Can't match that ship's path from here".to_string(),
        Infeasible::CantLineUp => format!("Can't line up with {target}"),
        Infeasible::AlreadyThere => "Already there".to_string(),
        Infeasible::UnknownPlace => format!("{target} can't be found"),
        Infeasible::NoHighBand => format!("{target} has no High orbit"),
        Infeasible::HoldIsNotACrossing => "Holding is not a trip".to_string(),
        Infeasible::NoPath => "No path gets there in that time".to_string(),
        Infeasible::LowThrustNeedsCircularEnds => {
            "This drive can't match a coasting path".to_string()
        }
        Infeasible::TargetInFlight => format!("{target} is under way, so it can't be met"),
        Infeasible::ShipInFlight => "The ship is under way".to_string(),
        Infeasible::Busy => "The ship already has an order".to_string(),
        Infeasible::NoDrive => "The ship has no drive".to_string(),
        Infeasible::MultiLeg => "Only one leg can be ordered".to_string(),
        Infeasible::PhasingIsRendezvous => {
            format!("{target} is in this ship's orbit, so it can only rendezvous")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The words PLNR-90 keeps out of the planner.
    const FORBIDDEN: [&str; 8] = [
        "hohmann",
        "synodic",
        "lambert",
        "phase angle",
        "apoapsis",
        "periapsis",
        "eccentricity",
        "inclination",
    ];

    /// Every reason the solver gives.
    fn every_reason() -> Vec<Infeasible> {
        vec![
            Infeasible::UnknownPlace,
            Infeasible::NoHighBand,
            Infeasible::HoldIsNotACrossing,
            Infeasible::NoPath,
            Infeasible::LowThrustNeedsCircularEnds,
            Infeasible::ShipPrimary,
            Infeasible::TargetInFlight,
            Infeasible::ShipInFlight,
            Infeasible::Periapsis { periapsis_km: 1.0 },
            Infeasible::CoastPeriapsis { periapsis_km: 1.0 },
            Infeasible::DeltaV {
                needed_kms: 12.34,
                available_kms: 9.9,
            },
            Infeasible::Busy,
            Infeasible::NoDrive,
            Infeasible::MultiLeg,
            Infeasible::AlreadyThere,
            Infeasible::NoPhasingOrbit,
            Infeasible::PhasingNeedsImpulsive,
            Infeasible::PhasingIsRendezvous,
            Infeasible::CantLineUp,
        ]
    }

    fn legs() -> [LegWords<'static>; 4] {
        [false, true]
            .map(|to_ship| {
                [false, true].map(|intercept| LegWords {
                    intercept,
                    to_ship,
                    destination: "Iver",
                })
            })
            .concat()
            .try_into()
            .unwrap()
    }

    /// PLNR-90: no line the planner shows uses the orbital mechanic's
    /// words: every constant, every reason and every built line is
    /// checked.
    #[test]
    fn the_planner_never_uses_the_mechanics_words() {
        let mut lines: Vec<String> = [
            HEADING,
            CHOOSE_PROMPT,
            DEPART,
            DEPARTS,
            DEPARTED,
            NOW,
            TRAVEL_TIME,
            TRAVEL_FIXED,
            DELTA_V,
            DRIVE,
            FASTEST,
            CHEAPEST,
            NEXT_WINDOW,
            RENDEZVOUS,
            INTERCEPT,
            HIGH,
            LOW,
            SCHEDULE,
            CANCEL,
            CHANGE,
            DELTA_V_TOOLTIP,
            RENDEZVOUS_TOOLTIP,
            INTERCEPT_TOOLTIP,
            DEPARTED_WHILE_CHANGING,
            ORDER_FAILED,
        ]
        .map(String::from)
        .to_vec();
        lines.extend(
            every_reason()
                .into_iter()
                .map(|r| reason(r, "Halden IV", "Iver")),
        );
        for leg in legs() {
            lines.push(leg.arrival_label().to_string());
            lines.push(leg.speed_label(true).to_string());
            lines.push(leg.countdown(false, 3.0));
            lines.push(leg.countdown(true, 3.0));
            lines.push(leg.ghost_label(40.0, Some(4.2), Some(1.0)));
            lines.push(leg.left_after(3.0, 9.9));
            lines.push(leg.left_after(12.0, 9.9));
            lines.push(fastest_line(&leg, 40.0, 3.2));
        }
        lines.extend([
            cant_reach_line(12.0, 9.9),
            route("Orison", "Iver"),
            band_place(Band::High, "Halden IV"),
            at_place("Arden Highport"),
            place_and_drive("High orbit, Halden IV", "hydrogen NTR"),
            travel_with_laps(4.9, 3),
            trim_line(0.2, "Arden Highport"),
            delta_v_readout(3.2, 9.9),
            delta_v_fact(9.9, 3.2),
            gap_line(12_000.0, true, 0.1),
            order_heading("Iver"),
            coasting_place("Iver", "Halden IV"),
            coast_label(Some("Iver"), 41.0),
            coast_label(None, 800.0),
            leave_readout(12.0, 10.0),
        ]);
        for line in &lines {
            let lower = line.to_lowercase();
            for word in FORBIDDEN {
                assert!(!lower.contains(word), "{line:?} says {word}");
            }
        }
    }

    /// PLNR-92: each reason the spec words reads as it says, naming the
    /// primary or the target; none reads as a code.
    #[test]
    fn every_reason_reads_as_a_plain_sentence() {
        let say = |r| reason(r, "Halden IV", "Arden Highport");
        assert_eq!(
            say(Infeasible::DeltaV {
                needed_kms: 12.34,
                available_kms: 9.9
            }),
            "Needs 12.3 km/s, has 9.9"
        );
        assert_eq!(
            say(Infeasible::Periapsis { periapsis_km: 1.0 }),
            "Path passes too close to Halden IV"
        );
        assert_eq!(
            say(Infeasible::CoastPeriapsis { periapsis_km: 1.0 }),
            "The coast after the intercept goes too close to Halden IV"
        );
        assert_eq!(
            say(Infeasible::NoPhasingOrbit),
            "Can't close the gap in that time"
        );
        assert_eq!(
            say(Infeasible::PhasingNeedsImpulsive),
            "This drive can't close a gap in its orbit"
        );
        assert_eq!(
            say(Infeasible::ShipPrimary),
            "Can't match that ship's path from here"
        );
        assert_eq!(
            say(Infeasible::CantLineUp),
            "Can't line up with Arden Highport"
        );
        assert_eq!(say(Infeasible::AlreadyThere), "Already there");
        for r in every_reason() {
            let sentence = say(r);
            assert!(
                sentence.chars().next().unwrap().is_uppercase(),
                "{sentence}"
            );
            assert!(!sentence.contains('{') && !sentence.contains("Infeasible"));
            assert!(!sentence.contains('_'), "{sentence}");
        }
    }

    /// PLNR-93: dates read as the top bar's, durations as UNIT-02, speeds
    /// as UNIT-04 and distances as UNIT-01; a leave day of today is now.
    #[test]
    fn figures_read_as_everywhere_else() {
        assert_eq!(date(0.0), "14 Mar 2341");
        assert_eq!(dated(3.0, 0.5), "17 Mar 2341 (in 2.5 d)");
        assert_eq!(leave_readout(10.0, 10.0), NOW);
        assert_eq!(leave_readout(10.25, 10.0), "24 Mar 2341 (in 6.0 h)");
        assert_eq!(delta_v_readout(3.24, 9.9), "3.2 of 9.9 km/s");
        assert_eq!(delta_v_readout(0.29, 9.9), "0.29 of 9.9 km/s");
        assert_eq!(gap_line(12_000.0, true, 0.1), "Gap 12,000 km ahead");
        assert_eq!(gap_line(12_000.4, false, 0.1), "Gap 12,000 km behind");
        assert_eq!(travel_with_laps(4.86, 3), "4.9 d · 3 laps");
        assert_eq!(travel_with_laps(0.97, 1), "23.3 h · 1 lap");
        assert_eq!(
            trim_line(0.1, "Arden Highport"),
            "+ 2.4 h to meet Arden Highport"
        );
        assert_eq!(
            cant_reach_line(12.0, 9.9),
            "Can't reach: the cheapest trip needs 12.0 km/s, has 9.9"
        );
        assert_eq!(
            place_and_drive(&band_place(Band::High, "Halden IV"), "hydrogen NTR"),
            "High orbit, Halden IV · hydrogen NTR"
        );
        assert_eq!(at_place("Arden Highport"), "At Arden Highport");
        assert_eq!(route("Orison", "Iver"), "Orison → Iver");
        assert_eq!(delta_v_fact(9.9, 0.0), "9.9 km/s");
        assert_eq!(delta_v_fact(9.9, 3.24), "9.9 km/s · 3.2 reserved");
    }

    /// PLNR-25 / PLNR-26 / PLNR-27 / PLNR-65 / PLNR-66: the words that
    /// change with the leg: Arrives, Intercepts or Meeting; the speed
    /// label; what is left; the ghost's label and the countdown.
    #[test]
    fn the_words_follow_the_kind_of_leg() {
        let [body, body_pass, ship, ship_pass] = legs();
        assert_eq!(body.arrival_label(), "Arrives");
        assert_eq!(body_pass.arrival_label(), "Intercepts");
        assert_eq!(ship.arrival_label(), "Meeting");
        assert_eq!(ship_pass.arrival_verb(), "meeting");
        assert_eq!(body.speed_label(true), "Arrival speed");
        assert_eq!(body_pass.speed_label(true), "Closing speed");
        assert_eq!(body_pass.speed_label(false), "Arrival speed");
        assert_eq!(ship.speed_label(false), "Closing speed");
        assert_eq!(body.left_after(3.2, 9.9), "6.7 km/s left after arrival");
        assert_eq!(
            body_pass.left_after(3.2, 9.9),
            "6.7 km/s left after the intercept"
        );
        assert_eq!(body.left_after(12.0, 9.9), "Over budget by 2.1 km/s");
        assert_eq!(body.ghost_label(0.0, None, None), "arrive 14 Mar 2341");
        assert_eq!(
            body_pass.ghost_label(0.0, Some(4.2), Some(78_f64.to_radians())),
            "intercept 14 Mar 2341 · 4.2 km/s · turn 78°"
        );
        assert_eq!(
            ship.ghost_label(0.0, Some(1.3), None),
            "meeting 14 Mar 2341 · 1.3 km/s closing"
        );
        assert_eq!(body.countdown(false, 3.0), "departs in 3.0 d");
        assert_eq!(body.countdown(true, 3.0), "arrives in 3.0 d");
        assert_eq!(body_pass.countdown(true, 3.0), "intercepts Iver in 3.0 d");
        assert_eq!(ship.countdown(true, 3.0), "rendezvous in 3.0 d");
        assert_eq!(ship_pass.countdown(true, 3.0), "intercept in 3.0 d");
        assert_eq!(
            fastest_line(&body, 0.0, 3.2),
            "Fastest: arrives 14 Mar 2341, 3.2 km/s"
        );
        assert_eq!(
            coast_label(Some("Iver"), 41.0),
            "then coasting · crosses Iver's orbit in 41.0 d"
        );
        assert_eq!(
            coast_label(None, 800.0),
            "then coasting · leaves the system in 2.2 y"
        );
    }
}
