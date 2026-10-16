//! The types a system's summary is made of: its template, its planets and
//! their facts, and its belts (SGEN-30 to SGEN-43). A summary is the coarse
//! skeleton of a system; the summaries stage (`summaries/`) makes one for
//! every system, and detail for a gated system refines it without
//! re-rolling it. Composition is a class only: no resource amounts
//! (SGEN-43).

/// A system's architecture (SGEN-31).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Template {
    /// Three to seven small planets packed close in, as round many M
    /// dwarfs.
    CompactMultiPlanet,
    /// Rocky planets inside the snow line, as in the solar system.
    SolarLike,
    /// A gas giant on a period of days that has cleared the inner system.
    HotGiant,
    /// Few planets and heavy belts.
    DebrisDisc,
    /// What survives round a white dwarf: a few giants and super-Earths
    /// far out, and debris.
    WhiteDwarfRemnant,
}

/// A planet's kind (SGEN-32).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlanetKind {
    Rocky,
    SuperEarth,
    SubNeptune,
    IceGiant,
    GasGiant,
}

impl PlanetKind {
    /// Ice and gas giants are the giants (SGEN-32).
    pub fn is_giant(self) -> bool {
        matches!(self, PlanetKind::IceGiant | PlanetKind::GasGiant)
    }

    /// Rocky planets and super-Earths have a solid surface and may have an
    /// atmosphere and water (SGEN-40, SGEN-41); the other kinds have an
    /// envelope.
    pub fn is_rocky(self) -> bool {
        matches!(self, PlanetKind::Rocky | PlanetKind::SuperEarth)
    }
}

/// A planet's atmosphere class (SGEN-40).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Atmosphere {
    Airless,
    Thin,
    Dense,
    /// The deep hydrogen and helium of a sub-Neptune or a giant.
    Envelope,
}

/// A planet's water (SGEN-41).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Water {
    None,
    Ice,
    Surface,
}

/// One planet of a summary, with its facts (SGEN-32 to SGEN-41).
#[derive(Debug, Clone, PartialEq)]
pub struct PlanetSummary {
    pub kind: PlanetKind,
    /// In Earth masses.
    pub mass_earth: f64,
    /// In Earth radii (SGEN-36).
    pub radius_earth: f64,
    /// Its circular orbit round the primary, in AU.
    pub orbit_au: f64,
    /// Surface gravity, in units of standard gravity (SGEN-38).
    pub gravity_g: f64,
    /// Equilibrium temperature, in K (SGEN-39).
    pub equilibrium_temperature_k: f64,
    pub atmosphere: Atmosphere,
    pub water: Water,
}

/// The fractions of a belt's mass that are rocky, metallic and icy; they
/// sum to 1 (SGEN-37).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BeltComposition {
    pub rocky: f64,
    pub metallic: f64,
    pub icy: f64,
}

/// One belt of a summary: an annulus round the primary (SGEN-37).
#[derive(Debug, Clone, PartialEq)]
pub struct BeltSummary {
    /// In AU.
    pub inner_au: f64,
    /// In AU.
    pub outer_au: f64,
    pub mass_kg: f64,
    pub composition: BeltComposition,
}

impl BeltSummary {
    /// Whether an orbit of this radius, in AU, lies inside the annulus.
    pub fn contains(&self, radius_au: f64) -> bool {
        (self.inner_au..=self.outer_au).contains(&radius_au)
    }

    /// Whether two belts share any radius.
    pub fn overlaps(&self, other: &BeltSummary) -> bool {
        self.inner_au <= other.outer_au && other.inner_au <= self.outer_au
    }
}

/// A system's coarse description (SGEN-30).
#[derive(Debug, Clone, PartialEq)]
pub struct Summary {
    pub template: Template,
    /// The snow line, in AU (SGEN-42).
    pub snow_line_au: f64,
    /// The habitable zone's inner edge, in AU (SGEN-42).
    pub habitable_zone_inner_au: f64,
    /// The habitable zone's outer edge, in AU (SGEN-42).
    pub habitable_zone_outer_au: f64,
    /// Its planets, innermost first.
    pub planets: Vec<PlanetSummary>,
    /// Its belts, innermost first; at most three.
    pub belts: Vec<BeltSummary>,
}
