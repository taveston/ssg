//! Which paths, ghosts and labels the map shows, and how strongly
//! (PLNR-62 to PLNR-64), as plain functions of an order's state and
//! what is selected and hovered.

use crate::sim::transfer::plan::Status;

/// How strongly a path is drawn (PLNR-62).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Emphasis {
    /// At `path_width_px` and full opacity: planned, hovered or selected.
    Full,
    /// At `traffic_path_width_px` and `traffic_path_alpha`: a ship in
    /// flight that is neither hovered nor selected.
    Traffic,
}

/// What decides how a ship's order shows on the map.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrderFacts {
    /// The faction commands the ship (SYM-01).
    pub commanded: bool,
    pub status: Status,
    /// The ship is coasting after an Intercept (BAND-07, PLNR-54).
    pub coasting: bool,
    /// The ship's leave day (PLNR-21) has come.
    pub left: bool,
    pub selected: bool,
    pub hovered: bool,
}

/// How an order's path shows: its emphasis, and whether its arrival ghost
/// shows with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrderLook {
    pub emphasis: Emphasis,
    pub arrival_ghost: bool,
}

/// PLNR-62 / PLNR-64: a Scheduled order shows only while its ship is
/// selected, as its path and arrival ghost. From the leave day until the
/// leg ends, and while the ship coasts after an Intercept, the path always
/// shows: faint as traffic, full while the ship is hovered or selected,
/// with the arrival ghost while it is selected. A ship the faction doesn't
/// command shows no path.
pub fn order_look(facts: OrderFacts) -> Option<OrderLook> {
    if !facts.commanded {
        return None;
    }
    let active = matches!(facts.status, Status::Scheduled | Status::InFlight);
    let under_way = facts.coasting || (facts.status == Status::InFlight && facts.left);
    if under_way {
        let full = facts.selected || facts.hovered;
        return Some(OrderLook {
            emphasis: if full {
                Emphasis::Full
            } else {
                Emphasis::Traffic
            },
            arrival_ghost: facts.selected,
        });
    }
    (active && facts.selected).then_some(OrderLook {
        emphasis: Emphasis::Full,
        arrival_ghost: true,
    })
}

/// PLNR-64: a ghost's opacity: `ghost_alpha` times the opacity of its
/// object's dot, so it is gone while the object is an exposure ring.
pub fn ghost_opacity(ghost_alpha: f32, dot_alpha: f32) -> f32 {
    ghost_alpha * dot_alpha
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(status: Status) -> OrderFacts {
        OrderFacts {
            commanded: true,
            status,
            coasting: false,
            left: false,
            selected: false,
            hovered: false,
        }
    }

    /// PLNR-62: a Scheduled order shows nothing until its ship is
    /// selected, then its full path and arrival ghost; so does an order
    /// whose ship hasn't left yet.
    #[test]
    fn a_scheduled_order_shows_only_while_selected() {
        assert_eq!(order_look(facts(Status::Scheduled)), None);
        let selected = OrderFacts {
            selected: true,
            ..facts(Status::Scheduled)
        };
        let shown = Some(OrderLook {
            emphasis: Emphasis::Full,
            arrival_ghost: true,
        });
        assert_eq!(order_look(selected), shown);
        let waiting = OrderFacts {
            selected: true,
            ..facts(Status::InFlight)
        };
        assert_eq!(order_look(waiting), shown);
        assert_eq!(order_look(facts(Status::InFlight)), None);
    }

    /// PLNR-62: once the ship has left, its path always shows: faint as
    /// traffic, full while hovered or selected, with the arrival ghost
    /// only while selected. A coasting ship's path shows the same way.
    #[test]
    fn a_ship_under_way_always_shows_its_path() {
        let flying = OrderFacts {
            left: true,
            ..facts(Status::InFlight)
        };
        assert_eq!(
            order_look(flying),
            Some(OrderLook {
                emphasis: Emphasis::Traffic,
                arrival_ghost: false
            })
        );
        let hovered = OrderFacts {
            hovered: true,
            ..flying
        };
        assert_eq!(
            order_look(hovered),
            Some(OrderLook {
                emphasis: Emphasis::Full,
                arrival_ghost: false
            })
        );
        let selected = OrderFacts {
            selected: true,
            ..flying
        };
        assert_eq!(
            order_look(selected),
            Some(OrderLook {
                emphasis: Emphasis::Full,
                arrival_ghost: true
            })
        );
        let coasting = OrderFacts {
            coasting: true,
            ..facts(Status::Done)
        };
        assert_eq!(order_look(coasting).unwrap().emphasis, Emphasis::Traffic);
    }

    /// PLNR-62: a finished or failed order shows nothing, and a ship the
    /// faction doesn't command shows no path at all.
    #[test]
    fn finished_orders_and_foreign_ships_show_no_path() {
        for status in [Status::Done, Status::Failed, Status::Holding] {
            let selected = OrderFacts {
                selected: true,
                left: true,
                ..facts(status)
            };
            assert_eq!(order_look(selected), None, "{status:?}");
        }
        let foreign = OrderFacts {
            commanded: false,
            left: true,
            selected: true,
            ..facts(Status::InFlight)
        };
        assert_eq!(order_look(foreign), None);
    }

    /// PLNR-64: a ghost fades with its object's dot, and is gone in
    /// exposure-ring mode.
    #[test]
    fn a_ghost_fades_with_its_objects_dot() {
        assert_eq!(ghost_opacity(0.5, 1.0), 0.5);
        assert_eq!(ghost_opacity(0.5, 0.5), 0.25);
        assert_eq!(ghost_opacity(0.5, 0.0), 0.0);
    }
}
