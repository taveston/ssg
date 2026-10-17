//! Tides and inner heat: whether a body's spin is locked to its orbit
//! round its parent, and how geologically active it is from the heat its
//! interior gives off, radiogenic and tidal (SGEN-44, SGEN-45, SYSG-33).
//! Every figure here is in SI units inside and real units at the edges.

use crate::content::generator::tunables::GeneratorTunables;
use crate::sim::planet_facts::Activity;
use crate::sim::units::{EARTH_MASS_KG, GRAVITATIONAL_CONSTANT, M_PER_KM};
use std::f64::consts::{PI, TAU};

/// Seconds in a gigayear of Julian years.
const SECONDS_PER_GYR: f64 = 3.155_76e16;

/// Seconds in an hour.
const SECONDS_PER_HOUR: f64 = 3600.0;

/// Which tidal tunables a body takes (SGEN-44, SGEN-45, SYSG-33): a
/// rocky planet or super-Earth (and a belt body, which takes the same),
/// a planet with an envelope, or a moon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TideClass {
    RockyPlanet,
    EnvelopePlanet,
    Moon,
}

/// What a body's tides are worked out against: the parent it orbits (the
/// star for a planet or belt body, the planet for a moon), its circular
/// orbit's radius, its tidal class and the system's age.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TidalSetting {
    pub parent_mass_kg: f64,
    pub orbit_km: f64,
    pub class: TideClass,
    pub age_gyr: f64,
}

/// A body's two tidal facts (SGEN-44, SGEN-45).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tides {
    pub locked: bool,
    /// `None` for a planet with an envelope.
    pub activity: Option<Activity>,
}

/// The time in Gyr tides take to slow a body's spin to its orbit
/// (SGEN-44): t = 0.4 ω a⁶ m (Q ÷ k₂) ÷ (3 G M² R³), with ω the spin of
/// an `initial_day_h` day, a the orbit radius, m and R the body's mass and
/// radius and M its parent's mass. A body far out, or round a light
/// parent, takes far longer.
pub fn locking_time_gyr(
    mass_kg: f64,
    radius_km: f64,
    parent_mass_kg: f64,
    orbit_km: f64,
    q_over_k2: f64,
    initial_day_h: f64,
) -> f64 {
    let spin = TAU / (initial_day_h * SECONDS_PER_HOUR);
    let a = orbit_km * M_PER_KM;
    let r = radius_km * M_PER_KM;
    let seconds = 0.4 * spin * a.powi(6) * mass_kg * q_over_k2
        / (3.0 * GRAVITATIONAL_CONSTANT * parent_mass_kg.powi(2) * r.powi(3));
    seconds / SECONDS_PER_GYR
}

/// Whether a body is tidally locked to its parent (SGEN-44): its locking
/// time, with its class's Q ÷ k₂, is less than the system's age.
pub fn is_locked(
    mass_kg: f64,
    radius_km: f64,
    setting: &TidalSetting,
    t: &GeneratorTunables,
) -> bool {
    let time = locking_time_gyr(
        mass_kg,
        radius_km,
        setting.parent_mass_kg,
        setting.orbit_km,
        t.lock_q_over_k2.get(setting.class),
        t.lock_initial_day_h,
    );
    time < setting.age_gyr
}

/// The heat flux from a body's decaying radioactive elements, in W/m²
/// (SGEN-45): `radiogenic_flux_w_m2` × m^`radiogenic_mass_exponent` ×
/// exp((`radiogenic_reference_age_gyr` − age) ÷ `radiogenic_decay_gyr`),
/// with m in Earth masses. A system younger than the reference age is
/// hotter than the reference.
pub fn radiogenic_flux_w_m2(mass_earth: f64, age_gyr: f64, t: &GeneratorTunables) -> f64 {
    let decay = ((t.radiogenic_reference_age_gyr - age_gyr) / t.radiogenic_decay_gyr).exp();
    t.radiogenic_flux_w_m2 * mass_earth.powf(t.radiogenic_mass_exponent) * decay
}

