//! What a click picks while the planner is open (PLNR-11): a body's band,
//! a station, a gate, a site or a ship, and what can't be picked at all.
//! The rules are plain functions of what the faction knows of the thing
//! (ARCH-20); `Pickables` gathers those facts from the world.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::sim::body::BodyClass;
use crate::sim::group::ShipGroup;
use crate::sim::named_orbit::{DockingOrbitOf, NamedOrbit};
use crate::sim::ship::InFlight;
use crate::sim::track::{ShipRecord, Track};
use crate::sim::transfer::places::{Band, BandRadii, Place};
use crate::sim::transfer::plan::{Plan, Status};
use crate::view::planner_state::PickTarget;

/// What PLNR-11 reads of a body to decide what a click on it picks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BodyFacts {
    pub class: BodyClass,
    /// A star, planet or moon with a High band (BAND-01).
    pub has_high_band: bool,
    pub group: bool,
    /// The faction's track on a ship or station is firm (SYM-08).
    pub firm_track: bool,
    pub commanded: bool,
    /// On a Conic or Spiral segment (RAIL-09), coasting or under orders.
    pub in_flight: bool,
    /// Its plan is Scheduled, InFlight or Holding (PLAN-02).
    pub active_plan: bool,
}

/// PLNR-11: whether a ship can be picked: not a group, not one whose
/// track is weak or stale, not one in flight the faction doesn't command
/// (its true path isn't known), and not one under orders (its meeting
/// point could change). A parked ship, or one coasting with no plan, can.
pub fn ship_can_be_picked(facts: &BodyFacts) -> bool {
    !facts.group && facts.firm_track && !(facts.in_flight && !facts.commanded) && !facts.active_plan
}

/// PLNR-11: the place a click on a body's marker, disc, pill, outline
/// entry or lone ring picks: a star, planet or moon's High band (its Low
/// band if it has no High); the station, gate or site itself; a ship
/// that can be picked. `None` for anything else.
pub fn body_place(body: Entity, facts: &BodyFacts) -> Option<Place> {
    match facts.class {
        BodyClass::Star | BodyClass::Planet | BodyClass::Moon => Some(Place::Body {
            body,
            band: if facts.has_high_band {
                Band::High
            } else {
                Band::Low
            },
        }),
        BodyClass::Station => Some(Place::Station { station: body }),
        BodyClass::Gate => Some(Place::Gate { gate: body }),
        BodyClass::SalvageSite => Some(Place::Site { site: body }),
        BodyClass::Ship => ship_can_be_picked(facts).then_some(Place::Ship { ship: body }),
    }
}

/// PLNR-11: the place a click on a shared orbit's ring or outline row
/// picks: that body's band, or the gate or site the orbit is the docking
/// orbit of. A named orbit that is neither can't be picked.
pub fn orbit_place(parent: Entity, band: Option<Band>, docking_of: Option<Place>) -> Option<Place> {
    match band {
        Some(band) => Some(Place::Body { body: parent, band }),
        None => docking_of,
    }
}

/// PLAN-02: a plan that keeps its ship from taking another order.
pub fn is_busy(status: Status) -> bool {
    matches!(
        status,
        Status::Scheduled | Status::InFlight | Status::Holding
    )
}

/// What PLNR-11 reads of a body.
type PickedBody = (
    Entity,
    &'static BodyClass,
    Option<&'static BandRadii>,
    Has<ShipGroup>,
    Option<&'static ShipRecord>,
    Has<InFlight>,
    Option<&'static Plan>,
);

/// Everything PLNR-11 reads in the world.
#[derive(SystemParam)]
pub struct Pickables<'w, 's> {
    bodies: Query<'w, 's, PickedBody>,
    orbits: Query<
        'w,
        's,
        (
            Entity,
            &'static NamedOrbit,
            Option<&'static Band>,
            Option<&'static DockingOrbitOf>,
        ),
    >,
}

impl Pickables<'_, '_> {
    /// What PLNR-11 needs to know of a body.
    pub fn facts(&self, body: Entity) -> Option<BodyFacts> {
        let (_, class, radii, group, record, in_flight, plan) = self.bodies.get(body).ok()?;
        Some(BodyFacts {
            class: *class,
            has_high_band: radii.is_some_and(|r| r.high_km.is_some()),
            group,
            firm_track: record.is_none_or(|r| r.track == Track::Firm),
            commanded: record.is_some_and(|r| r.commanded),
            in_flight,
            active_plan: plan.is_some_and(|p| is_busy(p.status)),
        })
    }

