//! The gate ring of a gated system: the circle its gates sit on, sized
//! from the snow line and kept clear of the planets' orbits (SGEN-58), and
//! the planets moved out of its way when no clear radius is near enough
//! (SGEN-59). Belts do not constrain the ring.

use crate::content::generator::bodies::{PlanetSummary, Summary};
use crate::content::generator::catalogue::Star;
use crate::content::generator::summaries::planets::star_mass_earth;
use crate::content::generator::summaries::rework_moved_planet;
use crate::content::generator::tunables::GeneratorTunables;

/// Moving planets clear of the ring can push others in turn; this many
/// passes is far more than any system needs, and a moved planet still out
/// of place after them is dropped (SGEN-59).
const MAX_SETTLE_PASSES: usize = 64;
/// A moved planet sits this share beyond the edge it moves to, so that
/// rounding never leaves it a hair inside the range it left.
const EDGE_MARGIN: f64 = 1e-9;

/// Where a system's ring ended up, and its planets after any moves.
#[derive(Debug, Clone, PartialEq)]
pub struct RingOutcome {
    /// The ring's radius, in AU.
    pub radius_au: f64,
    /// The planets, innermost first, with moved ones at their new orbits
    /// and their facts worked out again.
    pub planets: Vec<PlanetSummary>,
    /// Indices into `planets` of those that moved (SGEN-59).
    pub moved: Vec<usize>,
}

/// A planet's Hill radius, in AU: a × (m ÷ 3 M★)^(1/3), with the star's
/// mass in Earth masses (SGEN-58).
pub fn hill_radius_au(planet: &PlanetSummary, star_mass_earth: f64) -> f64 {
    planet.orbit_au * hill_factor(planet, star_mass_earth)
}

/// (m ÷ 3 M★)^(1/3): a planet's Hill radius as a share of its orbit.
fn hill_factor(planet: &PlanetSummary, star_mass_earth: f64) -> f64 {
    (planet.mass_earth / (3.0 * star_mass_earth)).cbrt()
}

/// The open range of ring radii, in AU, that come too close to this
/// planet's orbit (SGEN-58): within `ring_planet_gap` × the ring's radius
/// of it, and for a giant also within `ring_giant_clearance_hill` Hill
/// radii. Both contain the orbit, so together they are one range.
fn blocked_ring_radii(
    planet: &PlanetSummary,
    star_earth: f64,
    t: &GeneratorTunables,
) -> (f64, f64) {
    let a = planet.orbit_au;
    // |r − a| < gap × r, solved for r.
    let mut low = a / (1.0 + t.ring_planet_gap);
    let mut high = a / (1.0 - t.ring_planet_gap);
    if planet.kind.is_giant() {
        let clearance = t.ring_giant_clearance_hill * hill_radius_au(planet, star_earth);
        low = low.min(a - clearance);
        high = high.max(a + clearance);
    }
    (low, high)
}

/// Whether a ring of `radius_au` clears every planet's orbit (SGEN-58).
#[cfg(test)]
pub fn ring_is_clear(
    radius_au: f64,
    planets: &[PlanetSummary],
    star: &Star,
    t: &GeneratorTunables,
) -> bool {
    let star_earth = star_mass_earth(star.mass_msun);
    planets.iter().all(|planet| {
        let (low, high) = blocked_ring_radii(planet, star_earth, t);
        radius_au <= low || radius_au >= high
    })
}

/// The nearest clear radius from `start_au` in one direction: step to the
/// edge of each blocked range the radius falls in until none holds it.
fn nearest_clear(start_au: f64, blocked: &[(f64, f64)], outward: bool) -> f64 {
    let mut radius = start_au;
    while let Some(&(low, high)) = blocked
        .iter()
        .find(|(low, high)| *low < radius && radius < *high)
    {
        radius = if outward { high } else { low };
    }
    radius
}

