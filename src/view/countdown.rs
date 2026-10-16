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
use crate::sim::transfer::places::Place;
use crate::sim::transfer::plan::Plan;

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
        let destination = leg
            .destination
            .entity()
            .and_then(|e| self.names.get(e).ok())
            .map(|n| n.0.as_str())
            .unwrap_or_default();
        let words = LegWords {
            intercept: leg.arrival.is_intercept(),
            to_ship: matches!(leg.destination, Place::Ship { .. }),
            destination,
        };
        Some(order_countdown(
            &words,
            leg.leave_day(),
            leg.end_day(),
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
}
