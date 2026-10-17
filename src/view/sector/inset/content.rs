//! What a sector entry's inset says (SECT-21 to SECT-24), as plain values
//! gathered from the sector, so the words can be tested apart from the
//! panel. Figures go through the shared readout formats (UNIT-01,
//! UNIT-05); world kinds, atmospheres and water are named as the system
//! view's inset names them (INS-08).

use crate::content::generator::Sector;
use crate::content::generator::bodies::{BeltSummary, PlanetSummary, Water};
use crate::content::generator::catalogue::{Entry, EntryKind, StarSystem};
use crate::content::generator::network::{GatedSystem, Link, LinkState, Reason};
use crate::content::generator::system::belt_bodies::belt_names;
use crate::content::generator::system::worlds::{atmosphere, world_kind};
use crate::sim::units::KM_PER_AU;
use crate::view::inset::world_facts::{atmosphere_words, kind_words};
use crate::view::readout::{format_distance, format_light_years, significant};
use crate::view::sector::nodes::node_name;

/// A star's luminosity is shown to this many significant figures
/// (SECT-22), a planet's mass to this many (SECT-23).
const LUMINOSITY_FIGURES: i32 = 3;
const PLANET_MASS_FIGURES: i32 = 2;
/// The parts of a row's second line are set apart by this.
const PART: &str = " · ";

/// One row of a section: a first line and a second.
#[derive(Debug, Clone, PartialEq)]
pub struct InsetRow {
    pub title: String,
    pub detail: String,
}

/// One gate row (SECT-24): the entry it leads to, and its lines.
#[derive(Debug, Clone, PartialEq)]
pub struct GateRow {
    pub destination: usize,
    pub row: InsetRow,
}

/// Everything an entry's inset shows (SECT-21 to SECT-24).
#[derive(Debug, Clone, PartialEq)]
pub struct SectorInset {
    pub id: usize,
    /// The system's name, or the entry's designation.
    pub name: String,
    /// What it is (SECT-21).
    pub class_line: String,
    /// Whether it has an Enter system button: only a gated system does.
    pub enterable: bool,
    pub facts: Vec<(String, String)>,
    /// The "Planets" and "Belts" sections, for an entry with a summary.
    pub planets: Vec<InsetRow>,
    pub belts: Vec<InsetRow>,
    /// The "Gates" section, for a gated system.
    pub gates: Vec<GateRow>,
}

/// SECT-21: a reason in a word.
pub fn reason_words(reason: Reason) -> &'static str {
    match reason {
        Reason::Settlement => "Settlement",
        Reason::Extraction => "Extraction",
        Reason::Industry => "Industry",
        Reason::Science => "Science",
    }
}

/// SECT-21: what an entry is: "Gated system · *main reason*", "Ungated
/// system", "Brown dwarf" or "Giant".
pub fn class_line(entry: &Entry, gated: Option<&GatedSystem>) -> String {
    match (gated, &entry.kind) {
        (Some(gated), _) => format!("Gated system{PART}{}", reason_words(gated.main_reason)),
        (None, EntryKind::BrownDwarf { .. }) => "Brown dwarf".to_string(),
        (None, EntryKind::System(_)) if entry.is_giant() => "Giant".to_string(),
        (None, EntryKind::System(_)) => "Ungated system".to_string(),
    }
}

fn fact(label: &str, value: String) -> (String, String) {
    (label.to_string(), value)
}

/// SECT-22: the companions fact: "none", or how many and how far the
/// nearest is, in AU.
pub fn companions_words(system: &StarSystem) -> String {
    match system.nearest_companion_au() {
        None => "none".to_string(),
        Some(au) => format!(
            "{}, nearest at {}",
            system.companions.len(),
            format_distance(au * KM_PER_AU, 0.0)
        ),
    }
}

