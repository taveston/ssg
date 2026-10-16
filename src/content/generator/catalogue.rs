//! The catalogue's types: entries, their stars and companions (SGEN-20 to
//! SGEN-29). An **entry** is one place in the catalogue: a star system
//! (one or more bound stars), a brown dwarf, or a giant. The stars stage
//! (`stars.rs`) fills them in.

use bevy::math::DVec3;

/// The class of a star (SGEN-21). The main-sequence classes run from the
/// least massive, M, to the most, A.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StarClass {
    M,
    K,
    G,
    F,
    A,
    WhiteDwarf,
    Giant,
}

impl StarClass {
    /// Every class, in the order of the star mix (SGEN-21).
    pub const ALL: [StarClass; 7] = [
        StarClass::M,
        StarClass::K,
        StarClass::G,
        StarClass::F,
        StarClass::A,
        StarClass::WhiteDwarf,
        StarClass::Giant,
    ];

    /// The main-sequence classes a companion can take, least massive
    /// first (SGEN-25).
    pub const MAIN_SEQUENCE: [StarClass; 5] = [
        StarClass::M,
        StarClass::K,
        StarClass::G,
        StarClass::F,
        StarClass::A,
    ];
}

/// One star, with its properties in solar units (SGEN-22).
#[derive(Debug, Clone, PartialEq)]
pub struct Star {
    pub class: StarClass,
    /// In solar masses.
    pub mass_msun: f64,
    /// In solar luminosities.
    pub luminosity_lsun: f64,
    /// In solar radii.
    pub radius_rsun: f64,
    /// Effective (surface) temperature, in K.
    pub temperature_k: f64,
    /// For example "G2V" or "M4V"; a white dwarf's is "D" and a digit
    /// ("D3"), a giant's ends in "III" ("K2III") (SGEN-22).
    pub spectral_type: String,
}

/// A star bound to a system's primary (SGEN-25, SGEN-26).
#[derive(Debug, Clone, PartialEq)]
pub struct Companion {
    pub star: Star,
    /// Its distance from the primary, in AU.
    pub separation_au: f64,
    /// The entry's designation with a letter: "OW-0173 B" (SGEN-20).
    pub designation: String,
}

/// A star system: a primary star and any companions (SGEN-25).
#[derive(Debug, Clone, PartialEq)]
pub struct StarSystem {
    pub primary: Star,
    /// Bound companions, nearest first.
    pub companions: Vec<Companion>,
    /// The other member of a wide pair, by entry id (SGEN-27).
    pub wide_partner: Option<usize>,
}

impl StarSystem {
    /// The separation of the nearest companion, in AU, if there is one.
    pub fn nearest_companion_au(&self) -> Option<f64> {
        self.companions.first().map(|c| c.separation_au)
    }
}

/// What kind of catalogue entry this is.
#[derive(Debug, Clone, PartialEq)]
pub enum EntryKind {
    /// A star system. A system whose primary is a giant is a giant: it is
    /// in the catalogue only (SGEN-28).
    System(StarSystem),
    /// A brown dwarf, with its mass in solar masses (SGEN-28).
    BrownDwarf { mass_msun: f64 },
}

/// One place in the catalogue (SGEN-20).
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// Its place in the catalogue, from 0.
    pub id: usize,
    /// "OW-0173" (SGEN-20).
    pub designation: String,
    /// Its position from the slab's centre, in km (SGEN-12).
    pub position_km: DVec3,
    /// In Gyr (SGEN-23).
    pub age_gyr: f64,
    /// [Fe/H] (SGEN-24).
    pub metallicity: f64,
    pub kind: EntryKind,
    /// Whether it can carry a gate (SGEN-29).
    pub eligible: bool,
}

impl Entry {
    /// The star system, unless this is a brown dwarf.
    pub fn system(&self) -> Option<&StarSystem> {
        match &self.kind {
            EntryKind::System(system) => Some(system),
            EntryKind::BrownDwarf { .. } => None,
        }
    }

    /// The primary star, unless this is a brown dwarf.
    pub fn primary(&self) -> Option<&Star> {
        self.system().map(|system| &system.primary)
    }

    /// Whether this is a system with a giant primary (SGEN-28).
    pub fn is_giant(&self) -> bool {
        self.primary()
            .is_some_and(|star| star.class == StarClass::Giant)
    }

    /// Whether this entry gets a summary: a system that is not a giant
    /// (SGEN-30).
    pub fn has_summary(&self) -> bool {
        self.system().is_some() && !self.is_giant()
    }
}
