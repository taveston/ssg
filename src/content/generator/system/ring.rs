//! The gate ring (SYSG-50 to SYSG-54): one record for each link the
//! system belongs to, on the circle the sector sized for its gates. A
//! working or dormant link's end is a gate; a collapsed one's is a wreck.
//! They are spread evenly round the ring, each nudged a little, in the
//! order of their links.

use super::Detail;
use crate::content::generator::Sector;
use crate::content::generator::network::{Link, LinkState};
use crate::content::generator::rng::{Rng, derive_seed};
use crate::content::generator::tunables::GeneratorTunables;
use crate::sim::body::{Affiliation, BodyClass, GateState, Knowledge};
use crate::sim::system::{BodyBlueprint, GateBlueprint, OrbitPlacement};
use crate::sim::units::KM_PER_AU;

/// A gate's mean physical radius, in km, and its wreck's: the size of
/// Corvane's gates. Nothing shows it; it only keeps the zoom from going
/// inside the gate.
const GATE_RADIUS_KM: f64 = 5.0;

/// The records of a system's gate ring: its gates, and its wrecks, which
/// are salvage sites round the star.
#[derive(Debug, Clone, PartialEq)]
pub struct RingRecords {
    pub gates: Vec<GateBlueprint>,
    pub wrecks: Vec<BodyBlueprint>,
}

/// SYSG-54: the phases, in radians from 0 to 2π, of `count` records
/// spread round the ring: the i-th at θ + i × 2π ÷ n, plus a jitter drawn
/// uniformly within ± `gate_jitter_fraction` of that spacing, with θ drawn
/// once for the ring.
pub fn ring_phases(count: usize, rng: &mut Rng, t: &GeneratorTunables) -> Vec<f64> {
    let tau = std::f64::consts::TAU;
    let theta = rng.uniform(0.0, tau);
    let spacing = tau / count.max(1) as f64;
    let jitter = t.gate_jitter_fraction * spacing;
    (0..count)
        .map(|i| (theta + i as f64 * spacing + rng.uniform(-jitter, jitter)).rem_euclid(tau))
        .collect()
}

/// SYSG-51 / SYSG-52: the state of the gate at `id`'s end of a link: its
/// working or dormant state, with whether this end is faulted; `None` if
/// the link collapsed and the end is a wreck.
pub fn gate_state(link: &Link, id: usize) -> Option<GateState> {
    match link.state {
        LinkState::Working => Some(GateState::Working),
        LinkState::Dormant { faulted } => {
            let end = usize::from(link.ends[0] != id);
            Some(GateState::Dormant {
                faulted_here: faulted[end],
            })
        }
        LinkState::Collapsed => None,
    }
}

/// SYSG-53: a collapsed link's end: a salvage site round the star named
/// "*Destination* gate wreck", whose contents are not generated.
fn wreck(destination: &str, radius_km: f64, phase0_rad: f64) -> BodyBlueprint {
    BodyBlueprint {
        name: format!("{destination} gate wreck"),
        class: BodyClass::SalvageSite,
        affiliation: Affiliation::Neutral,
        record: None,
        propulsion: None,
        group: None,
        description: "What is left of a gate that collapsed.".to_string(),
        key_facts: vec![(
            "Site".to_string(),
            format!("Wreck of the gate to {destination}"),
        )],
        planet_facts: None,
        surface_seed: None,
        look_override: None,
        knowledge: Knowledge::Surveyed,
        orbit: OrbitPlacement::Own { radius_km },
        radius_km: GATE_RADIUS_KM,
        phase0_rad,
        mass_kg: 0.0,
        orbits: vec![],
        children: vec![],
    }
}

/// SYSG-50 to SYSG-54: one record per link the system belongs to, in link
/// order, at the ring's radius and spread round it: a gate named
/// "*Destination* gate" with its state and entry tolerance for a working
/// or dormant link, a wreck for a collapsed one. The ring's radius is the
/// sector's; its draws come from the system's own "ring" seed.
pub fn ring_records(sector: &Sector, detail: &Detail, t: &GeneratorTunables) -> RingRecords {
    let id = detail.gated.id;
    let links: Vec<&Link> = sector.links_of(id).collect();
    let mut rng = Rng::new(derive_seed(detail.seed, "ring"));
    let phases = ring_phases(links.len(), &mut rng, t);
    let radius_km = detail.gated.ring_radius_au * KM_PER_AU;
    let mut records = RingRecords {
        gates: Vec::new(),
        wrecks: Vec::new(),
    };
    for (link, phase) in links.into_iter().zip(phases) {
        let other = link.other_end(id);
        let destination = &sector
            .gated_system(other)
            .expect("links join gated systems")
            .names
            .system;
        match gate_state(link, id) {
            Some(state) => records.gates.push(GateBlueprint {
                name: format!("{destination} gate"),
                destination: destination.clone(),
                radius_km: GATE_RADIUS_KM,
                distance_km: radius_km,
                angle_deg: phase.to_degrees(),
                entry_tolerance_kms: t.gate_entry_tolerance_km_s,
                state,
            }),
            None => records.wrecks.push(wreck(destination, radius_km, phase)),
        }
    }
    records
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::generator::network::LinkOrigin;

    fn link(state: LinkState) -> Link {
        Link {
            ends: [3, 9],
            length_km: 1.0,
            origin: LinkOrigin::Spanning,
            state,
        }
    }

    /// SYSG-52: a working link's ends are working gates; a dormant one's
    /// say which end is faulted; a collapsed one's are wrecks.
    #[test]
    fn each_end_takes_its_links_state() {
        assert_eq!(
            gate_state(&link(LinkState::Working), 9),
            Some(GateState::Working)
        );
        let dormant = link(LinkState::Dormant {
            faulted: [true, false],
        });
        assert_eq!(
            gate_state(&dormant, 3),
            Some(GateState::Dormant { faulted_here: true })
        );
        assert_eq!(
            gate_state(&dormant, 9),
            Some(GateState::Dormant {
                faulted_here: false
            })
        );
        assert_eq!(gate_state(&link(LinkState::Collapsed), 3), None);
    }

    /// SYSG-53: a wreck is a salvage site named for its destination, with
    /// the key fact saying whose gate it was.
    #[test]
    fn a_wreck_is_a_salvage_site_named_for_its_destination() {
        let wreck = wreck("Ostra", 6.0e8, 1.0);
        assert_eq!(wreck.name, "Ostra gate wreck");
        assert_eq!(wreck.class, BodyClass::SalvageSite);
        assert_eq!(wreck.key_facts[0].1, "Wreck of the gate to Ostra");
        assert_eq!(wreck.orbit, OrbitPlacement::Own { radius_km: 6.0e8 });
    }
}
