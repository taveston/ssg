//! Every number the sector generator spec marks tunable, in one plain
//! struct passed to the generator (SGEN-05, ARCH-22). It is not a Bevy
//! resource: the generator is plain Rust and takes it as an argument.
//!
//! The tunables table at the end of `docs/spec/sector-generator.md` lists
//! them with the requirements that use them. Fields are grouped by stage,
//! in the table's order. Several rows of the table are small tables of
//! their own (a share per star class, a range per planet kind); those use
//! the helper types below.

use super::bodies::{PlanetKind, Template};
use super::catalogue::StarClass;

/// A range of values, from `min` to `max`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Span {
    pub min: f64,
    pub max: f64,
}

impl Span {
    pub const fn new(min: f64, max: f64) -> Self {
        Self { min, max }
    }

    /// Whether `value` lies in the range, ends included.
    pub fn contains(&self, value: f64) -> bool {
        (self.min..=self.max).contains(&value)
    }
}

/// A whole-number range, both ends included.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CountSpan {
    pub min: u32,
    pub max: u32,
}

impl CountSpan {
    pub const fn new(min: u32, max: u32) -> Self {
        Self { min, max }
    }
}

/// One value for each class of star.
#[derive(Debug, Clone, PartialEq)]
pub struct ByClass<T> {
    pub m: T,
    pub k: T,
    pub g: T,
    pub f: T,
    pub a: T,
    pub white_dwarf: T,
    pub giant: T,
}

impl<T> ByClass<T> {
    /// The value for `class`.
    pub fn get(&self, class: StarClass) -> &T {
        match class {
            StarClass::M => &self.m,
            StarClass::K => &self.k,
            StarClass::G => &self.g,
            StarClass::F => &self.f,
            StarClass::A => &self.a,
            StarClass::WhiteDwarf => &self.white_dwarf,
            StarClass::Giant => &self.giant,
        }
    }
}

/// One value for each kind of planet.
#[derive(Debug, Clone, PartialEq)]
pub struct ByKind<T> {
    pub rocky: T,
    pub super_earth: T,
    pub sub_neptune: T,
    pub ice_giant: T,
    pub gas_giant: T,
}

impl<T> ByKind<T> {
    /// The value for `kind`.
    pub fn get(&self, kind: PlanetKind) -> &T {
        match kind {
            PlanetKind::Rocky => &self.rocky,
            PlanetKind::SuperEarth => &self.super_earth,
            PlanetKind::SubNeptune => &self.sub_neptune,
            PlanetKind::IceGiant => &self.ice_giant,
            PlanetKind::GasGiant => &self.gas_giant,
        }
    }
}

/// A main-sequence star's luminosity from its mass, in solar units, as
/// three power laws: `low_factor` × M^`low_exponent` below `low_below`,
/// M^`mid_exponent` up to `high_above`, and `high_factor` ×
/// M^`high_exponent` above it (SGEN-22).
#[derive(Debug, Clone, PartialEq)]
pub struct MassLuminosity {
    pub low_factor: f64,
    pub low_exponent: f64,
    pub low_below: f64,
    pub mid_exponent: f64,
    pub high_above: f64,
    pub high_factor: f64,
    pub high_exponent: f64,
}

/// A main-sequence star's radius from its mass, in solar units:
/// M^`low_exponent` below `break_msun`, M^`high_exponent` above (SGEN-22).
#[derive(Debug, Clone, PartialEq)]
pub struct MassRadius {
    pub break_msun: f64,
    pub low_exponent: f64,
    pub high_exponent: f64,
}

/// A giant star's radius and luminosity ranges (SGEN-22).
#[derive(Debug, Clone, PartialEq)]
pub struct GiantProperties {
    pub radius_rsun: Span,
    pub luminosity_lsun: Span,
}

/// A log-normal spread: the base-10 logarithm of the value has this mean
/// and spread (standard deviation).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Log10Normal {
    pub mean: f64,
    pub spread: f64,
}

/// Companion separations: one spread for M-dwarf primaries and one for
/// every other class (SGEN-26).
#[derive(Debug, Clone, PartialEq)]
pub struct SeparationLog10 {
    pub m: Log10Normal,
    pub other: Log10Normal,
}

/// Where a template puts its planets (SGEN-33).
#[derive(Debug, Clone, PartialEq)]
pub enum OrbitRule {
    /// The innermost planet's period is drawn log-uniformly from
    /// `first_period_days`; each next period is the one before times a
    /// factor drawn uniformly from `period_ratio`. None lies beyond
    /// `outer_snow_lines` times the snow line.
    PeriodChain {
        first_period_days: Span,
        period_ratio: Span,
        outer_snow_lines: f64,
    },
    /// Orbits drawn log-uniformly between these multiples of the snow
    /// line.
    SnowLines(Span),
    /// Orbits drawn log-uniformly between these radii, in AU.
    Au(Span),
}

