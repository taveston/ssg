//! A planet's facts, worked out from its kind, mass and orbit: radius,
//! surface gravity, equilibrium temperature, atmosphere and water
//! (SGEN-36, SGEN-38 to SGEN-41).

use super::Context;
use super::planets::Placed;
use crate::content::generator::bodies::{Atmosphere, PlanetKind, PlanetSummary, Water};
use crate::content::generator::rng::Rng;
use crate::content::generator::tunables::{GeneratorTunables, MassRadiusCurves};
use crate::sim::units::{
    EARTH_MASS_KG, EARTH_RADIUS_KM, GRAVITATIONAL_CONSTANT, M_PER_KM, STANDARD_GRAVITY_M_PER_S2,
};

/// The equilibrium temperature of a black body 1 AU from the Sun with no
/// albedo, in K: the 278.6 K that SGEN-39's formula scales.
const EQUILIBRIUM_T_AT_1_AU_K: f64 = 278.6;

/// A planet's radius in Earth radii from its mass in Earth masses, by its
/// kind's curve (SGEN-36). The gas-giant curve flattens at one Jupiter
/// mass: heavier giants are compressed, not larger.
pub fn radius_earth(kind: PlanetKind, mass_earth: f64, curves: &MassRadiusCurves) -> f64 {
    match kind {
        PlanetKind::Rocky | PlanetKind::SuperEarth => curves.rocky.radius(mass_earth),
        PlanetKind::SubNeptune => curves.sub_neptune.radius(mass_earth),
        PlanetKind::IceGiant => curves.ice_giant.radius(mass_earth),
        PlanetKind::GasGiant => {
            let law = curves.gas_giant;
            law.radius(mass_earth.min(law.reference_mass))
        }
    }
}

/// G × M ÷ R², in m/s², for a mass in Earth masses and a radius in Earth
/// radii.
fn surface_gravity_m_per_s2(mass_earth: f64, radius_earth: f64) -> f64 {
    let radius_m = radius_earth * EARTH_RADIUS_KM * M_PER_KM;
    GRAVITATIONAL_CONSTANT * mass_earth * EARTH_MASS_KG / radius_m.powi(2)
}

/// Surface gravity in units of standard gravity, g₀ = 9.80665 m/s²
/// (SGEN-38).
pub fn gravity_g(mass_earth: f64, radius_earth: f64) -> f64 {
    surface_gravity_m_per_s2(mass_earth, radius_earth) / STANDARD_GRAVITY_M_PER_S2
}

/// Escape velocity from the surface, in km/s: √(2 G M ÷ R).
pub fn escape_velocity_kms(mass_earth: f64, radius_earth: f64) -> f64 {
    let radius_m = radius_earth * EARTH_RADIUS_KM * M_PER_KM;
    (2.0 * surface_gravity_m_per_s2(mass_earth, radius_earth) * radius_m).sqrt() / M_PER_KM
}

/// Equilibrium temperature in K (SGEN-39): 278.6 K × L^(1/4) × a^(−1/2) ×
/// (1 − A)^(1/4), with L in solar units, a in AU and A the Bond albedo.
/// It assumes heat spreads evenly and ignores any greenhouse effect, so it
/// is not the surface temperature.
pub fn equilibrium_temperature_k(luminosity_lsun: f64, orbit_au: f64, albedo: f64) -> f64 {
    EQUILIBRIUM_T_AT_1_AU_K * luminosity_lsun.powf(0.25) / orbit_au.sqrt()
        * (1.0 - albedo).powf(0.25)
}

/// How well a planet holds an atmosphere (SGEN-40): v² ÷ T, with v the
/// escape velocity in km/s and T the equilibrium temperature in K, times
/// its volatile endowment. Heavier, colder planets hold more.
pub fn retention_index(escape_kms: f64, temperature_k: f64, endowment: f64) -> f64 {
    escape_kms.powi(2) / temperature_k * endowment
}

/// A rocky planet's atmosphere from its retention index (SGEN-40):
/// airless below `atmosphere_thin_index`, dense above
/// `atmosphere_dense_index`, thin between.
pub fn atmosphere_for_index(index: f64, t: &GeneratorTunables) -> Atmosphere {
    if index < t.atmosphere_thin_index {
        Atmosphere::Airless
    } else if index <= t.atmosphere_dense_index {
        Atmosphere::Thin
    } else {
        Atmosphere::Dense
    }
}

