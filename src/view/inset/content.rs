//! What an inset says (INS-02, INS-04, INS-08, PLNR-01, PLNR-05), as
//! plain values gathered from the selected body: its name, class line,
//! image, description and key facts, and whether it is a ship the
//! faction commands. `inset/inspect.rs` lays them out; the rows for a
//! world's physical facts and a gate's state are in `world_facts.rs`.

use bevy::prelude::*;

use crate::sim::body::{
    Affiliation, BodyClass, Description, DisplayName, GateState, KeyFacts, Knowledge,
    PhysicalRadius, StellarMass,
};
use crate::sim::group::ShipGroup;
use crate::sim::orbit::Orbit;
use crate::sim::planet_facts::PlanetFacts;
use crate::sim::ship::{DeltaV, Drive};
use crate::sim::track::ShipRecord;
use crate::sim::transfer::plan::Status;
use crate::sim::units::SOLAR_MASS_KG;
use crate::view::planner::words::{DELTA_V, DRIVE, delta_v_fact};
use crate::view::readout::{format_distance, format_duration};
use crate::view::surfaces::inset::shows_globe;
use crate::view::symbols::ownership_text;
use crate::view::tunables::Tunables;

use super::world_facts::{gate_state_row, planet_fact_rows};

/// INS-02: the class line's readable class name.
pub fn class_label(class: BodyClass) -> &'static str {
    match class {
        BodyClass::Star => "Star",
        BodyClass::Planet => "Planet",
        BodyClass::Moon => "Moon",
        BodyClass::Station => "Station",
        BodyClass::Gate => "Gate",
        BodyClass::SalvageSite => "Salvage site",
        BodyClass::Ship => "Ship",
    }
}

/// INS-04: which image an inset shows, by knowledge state.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InsetImage {
    /// A clean render.
    Surveyed,
    /// A grainy telescope frame, with corner brackets.
    Telescope,
}

/// INS-04: the placeholder image for a knowledge state. There is no
/// status text: what is known is shown by the image alone.
pub fn knowledge_image(knowledge: Knowledge) -> InsetImage {
    match knowledge {
        Knowledge::Surveyed => InsetImage::Surveyed,
        Knowledge::Unsurveyed => InsetImage::Telescope,
    }
}

/// Everything an inset shows, as plain values (INS-02).
pub struct InsetContent {
    pub body: Entity,
    pub name: String,
    pub class_line: String,
    pub image: InsetImage,
    /// SURF-45: a planet or moon, whose image is its globe rather than
    /// the placeholder.
    pub globe: bool,
    pub description: String,
    pub facts: Vec<(String, String)>,
    /// PLNR-01: a ship the faction commands, which may be given orders.
    pub commanded_ship: bool,
}

/// PLNR-05: the key facts a ship the faction commands adds: its delta-v
/// less what it has spent, with what is reserved, and its drive's name.
pub fn ship_facts(account: &DeltaV, drive: &Drive) -> Vec<(String, String)> {
    vec![
        (
            DELTA_V.to_string(),
            delta_v_fact(
                account.delta_v_kms - account.spent_kms,
                account.reserved_kms,
            ),
        ),
        (DRIVE.to_string(), drive.name.to_string()),
    ]
}

/// PLNR-01: whether a body is a ship the faction commands, the only kind
/// that can be given orders.
pub fn is_commanded_ship(class: BodyClass, record: Option<&ShipRecord>) -> bool {
    class == BodyClass::Ship && record.is_some_and(|r| r.commanded)
}

/// PLNR-01: whether a ship the faction commands shows the Transfer
/// button: it is not a group, it is parked in a named orbit (not in
/// flight or coasting), and it has no plan that is Scheduled, InFlight or
/// Holding. `plan` is its plan's status, if it has one.
pub fn offers_transfer(group: bool, parked: bool, plan: Option<Status>) -> bool {
    let busy = matches!(
        plan,
        Some(Status::Scheduled | Status::InFlight | Status::Holding)
    );
    !group && parked && !busy
}

pub type InsetBodies<'w, 's> = Query<
    'w,
    's,
    (
        &'static DisplayName,
        &'static Description,
        &'static KeyFacts,
        &'static BodyClass,
        &'static Affiliation,
        &'static Knowledge,
        Option<&'static ShipRecord>,
        PhysicalFacts,
        WorldFacts,
        ShipFacts,
    ),
>;

/// What INS-08 reports of a generated world or a gate: its planet facts
/// (SYSG-22) and its state (SYSG-52), where it has them.
pub type WorldFacts = (Option<&'static PlanetFacts>, Option<&'static GateState>);

/// What PLNR-01 and PLNR-05 read of a ship: its class, its delta-v and
/// drive, and whether it is a group.
pub type ShipFacts = (
    Option<&'static DeltaV>,
    Option<&'static Drive>,
    Has<ShipGroup>,
);

/// What INS-08 reports of a body's physics: its orbit, a star's mass and
/// its radius.
pub type PhysicalFacts = (
    Option<&'static Orbit>,
    Option<&'static StellarMass>,
    Option<&'static PhysicalRadius>,
);

/// INS-08: whether a body is a natural one (star, planet or moon), the
/// only kind whose mean radius the inset lists.
pub fn is_natural(class: BodyClass) -> bool {
    matches!(class, BodyClass::Star | BodyClass::Planet | BodyClass::Moon)
}