/// How one template builds a system (SGEN-33, SGEN-37).
#[derive(Debug, Clone, PartialEq)]
pub struct TemplateRule {
    /// A single gas giant on a short period, placed before the other
    /// planets: the hot giant's own planet. Its period in days is drawn
    /// log-uniformly from this range.
    pub hot_giant_period_days: Option<Span>,
    /// How many planets the template draws (besides any hot giant).
    pub planet_count: CountSpan,
    /// The kinds those planets take, with their weights.
    pub kinds: Vec<(PlanetKind, f64)>,
    /// Where those planets go.
    pub orbits: OrbitRule,
    /// How many belts the system gets (SGEN-37).
    pub belt_count: CountSpan,
    /// Whether the system may also get cold giants (SGEN-35).
    pub cold_giants: bool,
}

/// One rule for each template.
#[derive(Debug, Clone, PartialEq)]
pub struct TemplateRules {
    pub compact: TemplateRule,
    pub solar_like: TemplateRule,
    pub hot_giant: TemplateRule,
    pub debris_disc: TemplateRule,
    pub remnant: TemplateRule,
}

impl TemplateRules {
    /// The rule for `template`.
    pub fn get(&self, template: Template) -> &TemplateRule {
        match template {
            Template::CompactMultiPlanet => &self.compact,
            Template::SolarLike => &self.solar_like,
            Template::HotGiant => &self.hot_giant,
            Template::DebrisDisc => &self.debris_disc,
            Template::WhiteDwarfRemnant => &self.remnant,
        }
    }
}

/// The chance of cold giants, by the primary's class group (SGEN-35).
#[derive(Debug, Clone, PartialEq)]
pub struct ColdGiantChance {
    /// F, G, K and A primaries.
    pub fgka: f64,
    /// M-dwarf primaries.
    pub m: f64,
}

/// A planet's radius from its mass, in Earth units:
/// `radius_at_reference` × (M ÷ `reference_mass`)^`exponent`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PowerLaw {
    pub radius_at_reference: f64,
    pub reference_mass: f64,
    pub exponent: f64,
}

impl PowerLaw {
    /// The radius this law gives a planet of `mass`.
    pub fn radius(&self, mass: f64) -> f64 {
        self.radius_at_reference * (mass / self.reference_mass).powf(self.exponent)
    }
}

/// The mass-radius curves (SGEN-36). Rocky planets and super-Earths share
/// one curve. The gas-giant curve stops rising at its reference mass (one
/// Jupiter mass): heavier giants keep that radius.
#[derive(Debug, Clone, PartialEq)]
pub struct MassRadiusCurves {
    pub rocky: PowerLaw,
    pub sub_neptune: PowerLaw,
    pub ice_giant: PowerLaw,
    pub gas_giant: PowerLaw,
}

/// A belt's composition by its side of the snow line (SGEN-37). The rocky
/// fraction is what remains.
#[derive(Debug, Clone, PartialEq)]
pub struct BeltCompositionRule {
    /// Inside the snow line: the icy fraction is drawn up to this.
    pub inside_icy_max: f64,
    /// Inside the snow line: the metallic fraction's range.
    pub inside_metallic: Span,
    /// Beyond the snow line: the icy fraction's range.
    pub beyond_icy: Span,
    /// Beyond the snow line: the metallic fraction is drawn up to this.
    pub beyond_metallic_max: f64,
}

/// Belt masses in kg, by side of the snow line, and round white dwarfs
/// whatever the side (SGEN-37).
#[derive(Debug, Clone, PartialEq)]
pub struct BeltMasses {
    pub inside: Span,
    pub beyond: Span,
    pub white_dwarf: Span,
}

/// The equilibrium temperatures, in K, at which surface water is possible,
/// by atmosphere (SGEN-41).
#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceWaterTemperatures {
    pub dense: Span,
    pub thin: Span,
}

/// A fit that takes four values: 0 below the first and above the last, 1
/// between the middle two, linear between (SGEN-50).
pub type FourPointFit = [f64; 4];

/// The value of each kind of unusual object to science (SGEN-53).
#[derive(Debug, Clone, PartialEq)]
pub struct ScienceValues {
    pub white_dwarf: f64,
    pub young_system: f64,
    pub flare_star: f64,
    pub debris_disc: f64,
    pub hot_giant: f64,
    pub extreme_metallicity: f64,
}

/// Every tunable of the sector generator (SGEN-05).
#[derive(Debug, Clone, PartialEq)]
pub struct GeneratorTunables {
    // --- Settings ---
    /// The number of gated systems of the Small preset (SGEN-01).
    pub preset_small: u32,
    /// The number of gated systems of the Medium preset (SGEN-01).
    pub preset_medium: u32,
    /// The number of gated systems of the Large preset (SGEN-01).
    pub preset_large: u32,
    /// The fewest gated systems the generator accepts. Below the Small
    /// preset is for development and tests only (SGEN-01, SGEN-91).
    pub min_gated_count: u32,
    /// The most gated systems the generator accepts (SGEN-01, SGEN-92).
    pub max_gated_count: u32,