/// SECT-22: the age, metallicity and distance from the sector's centre
/// every entry has.
fn place_facts(entry: &Entry) -> [(String, String); 3] {
    [
        fact("Age", format!("{:.1} Gyr", entry.age_gyr)),
        fact("Metallicity", format!("[Fe/H] {:+.2}", entry.metallicity)),
        fact(
            "Distance from centre",
            format_light_years(entry.position_km.length()),
        ),
    ]
}

/// SECT-22: the key facts of an entry with a star: its stars, mass,
/// luminosity, age, metallicity, distance from the centre, companions
/// and any wide partner; a gated system adds its reason and, if it is
/// labelled so, that it is strategic. A brown dwarf, which has no star,
/// shows its mass, age, metallicity and distance.
pub fn key_facts(sector: &Sector, entry: &Entry) -> Vec<(String, String)> {
    let mut facts = Vec::new();
    match &entry.kind {
        EntryKind::System(system) => {
            let stars: Vec<&str> = std::iter::once(&system.primary)
                .chain(system.companions.iter().map(|c| &c.star))
                .map(|star| star.spectral_type.as_str())
                .collect();
            let primary = &system.primary;
            facts.push(fact("Star", stars.join(", ")));
            facts.push(fact("Mass", solar_masses(primary.mass_msun)));
            let luminosity = significant(primary.luminosity_lsun, LUMINOSITY_FIGURES);
            facts.push(fact(
                "Luminosity",
                format!("{luminosity} solar luminosities"),
            ));
            facts.extend(place_facts(entry));
            facts.push(fact("Companions", companions_words(system)));
            if let Some(partner) = system.wide_partner {
                facts.push(fact("Wide partner", node_name(sector, partner)));
            }
        }
        EntryKind::BrownDwarf { mass_msun } => {
            facts.push(fact("Mass", solar_masses(*mass_msun)));
            facts.extend(place_facts(entry));
        }
    }
    if let Some(gated) = sector.gated_system(entry.id) {
        facts.push(fact("Reason", reason_words(gated.main_reason).to_string()));
        if gated.strategic {
            facts.push(fact("Strategic", "yes".to_string()));
        }
    }
    facts
}

fn solar_masses(mass: f64) -> String {
    format!("{mass:.2} solar masses")
}

/// A planet's water, as a part of its row.
fn water_words(water: Water) -> &'static str {
    match water {
        Water::None => "no water",
        Water::Ice => "ice",
        Water::Surface => "surface water",
    }
}

/// SECT-23: a planet's row: its name and kind, then its orbit, mass,
/// gravity, equilibrium temperature, atmosphere and water.
pub fn planet_row(name: String, planet: &PlanetSummary, au_threshold_au: f64) -> InsetRow {
    let detail = [
        format_distance(planet.orbit_au * KM_PER_AU, au_threshold_au),
        format!(
            "{} Earth masses",
            significant(planet.mass_earth, PLANET_MASS_FIGURES)
        ),
        format!("{:.2} g", planet.gravity_g),
        format!("{:.0} K", planet.equilibrium_temperature_k),
        atmosphere_words(atmosphere(planet.atmosphere)).to_string(),
        water_words(planet.water).to_string(),
    ];
    InsetRow {
        title: format!("{name}{PART}{}", kind_words(world_kind(planet.kind))),
        detail: detail.join(PART),
    }
}

/// SECT-23: a belt's row: its name, then its inner and outer radius, its
/// mass in kg in scientific form, and its rocky, metallic and icy shares.
pub fn belt_row(name: String, belt: &BeltSummary, au_threshold_au: f64) -> InsetRow {
    let radius = |au: f64| format_distance(au * KM_PER_AU, au_threshold_au);
    let share = |fraction: f64| format!("{:.0}%", fraction * 100.0);
    let c = belt.composition;
    let detail = [
        format!("{} to {}", radius(belt.inner_au), radius(belt.outer_au)),
        format!("{:.1e} kg", belt.mass_kg),
        format!("{} rocky", share(c.rocky)),
        format!("{} metallic", share(c.metallic)),
        format!("{} icy", share(c.icy)),
    ];
    InsetRow {
        title: name,
        detail: detail.join(PART),
    }
}