/// A planet's water (SGEN-41). Surface water needs a thin or dense
/// atmosphere, a temperature in that atmosphere's range and a lucky draw.
/// Otherwise a cool enough planet has ice with a chance that is higher at
/// or beyond the snow line. A planet with an envelope has none.
pub fn draw_water(
    atmosphere: Atmosphere,
    temperature_k: f64,
    beyond_snow_line: bool,
    rng: &mut Rng,
    t: &GeneratorTunables,
) -> Water {
    let surface_range = match atmosphere {
        Atmosphere::Envelope => return Water::None,
        Atmosphere::Airless => None,
        Atmosphere::Thin => Some(t.surface_water_t_k.thin),
        Atmosphere::Dense => Some(t.surface_water_t_k.dense),
    };
    if surface_range.is_some_and(|range| range.contains(temperature_k))
        && rng.chance(t.surface_water_chance)
    {
        return Water::Surface;
    }
    let ice_chance = if beyond_snow_line {
        t.ice_chance_beyond_snow_line
    } else {
        t.ice_chance_inside_snow_line
    };
    if temperature_k <= t.ice_t_eq_max_k && rng.chance(ice_chance) {
        Water::Ice
    } else {
        Water::None
    }
}

/// A planet's summary, with every fact worked out (SGEN-36, SGEN-38 to
/// SGEN-41). Only rocky planets and super-Earths draw a volatile
/// endowment and water; the other kinds have an envelope.
pub fn planet_summary(
    planet: &Placed,
    ctx: &Context,
    rng: &mut Rng,
    t: &GeneratorTunables,
) -> PlanetSummary {
    let radius = radius_earth(planet.kind, planet.mass_earth, &t.mass_radius);
    let temperature = equilibrium_temperature_k(
        ctx.star.luminosity_lsun,
        planet.orbit_au,
        t.equilibrium_albedo,
    );
    let atmosphere = if planet.kind.is_rocky() {
        let range = t.volatile_endowment_range;
        let endowment = rng.log_uniform(range.min, range.max);
        let escape = escape_velocity_kms(planet.mass_earth, radius);
        atmosphere_for_index(retention_index(escape, temperature, endowment), t)
    } else {
        Atmosphere::Envelope
    };
    let beyond = planet.orbit_au >= ctx.snow_line_au;
    PlanetSummary {
        kind: planet.kind,
        mass_earth: planet.mass_earth,
        radius_earth: radius,
        orbit_au: planet.orbit_au,
        gravity_g: gravity_g(planet.mass_earth, radius),
        equilibrium_temperature_k: temperature,
        atmosphere,
        water: draw_water(atmosphere, temperature, beyond, rng, t),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::generator::testing::all_summaries;

    fn within(value: f64, target: f64, share: f64) -> bool {
        (value - target).abs() <= share * target
    }

    /// SGEN-36: the curves give Mars, Venus, Earth, Uranus, Neptune,
    /// Saturn and Jupiter within 5% of their real mean radii.
    #[test]
    fn the_mass_radius_curves_fit_the_solar_system() {
        let curves = GeneratorTunables::default().mass_radius;
        let bodies = [
            ("Mars", PlanetKind::Rocky, 0.107, 0.532),
            ("Venus", PlanetKind::Rocky, 0.815, 0.949),
            ("Earth", PlanetKind::Rocky, 1.0, 1.0),
            ("Uranus", PlanetKind::IceGiant, 14.54, 4.007),
            ("Neptune", PlanetKind::IceGiant, 17.15, 3.883),
            ("Saturn", PlanetKind::GasGiant, 95.16, 9.140),
            ("Jupiter", PlanetKind::GasGiant, 317.8, 10.973),
        ];
        for (name, kind, mass, radius) in bodies {
            let r = radius_earth(kind, mass, &curves);
            assert!(within(r, radius, 0.05), "{name}: {r} vs {radius}");
        }
        assert_eq!(radius_earth(PlanetKind::SubNeptune, 6.0, &curves), 2.4);
        // Heavier than Jupiter, no larger.
        assert_eq!(radius_earth(PlanetKind::GasGiant, 1500.0, &curves), 11.21);
    }

    /// SGEN-38: Earth's surface gravity is 1 g, Mars's about 0.38.
    #[test]
    fn gravity_is_in_standard_gravities() {
        assert!(within(gravity_g(1.0, 1.0), 1.0, 0.01));
        assert!(within(gravity_g(0.107, 0.532), 0.378, 0.02));
    }

    /// SGEN-39: Earth's equilibrium temperature is about 255 K.
    #[test]
    fn earth_comes_out_at_255_kelvin() {
        let earth = equilibrium_temperature_k(1.0, 1.0, 0.3);
        assert!((earth - 254.8).abs() < 0.1, "{earth}");
        // Four times the light at the same distance is √2 times hotter.
        let bright = equilibrium_temperature_k(4.0, 1.0, 0.3);
        assert!((bright / earth - 2f64.sqrt()).abs() < 1e-12);
        assert!((equilibrium_temperature_k(1.0, 4.0, 0.3) - earth / 2.0).abs() < 1e-9);
    }

    /// SGEN-40: with an endowment of 1, Mercury, the Moon, Ganymede and
    /// Io are airless, Mars and Titan thin, Venus and Earth dense.
    #[test]
    fn the_atmosphere_rule_matches_the_solar_system() {
        let t = GeneratorTunables::default();
        // Name, mass (Earth masses), radius (Earth radii), distance from
        // the Sun (AU), expected atmosphere.
        let bodies = [
            ("Mercury", 0.0553, 0.383, 0.387, Atmosphere::Airless),
            ("Moon", 0.0123, 0.273, 1.0, Atmosphere::Airless),
            ("Ganymede", 0.0248, 0.413, 5.2, Atmosphere::Airless),
            ("Io", 0.0150, 0.286, 5.2, Atmosphere::Airless),
            ("Mars", 0.107, 0.532, 1.524, Atmosphere::Thin),
            ("Titan", 0.0225, 0.404, 9.58, Atmosphere::Thin),
            ("Venus", 0.815, 0.949, 0.723, Atmosphere::Dense),
            ("Earth", 1.0, 1.0, 1.0, Atmosphere::Dense),
        ];
        for (name, mass, radius, orbit, expected) in bodies {
            let temperature = equilibrium_temperature_k(1.0, orbit, t.equilibrium_albedo);
            let index = retention_index(escape_velocity_kms(mass, radius), temperature, 1.0);
            assert_eq!(
                atmosphere_for_index(index, &t),
                expected,
                "{name}: index {index}"
            );
        }
    }

    /// SGEN-40: only rocky planets and super-Earths have an airless, thin
    /// or dense atmosphere; the rest have an envelope.
    #[test]
    fn only_rocky_planets_lack_an_envelope() {
        for summary in all_summaries(5) {
            for planet in &summary.planets {
                assert_eq!(
                    planet.kind.is_rocky(),
                    planet.atmosphere != Atmosphere::Envelope
                );
            }
        }
    }

    /// SGEN-41: surface water needs an atmosphere and a temperature in its
    /// range; ice needs a cool planet; envelopes have none.
    #[test]
    fn water_follows_atmosphere_and_temperature() {
        let t = GeneratorTunables::default();
        let mut seen_surface = false;
        for summary in all_summaries(10) {
            for planet in &summary.planets {
                let temperature = planet.equilibrium_temperature_k;
                match planet.water {
                    Water::Surface => {
                        seen_surface = true;
                        let range = match planet.atmosphere {
                            Atmosphere::Thin => t.surface_water_t_k.thin,
                            Atmosphere::Dense => t.surface_water_t_k.dense,
                            other => panic!("surface water under {other:?}"),
                        };
                        assert!(range.contains(temperature));
                    }
                    Water::Ice => {
                        assert!(planet.kind.is_rocky());
                        assert!(temperature <= t.ice_t_eq_max_k);
                    }
                    Water::None => {}
                }
            }
        }
        assert!(seen_surface);
        // The chances: a cold airless planet beyond the snow line has ice
        // about 90% of the time, inside it about 35%.
        let mut rng = Rng::new(4);
        let share = |beyond: bool, rng: &mut Rng| {
            (0..4000)
                .filter(|_| draw_water(Atmosphere::Airless, 150.0, beyond, rng, &t) == Water::Ice)
                .count() as f64
                / 4000.0
        };
        assert!((share(true, &mut rng) - 0.9).abs() < 0.03);
        assert!((share(false, &mut rng) - 0.35).abs() < 0.03);
        assert_eq!(
            draw_water(Atmosphere::Envelope, 150.0, true, &mut rng, &t),
            Water::None
        );
        assert_eq!(
            draw_water(Atmosphere::Dense, 400.0, true, &mut rng, &t),
            Water::None
        );
    }
}