    // --- Stage 1: volume ---
    /// Star systems in the sector for every gated one (SGEN-10).
    pub stars_per_gated: f64,
    /// The share of systems that are members of a wide pair; each pair is
    /// two systems on one site (SGEN-10, SGEN-27).
    pub wide_pair_fraction: f64,
    /// Brown dwarfs in the catalogue for every star system (SGEN-10).
    pub brown_dwarf_per_system: f64,
    /// Star systems per cubic parsec (SGEN-11).
    pub system_density_per_pc3: f64,
    /// The slab's thickness as a share of its width (SGEN-11).
    pub slab_thickness_ratio: f64,
    /// The plan's ellipse is stretched by a factor drawn from 1 to this
    /// (SGEN-12).
    pub plan_elongation_max: f64,
    /// Sites are placed at least this far apart, in parsecs (SGEN-12).
    pub min_system_separation_pc: f64,

    // --- Stage 2: stars ---
    /// The prefix of every designation, as in "OW-0173" (SGEN-20).
    pub designation_prefix: String,
    /// The share of primaries of each class (SGEN-21).
    pub star_mix: ByClass<f64>,
    /// Each class's mass range, in solar masses; masses are drawn
    /// log-uniformly from it (SGEN-22).
    pub star_mass_ranges: ByClass<Span>,
    /// A main-sequence star's luminosity from its mass (SGEN-22).
    pub star_mass_luminosity: MassLuminosity,
    /// A main-sequence star's radius from its mass (SGEN-22).
    pub star_mass_radius: MassRadius,
    /// A white dwarf's radius, in solar radii (SGEN-22).
    pub white_dwarf_radius_rsun: f64,
    /// A white dwarf's luminosity range, in solar units, drawn
    /// log-uniformly (SGEN-22).
    pub white_dwarf_luminosity_lsun: Span,
    /// A giant's radius and luminosity ranges (SGEN-22).
    pub giant_properties: GiantProperties,
    /// The youngest age a system is drawn with, in Gyr (SGEN-23).
    pub age_min_gyr: f64,
    /// The oldest age a system is drawn with, in Gyr (SGEN-23).
    pub age_max_gyr: f64,
    /// The youngest a white dwarf can be, in Gyr: its progenitor had to
    /// live and die first (SGEN-23).
    pub white_dwarf_age_min_gyr: f64,
    /// The mean metallicity, [Fe/H] (SGEN-24).
    pub metallicity_mean: f64,
    /// The spread of metallicity around the mean (SGEN-24).
    pub metallicity_sigma: f64,
    /// Metallicity is clamped to plus or minus this (SGEN-24).
    pub metallicity_clamp: f64,
    /// The chance that a system is multiple, by its primary's class
    /// (SGEN-25).
    pub multiplicity_by_class: ByClass<f64>,
    /// How many stars a multiple system has, with weights (SGEN-25).
    pub star_count_weights: Vec<(u32, f64)>,
    /// Companion separations (SGEN-26).
    pub separation_log10_au: SeparationLog10,
    /// The closest a companion can be, in AU (SGEN-26).
    pub companion_min_au: f64,
    /// Companions lie inside this many AU; two systems this far apart or
    /// more are a wide pair, two entries (SGEN-26, SGEN-27).
    pub wide_pair_min_au: f64,
    /// The widest a wide pair can be, in AU (SGEN-27).
    pub wide_pair_max_au: f64,
    /// A brown dwarf's mass range, in solar masses (SGEN-28).
    pub brown_dwarf_mass_msun: Span,

    // --- Stage 3: summaries ---
    /// Planets lie inside this share of the nearest companion's
    /// separation (SGEN-30).
    pub companion_truncation_fraction: f64,
    /// The weight of each template, by the primary's class (SGEN-31).
    pub template_weights: ByClass<Vec<(Template, f64)>>,
    /// Each planet kind's mass range, in Earth masses (SGEN-32).
    pub planet_kind_masses: ByKind<Span>,
    /// How each template builds a system (SGEN-33, SGEN-37).
    pub template_rules: TemplateRules,
    /// Neighbouring planets lie at least this many mutual Hill radii
    /// apart (SGEN-34).
    pub min_mutual_hill: f64,
    /// The base chance of cold giants (SGEN-35).
    pub cold_giant_chance: ColdGiantChance,
    /// Giants grow likelier by 10^(this × [Fe/H]) (SGEN-35).
    pub giant_metallicity_exponent: f64,
    /// The chance of cold giants is never above this (SGEN-35).
    pub cold_giant_chance_cap: f64,
    /// How many cold giants a system that has them gets, with weights
    /// (SGEN-35).
    pub cold_giant_count_weights: Vec<(u32, f64)>,
    /// The share of cold giants that are gas giants; the rest are ice
    /// giants (SGEN-35).
    pub cold_giant_gas_share: f64,
    /// The innermost cold giant lies between these multiples of the snow
    /// line (SGEN-35).
    pub cold_giant_orbit_snow_lines: Span,
    /// Planet radius from mass, by kind (SGEN-36).
    pub mass_radius: MassRadiusCurves,
    /// A belt's outer radius over its inner one (SGEN-37).
    pub belt_width_ratio: Span,
    /// A belt's centre lies between these multiples of the snow line
    /// (SGEN-37).
    pub belt_centre_snow_lines: Span,
    /// A belt's composition by its side of the snow line (SGEN-37).
    pub belt_composition: BeltCompositionRule,
    /// A belt's mass range in kg (SGEN-37).
    pub belt_mass_kg: BeltMasses,
    /// A debris disc's belts are this many times heavier (SGEN-37).
    pub debris_mass_factor: f64,
    /// The Bond albedo every planet's equilibrium temperature assumes
    /// (SGEN-39).
    pub equilibrium_albedo: f64,
    /// A rocky planet's volatile endowment, drawn log-uniformly from this
    /// range, scales its atmosphere retention index (SGEN-40).
    pub volatile_endowment_range: Span,
    /// A retention index below this leaves a planet airless (SGEN-40).
    pub atmosphere_thin_index: f64,
    /// A retention index above this gives a dense atmosphere (SGEN-40).
    pub atmosphere_dense_index: f64,
    /// The equilibrium temperatures that allow surface water, by
    /// atmosphere (SGEN-41).
    pub surface_water_t_k: SurfaceWaterTemperatures,
    /// The chance a planet that could have surface water has it (SGEN-41).
    pub surface_water_chance: f64,
    /// A planet warmer than this, in K, has no ice (SGEN-41).
    pub ice_t_eq_max_k: f64,
    /// The chance of ice on a cool enough planet at or beyond the snow
    /// line (SGEN-41).
    pub ice_chance_beyond_snow_line: f64,
    /// The chance of ice on a cool enough planet inside the snow line
    /// (SGEN-41).
    pub ice_chance_inside_snow_line: f64,
    /// The habitable zone's inner edge for a star like the Sun, in AU
    /// (SGEN-42).
    pub hz_inner_au: f64,
    /// The habitable zone's outer edge for a star like the Sun, in AU
    /// (SGEN-42).
    pub hz_outer_au: f64,
    /// The snow line for a star like the Sun, in AU (SGEN-42).
    pub snow_line_au: f64,