/// The ring's radius (SGEN-58): `ring_snow_line_factor` × the snow line
/// if that clears every planet, else the nearest clear radius outward if
/// it is within `ring_slide_limit` of the start (as a share of it), else
/// the nearest clear radius inward within the limit. `None` if neither
/// is within the limit.
pub fn slide_ring(summary: &Summary, star: &Star, t: &GeneratorTunables) -> Option<f64> {
    let start = t.ring_snow_line_factor * summary.snow_line_au;
    let star_earth = star_mass_earth(star.mass_msun);
    let blocked: Vec<(f64, f64)> = summary
        .planets
        .iter()
        .map(|planet| blocked_ring_radii(planet, star_earth, t))
        .collect();
    let outward = nearest_clear(start, &blocked, true);
    if outward <= start * (1.0 + t.ring_slide_limit) {
        return Some(outward);
    }
    let inward = nearest_clear(start, &blocked, false);
    (inward > 0.0 && inward >= start * (1.0 - t.ring_slide_limit)).then_some(inward)
}

/// The ring and the system's planets (SGEN-58, SGEN-59). If the ring
/// cannot slide clear, it stays at its starting radius and the planets in
/// its way move instead.
pub fn place_ring(summary: &Summary, star: &Star, t: &GeneratorTunables) -> RingOutcome {
    match slide_ring(summary, star, t) {
        Some(radius_au) => RingOutcome {
            radius_au,
            planets: summary.planets.clone(),
            moved: Vec::new(),
        },
        None => {
            let radius_au = t.ring_snow_line_factor * summary.snow_line_au;
            let (planets, moved) = Settling::new(radius_au, summary, star, t).settle();
            RingOutcome {
                radius_au,
                planets,
                moved,
            }
        }
    }
}

/// Which way a planet was moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Outward,
    Inward,
}

/// A planet while the ring clears, and which way it was moved, if it was.
#[derive(Debug, Clone)]
struct Mover {
    planet: PlanetSummary,
    moved: Option<Direction>,
}

/// The planets of one system being moved clear of a fixed ring (SGEN-59).
struct Settling<'a> {
    ring_au: f64,
    summary: &'a Summary,
    star: &'a Star,
    star_earth: f64,
    t: &'a GeneratorTunables,
    planets: Vec<Mover>,
}

impl<'a> Settling<'a> {
    fn new(ring_au: f64, summary: &'a Summary, star: &'a Star, t: &'a GeneratorTunables) -> Self {
        Self {
            ring_au,
            summary,
            star,
            star_earth: star_mass_earth(star.mass_msun),
            t,
            planets: summary
                .planets
                .iter()
                .map(|planet| Mover {
                    planet: planet.clone(),
                    moved: None,
                })
                .collect(),
        }
    }

    /// Moves each planet that breaks the ring's clearance to the nearest
    /// clear orbit outward, or inward if none lies outward. Then a planet
    /// a move crowds (SGEN-34) is pushed on in the same direction by the
    /// least amount that restores the spacing, and past any belt it lands
    /// in, and so on until nothing moves. Returns the planets, innermost
    /// first, and the indices of those that moved, whose facts are worked
    /// out again (SGEN-59).
    fn settle(mut self) -> (Vec<PlanetSummary>, Vec<usize>) {
        for _ in 0..MAX_SETTLE_PASSES {
            self.sort();
            let moved = self.clear_of_ring() | self.push_crowded() | self.push_off_belts();
            if !moved {
                break;
            }
        }
        self.sort();
        self.drop_misplaced();
        let mut moved = Vec::new();
        let mut settled = Vec::with_capacity(self.planets.len());
        for (index, mut mover) in self.planets.into_iter().enumerate() {
            if mover.moved.is_some() {
                rework_moved_planet(&mut mover.planet, self.star.luminosity_lsun, self.t);
                moved.push(index);
            }
            settled.push(mover.planet);
        }
        (settled, moved)
    }

    fn sort(&mut self) {
        self.planets
            .sort_by(|a, b| a.planet.orbit_au.total_cmp(&b.planet.orbit_au));
    }

    /// The open range of orbits, in AU, at which a planet breaks the
    /// ring's clearance (SGEN-58 turned round: the ring is fixed and the
    /// orbit varies). The outer edge is infinite for a giant so heavy
    /// that no orbit outside the ring clears it.
    fn blocked_orbits(&self, planet: &PlanetSummary) -> (f64, f64) {
        let (ring, t) = (self.ring_au, self.t);
        // |r − a| < gap × r, solved for a.
        let mut low = ring * (1.0 - t.ring_planet_gap);
        let mut high = ring * (1.0 + t.ring_planet_gap);
        if planet.kind.is_giant() {
            // |r − a| < clearance × a × k, k the Hill factor.
            let reach = t.ring_giant_clearance_hill * hill_factor(planet, self.star_earth);
            low = low.min(ring / (1.0 + reach));
            high = if reach < 1.0 {
                high.max(ring / (1.0 - reach))
            } else {
                f64::INFINITY
            };
        }
        (low, high)
    }

