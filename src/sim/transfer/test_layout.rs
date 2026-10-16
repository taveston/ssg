//! The reference layout the transfer rules are tested against (XFER-12):
//! a star of 0.75 solar masses, a capital of radius 5,753 km and 0.82 g
//! at 0.8 AU and an inner planet at 0.5 AU, each with its bands; a
//! station in the capital's Low band and one in the inner planet's High
//! band; a gate round the star; and the two drives of TEST-20. Shared by
//! the tests of `legs.rs`, `phasing.rs` and their neighbours.

use bevy::prelude::*;
use orbits::{CircularOrbit, circular_period, synodic_period};

use super::legs::solve_leg;
use super::order::{Arrival, Departure, Infeasible, Order, SolvedLeg};
use super::places::{Band, Place, approach_days, band_radii, departure_cost_kms, hill_radius_km};
use super::tree::{BodyNode, OrbitNode, ShipState, StationNode, SystemTree};
use crate::sim::body::BodyClass;
use crate::sim::ship::Drive;
use crate::sim::tunables::SimTunables;
use crate::sim::units::{
    EARTH_MASS_KG, GRAVITATIONAL_CONSTANT, KM_PER_AU, M_PER_KM, SOLAR_MASS_KG,
    STANDARD_GRAVITY_M_PER_S2, gravitational_parameter_km3_per_day2,
};

/// The capital's station's phase in its Low band.
pub const STATION_PHASE: f64 = 0.5;
/// The inner planet's station's phase in its High band.
pub const FAR_STATION_PHASE: f64 = 1.3;

pub struct Layout {
    /// Hands out entity ids, so ships added by a test never clash with the
    /// layout's own.
    world: World,
    pub tree: SystemTree,
    pub tunables: SimTunables,
    pub star: Entity,
    pub inner: Entity,
    pub capital: Entity,
    /// In the capital's Low band.
    pub station: Entity,
    /// In the inner planet's High band.
    pub far_station: Entity,
    pub gate: Entity,
    pub ntr: Drive,
    pub nep: Drive,
}

pub fn capital_mass_kg() -> f64 {
    let radius_m = 5_753.0 * M_PER_KM;
    0.82 * STANDARD_GRAVITY_M_PER_S2 * radius_m * radius_m / GRAVITATIONAL_CONSTANT
}

/// A body record, with its Hill radius from its orbit and parent mass.
fn body_node(
    parent: Option<Entity>,
    class: BodyClass,
    mass_kg: f64,
    radius_km: f64,
    orbit: Option<CircularOrbit>,
    parent_mass: f64,
) -> BodyNode {
    BodyNode {
        parent,
        class,
        mass_kg,
        mu: gravitational_parameter_km3_per_day2(mass_kg),
        radius_km,
        orbit,
        hill_km: orbit.and_then(|o| hill_radius_km(o.radius_km, mass_kg, parent_mass)),
    }
}

pub fn layout() -> Layout {
    let mut world = World::new();
    let [star, inner, capital, station, far_station, gate, gate_orbit] =
        [(); 7].map(|_| world.spawn_empty().id());
    // The reference layout fixes the High band at ten radii (XFER-12,
    // PHAS-13), whatever the game's default.
    let tunables = SimTunables {
        high_band_radii: 10.0,
        ..SimTunables::default()
    };
    let star_mass = 0.75 * SOLAR_MASS_KG;
    let star_mu = gravitational_parameter_km3_per_day2(star_mass);
    let mut tree = SystemTree::default();
    let planets = [
        (inner, 0.3 * EARTH_MASS_KG, 3_000.0, 0.5, 0.0),
        (capital, capital_mass_kg(), 5_753.0, 0.8, 1.0),
    ];
    tree.bodies.insert(
        star,
        body_node(None, BodyClass::Star, star_mass, 500_000.0, None, 0.0),
    );
    for (planet, mass, radius, au, phase) in planets {
        let orbit = CircularOrbit::new(star_mu, au * KM_PER_AU, phase);
        let node = body_node(
            Some(star),
            BodyClass::Planet,
            mass,
            radius,
            Some(orbit),
            star_mass,
        );
        tree.bodies.insert(planet, node);
    }
    let gate_path = CircularOrbit::new(star_mu, 2.0 * KM_PER_AU, 2.0);
    tree.bodies.insert(
        gate,
        body_node(
            Some(star),
            BodyClass::Gate,
            0.0,
            5.0,
            Some(gate_path),
            star_mass,
        ),
    );
    for (body, radius_km, mass) in [
        (star, 500_000.0, star_mass),
        (inner, 3_000.0, 0.3 * EARTH_MASS_KG),
        (capital, 5_753.0, capital_mass_kg()),
    ] {
        let radii = band_radii(radius_km, tree.bodies[&body].hill_km, &tunables);
        for band in [Band::Low, Band::High] {
            let Some(r) = radii.radius_km(band) else {
                continue;
            };
            let node = OrbitNode {
                body,
                radius_km: r,
                period_days: circular_period(gravitational_parameter_km3_per_day2(mass), r),
                band: Some(band),
                docking_for: None,
            };
            tree.orbits.insert(world.spawn_empty().id(), node);
        }
    }
    // SCL-07: each station is a place in a band, at its own phase.
    for (entity, body, band, phase0_rad) in [
        (station, capital, Band::Low, STATION_PHASE),
        (far_station, inner, Band::High, FAR_STATION_PHASE),
    ] {
        let orbit = tree.band_orbit(body, band).unwrap();
        let path = tree.orbits[&orbit].path(phase0_rad);
        let mass = tree.bodies[&body].mass_kg;
        let node = body_node(Some(body), BodyClass::Station, 0.0, 1.0, Some(path), mass);
        tree.bodies.insert(entity, node);
        tree.stations
            .insert(entity, StationNode { orbit, phase0_rad });
    }
    tree.orbits.insert(
        gate_orbit,
        OrbitNode {
            body: star,
            radius_km: 2.0 * KM_PER_AU,
            period_days: gate_path.period_days,
            band: None,
            docking_for: Some(gate),
        },
    );
    Layout {
        world,
        tree,
        tunables,
        star,
        inner,
        capital,
        station,
        far_station,
        gate,
        ntr: Drive::new("hydrogen NTR", true, 9.0, 0.2 * STANDARD_GRAVITY_M_PER_S2),
        nep: Drive::new(
            "NEP ion/Hall",
            false,
            35.0,
            0.048e-3 * STANDARD_GRAVITY_M_PER_S2,
        ),
    }
}