/// The heat flux from tidal flexing, in W/m² (SGEN-45): the tidal power
/// (21 ÷ 2) × (k₂ ÷ Q) × G M² R⁵ n e² ÷ a⁶ spread over the surface
/// 4 π R², with M the parent's mass, R the body's radius, a its orbit
/// radius, n = √(G M ÷ a³) its mean motion and e the eccentricity the
/// heat assumes. The orbit itself stays circular; e is a heat input only.
pub fn tidal_flux_w_m2(
    radius_km: f64,
    parent_mass_kg: f64,
    orbit_km: f64,
    eccentricity: f64,
    tidal_response: f64,
) -> f64 {
    let gm = GRAVITATIONAL_CONSTANT * parent_mass_kg;
    let a = orbit_km * M_PER_KM;
    let r = radius_km * M_PER_KM;
    let mean_motion = (gm / a.powi(3)).sqrt();
    let power = 10.5
        * tidal_response
        * gm
        * parent_mass_kg
        * r.powi(5)
        * mean_motion
        * eccentricity.powi(2)
        / a.powi(6);
    power / (4.0 * PI * r.powi(2))
}

/// A body's activity from its interior heat flux in W/m² (SGEN-45): dead
/// below `activity_active_w_m2`, active up to and including
/// `activity_extreme_w_m2`, extreme above.
pub fn activity_for_flux(flux_w_m2: f64, t: &GeneratorTunables) -> Activity {
    if flux_w_m2 < t.activity_active_w_m2 {
        Activity::Dead
    } else if flux_w_m2 <= t.activity_extreme_w_m2 {
        Activity::Active
    } else {
        Activity::Extreme
    }
}

/// A solid body's activity (SGEN-45, SYSG-33): from the sum of its
/// radiogenic and tidal heat fluxes, the tidal one with its class's
/// forced eccentricity.
pub fn activity(
    mass_kg: f64,
    radius_km: f64,
    setting: &TidalSetting,
    t: &GeneratorTunables,
) -> Activity {
    let radiogenic = radiogenic_flux_w_m2(mass_kg / EARTH_MASS_KG, setting.age_gyr, t);
    let tidal = tidal_flux_w_m2(
        radius_km,
        setting.parent_mass_kg,
        setting.orbit_km,
        t.forced_eccentricity.get(setting.class),
        t.tidal_response,
    );
    activity_for_flux(radiogenic + tidal, t)
}