    /// Moves every planet that breaks the ring's clearance to the edge of
    /// its blocked range, outward if it can. Returns whether any moved.
    fn clear_of_ring(&mut self) -> bool {
        let mut any = false;
        for index in 0..self.planets.len() {
            let (low, high) = self.blocked_orbits(&self.planets[index].planet);
            let mover = &mut self.planets[index];
            if low < mover.planet.orbit_au && mover.planet.orbit_au < high {
                let (orbit, direction) = if high.is_finite() {
                    (high * (1.0 + EDGE_MARGIN), Direction::Outward)
                } else {
                    (low * (1.0 - EDGE_MARGIN), Direction::Inward)
                };
                mover.planet.orbit_au = orbit;
                mover.moved = Some(direction);
                any = true;
            }
        }
        any
    }

    /// The factor by which the outer of two planets' orbits must exceed
    /// the inner one's for them to keep `min_mutual_hill` mutual Hill
    /// radii apart (SGEN-34): from a₂ − a₁ ≥ K × q × (a₁ + a₂) ÷ 2, with q
    /// = ((m₁ + m₂) ÷ 3 M★)^(1/3), a₂ ≥ a₁ × (1 + h) ÷ (1 − h) for h =
    /// K q ÷ 2. `None` if the pair is too heavy for any spacing to do.
    fn spacing_factor(&self, a: &PlanetSummary, b: &PlanetSummary) -> Option<f64> {
        let q = ((a.mass_earth + b.mass_earth) / (3.0 * self.star_earth)).cbrt();
        let h = self.t.min_mutual_hill * q / 2.0;
        (h < 1.0).then(|| (1.0 + h) / (1.0 - h))
    }

    /// Whether two neighbours, `inner` inside `outer`, crowd each other.
    fn crowded(&self, inner: &PlanetSummary, outer: &PlanetSummary) -> bool {
        self.spacing_factor(inner, outer)
            .is_none_or(|f| outer.orbit_au < inner.orbit_au * f)
    }

    /// One pass of the crowding rule over neighbours, innermost first. A
    /// pair whose outer member was moved inward pushes the inner one
    /// inward; any other crowded pair pushes the outer one outward. A
    /// pair no spacing can part loses the pushed planet. Returns whether
    /// anything moved.
    fn push_crowded(&mut self) -> bool {
        let mut any = false;
        let mut index = 0;
        while index + 1 < self.planets.len() {
            let (inner, outer) = (&self.planets[index].planet, &self.planets[index + 1].planet);
            if !self.crowded(inner, outer) {
                index += 1;
                continue;
            }
            any = true;
            let factor = self.spacing_factor(inner, outer);
            let inward = self.planets[index + 1].moved == Some(Direction::Inward);
            let (pushed, anchor) = if inward {
                (index, index + 1)
            } else {
                (index + 1, index)
            };
            let Some(factor) = factor else {
                self.planets.remove(pushed);
                continue;
            };
            let anchor_au = self.planets[anchor].planet.orbit_au;
            let mover = &mut self.planets[pushed];
            if inward {
                mover.planet.orbit_au = anchor_au / factor;
                mover.moved = Some(Direction::Inward);
            } else {
                mover.planet.orbit_au = anchor_au * factor;
                mover.moved = Some(Direction::Outward);
            }
            index += 1;
        }
        any
    }

    /// Moves each moved planet that lies in a belt past the belt's edge in
    /// the direction it was moving, so no orbit lies in a belt (SGEN-37).
    /// Returns whether any moved.
    fn push_off_belts(&mut self) -> bool {
        let mut any = false;
        for mover in &mut self.planets {
            let Some(direction) = mover.moved else {
                continue;
            };
            let belts = &self.summary.belts;
            if let Some(belt) = belts.iter().find(|b| b.contains(mover.planet.orbit_au)) {
                mover.planet.orbit_au = match direction {
                    Direction::Outward => belt.outer_au * (1.0 + EDGE_MARGIN),
                    Direction::Inward => belt.inner_au * (1.0 - EDGE_MARGIN),
                };
                any = true;
            }
        }
        any
    }

