//! Every number the transfer spec marks *tunable*, in one resource
//! (ARCH-22), separate from the view's `Tunables`.
//!
//! The tunables tables in `docs/spec/transfers.md` and
//! `docs/spec/planner.md` list them with the spec IDs that use them. They
//! live here, documented field by field, so a band altitude or a loss
//! distance is changed in one place.

use bevy::prelude::*;

#[derive(Resource, Debug, Clone)]
pub struct SimTunables {
    /// A body's Low band sits this far above its mean radius (BAND-01).
    pub low_band_altitude_km: f64,
    /// A body's High band sits at this many mean radii, unless its Hill
    /// sphere caps it lower (BAND-01).
    pub high_band_radii: f64,
    /// The High band never exceeds this fraction of the body's Hill radius
    /// (BAND-01).
    pub high_band_hill_fraction: f64,
    /// A coasting ship whose orbit reaches past this many AU from the star
    /// is escaping, and is removed once it is that far out (BAND-08).
    pub lost_beyond_au: f64,
    /// A crossing, or the coast after an Intercept, whose path dips
    /// inside this many mean radii of its primary is infeasible. One that
    /// starts or ends round the primary itself has its Low band as the
    /// limit instead (XFER-07).
    pub min_periapsis_radii: f64,
    /// A conic's eccentricity is kept at least this far from 1, so there
    /// is never a parabolic path (RAIL-05).
    pub parabola_epsilon: f64,
    /// Two things in one named orbit closer than this, in km, are
    /// together: a ship this close to a station is docked at it (PHAS-02).
    pub together_km: f64,
    /// A phasing leg tries meetings up to this many passes of the burn
    /// point before it gives up (PHAS-08).
    pub max_phasing_laps: u32,
    /// An Intercept trimmed to meet a station or ship is solved again at
    /// most this many times before it counts as unable to line up
    /// (PHAS-15).
    pub trim_max_iterations: u32,
    /// A leg whose windows come round more than once a day looks for its
    /// cheapest moment to leave at this many evenly spaced starts within
    /// one window interval (PHAS-16).
    pub window_wait_samples: usize,
    /// The planner's Depart range runs this many days from Soonest when no
    /// pair of orbit periods sets it (PLNR-80).
    pub depart_range_days: f64,
    /// When two orbit periods set the Depart range, it runs this fraction
    /// past one window-to-window time, so the next window is always on
    /// the slider (PLNR-80).
    pub depart_margin_fraction: f64,
    /// The Travel range starts at this multiple of the least-fuel travel
    /// time between the two orbits (PLNR-81).
    pub travel_min_factor: f64,
    /// And ends at this multiple of it (PLNR-81).
    pub travel_max_factor: f64,
    /// The planner's grid and its Depart track have at most this many
    /// depart columns: Soonest and the Depart positions, thinned to every
    /// nth position when there are more (PLNR-82).
    pub track_depart_samples: usize,
    /// Travel times sampled across the Travel range, ends included, for
    /// the grid and the Travel time track (PLNR-82, PLNR-28).
    pub track_travel_samples: usize,
    /// Settings within this many km/s of the cheapest count as equally
    /// cheap, and the earliest arrival among them wins (PLNR-84).
    pub preset_tie_kms: f64,
    /// A depart day whose cheapest trip is within this many km/s of the
    /// cheapest of all is in a window (PLNR-85).
    pub window_margin_kms: f64,
    /// A planner slider has at most this many steps along its range; the
    /// step is the smallest round duration that keeps within it (PLNR-23,
    /// PLNR-86).
    pub slider_max_steps: u32,
}

impl Default for SimTunables {
    fn default() -> Self {
        Self {
            low_band_altitude_km: 500.0,
            high_band_radii: 34.0,
            high_band_hill_fraction: 0.5,
            lost_beyond_au: 6.0,
            min_periapsis_radii: 2.0,
            parabola_epsilon: 1e-6,
            together_km: 10.0,
            max_phasing_laps: 50,
            trim_max_iterations: 8,
            window_wait_samples: 24,
            depart_range_days: 180.0,
            depart_margin_fraction: 0.1,
            travel_min_factor: 0.3,
            travel_max_factor: 2.0,
            track_depart_samples: 60,
            track_travel_samples: 36,
            preset_tie_kms: 0.05,
            window_margin_kms: 0.5,
            slider_max_steps: 400,
        }
    }
}
