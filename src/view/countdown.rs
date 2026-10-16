//! The countdown of a ship's order, the last part of its label's second
//! line (PLNR-66, SYM-13): "departs in 12 d" until the ship leaves its
//! orbit, then how long until its leg ends, in the words of the leg's
//! arrival. Shared, so the markers' labels read it without importing the
//! planner (ARCH-03). The words are `planner_words.rs`'s.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::planner_words::LegWords;
use crate::sim::body::DisplayName;
use crate::sim::clock::GameClock;
use crate::sim::transfer::order::Arrival;
use crate::sim::transfer::places::Place;
use crate::sim::transfer::plan::{Leg, Plan};

/// The object a place stands for: the body of a band, or the station,
/// gate, site or ship itself; `None` for Hold.
pub fn place_entity(place: Place) -> Option<Entity> {
    match place {
        Place::Body { body, .. } => Some(body),
        Place::Station { station } => Some(station),
        Place::Gate { gate } => Some(gate),
        Place::Site { site } => Some(site),
        Place::Ship { ship } => Some(ship),
        Place::Hold => None,
    }
}

/// PLNR-21: the day the ship leaves its orbit: the depart day plus the
/// wait for the exit point.
pub fn leave_day(leg: &Leg) -> f64 {
    leg.depart_day + leg.departure_wait_days
}

/// PLNR-25 / PLNR-66: the day a leg ends: the crossing's end for an
/// Intercept, the arrival for a Rendezvous.
pub fn leg_end_day(leg: &Leg) -> f64 {
    if matches!(leg.arrival, Arrival::Intercept { .. }) {
        leg.crossing_end_day
    } else {
        leg.arrival_day
    }
}

/// PLNR-66: the countdown on `today`: to the leave day until it comes,
/// then to the end of the leg. It counts down as the clock runs.
pub fn order_countdown(words: &LegWords, leave_day: f64, end_day: f64, today: f64) -> String {
    let left = today >= leave_day;
    let to = if left { end_day } else { leave_day };
    words.countdown(left, to - today)
}

/// What a ship's countdown is read from.
#[derive(SystemParam)]
pub struct Countdowns<'w, 's> {
    plans: Query<'w, 's, &'static Plan>,
    names: Query<'w, 's, &'static DisplayName>,
    clock: Res<'w, GameClock>,
}

impl Countdowns<'_, '_> {
    /// PLNR-66: the countdown of `ship`'s order while it is Scheduled or
    /// InFlight; `None` for a ship with no such order.
    pub fn of(&self, ship: Entity) -> Option<String> {
        let plan = self.plans.get(ship).ok()?;
        if !plan.is_active() {
            return None;
        }
        let leg = plan.leg();
        let destination = place_entity(leg.destination)
            .and_then(|e| self.names.get(e).ok())
            .map(|n| n.0.as_str())
            .unwrap_or_default();
        let words = LegWords {
            intercept: matches!(leg.arrival, Arrival::Intercept { .. }),
            to_ship: matches!(leg.destination, Place::Ship { .. }),
            destination,
        };
        Some(order_countdown(
            &words,
            leave_day(leg),
            leg_end_day(leg),
            self.clock.elapsed_days(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(intercept: bool, to_ship: bool) -> LegWords<'static> {
        LegWords {
            intercept,
            to_ship,
            destination: "Iver",
        }
    }

    /// PLNR-66: "departs in N" until the leave day, counting down as the
    /// clock runs; then the leg's own words, counting down to its end.
    #[test]
    fn the_countdown_runs_to_the_leave_day_then_to_the_end() {
        let rendezvous = words(false, false);
        assert_eq!(
            order_countdown(&rendezvous, 12.0, 100.0, 0.0),
            "departs in 12.0 d"
        );
        assert_eq!(
            order_countdown(&rendezvous, 12.0, 100.0, 7.0),
            "departs in 5.0 d"
        );
        assert_eq!(
            order_countdown(&rendezvous, 12.0, 100.0, 12.0),
            "arrives in 88.0 d"
        );
        assert_eq!(
            order_countdown(&words(true, false), 12.0, 100.0, 81.0),
            "intercepts Iver in 19.0 d"
        );
        assert_eq!(
            order_countdown(&words(false, true), 0.0, 3.0, 1.0),
            "rendezvous in 2.0 d"
        );
        assert_eq!(
            order_countdown(&words(true, true), 0.0, 3.0, 1.0),
            "intercept in 2.0 d"
        );
    }

    /// PLNR-66: every place but Hold stands for an object.
    #[test]
    fn every_place_but_hold_names_an_object() {
        let mut world = World::new();
        let e = world.spawn_empty().id();
        assert_eq!(place_entity(Place::Station { station: e }), Some(e));
        assert_eq!(place_entity(Place::Ship { ship: e }), Some(e));
        assert_eq!(place_entity(Place::Hold), None);
    }
}