    // --- Stage 4: gate choice ---
    /// The gravity fit for settlement, in g (SGEN-50).
    pub settle_gravity_g: FourPointFit,
    /// The equilibrium temperature fit for settlement, in K (SGEN-50).
    pub settle_t_eq_k: FourPointFit,
    /// The calm factor of a young M dwarf (SGEN-50).
    pub young_m_calm: f64,
    /// An M dwarf younger than this, in Gyr, is not calm (SGEN-50).
    pub calm_age_gyr: f64,
    /// A belt's mass is divided by this, in kg, to give its value
    /// (SGEN-51).
    pub belt_value_scale_kg: f64,
    /// The value of a belt's metallic fraction (SGEN-51).
    pub value_metal: f64,
    /// The value of a belt's rocky fraction (SGEN-51).
    pub value_rock: f64,
    /// The value of a belt's icy fraction (SGEN-51).
    pub value_ice: f64,
    /// The extraction value of each giant planet (SGEN-51).
    pub giant_extraction_bonus: f64,
    /// Extraction value grows by 10^(this × [Fe/H]) (SGEN-51).
    pub metallicity_value_slope: f64,
    /// A water source and a metal source pair up for industry when their
    /// orbit radii differ by at most this factor (SGEN-52).
    pub industry_radius_ratio: f64,
    /// A planet's mass index as an industry source (SGEN-52).
    pub planet_source_index: f64,
    /// The value of each kind of unusual object to science (SGEN-53).
    pub science_values: ScienceValues,
    /// A system younger than this, in Gyr, is young (SGEN-53).
    pub young_system_gyr: f64,
    /// An M dwarf younger than this, in Gyr, is a flare star (SGEN-53).
    pub flare_age_gyr: f64,
    /// A metallicity farther than this from zero is extreme (SGEN-53).
    pub extreme_metallicity: f64,
    /// The core's share of the gated systems (SGEN-55).
    pub core_share: f64,
    /// Each reason's quota as a share of the gated systems (SGEN-55).
    pub reason_quota: f64,
    /// The most rounds of dropping and adding before the fallback
    /// (SGEN-57).
    pub max_gate_rounds: u32,
    /// The gate ring starts at this multiple of the snow line (SGEN-58).
    pub ring_snow_line_factor: f64,
    /// The ring keeps this many Hill radii from every giant's orbit
    /// (SGEN-58).
    pub ring_giant_clearance_hill: f64,
    /// The ring keeps this share of its radius from every planet's orbit
    /// (SGEN-58).
    pub ring_planet_gap: f64,
    /// The ring slides at most this share of its starting radius
    /// (SGEN-58).
    pub ring_slide_limit: f64,

    // --- Stage 5: links ---
    /// The link range is this many mean spacings of gated systems
    /// (SGEN-56, SGEN-60).
    pub link_range_factor: f64,
    /// The share by which the link range grows when a step cannot be done
    /// within it (SGEN-56, SGEN-60).
    pub link_range_growth: f64,
    /// The fewest links a gated system has (SGEN-61, SGEN-62).
    pub links_min: u32,
    /// The most links a gated system has (SGEN-61, SGEN-62).
    pub links_max: u32,
    /// The least mean number of links per system for the presets
    /// (SGEN-62).
    pub mean_links_min: f64,
    /// The share of gated systems with the highest overall scores that
    /// count as valuable (SGEN-64).
    pub strategic_top_share: f64,
    /// A system linked to at least this many valuable systems is strategic
    /// (SGEN-64).
    pub strategic_min_valuable: u32,

