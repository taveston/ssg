//! The names stage (SGEN-80 to SGEN-82): unique placeholder names for
//! each gated system, its planets and spare names for its belts' large
//! bodies, made from the stems and endings of the name file (`list`).
//! Ungated entries keep only their designation. Moons and belts are named
//! later from these, by the system generator.

pub mod list;

use std::collections::HashSet;

use super::bodies::Summary;
use super::gates::GateRecord;
use super::network::SystemNames;
use super::rng::Rng;
use super::tunables::GeneratorTunables;
use list::NameList;

/// Once two-part names stop coming out new, three-part names are drawn up
/// to this many times; the name file's three-part names number in the
/// hundreds of thousands, so running out means the file is far too
/// small.
const THREE_PART_TRIES: usize = 1_000_000;

/// The names made so far for a sector, and the parts to make more from.
pub struct Namer<'a> {
    list: &'a NameList,
    /// Every name made or reserved, in lower case (SGEN-81).
    taken: HashSet<String>,
    max_retries: u32,
}

/// `text` with its first letter in upper case and the rest in lower.
pub fn capitalised(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first
            .to_uppercase()
            .chain(chars.flat_map(char::to_lowercase))
            .collect(),
        None => String::new(),
    }
}

impl<'a> Namer<'a> {
    pub fn new(list: &'a NameList, t: &GeneratorTunables) -> Self {
        Self {
            list,
            taken: list.reserved.iter().cloned().collect(),
            max_retries: t.max_name_retries,
        }
    }

    fn pick<'b>(parts: &'b [String], rng: &mut Rng) -> &'b str {
        &parts[rng.below(parts.len())]
    }

    /// A new name (SGEN-80, SGEN-81): a stem and an ending, capitalised,
    /// unique in the sector ignoring case and not reserved. If
    /// `max_name_retries` draws in a row repeat a name, it is a stem, a
    /// stem and an ending instead.
    pub fn name(&mut self, rng: &mut Rng) -> String {
        for _ in 0..self.max_retries {
            let name = format!(
                "{}{}",
                Self::pick(&self.list.stems, rng),
                Self::pick(&self.list.endings, rng)
            );
            if self.taken.insert(name.clone()) {
                return capitalised(&name);
            }
        }
        for _ in 0..THREE_PART_TRIES {
            let name = format!(
                "{}{}{}",
                Self::pick(&self.list.stems, rng),
                Self::pick(&self.list.stems, rng),
                Self::pick(&self.list.endings, rng)
            );
            if self.taken.insert(name.clone()) {
                return capitalised(&name);
            }
        }
        panic!("the name file has too few stems and endings to name a sector");
    }

    /// `count` new names.
    fn names(&mut self, count: usize, rng: &mut Rng) -> Vec<String> {
        (0..count).map(|_| self.name(rng)).collect()
    }
}

/// Runs the names stage (SGEN-82): for each gated system in id order, a
/// name for the system, one for each planet of its summary, then
/// `belt_body_max` spare names for each of its belts, all from the
/// stage's own random stream.
pub fn run(
    gated: &[GateRecord],
    summaries: &[Option<Summary>],
    list: &NameList,
    rng: &mut Rng,
    t: &GeneratorTunables,
) -> Vec<SystemNames> {
    let mut namer = Namer::new(list, t);
    gated
        .iter()
        .map(|system| {
            let summary = summaries[system.id]
                .as_ref()
                .expect("gated systems have summaries");
            SystemNames {
                system: namer.name(rng),
                planets: namer.names(summary.planets.len(), rng),
                belt_bodies: summary
                    .belts
                    .iter()
                    .map(|_| namer.names(t.belt_body_max as usize, rng))
                    .collect(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_list() -> NameList {
        NameList::parse("[stems]\nbel\ncor\n[endings]\nane\n[reserved]\nCorane\n").unwrap()
    }

    /// SGEN-80: a name is a stem and an ending, capitalised.
    #[test]
    fn names_are_a_capitalised_stem_and_ending() {
        let t = GeneratorTunables::default();
        let list = NameList::bundled();
        let mut namer = Namer::new(list, &t);
        let mut rng = Rng::new(80);
        for _ in 0..200 {
            let name = namer.name(&mut rng);
            let lower = name.to_lowercase();
            assert!(name.chars().next().unwrap().is_uppercase());
            assert_eq!(name[1..], lower[1..]);
            let split = list.stems.iter().any(|stem| {
                lower
                    .strip_prefix(stem.as_str())
                    .is_some_and(|rest| list.endings.iter().any(|e| e == rest))
            });
            assert!(split, "{name}");
        }
        assert_eq!(capitalised("bELANE"), "Belane");
    }

    /// SGEN-81: names never repeat, ignoring case, and never take a
    /// reserved one; when two-part names run out, three-part ones follow.
    #[test]
    fn names_are_unique_and_never_reserved() {
        let t = GeneratorTunables::default();
        let list = tiny_list();
        let mut namer = Namer::new(&list, &t);
        let mut rng = Rng::new(81);
        // Two stems and one ending give two two-part names, one reserved.
        let first = namer.name(&mut rng);
        assert_eq!(first, "Belane");
        let names: Vec<String> = (0..3).map(|_| namer.name(&mut rng)).collect();
        for name in &names {
            assert!(["Belbelane", "Belcorane", "Corbelane", "Corcorane"].contains(&name.as_str()));
        }
        let mut all: Vec<String> = names.iter().map(|n| n.to_lowercase()).collect();
        all.push(first.to_lowercase());
        all.sort();
        all.dedup();
        assert_eq!(all.len(), 4);
        assert!(!all.contains(&"corane".to_string()));
    }
}