/// SECT-24: which ends of a dormant link are faulted, seen from `id`:
/// "faulted here", "faulted there", "faulted at both ends", or nothing.
pub fn faulted_words(link: &Link, id: usize) -> Option<&'static str> {
    let LinkState::Dormant { faulted } = link.state else {
        return None;
    };
    let here = usize::from(link.ends[0] != id);
    match (faulted[here], faulted[1 - here]) {
        (true, true) => Some("faulted at both ends"),
        (true, false) => Some("faulted here"),
        (false, true) => Some("faulted there"),
        (false, false) => None,
    }
}

/// SECT-24: a gate row for one of `id`'s links: the destination's name,
/// then the link's state, which ends are faulted, and its length.
pub fn gate_row(sector: &Sector, link: &Link, id: usize) -> GateRow {
    let destination = link.other_end(id);
    let state = match link.state {
        LinkState::Working => "Working",
        LinkState::Dormant { .. } => "Dormant",
        LinkState::Collapsed => "Collapsed",
    };
    let parts: Vec<String> = [Some(state), faulted_words(link, id)]
        .into_iter()
        .flatten()
        .map(str::to_string)
        .chain(std::iter::once(format_light_years(link.length_km)))
        .collect();
    GateRow {
        destination,
        row: InsetRow {
            title: node_name(sector, destination),
            detail: parts.join(PART),
        },
    }
}

