//! The two ends of a leg (BAND-02, BAND-05, XFER-01, XFER-10): what a
//! destination stands for, where the ship sets out from, and how each end
//! is read round the crossing's primary on a day. Pure functions over the
//! system snapshot (`tree.rs`, ARCH-20).
//!
//! A band, a gate's or a site's orbit is a whole ring: any point of it
//! will do. A station or a parked ship is one point of a ring, at its own
//! phase, so a leg aimed at it is aimed at where it is.

use std::f64::consts::{PI, TAU};

use bevy::prelude::*;
use orbits::{CircularOrbit, Conic, Rail, State};

use super::order::{Departure, Infeasible, TargetLock};
use super::places::{Place, approach_days, departure_cost_kms};
use super::tree::{OrbitNode, ShipState, SystemTree};
use crate::sim::body::BodyClass;

/// What a destination comes to (BAND-02).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Target {
    /// A band, or a gate's or site's docking orbit: the whole ring.
    Ring { orbit: Entity, node: OrbitNode },
    /// A station, or a parked ship (`lock`), at its own phase in a named
    /// orbit (BAND-05, XFER-10).
    Point {
        orbit: Entity,
        node: OrbitNode,
        phase0_rad: f64,
        lock: Option<TargetLock>,
    },
    /// A coasting ship's conic round `primary`.
    Coasting { primary: Entity, conic: Conic },
}

impl Target {
    /// The named orbit the target is in, if it is in one.
    pub(super) fn orbit(&self) -> Option<(Entity, OrbitNode)> {
        match *self {
            Target::Ring { orbit, node } | Target::Point { orbit, node, .. } => Some((orbit, node)),
            Target::Coasting { .. } => None,
        }
    }
}

/// BAND-02 / XFER-10: the ring, point or conic a place stands for.
pub(super) fn resolve(tree: &SystemTree, place: Place) -> Result<Target, Infeasible> {
    let node = |orbit: Entity| {
        tree.orbits
            .get(&orbit)
            .copied()
            .ok_or(Infeasible::UnknownPlace)
    };
    let orbit = match place {
        Place::Body { body, band } => {
            if !tree.bodies.contains_key(&body) {
                return Err(Infeasible::UnknownPlace);
            }
            tree.band_orbit(body, band).ok_or(Infeasible::NoHighBand)?
        }
        Place::Station { station } => {
            let at = tree
                .stations
                .get(&station)
                .ok_or(Infeasible::UnknownPlace)?;
            return Ok(Target::Point {
                orbit: at.orbit,
                node: node(at.orbit)?,
                phase0_rad: at.phase0_rad,
                lock: None,
            });
        }
        Place::Gate { gate: place } | Place::Site { site: place } => {
            tree.docking_orbit(place).ok_or(Infeasible::UnknownPlace)?
        }
        Place::Ship { ship } => {
            return match *tree.ships.get(&ship).ok_or(Infeasible::UnknownPlace)? {
                ShipState::Parked { orbit, phase0_rad } => Ok(Target::Point {
                    orbit,
                    node: node(orbit)?,
                    phase0_rad,
                    lock: Some(TargetLock {
                        ship,
                        orbit,
                        phase0_rad,
                    }),
                }),
                ShipState::Coasting { primary, conic } => Ok(Target::Coasting { primary, conic }),
                ShipState::InFlight => Err(Infeasible::TargetInFlight),
            };
        }
        Place::Hold => return Err(Infeasible::HoldIsNotACrossing),
    };
    Ok(Target::Ring {
        orbit,
        node: node(orbit)?,
    })
}

/// PLAN-12: the place a named orbit stands for.
fn place_of_orbit(node: &OrbitNode, tree: &SystemTree) -> Option<Place> {
    if let Some(band) = node.band {
        return Some(Place::Body {
            body: node.body,
            band,
        });
    }
    let owner = node.docking_for?;
    match tree.bodies.get(&owner)?.class {
        BodyClass::Gate => Some(Place::Gate { gate: owner }),
        BodyClass::SalvageSite => Some(Place::Site { site: owner }),
        _ => None,
    }
}

/// PLAN-12 / BAND-02: the place a ship parked in `orbit` at `phase0_rad`
/// sets out from: the station it is docked at, if it is together with one
/// (PHAS-02), and otherwise the place the orbit stands for.
pub(super) fn place_of_parked(
    orbit: Entity,
    phase0_rad: f64,
    tree: &SystemTree,
    together_km: f64,
) -> Option<Place> {
    if let Some(station) = tree.station_at(orbit, phase0_rad, together_km) {
        return Some(Place::Station { station });
    }
    place_of_orbit(tree.orbits.get(&orbit)?, tree)
}

