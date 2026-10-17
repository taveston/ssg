//! INS-08: the inset rows for a world's physical facts (SYSG-22) and a
//! gate's state (SYSG-52), in words. Hand-made bodies carry neither, so
//! their insets don't change.

use crate::sim::body::GateState;
use crate::sim::planet_facts::{Atmosphere, PlanetFacts, Water, WorldKind};

/// A world's kind, as the inset's Type row reads it.
pub fn kind_words(kind: WorldKind) -> &'static str {
    match kind {
        WorldKind::Rocky => "Rocky",
        WorldKind::SuperEarth => "Super-Earth",
        WorldKind::SubNeptune => "Sub-Neptune",
        WorldKind::IceGiant => "Ice giant",
        WorldKind::GasGiant => "Gas giant",
        WorldKind::Moon => "Moon",
    }
}

/// An atmosphere class in a word.
pub fn atmosphere_words(atmosphere: Atmosphere) -> &'static str {
    match atmosphere {
        Atmosphere::Airless => "Airless",
        Atmosphere::Thin => "Thin",
        Atmosphere::Dense => "Dense",
        Atmosphere::Envelope => "Envelope",
    }
}

/// A world's water in a word.
pub fn water_words(water: Water) -> &'static str {
    match water {
        Water::None => "None",
        Water::Ice => "Ice",
        Water::Surface => "Surface",
    }
}

/// INS-08: a world's physical facts as inset rows, in this order: Type,
/// Gravity (in g to 2 decimals), Equilibrium temperature (whole kelvin),
/// Atmosphere and Water.
pub fn planet_fact_rows(facts: &PlanetFacts) -> Vec<(String, String)> {
    vec![
        ("Type".to_string(), kind_words(facts.kind).to_string()),
        ("Gravity".to_string(), format!("{:.2} g", facts.gravity_g)),
        (
            "Equilibrium temperature".to_string(),
            format!("{:.0} K", facts.equilibrium_temperature_k),
        ),
        (
            "Atmosphere".to_string(),
            atmosphere_words(facts.atmosphere).to_string(),
        ),
        ("Water".to_string(), water_words(facts.water).to_string()),
    ]
}

/// INS-08 / SYSG-55: a gate's State row: working, dormant, or dormant
/// with this end the faulted one.
pub fn gate_state_row(state: GateState) -> (String, String) {
    let words = match state {
        GateState::Working => "Working",
        GateState::Dormant {
            faulted_here: false,
        } => "Dormant",
        GateState::Dormant { faulted_here: true } => "Dormant, faulted here",
    };
    ("State".to_string(), words.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// INS-08: a world's facts read as Type, Gravity, Equilibrium
    /// temperature, Atmosphere and Water, in words and rounded figures.
    #[test]
    fn planet_facts_read_as_words_and_rounded_figures() {
        let facts = PlanetFacts {
            kind: WorldKind::SuperEarth,
            gravity_g: 1.234,
            equilibrium_temperature_k: 254.6,
            atmosphere: Atmosphere::Thin,
            water: Water::Ice,
        };
        let rows = planet_fact_rows(&facts);
        let rows: Vec<(&str, &str)> = rows.iter().map(|(k, v)| (&k[..], &v[..])).collect();
        assert_eq!(
            rows,
            [
                ("Type", "Super-Earth"),
                ("Gravity", "1.23 g"),
                ("Equilibrium temperature", "255 K"),
                ("Atmosphere", "Thin"),
                ("Water", "Ice"),
            ]
        );
        assert_eq!(kind_words(WorldKind::Moon), "Moon");
        assert_eq!(kind_words(WorldKind::GasGiant), "Gas giant");
        assert_eq!(atmosphere_words(Atmosphere::Envelope), "Envelope");
        assert_eq!(water_words(Water::Surface), "Surface");
    }

    /// INS-08 / SYSG-55: a gate's state reads "Working", "Dormant" or
    /// "Dormant, faulted here".
    #[test]
    fn a_gate_lists_its_state() {
        let state = |s| gate_state_row(s).1;
        assert_eq!(gate_state_row(GateState::Working).0, "State");
        assert_eq!(state(GateState::Working), "Working");
        assert_eq!(
            state(GateState::Dormant {
                faulted_here: false
            }),
            "Dormant"
        );
        assert_eq!(
            state(GateState::Dormant { faulted_here: true }),
            "Dormant, faulted here"
        );
    }
}