    // --- Failure of links ---
    /// The target share of working links in the result (SGEN-70).
    pub working_share: f64,
    /// The target share of dormant links in the result (SGEN-70).
    pub dormant_share: f64,
    /// The target share of collapsed links in the result (SGEN-70).
    pub collapsed_share: f64,
    /// The share of links the draw leaves working (SGEN-70, SGEN-71).
    pub draw_working_share: f64,
    /// The share of links the draw makes dormant; the rest collapse
    /// (SGEN-70, SGEN-71).
    pub draw_dormant_share: f64,
    /// Each sector shifts the draw's shares by up to this much (SGEN-70).
    pub state_share_shift: f64,
    /// How much a link's traffic, rather than chance, decides its failure
    /// (SGEN-71).
    pub failure_traffic_weight: f64,
    /// The main region holds at least this share of the gated systems
    /// (SGEN-72).
    pub main_region_min_share: f64,

    // --- Names ---
    /// A name that repeats this many times becomes a three-part name
    /// (SGEN-81).
    pub max_name_retries: u32,
    /// Spare names stored for each belt, for its large bodies (SGEN-82).
    pub belt_body_max: u32,

    // --- Checks ---
    /// The invariants are tested over this many seeds per size (SGEN-91).
    pub invariant_seeds: u32,
    /// The largest sector must generate within this many seconds in a
    /// debug test build (SGEN-92).
    pub generation_test_cap_s: f64,
}

