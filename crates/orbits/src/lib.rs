//! Orbital mechanics in one plane, in kilometres and days (ARCH-40).
//!
//! This crate answers questions like: where is a planet on day 300? If a
//! ship leaves here now and must be there in 90 days, how fast must it
//! go, and what will that cost? What path does a weak engine trace
//! between two orbits? It knows nothing about any game: just a primary (a
//! star or planet, given by its gravitational parameter μ in km³/day²),
//! positions in km, velocities in km/day, and the day count.
//!
//! Conventions: everything lies in the XZ plane (y is zero), angles are
//! measured from +x towards +z, and the sense in which the angle grows is
//! **prograde**. Speeds quoted in km/s and gravitational parameters quoted
//! in km³/s² come in through [`km_per_s_to_km_per_day`] and
//! [`km3_per_s2_to_km3_per_day2`], and results go out through
//! [`km_per_day_to_km_per_s`].
//!
//! # A transfer, end to end
//!
//! A ship parked round planet A wants to reach planet B, 170 days from
//! now. Both planets go round the star on circular orbits. The plan is:
//! leave A's parking orbit (escape cost), coast along the arc that joins
//! A's position today to B's position in 170 days (a Lambert arc, with a
//! burn at each end), then settle into a parking orbit round B (capture
//! cost). The whole trip becomes a trajectory: parked, coasting, parked.
//!
//! ```
//! use glam::DVec3;
//! use orbits::{
//!     CircularOrbit, Conic, Path, Rail, Segment, Sense, State, Trajectory, escape_cost,
//!     km3_per_s2_to_km3_per_day2, km_per_day_to_km_per_s, lambert,
//! };
//!
//! // The star, and the two planets on circular orbits round it.
//! let star_mu = km3_per_s2_to_km3_per_day2(1.0e11);
//! let planet_a = CircularOrbit::new(star_mu, 1.0e8, 0.0);
//! let planet_b = CircularOrbit::new(star_mu, 1.6e8, 0.85);
//!
//! // Each planet's own gravity, and the parking orbit 7,000 km out.
//! let planet_mu = km3_per_s2_to_km3_per_day2(3.0e5);
//! let parking_km = 7_000.0;
//!
//! // The crossing: from A's position today to B's position in 170 days.
//! let depart_day = 0.0;
//! let travel_days = 170.0;
//! let arrive_day = depart_day + travel_days;
//! let a_now = planet_a.state_at(depart_day);
//! let b_then = planet_b.state_at(arrive_day);
//! let arc = lambert(star_mu, a_now.position, b_then.position, travel_days, Sense::Prograde)
//!     .expect("the planets are not on one ray from the star");
//!
//! // The burns: change A's velocity to the arc's, then the arc's to B's.
//! let burn_1 = (arc.departure_velocity - a_now.velocity).length();
//! let burn_2 = (b_then.velocity - arc.arrival_velocity).length();
//! let total = escape_cost(planet_mu, parking_km) + burn_1 + burn_2 + escape_cost(planet_mu, parking_km);
//! let total_kms = km_per_day_to_km_per_s(total);
//! assert!(total_kms > 10.0 && total_kms < 15.0, "{total_kms} km/s");
//!
//! // The coasting path as a conic round the star.
//! let coast = Conic::from_state(star_mu, State::new(a_now.position, arc.departure_velocity), depart_day, 1e-6)
//!     .unwrap();
//!
//! // The whole trip, read by day. Parked segments here are the planets'
//! // own orbits: a parked ship moves with its planet.
//! let trip = Trajectory::new(vec![
//!     Segment { start_day: f64::NEG_INFINITY, end_day: Some(depart_day), kind: Path::Circular(planet_a) },
//!     Segment { start_day: depart_day, end_day: Some(arrive_day), kind: Path::Conic(coast) },
//!     Segment { start_day: arrive_day, end_day: None, kind: Path::Circular(planet_b) },
//! ])
//! .unwrap();
//!
//! // Halfway, the ship is between the two orbits; on arrival it is at B.
//! let halfway = trip.state_at(85.0).unwrap();
//! assert!(halfway.radius() > 1.0e8 && halfway.radius() < 1.6e8);
//! let arrival = trip.state_at(arrive_day - 1e-9).unwrap();
//! assert!((arrival.position - b_then.position).length() < 1.0);
//! assert_eq!(trip.state_at(500.0).unwrap().position, planet_b.position_at(500.0));
//! ```

#![warn(missing_docs)]

mod circular;
mod conic;
mod lambert;
mod spiral;
mod state;
mod trajectory;
mod units;

pub use circular::{
    CircularOrbit, Hohmann, circular_period, circular_radius_for_period, circular_speed,
    escape_cost, hill_radius, hohmann, synodic_period,
};
pub use conic::Conic;
pub use lambert::{Lambert, lambert};
pub use spiral::Spiral;
pub use state::{Sense, State, angle_of, point_in_plane, wrap_angle};
pub use trajectory::{Path, Rail, Segment, Trajectory, TrajectoryError};
pub use units::{
    SECONDS_PER_DAY, km_per_day_to_km_per_s, km_per_s_to_km_per_day, km3_per_s2_to_km3_per_day2,
};