/// A body's tidal facts (SGEN-44, SGEN-45, SYSG-33): whether it is
/// locked, and its activity unless it is a planet with an envelope.
pub fn tides(mass_kg: f64, radius_km: f64, setting: &TidalSetting, t: &GeneratorTunables) -> Tides {
    Tides {
        locked: is_locked(mass_kg, radius_km, setting, t),
        activity: (setting.class != TideClass::EnvelopePlanet)
            .then(|| activity(mass_kg, radius_km, setting, t)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::units::{KM_PER_AU, SOLAR_MASS_KG};

    const SOL_AGE_GYR: f64 = 4.6;
    const EARTH_KG: f64 = 5.972e24;
    const JUPITER_KG: f64 = 1.898e27;
    const SATURN_KG: f64 = 5.683e26;

    /// A body of the solar system: its mass in kg, mean radius in km, its
    /// parent's mass in kg, its orbit radius in km and its class.
    struct Body(&'static str, f64, f64, f64, f64, TideClass);

    const EARTH: Body = Body(
        "Earth",
        EARTH_KG,
        6371.0,
        SOLAR_MASS_KG,
        KM_PER_AU,
        TideClass::RockyPlanet,
    );
    const MARS: Body = Body(
        "Mars",
        6.417e23,
        3389.5,
        SOLAR_MASS_KG,
        1.524 * KM_PER_AU,
        TideClass::RockyPlanet,
    );
    const MOON: Body = Body(
        "Moon",
        7.342e22,
        1737.4,
        EARTH_KG,
        384_400.0,
        TideClass::Moon,
    );
    const IO: Body = Body(
        "Io",
        8.932e22,
        1821.6,
        JUPITER_KG,
        421_700.0,
        TideClass::Moon,
    );
    const EUROPA: Body = Body(
        "Europa",
        4.800e22,
        1560.8,
        JUPITER_KG,
        670_900.0,
        TideClass::Moon,
    );
    const CALLISTO: Body = Body(
        "Callisto",
        1.0759e23,
        2410.3,
        JUPITER_KG,
        1_882_700.0,
        TideClass::Moon,
    );
    const IAPETUS: Body = Body(
        "Iapetus",
        1.8056e21,
        734.5,
        SATURN_KG,
        3_560_820.0,
        TideClass::Moon,
    );

    fn tides_of(body: &Body) -> Tides {
        let Body(_, mass, radius, parent, orbit, class) = *body;
        let setting = TidalSetting {
            parent_mass_kg: parent,
            orbit_km: orbit,
            class,
            age_gyr: SOL_AGE_GYR,
        };
        tides(mass, radius, &setting, &GeneratorTunables::default())
    }

    /// SGEN-44: with the defaults and Sol's age, the Moon, Callisto and
    /// Iapetus are locked to their planets and Earth is free of the Sun.
    #[test]
    fn the_locking_rule_matches_the_solar_system() {
        for body in [&MOON, &CALLISTO, &IAPETUS] {
            assert!(tides_of(body).locked, "{} should be locked", body.0);
        }
        assert!(!tides_of(&EARTH).locked);
    }

    /// SGEN-44: the locking time grows as a⁶ and falls as M².
    #[test]
    fn locking_time_grows_with_distance_and_falls_with_parent_mass() {
        let base = locking_time_gyr(1e22, 1000.0, 1e27, 1e6, 100.0, 12.0);
        let far = locking_time_gyr(1e22, 1000.0, 1e27, 2e6, 100.0, 12.0);
        let heavy = locking_time_gyr(1e22, 1000.0, 2e27, 1e6, 100.0, 12.0);
        assert!((far / base - 64.0).abs() < 1e-9);
        assert!((heavy / base - 0.25).abs() < 1e-12);
    }

    /// SGEN-45: with the defaults and Sol's age, Earth is active, Mars and
    /// the Moon dead, Io extreme and Europa active.
    #[test]
    fn the_activity_rule_matches_the_solar_system() {
        let expected = [
            (&EARTH, Activity::Active),
            (&MARS, Activity::Dead),
            (&MOON, Activity::Dead),
            (&IO, Activity::Extreme),
            (&EUROPA, Activity::Active),
        ];
        for (body, activity) in expected {
            assert_eq!(tides_of(body).activity, Some(activity), "{}", body.0);
        }
    }

    /// SGEN-45: radiogenic heat is the reference flux for an Earth mass at
    /// the reference age, and more in a younger system.
    #[test]
    fn radiogenic_heat_scales_with_mass_and_age() {
        let t = GeneratorTunables::default();
        assert!((radiogenic_flux_w_m2(1.0, 4.5, &t) - 0.09).abs() < 1e-12);
        assert!(radiogenic_flux_w_m2(1.0, 1.0, &t) > 0.09);
        let heavy = radiogenic_flux_w_m2(10.0, 4.5, &t);
        assert!((heavy - 0.09 * 10f64.powf(0.42)).abs() < 1e-12);
    }

    /// SGEN-45: the thresholds: active from 0.05 W/m² up to and including
    /// 0.5, extreme above.
    #[test]
    fn activity_thresholds_are_inclusive_of_active() {
        let t = GeneratorTunables::default();
        assert_eq!(activity_for_flux(0.049, &t), Activity::Dead);
        assert_eq!(activity_for_flux(0.05, &t), Activity::Active);
        assert_eq!(activity_for_flux(0.5, &t), Activity::Active);
        assert_eq!(activity_for_flux(0.51, &t), Activity::Extreme);
    }

    /// SGEN-45: a planet with an envelope has no activity, whatever its
    /// heat.
    #[test]
    fn a_body_with_an_envelope_has_no_activity() {
        let setting = TidalSetting {
            parent_mass_kg: SOLAR_MASS_KG,
            orbit_km: 0.05 * KM_PER_AU,
            class: TideClass::EnvelopePlanet,
            age_gyr: 1.0,
        };
        let hot_jupiter = tides(
            JUPITER_KG,
            70_000.0,
            &setting,
            &GeneratorTunables::default(),
        );
        assert_eq!(hot_jupiter.activity, None);
        assert!(hot_jupiter.locked);
    }
}