impl Default for GeneratorTunables {
    fn default() -> Self {
        use Template::*;
        Self {
            preset_small: 30,
            preset_medium: 80,
            preset_large: 200,
            min_gated_count: 8,
            max_gated_count: 300,

            stars_per_gated: 3.0,
            wide_pair_fraction: 0.035,
            brown_dwarf_per_system: 0.2,
            system_density_per_pc3: 0.1,
            slab_thickness_ratio: 0.25,
            plan_elongation_max: 1.25,
            min_system_separation_pc: 0.4,

            designation_prefix: "OW".to_string(),
            star_mix: ByClass {
                m: 0.72,
                k: 0.12,
                g: 0.07,
                f: 0.03,
                a: 0.007,
                white_dwarf: 0.052,
                giant: 0.001,
            },
            star_mass_ranges: ByClass {
                m: Span::new(0.08, 0.45),
                k: Span::new(0.45, 0.80),
                g: Span::new(0.80, 1.04),
                f: Span::new(1.04, 1.40),
                a: Span::new(1.40, 2.10),
                white_dwarf: Span::new(0.50, 0.70),
                giant: Span::new(1.0, 3.0),
            },
            star_mass_luminosity: MassLuminosity {
                low_factor: 0.23,
                low_exponent: 2.3,
                low_below: 0.43,
                mid_exponent: 4.0,
                high_above: 2.0,
                high_factor: 1.4,
                high_exponent: 3.5,
            },
            star_mass_radius: MassRadius {
                break_msun: 1.0,
                low_exponent: 0.8,
                high_exponent: 0.57,
            },
            white_dwarf_radius_rsun: 0.012,
            white_dwarf_luminosity_lsun: Span::new(0.0001, 0.003),
            giant_properties: GiantProperties {
                radius_rsun: Span::new(10.0, 50.0),
                luminosity_lsun: Span::new(30.0, 1000.0),
            },
            age_min_gyr: 0.2,
            age_max_gyr: 10.0,
            white_dwarf_age_min_gyr: 1.0,
            metallicity_mean: -0.05,
            metallicity_sigma: 0.2,
            metallicity_clamp: 0.6,
            multiplicity_by_class: ByClass {
                m: 0.27,
                k: 0.35,
                g: 0.44,
                f: 0.50,
                a: 0.60,
                white_dwarf: 0.25,
                // Giants are catalogue-only and record no companions
                // (SGEN-28).
                giant: 0.0,
            },
            star_count_weights: vec![(2, 0.75), (3, 0.18), (4, 0.07)],
            separation_log10_au: SeparationLog10 {
                m: Log10Normal {
                    mean: 1.0,
                    spread: 1.3,
                },
                other: Log10Normal {
                    mean: 1.7,
                    spread: 1.5,
                },
            },
            companion_min_au: 0.1,
            wide_pair_min_au: 1000.0,
            wide_pair_max_au: 20_000.0,
            brown_dwarf_mass_msun: Span::new(0.013, 0.075),

            companion_truncation_fraction: 0.3,
            template_weights: ByClass {
                m: vec![
                    (CompactMultiPlanet, 0.65),
                    (SolarLike, 0.12),
                    (HotGiant, 0.002),
                    (DebrisDisc, 0.228),
                ],
                k: vec![
                    (CompactMultiPlanet, 0.40),
                    (SolarLike, 0.35),
                    (HotGiant, 0.01),
                    (DebrisDisc, 0.24),
                ],
                g: vec![
                    (CompactMultiPlanet, 0.35),
                    (SolarLike, 0.40),
                    (HotGiant, 0.01),
                    (DebrisDisc, 0.24),
                ],
                f: vec![
                    (CompactMultiPlanet, 0.30),
                    (SolarLike, 0.40),
                    (HotGiant, 0.01),
                    (DebrisDisc, 0.29),
                ],
                a: vec![
                    (CompactMultiPlanet, 0.20),
                    (SolarLike, 0.35),
                    (HotGiant, 0.01),
                    (DebrisDisc, 0.44),
                ],
                white_dwarf: vec![(WhiteDwarfRemnant, 1.0)],
                // Giants get no summary (SGEN-28).
                giant: vec![],
            },
            planet_kind_masses: ByKind {
                rocky: Span::new(0.05, 2.0),
                super_earth: Span::new(2.0, 8.0),
                sub_neptune: Span::new(3.0, 12.0),
                ice_giant: Span::new(12.0, 60.0),
                gas_giant: Span::new(60.0, 1600.0),
            },
            template_rules: default_template_rules(),
            min_mutual_hill: 8.0,
            cold_giant_chance: ColdGiantChance {
                fgka: 0.17,
                m: 0.04,
            },
            giant_metallicity_exponent: 2.0,
            cold_giant_chance_cap: 0.6,
            cold_giant_count_weights: vec![(1, 0.65), (2, 0.25), (3, 0.10)],
            cold_giant_gas_share: 0.6,
            cold_giant_orbit_snow_lines: Span::new(0.8, 8.0),
            mass_radius: MassRadiusCurves {
                rocky: PowerLaw {
                    radius_at_reference: 1.0,
                    reference_mass: 1.0,
                    exponent: 0.29,
                },
                sub_neptune: PowerLaw {
                    radius_at_reference: 2.4,
                    reference_mass: 6.0,
                    exponent: 0.5,
                },
                ice_giant: PowerLaw {
                    radius_at_reference: 3.95,
                    reference_mass: 16.0,
                    exponent: 0.3,
                },
                gas_giant: PowerLaw {
                    radius_at_reference: 11.21,
                    reference_mass: 317.8,
                    exponent: 0.142,
                },
            },
            belt_width_ratio: Span::new(1.2, 2.0),
            belt_centre_snow_lines: Span::new(0.15, 6.0),
            belt_composition: BeltCompositionRule {
                inside_icy_max: 0.10,
                inside_metallic: Span::new(0.10, 0.30),
                beyond_icy: Span::new(0.60, 0.90),
                beyond_metallic_max: 0.05,
            },
            belt_mass_kg: BeltMasses {
                inside: Span::new(1e19, 5e21),
                beyond: Span::new(1e20, 2e23),
                white_dwarf: Span::new(1e18, 1e21),
            },
            debris_mass_factor: 30.0,
            equilibrium_albedo: 0.3,
            volatile_endowment_range: Span::new(0.5, 2.0),
            atmosphere_thin_index: 0.07,
            atmosphere_dense_index: 0.30,
            surface_water_t_k: SurfaceWaterTemperatures {
                dense: Span::new(200.0, 290.0),
                thin: Span::new(255.0, 290.0),
            },
            surface_water_chance: 0.5,
            ice_t_eq_max_k: 330.0,
            ice_chance_beyond_snow_line: 0.9,
            ice_chance_inside_snow_line: 0.35,
            hz_inner_au: 0.99,
            hz_outer_au: 1.70,
            snow_line_au: 2.7,

            settle_gravity_g: [0.2, 0.4, 1.2, 1.6],
            settle_t_eq_k: [150.0, 220.0, 300.0, 380.0],
            young_m_calm: 0.5,
            calm_age_gyr: 2.0,
            belt_value_scale_kg: 1e22,
            value_metal: 3.0,
            value_rock: 1.0,
            value_ice: 1.5,
            giant_extraction_bonus: 0.1,
            metallicity_value_slope: 0.5,
            industry_radius_ratio: 4.0,
            planet_source_index: 1.0,
            science_values: ScienceValues {
                white_dwarf: 1.0,
                young_system: 0.6,
                flare_star: 0.5,
                debris_disc: 0.5,
                hot_giant: 0.4,
                extreme_metallicity: 0.3,
            },
            young_system_gyr: 1.0,
            flare_age_gyr: 2.0,
            extreme_metallicity: 0.4,
            core_share: 0.8,
            reason_quota: 0.10,
            max_gate_rounds: 20,
            ring_snow_line_factor: 1.35,
            ring_giant_clearance_hill: 5.0,
            ring_planet_gap: 0.05,
            ring_slide_limit: 0.5,

            link_range_factor: 2.0,
            link_range_growth: 0.1,
            links_min: 2,
            links_max: 5,
            mean_links_min: 4.0,
            strategic_top_share: 0.15,
            strategic_min_valuable: 2,

            working_share: 0.30,
            dormant_share: 0.15,
            collapsed_share: 0.55,
            draw_working_share: 0.25,
            draw_dormant_share: 0.05,
            state_share_shift: 0.05,
            failure_traffic_weight: 0.5,
            main_region_min_share: 0.40,

            max_name_retries: 20,
            belt_body_max: 3,

            invariant_seeds: 50,
            generation_test_cap_s: 30.0,
        }
    }
}