/// SECT-21 to SECT-24: everything the inset of the entry `id` shows.
pub fn sector_inset(sector: &Sector, id: usize, au_threshold_au: f64) -> SectorInset {
    let entry = &sector.entries[id];
    let gated = sector.gated_system(id);
    let summary = sector.summary(id);
    let planets = summary.map_or(Vec::new(), |s| {
        let name = |i: usize| match gated {
            Some(g) => g.names.planets[i].clone(),
            None => format!("Planet {}", i + 1),
        };
        let rows = s.planets.iter().enumerate();
        rows.map(|(i, p)| planet_row(name(i), p, au_threshold_au))
            .collect()
    });
    let belts = summary.map_or(Vec::new(), |s| {
        let names = match gated {
            Some(g) => belt_names(&g.names.system, s.belts.len()),
            None => (1..=s.belts.len()).map(|n| format!("Belt {n}")).collect(),
        };
        let rows = s.belts.iter().zip(names);
        rows.map(|(b, name)| belt_row(name, b, au_threshold_au))
            .collect()
    });
    let gates = match gated {
        Some(_) => sector
            .links_of(id)
            .map(|link| gate_row(sector, link, id))
            .collect(),
        None => Vec::new(),
    };
    SectorInset {
        id,
        name: node_name(sector, id),
        class_line: class_line(entry, gated),
        enterable: gated.is_some(),
        facts: key_facts(sector, entry),
        planets,
        belts,
        gates,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::generator::network::LinkOrigin;
    use crate::content::generator::testing::sector;

    const THRESHOLD_AU: f64 = 0.1;

    fn labels(inset: &SectorInset) -> Vec<&str> {
        inset
            .facts
            .iter()
            .map(|(label, _)| label.as_str())
            .collect()
    }

    /// SECT-21: the header names a gated system by its name and says
    /// why it is gated; any other entry is named by its designation and
    /// called an ungated system, a brown dwarf or a giant. Only a gated
    /// system can be entered.
    #[test]
    fn the_header_names_the_entry_and_its_class() {
        let sector = sector(30, 11);
        let gated = &sector.gated[0];
        let inset = sector_inset(&sector, gated.id, THRESHOLD_AU);
        assert_eq!(inset.name, gated.names.system);
        let reason = reason_words(gated.main_reason);
        assert_eq!(inset.class_line, format!("Gated system · {reason}"));
        assert!(inset.enterable);
        for entry in &sector.entries {
            if sector.gated_system(entry.id).is_some() {
                continue;
            }
            let inset = sector_inset(&sector, entry.id, THRESHOLD_AU);
            assert_eq!(inset.name, entry.designation);
            assert!(!inset.enterable);
            let expected = match entry.kind {
                EntryKind::BrownDwarf { .. } => "Brown dwarf",
                _ if entry.is_giant() => "Giant",
                _ => "Ungated system",
            };
            assert_eq!(inset.class_line, expected);
        }
    }

    /// SECT-22: an entry with a star lists its stars, mass, luminosity,
    /// age, metallicity, distance from the centre and companions, in
    /// that order and in their units; a gated system adds its reason.
    /// SECT-13: the distance, stored in km, is shown in light years.
    #[test]
    fn key_facts_follow_the_spec_order_and_units() {
        let sector = sector(30, 11);
        let gated = &sector.gated[0];
        let inset = sector_inset(&sector, gated.id, THRESHOLD_AU);
        let order = [
            "Star",
            "Mass",
            "Luminosity",
            "Age",
            "Metallicity",
            "Distance from centre",
            "Companions",
        ];
        assert_eq!(&labels(&inset)[..order.len()], order);
        assert!(labels(&inset).contains(&"Reason"));
        let value = |label: &str| {
            inset
                .facts
                .iter()
                .find(|(l, _)| l == label)
                .map(|(_, v)| v.clone())
                .unwrap()
        };
        let entry = &sector.entries[gated.id];
        let primary = entry.primary().unwrap();
        assert!(value("Star").starts_with(&primary.spectral_type));
        assert_eq!(
            value("Mass"),
            format!("{:.2} solar masses", primary.mass_msun)
        );
        assert!(value("Luminosity").ends_with("solar luminosities"));
        assert_eq!(value("Age"), format!("{:.1} Gyr", entry.age_gyr));
        assert!(value("Metallicity").starts_with("[Fe/H] "));
        assert_eq!(
            value("Distance from centre"),
            format_light_years(entry.position_km.length())
        );
        assert!(value("Distance from centre").ends_with(" ly"));
        assert_eq!(value("Reason"), reason_words(gated.main_reason));
        let strategic = sector.gated.iter().find(|g| g.strategic);
        if let Some(strategic) = strategic {
            let inset = sector_inset(&sector, strategic.id, THRESHOLD_AU);
            assert!(inset.facts.contains(&fact("Strategic", "yes".to_string())));
        }
        let not_strategic = sector.gated.iter().find(|g| !g.strategic).unwrap();
        let inset = sector_inset(&sector, not_strategic.id, THRESHOLD_AU);
        assert!(!labels(&inset).contains(&"Strategic"));
    }

    /// SECT-22: companions read "none", or their number and the nearest
    /// separation in AU; a wide partner is named.
    #[test]
    fn companions_and_wide_partners_are_described() {
        let sector = (0..20)
            .map(|seed| sector(30, seed))
            .find(|s| {
                s.entries
                    .iter()
                    .any(|e| e.system().is_some_and(|s| !s.companions.is_empty()))
                    && s.entries
                        .iter()
                        .any(|e| e.system().is_some_and(|s| s.wide_partner.is_some()))
            })
            .expect("a sector with a companion and a wide pair");
        for entry in &sector.entries {
            let Some(system) = entry.system() else {
                continue;
            };
            let words = companions_words(system);
            match system.companions.len() {
                0 => assert_eq!(words, "none"),
                n => {
                    assert!(words.starts_with(&format!("{n}, nearest at ")), "{words}");
                    assert!(words.ends_with(" AU"), "{words}");
                }
            }
            let facts = key_facts(&sector, entry);
            let partner = facts.iter().find(|(l, _)| l == "Wide partner");
            match system.wide_partner {
                Some(id) => assert_eq!(partner.unwrap().1, node_name(&sector, id)),
                None => assert!(partner.is_none()),
            }
        }
    }

    /// SECT-23: a gated system's planets are listed in orbit order under
    /// their names with their kind, then orbit, mass, gravity,
    /// temperature, atmosphere and water; an ungated system's are
    /// "Planet *n*". Belts list their radii, mass and shares.
    #[test]
    fn planets_and_belts_are_listed_in_orbit_order() {
        let sector = sector(30, 11);
        let gated = sector
            .gated
            .iter()
            .find(|g| sector.summary(g.id).is_some_and(|s| !s.planets.is_empty()))
            .unwrap();
        let summary = sector.summary(gated.id).unwrap();
        let inset = sector_inset(&sector, gated.id, THRESHOLD_AU);
        assert_eq!(inset.planets.len(), summary.planets.len());
        let first = &summary.planets[0];
        let row = &inset.planets[0];
        assert!(
            row.title.starts_with(&gated.names.planets[0]),
            "{}",
            row.title
        );
        assert!(row.title.ends_with(kind_words(world_kind(first.kind))));
        let detail = &row.detail;
        assert!(
            detail.contains(&format!("{:.2} g", first.gravity_g)),
            "{detail}"
        );
        assert!(detail.contains(&format!("{:.0} K", first.equilibrium_temperature_k)));
        assert!(detail.contains("Earth masses"));
        let ungated = sector
            .entries
            .iter()
            .find(|e| {
                sector.gated_system(e.id).is_none()
                    && sector.summary(e.id).is_some_and(|s| !s.planets.is_empty())
            })
            .unwrap();
        let inset = sector_inset(&sector, ungated.id, THRESHOLD_AU);
        assert!(inset.planets[0].title.starts_with("Planet 1 · "));
        assert!(inset.gates.is_empty());

        let belt = BeltSummary {
            inner_au: 2.1,
            outer_au: 3.4,
            mass_kg: 2.345e21,
            composition: crate::content::generator::bodies::BeltComposition {
                rocky: 0.6,
                metallic: 0.25,
                icy: 0.15,
            },
        };
        let row = belt_row("Belt 1".to_string(), &belt, THRESHOLD_AU);
        assert_eq!(
            row.detail,
            "2.1 AU to 3.4 AU · 2.3e21 kg · 60% rocky · 25% metallic · 15% icy"
        );
    }

    /// SECT-24: one gate row per link of a gated system, naming the
    /// destination, the state, any faulted ends seen from this end, and
    /// the length in light years.
    #[test]
    fn gate_rows_name_the_destination_state_faults_and_length() {
        let sector = sector(30, 11);
        let gated = &sector.gated[0];
        let inset = sector_inset(&sector, gated.id, THRESHOLD_AU);
        assert_eq!(inset.gates.len(), sector.links_of(gated.id).count());
        for (row, link) in inset.gates.iter().zip(sector.links_of(gated.id)) {
            assert_eq!(row.destination, link.other_end(gated.id));
            assert_eq!(row.row.title, node_name(&sector, row.destination));
            assert!(
                row.row
                    .detail
                    .ends_with(&format_light_years(link.length_km))
            );
        }
        let link = |faulted| Link {
            ends: [3, 8],
            length_km: 9.4607e13,
            origin: LinkOrigin::Spanning,
            state: LinkState::Dormant { faulted },
        };
        assert_eq!(faulted_words(&link([true, false]), 3), Some("faulted here"));
        assert_eq!(
            faulted_words(&link([true, false]), 8),
            Some("faulted there")
        );
        assert_eq!(
            faulted_words(&link([true, true]), 8),
            Some("faulted at both ends")
        );
        assert_eq!(faulted_words(&link([false, false]), 3), None);
        let row = gate_row(&sector, &link([false, true]), 3);
        assert_eq!(row.destination, 8);
        assert_eq!(row.row.detail, "Dormant · faulted there · 10.0 ly");
    }
}
