//! The worked checks of the transfer spec (XFER-12, PHAS-13, ARCH-44), in plain
//! figures: a K-type star of 0.75 solar masses; a capital planet of
//! radius 5,753 km and surface gravity 0.82 g at 0.8 AU; an inner planet
//! at 0.5 AU; a belt at 0.64 AU. Every figure must hold within 3%.

use std::f64::consts::TAU;

use glam::DVec3;
use orbits::{
    CircularOrbit, PhasingOrbit, PhasingSide, Rail, Sense, capture_angle, circular_period,
    circular_speed, escape_cost, exit_angle, hohmann, km_per_day_to_km_per_s,
    km_per_s_to_km_per_day, km3_per_s2_to_km3_per_day2, lambert, meeting_days, pass_eccentricity,
    phasing_orbit, synodic_period,
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

// --- Phase (PHAS-13) ---

/// The capital's High band, ten radii out (PHAS-13).
const HIGH_KM: f64 = 57_530.0;
/// The capital's Low band, 500 km up (PHAS-13).
const LOW_KM: f64 = 6_253.0;

/// PHAS-13: the High band turns in 1.95 days at 2.15 km/s; the Low band
/// in 1.67 hours at 6.52 km/s.
#[test]
fn band_periods_and_speeds_match_the_phase_checks() {
    let mu = capital_mu();
    assert_within_3_percent(circular_period(mu, HIGH_KM), 1.95, "High period");
    assert_within_3_percent(
        km_per_day_to_km_per_s(circular_speed(mu, HIGH_KM)),
        2.15,
        "High speed",
    );
    assert_within_3_percent(circular_period(mu, LOW_KM) * 24.0, 1.67, "Low period");
    assert_within_3_percent(
        km_per_day_to_km_per_s(circular_speed(mu, LOW_KM)),
        6.52,
        "Low speed",
    );
}

/// The cheaper of the two phasing orbits for a meeting `k`, the inner on
/// a tie, ignoring whether either clears the planet.
fn cheaper(radius_km: f64, k: u32) -> PhasingOrbit {
    let inner = phasing_orbit(capital_mu(), radius_km, 0.5, k, PhasingSide::Inner);
    let outer = phasing_orbit(capital_mu(), radius_km, 0.5, k, PhasingSide::Outer);
    match (inner, outer) {
        (Some(i), Some(o)) if o.delta_v() < i.delta_v() => o,
        (Some(i), _) => i,
        (None, Some(o)) => o,
        (None, None) => panic!("no phasing orbit for k = {k}"),
    }
}

/// PHAS-13: in the High band, with the target half a turn ahead, k = 1
/// meets after 0.97 d for 1.54 km/s (inner only); k = 3 after 4.86 d for
/// 0.29 km/s (3 laps of a 1.62 d inner orbit or 2 of a 2.43 d outer one);
/// k = 10 after 18.5 d for 0.076 km/s.
#[test]
fn high_band_phasing_matches_the_spec() {
    let mu = capital_mu();
    let period = circular_period(mu, HIGH_KM);
    assert_within_3_percent(meeting_days(period, 0.5, 1), 0.97, "k = 1 meeting");
    assert!(phasing_orbit(mu, HIGH_KM, 0.5, 1, PhasingSide::Outer).is_none());
    let one = phasing_orbit(mu, HIGH_KM, 0.5, 1, PhasingSide::Inner).unwrap();
    assert_within_3_percent(km_per_day_to_km_per_s(one.delta_v()), 1.54, "k = 1 delta-v");

    assert_within_3_percent(meeting_days(period, 0.5, 3), 4.86, "k = 3 meeting");
    let inner = phasing_orbit(mu, HIGH_KM, 0.5, 3, PhasingSide::Inner).unwrap();
    let outer = phasing_orbit(mu, HIGH_KM, 0.5, 3, PhasingSide::Outer).unwrap();
    assert_eq!((inner.laps, outer.laps), (3, 2));
    assert_within_3_percent(inner.period_days, 1.62, "k = 3 inner lap");
    assert_within_3_percent(outer.period_days, 2.43, "k = 3 outer lap");
    assert_within_3_percent(
        km_per_day_to_km_per_s(cheaper(HIGH_KM, 3).delta_v()),
        0.29,
        "k = 3 delta-v",
    );

    assert_within_3_percent(meeting_days(period, 0.5, 10), 18.5, "k = 10 meeting");
    assert_within_3_percent(
        km_per_day_to_km_per_s(cheaper(HIGH_KM, 10).delta_v()),
        0.076,
        "k = 10 delta-v",
    );
}

/// PHAS-13: in the Low band, with the target half a turn ahead, k = 1
/// has only an inner orbit, which dips below the band; k = 2 meets after
/// 2.5 h on the outer orbit for 1.46 km/s; k = 10 after 16 h for 0.23 km/s.
#[test]
fn low_band_phasing_matches_the_spec() {
    let mu = capital_mu();
    let hours = circular_period(mu, LOW_KM) * 24.0;
    assert!(phasing_orbit(mu, LOW_KM, 0.5, 1, PhasingSide::Outer).is_none());
    let dip = phasing_orbit(mu, LOW_KM, 0.5, 1, PhasingSide::Inner);
    assert!(dip.is_none_or(|o| o.closest_km < LOW_KM));

    assert_within_3_percent(meeting_days(hours, 0.5, 2), 2.5, "k = 2 meeting, hours");
    let two = phasing_orbit(mu, LOW_KM, 0.5, 2, PhasingSide::Outer).unwrap();
    assert_within_3_percent(km_per_day_to_km_per_s(two.delta_v()), 1.46, "k = 2 delta-v");

    assert_within_3_percent(meeting_days(hours, 0.5, 10), 16.0, "k = 10 meeting, hours");
    let ten = phasing_orbit(mu, LOW_KM, 0.5, 10, PhasingSide::Outer).unwrap();
    assert_within_3_percent(
        km_per_day_to_km_per_s(ten.delta_v()),
        0.23,
        "k = 10 delta-v",
    );
}

/// PHAS-13: with the target half a turn ahead, for k ≥ 2, delta-v times
/// the meeting time is within 3% of two thirds of the radius times the
/// angle to make up: 2πg for an inner orbit, 2π(1 − g) for an outer one.
#[test]
fn delta_v_times_time_is_two_thirds_radius_times_angle() {
    let mu = capital_mu();
    let gap = 0.5;
    for radius in [HIGH_KM, LOW_KM] {
        let period = circular_period(mu, radius);
        for k in [2, 3, 5, 10] {
            for side in [PhasingSide::Inner, PhasingSide::Outer] {
                let Some(orbit) = phasing_orbit(mu, radius, gap, k, side) else {
                    continue;
                };
                let angle = match side {
                    PhasingSide::Inner => TAU * gap,
                    PhasingSide::Outer => TAU * (1.0 - gap),
                };
                let product = orbit.delta_v() * meeting_days(period, gap, k);
                assert_within_3_percent(
                    product,
                    2.0 / 3.0 * radius * angle,
                    &format!("r {radius}, k {k}, {side:?}"),
                );
            }
        }
    }
}

/// PHAS-13: at 5 km/s the High band's capture hyperbola has e = 6.40
/// and its closest point at α − 81.0°; the Low band's e = 1.59 at
/// α − 51.0°. The exit points are at α − 99.0° and α − 129.0°.
#[test]
fn capture_and_exit_points_match_the_spec() {
    let mu = capital_mu();
    let v = km_per_s_to_km_per_day(5.0);
    let alpha: f64 = 0.7;
    let v_inf = DVec3::new(v * alpha.cos(), 0.0, v * alpha.sin());
    assert_within_3_percent(pass_eccentricity(mu, HIGH_KM, v), 6.40, "High e");
    assert_within_3_percent(pass_eccentricity(mu, LOW_KM, v), 1.59, "Low e");
    let degrees_from_alpha = |angle: f64| (angle - alpha).to_degrees();
    assert_within_3_percent(
        degrees_from_alpha(capture_angle(mu, HIGH_KM, v_inf)),
        -81.0,
        "High capture point",
    );
    assert_within_3_percent(
        degrees_from_alpha(capture_angle(mu, LOW_KM, v_inf)),
        -51.0,
        "Low capture point",
    );
    assert_within_3_percent(
        degrees_from_alpha(exit_angle(mu, HIGH_KM, v_inf)),
        -99.0,
        "High exit point",
    );
    assert_within_3_percent(
        degrees_from_alpha(exit_angle(mu, LOW_KM, v_inf)),
        -129.0,
        "Low exit point",
    );
}