/// The template rules of SGEN-33 and the belt counts of SGEN-37. Where the
/// spec leaves a bound or a mix open, the choice is noted on the field
/// (F-255).
fn default_template_rules() -> TemplateRules {
    use PlanetKind::*;
    // Small planets of the compact template, also used by debris discs.
    let small_mix = vec![(Rocky, 0.35), (SuperEarth, 0.30), (SubNeptune, 0.35)];
    // Rocky planets of the solar-like template, also used beyond a hot
    // giant.
    let rocky_mix = vec![(Rocky, 0.7), (SuperEarth, 0.3)];
    TemplateRules {
        compact: TemplateRule {
            hot_giant_period_days: None,
            planet_count: CountSpan::new(3, 7),
            kinds: small_mix.clone(),
            orbits: OrbitRule::PeriodChain {
                first_period_days: Span::new(0.8, 6.0),
                period_ratio: Span::new(1.3, 2.0),
                outer_snow_lines: 0.5,
            },
            belt_count: CountSpan::new(0, 1),
            cold_giants: true,
        },
        solar_like: TemplateRule {
            hot_giant_period_days: None,
            planet_count: CountSpan::new(2, 5),
            kinds: rocky_mix.clone(),
            orbits: OrbitRule::SnowLines(Span::new(0.25, 1.2)),
            belt_count: CountSpan::new(1, 2),
            cold_giants: true,
        },
        hot_giant: TemplateRule {
            hot_giant_period_days: Some(Span::new(1.0, 5.0)),
            planet_count: CountSpan::new(0, 2),
            kinds: rocky_mix,
            // "Beyond half the snow line" has no outer bound in the spec;
            // two snow lines is chosen (F-255).
            orbits: OrbitRule::SnowLines(Span::new(0.5, 2.0)),
            belt_count: CountSpan::new(0, 1),
            cold_giants: false,
        },
        debris_disc: TemplateRule {
            hot_giant_period_days: None,
            planet_count: CountSpan::new(0, 3),
            kinds: small_mix,
            // "Inside twice the snow line" has no inner bound in the spec;
            // a twentieth of the snow line is chosen (F-255).
            orbits: OrbitRule::SnowLines(Span::new(0.05, 2.0)),
            belt_count: CountSpan::new(2, 3),
            cold_giants: true,
        },
        remnant: TemplateRule {
            hot_giant_period_days: None,
            planet_count: CountSpan::new(0, 2),
            // "Giants and super-Earths": the spec gives no mix, so the
            // three kinds are equally likely (F-255).
            kinds: vec![(GasGiant, 1.0), (IceGiant, 1.0), (SuperEarth, 1.0)],
            orbits: OrbitRule::Au(Span::new(2.0, 30.0)),
            belt_count: CountSpan::new(1, 2),
            cold_giants: false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SGEN-05: the defaults are the spec's tunables table, every field
    /// of it. Reading each field here also keeps the fields that later
    /// stages read from being unused until then.
    #[test]
    fn the_defaults_match_the_spec_table() {
        let t = GeneratorTunables::default();
        assert_eq!(
            (t.preset_small, t.preset_medium, t.preset_large),
            (30, 80, 200)
        );
        assert_eq!((t.min_gated_count, t.max_gated_count), (8, 300));
        assert_eq!(
            (
                t.stars_per_gated,
                t.wide_pair_fraction,
                t.brown_dwarf_per_system
            ),
            (3.0, 0.035, 0.2)
        );
        assert_eq!(
            (
                t.system_density_per_pc3,
                t.slab_thickness_ratio,
                t.plan_elongation_max,
                t.min_system_separation_pc
            ),
            (0.1, 0.25, 1.25, 0.4)
        );
        assert_eq!(t.designation_prefix, "OW");
        let mix = &t.star_mix;
        let total = mix.m + mix.k + mix.g + mix.f + mix.a + mix.white_dwarf + mix.giant;
        assert!((total - 1.0).abs() < 1e-9);
        assert_eq!(t.star_mass_ranges.g, Span::new(0.80, 1.04));
        assert_eq!(t.star_mass_luminosity.high_factor, 1.4);
        assert_eq!(t.star_mass_radius.high_exponent, 0.57);
        assert_eq!(t.white_dwarf_radius_rsun, 0.012);
        assert_eq!(t.white_dwarf_luminosity_lsun, Span::new(0.0001, 0.003));
        assert_eq!(t.giant_properties.luminosity_lsun.max, 1000.0);
        assert_eq!(
            (t.age_min_gyr, t.age_max_gyr, t.white_dwarf_age_min_gyr),
            (0.2, 10.0, 1.0)
        );
        assert_eq!(
            (t.metallicity_mean, t.metallicity_sigma, t.metallicity_clamp),
            (-0.05, 0.2, 0.6)
        );
        assert_eq!(t.multiplicity_by_class.f, 0.50);
        assert_eq!(t.star_count_weights, vec![(2, 0.75), (3, 0.18), (4, 0.07)]);
        assert_eq!(t.separation_log10_au.other.spread, 1.5);
        assert_eq!(
            (t.companion_min_au, t.wide_pair_min_au, t.wide_pair_max_au),
            (0.1, 1000.0, 20_000.0)
        );
        assert_eq!(t.brown_dwarf_mass_msun, Span::new(0.013, 0.075));

        assert_eq!(t.companion_truncation_fraction, 0.3);
        assert_eq!(t.template_weights.m[3], (Template::DebrisDisc, 0.228));
        assert_eq!(t.planet_kind_masses.gas_giant, Span::new(60.0, 1600.0));
        assert_eq!(
            t.template_rules.debris_disc.belt_count,
            CountSpan::new(2, 3)
        );
        assert_eq!(t.min_mutual_hill, 8.0);
        assert_eq!(
            (t.cold_giant_chance.fgka, t.cold_giant_chance.m),
            (0.17, 0.04)
        );
        assert_eq!(
            (t.giant_metallicity_exponent, t.cold_giant_chance_cap),
            (2.0, 0.6)
        );
        assert_eq!(t.cold_giant_count_weights[0], (1, 0.65));
        assert_eq!(t.cold_giant_gas_share, 0.6);
        assert_eq!(t.cold_giant_orbit_snow_lines, Span::new(0.8, 8.0));
        assert_eq!(t.mass_radius.gas_giant.reference_mass, 317.8);
        assert_eq!(t.belt_width_ratio, Span::new(1.2, 2.0));
        assert_eq!(t.belt_centre_snow_lines, Span::new(0.15, 6.0));
        assert_eq!(t.belt_composition.beyond_metallic_max, 0.05);
        assert_eq!(t.belt_mass_kg.beyond, Span::new(1e20, 2e23));
        assert_eq!(t.debris_mass_factor, 30.0);
        assert_eq!(t.equilibrium_albedo, 0.3);
        assert_eq!(t.volatile_endowment_range, Span::new(0.5, 2.0));
        assert_eq!(
            (t.atmosphere_thin_index, t.atmosphere_dense_index),
            (0.07, 0.30)
        );
        assert_eq!(t.surface_water_t_k.thin, Span::new(255.0, 290.0));
        assert_eq!(t.surface_water_chance, 0.5);
        assert_eq!(t.ice_t_eq_max_k, 330.0);
        assert_eq!(
            (t.ice_chance_beyond_snow_line, t.ice_chance_inside_snow_line),
            (0.9, 0.35)
        );
        assert_eq!(
            (t.hz_inner_au, t.hz_outer_au, t.snow_line_au),
            (0.99, 1.70, 2.7)
        );

        assert_eq!(t.settle_gravity_g, [0.2, 0.4, 1.2, 1.6]);
        assert_eq!(t.settle_t_eq_k, [150.0, 220.0, 300.0, 380.0]);
        assert_eq!((t.young_m_calm, t.calm_age_gyr), (0.5, 2.0));
        assert_eq!(t.belt_value_scale_kg, 1e22);
        assert_eq!((t.value_metal, t.value_rock, t.value_ice), (3.0, 1.0, 1.5));
        assert_eq!(
            (t.giant_extraction_bonus, t.metallicity_value_slope),
            (0.1, 0.5)
        );
        assert_eq!((t.industry_radius_ratio, t.planet_source_index), (4.0, 1.0));
        let s = &t.science_values;
        assert_eq!(
            [
                s.white_dwarf,
                s.young_system,
                s.flare_star,
                s.debris_disc,
                s.hot_giant,
                s.extreme_metallicity
            ],
            [1.0, 0.6, 0.5, 0.5, 0.4, 0.3]
        );
        assert_eq!(
            (t.young_system_gyr, t.flare_age_gyr, t.extreme_metallicity),
            (1.0, 2.0, 0.4)
        );
        assert_eq!(
            (t.core_share, t.reason_quota, t.max_gate_rounds),
            (0.8, 0.10, 20)
        );
        assert_eq!(
            (
                t.ring_snow_line_factor,
                t.ring_giant_clearance_hill,
                t.ring_planet_gap,
                t.ring_slide_limit
            ),
            (1.35, 5.0, 0.05, 0.5)
        );

        assert_eq!((t.link_range_factor, t.link_range_growth), (2.0, 0.1));
        assert_eq!((t.links_min, t.links_max, t.mean_links_min), (2, 5, 4.0));
        assert_eq!((t.strategic_top_share, t.strategic_min_valuable), (0.15, 2));

        assert_eq!(
            (t.working_share, t.dormant_share, t.collapsed_share),
            (0.30, 0.15, 0.55)
        );
        assert_eq!(
            (
                t.draw_working_share,
                t.draw_dormant_share,
                t.state_share_shift
            ),
            (0.25, 0.05, 0.05)
        );
        assert_eq!(
            (t.failure_traffic_weight, t.main_region_min_share),
            (0.5, 0.40)
        );
        assert_eq!((t.max_name_retries, t.belt_body_max), (20, 3));
        assert_eq!((t.invariant_seeds, t.generation_test_cap_s), (50, 30.0));
    }
}