/// One end of a crossing: the thing in the system tree it is at (a body
/// for a band, a gate or site for a docking orbit), the body whose
/// gravity charges for it, the orbit it sits in and how to read its own
/// state on a day.
#[derive(Clone, Copy)]
pub(super) struct End {
    /// The node whose place in the tree the end takes (XFER-01).
    pub(super) at: Entity,
    /// The body the end's orbit goes round, which charges for leaving and
    /// arriving (BAND-03, BAND-04).
    pub(super) host: Entity,
    /// The radius of the orbit round `host` the end sits in, if any.
    pub(super) radius_km: Option<f64>,
    pub(super) own: Own,
}

/// How an end's own state is read, when it sits at the primary itself.
#[derive(Clone, Copy)]
pub(super) enum Own {
    /// A point of a circular orbit: a parked ship, a station.
    Circular(CircularOrbit),
    /// A coasting ship.
    Conic(Conic),
    /// A whole ring of the primary, whose angle is free until the start is
    /// known.
    Ring(OrbitNode),
}

impl End {
    /// XFER-01: the end's state round `primary` on `day`: its ancestor
    /// directly below the primary, or its own state if it sits at the
    /// primary itself. A ring of the primary is aimed half a turn ahead
    /// of `start`.
    pub(super) fn state(
        &self,
        tree: &SystemTree,
        primary: Entity,
        day: f64,
        start: Option<&State>,
    ) -> Option<State> {
        if let Some(child) = tree.child_below(primary, self.at) {
            return Some(tree.bodies.get(&child)?.orbit?.state_at(day));
        }
        if self.at != primary {
            return None;
        }
        Some(match self.own {
            Own::Circular(orbit) => orbit.state_at(day),
            Own::Conic(conic) => conic.state_at(day),
            Own::Ring(node) => {
                let angle = start.map_or(0.0, |s| s.angle()) + PI;
                node.path(phase_for_angle(&node, angle, day)).state_at(day)
            }
        })
    }

    /// BAND-03 / BAND-04: what this end charges, in km/s and days; nothing
    /// at the primary itself or without an orbit of a massive body.
    pub(super) fn charges(&self, tree: &SystemTree, primary: Entity) -> (f64, f64) {
        let (Some(radius_km), Some(node)) = (self.radius_km, tree.bodies.get(&self.host)) else {
            return (0.0, 0.0);
        };
        if self.host == primary {
            return (0.0, 0.0);
        }
        (
            departure_cost_kms(node.mu, radius_km),
            approach_days(node.mu, radius_km, node.hill_km),
        )
    }
}

/// The start phase that puts a circular orbit at `angle` on `day`.
pub(super) fn phase_for_angle(node: &OrbitNode, angle: f64, day: f64) -> f64 {
    if node.period_days.is_finite() && node.period_days != 0.0 {
        (angle - TAU * day / node.period_days).rem_euclid(TAU)
    } else {
        angle
    }
}

/// The departure end of a leg, and the place the ship sets out from.
pub(super) fn departure_end(
    tree: &SystemTree,
    departure: &Departure,
    together_km: f64,
) -> Result<(End, Option<Place>), Infeasible> {
    match *departure {
        Departure::Parked { orbit, phase0_rad } => {
            let node = *tree.orbits.get(&orbit).ok_or(Infeasible::UnknownPlace)?;
            Ok((
                End {
                    at: node.docking_for.unwrap_or(node.body),
                    host: node.body,
                    radius_km: Some(node.radius_km),
                    own: Own::Circular(node.path(phase0_rad)),
                },
                place_of_parked(orbit, phase0_rad, tree, together_km),
            ))
        }
        Departure::Coasting { primary, conic } => Ok((
            End {
                at: primary,
                host: primary,
                radius_km: None,
                own: Own::Conic(conic),
            },
            None,
        )),
    }
}

/// The arrival end of a leg: a ring, a point at its own phase, or a
/// coasting ship's conic.
pub(super) fn target_end(target: &Target) -> End {
    match *target {
        Target::Ring { node, .. } => End {
            at: node.docking_for.unwrap_or(node.body),
            host: node.body,
            radius_km: Some(node.radius_km),
            own: Own::Ring(node),
        },
        Target::Point {
            node, phase0_rad, ..
        } => End {
            at: node.docking_for.unwrap_or(node.body),
            host: node.body,
            radius_km: Some(node.radius_km),
            own: Own::Circular(node.path(phase0_rad)),
        },
        Target::Coasting { primary, conic } => End {
            at: primary,
            host: primary,
            radius_km: None,
            own: Own::Conic(conic),
        },
    }
}