    /// Whether the planet at `index` is out of place: breaking the ring's
    /// clearance, inside a belt, or crowding a neighbour.
    fn misplaced(&self, index: usize) -> bool {
        let planet = &self.planets[index].planet;
        let (low, high) = self.blocked_orbits(planet);
        let blocks_ring = low < planet.orbit_au && planet.orbit_au < high;
        let in_belt = self
            .summary
            .belts
            .iter()
            .any(|b| b.contains(planet.orbit_au));
        let crowds_inner = index > 0 && self.crowded(&self.planets[index - 1].planet, planet);
        let crowds_outer = self
            .planets
            .get(index + 1)
            .is_some_and(|outer| self.crowded(planet, &outer.planet));
        blocks_ring || in_belt || crowds_inner || crowds_outer
    }

    /// Drops moved planets still out of place once the passes are spent,
    /// so the system's rules hold whatever happened (SGEN-34, SGEN-37,
    /// SGEN-58).
    fn drop_misplaced(&mut self) {
        while let Some(index) =
            (0..self.planets.len()).find(|&i| self.planets[i].moved.is_some() && self.misplaced(i))
        {
            self.planets.remove(index);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::generator::bodies::{
        Atmosphere, BeltComposition, BeltSummary, PlanetKind, Template, Water,
    };
    use crate::content::generator::testing::sample_star;

    fn planet(kind: PlanetKind, mass_earth: f64, orbit_au: f64) -> PlanetSummary {
        let rocky = kind.is_rocky();
        PlanetSummary {
            kind,
            mass_earth,
            radius_earth: 1.0,
            orbit_au,
            gravity_g: 1.0,
            equilibrium_temperature_k: 200.0,
            atmosphere: if rocky {
                Atmosphere::Thin
            } else {
                Atmosphere::Envelope
            },
            water: Water::None,
            volatile_endowment: rocky.then_some(1.0),
        }
    }

    fn system(planets: Vec<PlanetSummary>, belts: Vec<BeltSummary>) -> Summary {
        Summary {
            template: Template::SolarLike,
            snow_line_au: 2.7,
            habitable_zone_inner_au: 0.99,
            habitable_zone_outer_au: 1.7,
            planets,
            belts,
        }
    }

    fn belt(inner_au: f64, outer_au: f64) -> BeltSummary {
        BeltSummary {
            inner_au,
            outer_au,
            mass_kg: 1e21,
            composition: BeltComposition {
                rocky: 0.8,
                metallic: 0.1,
                icy: 0.1,
            },
        }
    }

    /// The ring of a Sun-like star starts at 1.35 × 2.7 = 3.645 AU.
    const START_AU: f64 = 3.645;

    /// Rocky planets every 6% from 0.45 to 1.6 times the ring's start:
    /// each blocks a tenth of the radii round it, so no radius within
    /// half the start is clear.
    fn packed_system() -> Summary {
        let mut planets = Vec::new();
        let mut orbit = START_AU * 0.45;
        while orbit < START_AU * 1.6 {
            planets.push(planet(PlanetKind::Rocky, 0.05, orbit));
            orbit *= 1.06;
        }
        system(planets, vec![])
    }

    /// SGEN-58: with nothing near, the ring stays at its starting radius.
    #[test]
    fn an_empty_system_keeps_the_ring_at_its_start() {
        let t = GeneratorTunables::default();
        let star = sample_star();
        let near_star = system(vec![planet(PlanetKind::Rocky, 1.0, 1.0)], vec![]);
        let outcome = place_ring(&near_star, &star, &t);
        assert!((outcome.radius_au - START_AU).abs() < 1e-12);
        assert!(outcome.moved.is_empty());
    }

    /// SGEN-58: a planet near the start pushes the ring outward to the
    /// edge of its gap; a giant pushes it past its Hill clearance; belts
    /// never block it.
    #[test]
    fn the_ring_slides_outward_clear_of_planets() {
        let t = GeneratorTunables::default();
        let star = sample_star();
        let rocky = system(vec![planet(PlanetKind::Rocky, 1.0, 3.6)], vec![]);
        let radius = slide_ring(&rocky, &star, &t).unwrap();
        assert!((radius - 3.6 / 0.95).abs() < 1e-12);
        let jupiter = system(vec![planet(PlanetKind::GasGiant, 317.8, 3.6)], vec![]);
        let radius = slide_ring(&jupiter, &star, &t).unwrap();
        let hill = hill_radius_au(&jupiter.planets[0], star_mass_earth(1.0));
        assert!((radius - (3.6 + 5.0 * hill)).abs() < 1e-9);
        assert!(ring_is_clear(radius, &jupiter.planets, &star, &t));
        let belted = system(vec![], vec![belt(3.0, 4.0)]);
        let slid = slide_ring(&belted, &star, &t).unwrap();
        assert!((slid - START_AU).abs() < 1e-12);
    }

    /// SGEN-58: when outward is blocked past the limit, the ring slides
    /// inward instead.
    #[test]
    fn the_ring_slides_inward_when_outward_is_too_far() {
        let t = GeneratorTunables::default();
        let star = sample_star();
        // A heavy giant just outside the start blocks far outward.
        let giant = system(vec![planet(PlanetKind::GasGiant, 600.0, 4.0)], vec![]);
        let radius = slide_ring(&giant, &star, &t).unwrap();
        assert!((START_AU * 0.5..START_AU).contains(&radius));
        assert!(ring_is_clear(radius, &giant.planets, &star, &t));
    }

    /// SGEN-59: with no clear radius within the limit, the ring stays at
    /// its start and the planets in its way move outward, pushing crowded
    /// neighbours on; moved planets have their temperature worked out
    /// again.
    #[test]
    fn planets_move_clear_when_the_ring_cannot_slide() {
        let t = GeneratorTunables::default();
        let star = sample_star();
        let packed = packed_system();
        assert_eq!(slide_ring(&packed, &star, &t), None);
        let outcome = place_ring(&packed, &star, &t);
        assert!((outcome.radius_au - START_AU).abs() < 1e-12);
        assert!(!outcome.moved.is_empty());
        assert_eq!(outcome.planets.len(), packed.planets.len());
        assert!(ring_is_clear(
            outcome.radius_au,
            &outcome.planets,
            &star,
            &t
        ));
        let settling = Settling::new(START_AU, &packed, &star, &t);
        for pair in outcome.planets.windows(2) {
            assert!(!settling.crowded(&pair[0], &pair[1]));
        }
        for &index in &outcome.moved {
            let moved = &outcome.planets[index];
            assert!(moved.orbit_au > START_AU, "every move here is outward");
            // 278.6 K × (1 − 0.3)^(1/4) ÷ √a round a Sun-like star.
            let expected = 278.6 * 0.7f64.powf(0.25) / moved.orbit_au.sqrt();
            assert!((moved.equilibrium_temperature_k - expected).abs() < 1e-9);
        }
        // Unmoved planets keep their orbits.
        let unmoved = (0..outcome.planets.len()).filter(|i| !outcome.moved.contains(i));
        for index in unmoved {
            let orbit = outcome.planets[index].orbit_au;
            assert!(packed.planets.iter().any(|p| p.orbit_au == orbit));
        }
    }

    /// SGEN-59: a moved planet that lands in a belt goes on past it.
    #[test]
    fn moved_planets_clear_belts() {
        let t = GeneratorTunables::default();
        let star = sample_star();
        let mut packed = packed_system();
        // A narrow belt just where the planet at 1.017 starts lands (1.05
        // starts), between it and the next planet at 1.079 starts.
        packed.belts.push(belt(START_AU * 1.045, START_AU * 1.06));
        assert!(
            packed
                .planets
                .iter()
                .all(|p| !packed.belts[0].contains(p.orbit_au))
        );
        let outcome = place_ring(&packed, &star, &t);
        assert!(!outcome.moved.is_empty());
        for planet in &outcome.planets {
            assert!(!packed.belts[0].contains(planet.orbit_au));
        }
        assert!(ring_is_clear(
            outcome.radius_au,
            &outcome.planets,
            &star,
            &t
        ));
    }
}