/// INS-08: the astronomical key facts: orbit and period for an orbiting
/// body, mass in solar masses for the star, and the mean radius. Figures
/// go through the shared readout formats (UNIT-01, UNIT-02).
pub fn astronomical_facts(
    orbit: Option<&Orbit>,
    stellar_mass: Option<&StellarMass>,
    radius: Option<&PhysicalRadius>,
    au_threshold_au: f64,
) -> Vec<(String, String)> {
    let mut facts = Vec::new();
    if let Some(mass) = stellar_mass {
        let solar = mass.0 / SOLAR_MASS_KG;
        facts.push(("Mass".to_string(), format!("{solar:.2} solar masses")));
    } else if let Some(orbit) = orbit {
        let distance = format_distance(orbit.radius_km, au_threshold_au);
        facts.push(("Orbit".to_string(), distance));
        facts.push(("Period".to_string(), format_duration(orbit.period_days)));
    }
    if let Some(radius) = radius {
        let text = format_distance(radius.0, au_threshold_au);
        facts.push(("Mean radius".to_string(), text));
    }
    facts
}

/// INS-02 / INS-08 / PLNR-05: what the inset shows for `body`, if it is
/// one.
pub fn gather_content(
    body: Entity,
    bodies: &InsetBodies,
    tunables: &Tunables,
) -> Option<InsetContent> {
    let (name, description, facts, class, affiliation, knowledge, record, physics, world, ship) =
        bodies.get(body).ok()?;
    let (orbit, mass, radius) = physics;
    let (account, drive, group) = ship;
    let commanded_ship = is_commanded_ship(*class, record);
    let radius = radius.filter(|_| is_natural(*class));
    let mut facts = facts.0.clone();
    facts.extend(astronomical_facts(
        orbit,
        mass,
        radius,
        tunables.distance_au_threshold_au,
    ));
    let (planet_facts, gate_state) = world;
    if let Some(planet_facts) = planet_facts {
        facts.extend(planet_fact_rows(planet_facts));
    }
    if let Some(&state) = gate_state {
        facts.push(gate_state_row(state));
    }
    if commanded_ship && let (Some(account), Some(drive)) = (account, drive) {
        facts.extend(ship_facts(account, drive));
    }
    let image = knowledge_image(*knowledge);
    Some(InsetContent {
        body,
        name: name.0.clone(),
        class_line: format!(
            "{} · {}",
            class_label(*class),
            ownership_text(*affiliation, record)
        ),
        image,
        globe: shows_globe(*class),
        description: description.0.clone(),
        facts,
        commanded_ship: commanded_ship && !group,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// INS-02: classes read as words, not enum names.
    #[test]
    fn class_names_are_readable() {
        assert_eq!(class_label(BodyClass::SalvageSite), "Salvage site");
        assert_eq!(class_label(BodyClass::Planet), "Planet");
        assert!(
            class_label(BodyClass::Ship)
                .chars()
                .all(|c| c.is_alphabetic() || c == ' ')
        );
    }

    fn orbit(radius_km: f64, period_days: f64) -> Orbit {
        Orbit {
            radius_km,
            period_days,
            phase0_rad: 0.0,
        }
    }

    fn fact_pairs(facts: &[(String, String)]) -> Vec<(&str, &str)> {
        facts
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect()
    }

    /// INS-08: an orbiting body lists its orbit, period and mean radius.
    #[test]
    fn an_orbiting_body_lists_orbit_period_and_radius() {
        let o = orbit(2.0 * crate::sim::units::KM_PER_AU, 1000.0);
        let facts = astronomical_facts(Some(&o), None, Some(&PhysicalRadius(6700.0)), 0.01);
        assert_eq!(
            fact_pairs(&facts),
            vec![
                ("Orbit", "2 AU"),
                ("Period", "2.7 y"),
                ("Mean radius", "6,700 km"),
            ]
        );
    }

    /// INS-08: the star lists its mass in solar masses instead of an orbit.
    #[test]
    fn the_star_lists_its_mass_in_solar_masses() {
        let mass = StellarMass(0.78 * SOLAR_MASS_KG);
        let radius = PhysicalRadius(540_000.0);
        let facts = astronomical_facts(None, Some(&mass), Some(&radius), 0.01);
        assert_eq!(
            fact_pairs(&facts),
            vec![("Mass", "0.78 solar masses"), ("Mean radius", "540,000 km")]
        );
    }

    /// INS-08: only stars, planets and moons list a mean radius; ships,
    /// stations, gates and salvage sites don't.
    #[test]
    fn only_natural_bodies_list_a_mean_radius() {
        for class in [BodyClass::Star, BodyClass::Planet, BodyClass::Moon] {
            assert!(is_natural(class), "{class:?}");
        }
        for class in [
            BodyClass::Ship,
            BodyClass::Station,
            BodyClass::Gate,
            BodyClass::SalvageSite,
        ] {
            assert!(!is_natural(class), "{class:?}");
        }
    }

    /// INS-04: surveyed is a clean render; unsurveyed a telescope frame.
    #[test]
    fn each_knowledge_state_has_its_placeholder_image() {
        assert_eq!(knowledge_image(Knowledge::Surveyed), InsetImage::Surveyed);
        assert_eq!(
            knowledge_image(Knowledge::Unsurveyed),
            InsetImage::Telescope
        );
    }
}