    /// PLNR-11: the place `target` picks for a planner planning `ship`,
    /// or `None` when it can't be picked. The ship being planned can't
    /// pick itself.
    pub fn place(&self, target: PickTarget, ship: Entity) -> Option<Place> {
        match target {
            PickTarget::Body(body) if body == ship => None,
            PickTarget::Body(body) => body_place(body, &self.facts(body)?),
            PickTarget::Orbit(orbit) => {
                let (_, named, band, docking) = self.orbits.get(orbit).ok()?;
                let docking = docking.and_then(|d| body_place(d.0, &self.facts(d.0)?));
                orbit_place(named.parent, band.copied(), docking)
            }
        }
    }

    /// PLNR-11: every body and named orbit a planner for `ship` can't
    /// pick, so they take no hover mark.
    pub fn unpickable(&self, ship: Entity) -> impl Iterator<Item = Entity> + '_ {
        let bodies = self
            .bodies
            .iter()
            .map(|(entity, ..)| entity)
            .filter(move |&body| self.place(PickTarget::Body(body), ship).is_none());
        let orbits = self
            .orbits
            .iter()
            .map(|(entity, ..)| entity)
            .filter(move |&orbit| self.place(PickTarget::Orbit(orbit), ship).is_none());
        bodies.chain(orbits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ship() -> BodyFacts {
        BodyFacts {
            class: BodyClass::Ship,
            has_high_band: false,
            group: false,
            firm_track: true,
            commanded: true,
            in_flight: false,
            active_plan: false,
        }
    }

    /// PLNR-11: a planet's marker picks its High band, a body without one
    /// its Low band; stations, gates and sites pick themselves.
    #[test]
    fn a_click_on_a_body_picks_its_high_band_or_the_place_itself() {
        let mut world = World::new();
        let body = world.spawn_empty().id();
        let planet = BodyFacts {
            class: BodyClass::Planet,
            has_high_band: true,
            ..ship()
        };
        assert_eq!(
            body_place(body, &planet),
            Some(Place::Body {
                body,
                band: Band::High
            })
        );
        let small = BodyFacts {
            has_high_band: false,
            class: BodyClass::Moon,
            ..planet
        };
        assert_eq!(
            body_place(body, &small),
            Some(Place::Body {
                body,
                band: Band::Low
            })
        );
        for (class, place) in [
            (BodyClass::Station, Place::Station { station: body }),
            (BodyClass::Gate, Place::Gate { gate: body }),
            (BodyClass::SalvageSite, Place::Site { site: body }),
        ] {
            assert_eq!(
                body_place(body, &BodyFacts { class, ..ship() }),
                Some(place)
            );
        }
    }

    /// PLNR-11: a ship parked or coasting with no plan can be picked; a
    /// group, a weak or stale track, a foreign ship in flight and a ship
    /// under orders can't.
    #[test]
    fn only_ships_with_a_known_steady_path_can_be_picked() {
        assert!(ship_can_be_picked(&ship()));
        let coasting = BodyFacts {
            in_flight: true,
            ..ship()
        };
        assert!(ship_can_be_picked(&coasting));
        for cant in [
            BodyFacts {
                group: true,
                ..ship()
            },
            BodyFacts {
                firm_track: false,
                ..ship()
            },
            BodyFacts {
                commanded: false,
                ..coasting
            },
            BodyFacts {
                active_plan: true,
                ..ship()
            },
        ] {
            assert!(!ship_can_be_picked(&cant), "{cant:?}");
            let mut world = World::new();
            let body = world.spawn_empty().id();
            assert_eq!(body_place(body, &cant), None);
        }
        assert!(is_busy(Status::Holding) && !is_busy(Status::Done));
    }

    /// PLNR-11 / PLNR-14: a shared ring or orbit row picks that body's
    /// band of that orbit, or the place it is the docking orbit of; any
    /// other named orbit can't be picked.
    #[test]
    fn a_click_on_an_orbit_picks_its_band() {
        let mut world = World::new();
        let [parent, gate] = [(); 2].map(|_| world.spawn_empty().id());
        assert_eq!(
            orbit_place(parent, Some(Band::Low), None),
            Some(Place::Body {
                body: parent,
                band: Band::Low
            })
        );
        let docking = Some(Place::Gate { gate });
        assert_eq!(orbit_place(parent, None, docking), docking);
        assert_eq!(orbit_place(parent, None, None), None);
    }
}