impl Layout {
    /// A fresh entity id.
    pub fn spawn(&mut self) -> Entity {
        self.world.spawn_empty().id()
    }

    /// A ship in the snapshot, in state `state`.
    pub fn add_ship(&mut self, state: ShipState) -> Entity {
        let ship = self.spawn();
        self.tree.ships.insert(ship, state);
        ship
    }

    pub fn band(&self, body: Entity, band: Band) -> Entity {
        self.tree.band_orbit(body, band).expect("the band exists")
    }

    pub fn node(&self, body: Entity, band: Band) -> OrbitNode {
        self.tree.orbits[&self.band(body, band)]
    }

    /// Parked in `body`'s `band` at phase 0.2.
    pub fn parked(&self, body: Entity, band: Band) -> Departure {
        self.parked_at(body, band, 0.2)
    }

    pub fn parked_at(&self, body: Entity, band: Band, phase0_rad: f64) -> Departure {
        Departure::Parked {
            orbit: self.band(body, band),
            phase0_rad,
        }
    }

    pub fn solve(
        &self,
        from: &Departure,
        order: &Order,
        drive: &Drive,
    ) -> Result<SolvedLeg, Infeasible> {
        solve_leg(&self.tree, &self.tunables, from, order, drive, 100.0)
    }

    /// The cheapest leg for `order` over one synodic period of departure
    /// days, a day at a time.
    pub fn cheapest(&self, from: &Departure, order: Order, drive: &Drive) -> (Order, SolvedLeg) {
        let inner = self.tree.bodies[&self.inner].orbit.unwrap();
        let capital = self.tree.bodies[&self.capital].orbit.unwrap();
        let synodic = synodic_period(inner.period_days, capital.period_days);
        let mut best: Option<(Order, SolvedLeg)> = None;
        let mut depart = 0.0;
        while depart < synodic {
            let order = Order {
                depart_day: depart,
                ..order
            };
            if let Ok(leg) = self.solve(from, &order, drive)
                && best
                    .as_ref()
                    .is_none_or(|(_, b)| leg.delta_v_kms < b.delta_v_kms)
            {
                best = Some((order, leg));
            }
            depart += 1.0;
        }
        best.expect("some departure day works")
    }

    /// What leaving or arriving at `body`'s `band` costs (km/s) and takes
    /// (days).
    pub fn charges(&self, body: Entity, band: Band) -> (f64, f64) {
        let node = self.tree.bodies[&body];
        let r = self.node(body, band).radius_km;
        (
            departure_cost_kms(node.mu, r),
            approach_days(node.mu, r, node.hill_km),
        )
    }
}

pub fn rendezvous_with(body: Entity, band: Band, travel_days: f64) -> Order {
    Order {
        destination: Place::Body { body, band },
        arrival: Arrival::Rendezvous,
        depart_day: 0.0,
        travel_days,
    }
}

pub fn within(actual: f64, expected: f64, fraction: f64, what: &str) {
    assert!(
        ((actual - expected) / expected).abs() < fraction,
        "{what}: expected about {expected}, got {actual}"
    );
}
