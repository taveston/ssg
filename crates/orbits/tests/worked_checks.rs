//! The worked checks of the transfer spec (XFER-12, ARCH-44), in plain
//! figures: a K-type star of 0.75 solar masses; a capital planet of
//! radius 5,753 km and surface gravity 0.82 g at 0.8 AU; an inner planet
//! at 0.5 AU; a belt at 0.64 AU. Every figure must hold within 3%.

use orbits::{
    CircularOrbit, Rail, Sense, circular_period, escape_cost, hohmann, km_per_day_to_km_per_s,
    km3_per_s2_to_km3_per_day2, lambert, synodic_period,
};

const KM_PER_AU: f64 = 149_597_870.7;
/// The Sun's gravitational parameter, km³/s².
const SUN_MU_KM3_S2: f64 = 1.327_124_4e11;
/// Standard gravity, km/s².
const G0_KM_S2: f64 = 9.806_65e-3;

fn star_mu() -> f64 {
    km3_per_s2_to_km3_per_day2(0.75 * SUN_MU_KM3_S2)
}

/// The capital planet's gravitational parameter from its surface gravity
/// and radius: μ = g R².
fn capital_mu() -> f64 {
    km3_per_s2_to_km3_per_day2(0.82 * G0_KM_S2 * 5_753.0 * 5_753.0)
}

fn assert_within_3_percent(actual: f64, expected: f64, what: &str) {
    let off = ((actual - expected) / expected).abs();
    assert!(
        off < 0.03,
        "{what}: expected about {expected}, got {actual} ({:.1}% off)",
        off * 100.0
    );
}

/// XFER-12: leaving a low band 500 km up costs about 2.7 km/s, and a high
/// band at ten planet radii about 0.9 km/s.
#[test]
fn band_departure_costs_match_the_spec() {
    let low = km_per_day_to_km_per_s(escape_cost(capital_mu(), 5_753.0 + 500.0));
    let high = km_per_day_to_km_per_s(escape_cost(capital_mu(), 10.0 * 5_753.0));
    assert_within_3_percent(low, 2.7, "low band departure cost");
    assert_within_3_percent(high, 0.9, "high band departure cost");
}

/// XFER-12: the Hohmann crossing from 0.5 AU to 0.8 AU costs about
/// 7.5 km/s in burns and takes about 111 days.
#[test]
fn hohmann_inner_planet_to_capital_matches_the_spec() {
    let trip = hohmann(star_mu(), 0.5 * KM_PER_AU, 0.8 * KM_PER_AU);
    let burns = km_per_day_to_km_per_s(trip.first_burn + trip.second_burn);
    assert_within_3_percent(burns, 7.5, "inner planet to capital burns");
    assert_within_3_percent(trip.days, 111.0, "inner planet to capital days");
}

/// XFER-12: the Hohmann crossing from 0.8 AU in to the belt at 0.64 AU
/// costs about 3.4 km/s and takes about 129 days.
#[test]
fn hohmann_capital_to_belt_matches_the_spec() {
    let trip = hohmann(star_mu(), 0.8 * KM_PER_AU, 0.64 * KM_PER_AU);
    let burns = km_per_day_to_km_per_s(trip.first_burn + trip.second_burn);
    assert_within_3_percent(burns, 3.4, "capital to belt burns");
    assert_within_3_percent(trip.days, 129.0, "capital to belt days");
}

/// XFER-12 as the solver sees it: searching departure days over one
/// synodic period and travel times with Lambert, the cheapest crossing
/// from 0.5 AU to 0.8 AU has the Hohmann burns and flight time.
#[test]
fn cheapest_lambert_crossing_is_the_hohmann_one() {
    let mu = star_mu();
    let inner = CircularOrbit::new(mu, 0.5 * KM_PER_AU, 0.0);
    let capital = CircularOrbit::new(mu, 0.8 * KM_PER_AU, 1.0);
    let synodic = synodic_period(inner.period_days, capital.period_days);
    let mut best = (f64::INFINITY, 0.0);
    let mut depart = 0.0;
    while depart < synodic {
        for travel in 90..=130 {
            let travel = travel as f64;
            let a = inner.state_at(depart);
            let b = capital.state_at(depart + travel);
            let Some(arc) = lambert(mu, a.position, b.position, travel, Sense::Prograde) else {
                continue;
            };
            let burns = (arc.departure_velocity - a.velocity).length()
                + (b.velocity - arc.arrival_velocity).length();
            if burns < best.0 {
                best = (burns, travel);
            }
        }
        depart += 1.0;
    }
    assert_within_3_percent(
        km_per_day_to_km_per_s(best.0),
        7.5,
        "cheapest crossing burns",
    );
    assert_within_3_percent(best.1, 111.0, "cheapest crossing days");
}

/// A sanity anchor for the figures above: the capital's year at 0.8 AU
/// round 0.75 solar masses is about 302 days, as the reference layout
/// says.
#[test]
fn capital_year_matches_the_reference_layout() {
    let days = circular_period(star_mu(), 0.8 * KM_PER_AU);
    assert_within_3_percent(days, 302.0, "capital's year");
}
