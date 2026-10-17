//! Which look a body's surface takes, and the rules the look's layers
//! read, as plain functions of the planet facts, the star and the
//! tunables (SURF-03, SURF-10 to SURF-22, SURF-25, SURF-32, SURF-34,
//! SURF-63). No Bevy types: the bake gathers the facts and calls these.
//!
//! The looks are tried in a fixed order and the first that matches wins:
//! the giants by kind, then a volcanic world by its activity, then a
//! dense atmosphere (which hides whatever is under it), a hazy thin one,
//! a surface hot enough to melt, a thin atmosphere, and last the airless
//! bodies, icy or bare. The temperate look is reserved: its place in the
//! order is left free and nothing selects it yet (SURF-14).

use crate::sim::body::Knowledge;
use crate::sim::planet_facts::{Activity, Atmosphere, LookOverride, PlanetFacts, Water, WorldKind};
use crate::view::tunables::Tunables;

/// A surface look: the row of the look table a body is drawn by
/// (SURF-10 to SURF-19). The temperate look is reserved and has no
/// variant (SURF-14).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Look {
    GasGiant,
    IceGiant,
    SubNeptune,
    Volcanic,
    Dense,
    Hazy,
    Lava,
    ColdTerrestrial,
    Icy,
    Airless,
}

/// The looks grouped by what surrounds the surface, which sets the
/// ambient light and the limb halo (SURF-23, SURF-24).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LookGroup {
    /// Airless, icy, lava, volcanic: no atmosphere, no halo.
    Bare,
    /// A thin atmosphere: cold terrestrial, hazy.
    Thin,
    /// A dense atmosphere.
    Dense,
    /// The envelope of a sub-Neptune or a giant.
    Envelope,
}

impl Look {
    /// Every look, in the chooser's order.
    #[cfg(test)]
    pub const ALL: [Look; 10] = [
        Look::GasGiant,
        Look::IceGiant,
        Look::SubNeptune,
        Look::Volcanic,
        Look::Dense,
        Look::Hazy,
        Look::Lava,
        Look::ColdTerrestrial,
        Look::Icy,
        Look::Airless,
    ];

    /// What surrounds the surface (SURF-23, SURF-24).
    pub fn group(self) -> LookGroup {
        match self {
            Look::GasGiant | Look::IceGiant | Look::SubNeptune => LookGroup::Envelope,
            Look::Dense => LookGroup::Dense,
            Look::Hazy | Look::ColdTerrestrial => LookGroup::Thin,
            Look::Volcanic | Look::Lava | Look::Icy | Look::Airless => LookGroup::Bare,
        }
    }

    /// SURF-20: the rocky looks, whose surface glows where it is hot:
    /// volcanic, lava, hazy, cold terrestrial, icy and airless.
    pub fn is_rocky(self) -> bool {
        matches!(
            self,
            Look::Volcanic
                | Look::Lava
                | Look::Hazy
                | Look::ColdTerrestrial
                | Look::Icy
                | Look::Airless
        )
    }

    /// SURF-24: whether the look draws a limb glow and rim: every look
    /// with an atmosphere or envelope.
    pub fn has_atmosphere(self) -> bool {
        self.group() != LookGroup::Bare
    }

    /// SURF-22: whether the look can carry crater rays: only the airless
    /// and icy looks.
    pub fn carries_rays(self) -> bool {
        matches!(self, Look::Airless | Look::Icy)
    }

    /// The look's number in the shaders (`LOOK_*` in `bake.wesl`).
    pub fn shader_kind(self) -> u32 {
        match self {
            Look::Airless => 0,
            Look::ColdTerrestrial => 1,
            Look::Dense => 2,
            Look::Icy => 3,
            Look::Lava => 4,
            Look::IceGiant => 5,
            Look::GasGiant => 6,
            Look::Volcanic => 7,
            Look::Hazy => 8,
            Look::SubNeptune => 9,
        }
    }
}

/// SURF-61: the look an authored override asks for. The temperate look is
/// reserved (SURF-14), so an override naming it asks for nothing and the
/// chooser decides.
pub fn override_look(look_override: LookOverride) -> Option<Look> {
    Some(match look_override {
        LookOverride::GasGiant => Look::GasGiant,
        LookOverride::IceGiant => Look::IceGiant,
        LookOverride::SubNeptune => Look::SubNeptune,
        LookOverride::Volcanic => Look::Volcanic,
        LookOverride::Temperate => return None,
        LookOverride::Lava => Look::Lava,
        LookOverride::Dense => Look::Dense,
        LookOverride::Hazy => Look::Hazy,
        LookOverride::ColdTerrestrial => Look::ColdTerrestrial,
        LookOverride::Icy => Look::Icy,
        LookOverride::Airless => Look::Airless,
    })
}

/// SURF-21: whether a body is locked to its star: a tidally locked planet
/// or belt body. A moon locked to its planet counts as not locked to the
/// star.
pub fn locked_to_star(facts: &PlanetFacts) -> bool {
    facts.locked && facts.kind != WorldKind::Moon
}

/// SURF-21: whether the surface temperature varies over the body: it
/// does for a body locked to its star with no atmosphere to carry heat.
pub fn has_temperature_map(facts: &PlanetFacts) -> bool {
    locked_to_star(facts) && facts.atmosphere == Atmosphere::Airless
}

/// SURF-21: the surface temperature at a point, in K. On a body with a
/// temperature map it is `locked_peak_factor` × Teq × (cos θ)^(1/4) for
/// θ the angle from the substellar point, and `night_floor_k` beyond
/// 90°; on any other body it is Teq everywhere. `cos_theta` is the
/// cosine of that angle. `bake.wesl` runs the same rule per texel.
pub fn surface_temperature_k(teq_k: f64, mapped: bool, cos_theta: f64, tunables: &Tunables) -> f64 {
    if !mapped {
        return teq_k;
    }
    let lit = f64::from(tunables.locked_peak_factor) * teq_k * cos_theta.max(0.0).powf(0.25);
    lit.max(f64::from(tunables.night_floor_k))
}

/// SURF-21: the hottest surface temperature on the body, in K: the
/// substellar point of a locked, airless body, or Teq.
pub fn peak_surface_temperature_k(teq_k: f64, facts: &PlanetFacts, tunables: &Tunables) -> f64 {
    surface_temperature_k(teq_k, has_temperature_map(facts), 1.0, tunables)
}

/// SURF-63: the equilibrium temperature the surface temperature rule
/// reads. It is the facts' own, unless a lava override is on a body too
/// cool to melt by those facts: then it is `earth_override_t_k`, for the
/// override's surface temperature only. The inset still reads the facts.
pub fn surface_teq_k(facts: &PlanetFacts, look_override: Option<Look>, tunables: &Tunables) -> f64 {
    let own = facts.equilibrium_temperature_k;
    let melts = peak_surface_temperature_k(own, facts, tunables) > f64::from(tunables.melt_t_k);
    if look_override == Some(Look::Lava) && !melts {
        f64::from(tunables.earth_override_t_k)
    } else {
        own
    }
}

/// SURF-10 to SURF-19: the look the facts select, the first row that
/// matches in the table's order. A look override (SURF-61) is drawn
/// instead of the chooser's.
pub fn choose_look(facts: &PlanetFacts, look_override: Option<Look>, tunables: &Tunables) -> Look {
    if let Some(look) = look_override {
        return look;
    }
    match facts.kind {
        WorldKind::GasGiant => return Look::GasGiant,
        WorldKind::IceGiant => return Look::IceGiant,
        WorldKind::SubNeptune => return Look::SubNeptune,
        WorldKind::Rocky | WorldKind::SuperEarth | WorldKind::Moon => {}
    }
    if facts.activity == Some(Activity::Extreme) {
        return Look::Volcanic;
    }
    // The temperate look's place: reserved (SURF-14).
    if facts.atmosphere == Atmosphere::Dense {
        return Look::Dense;
    }
    let teq = facts.equilibrium_temperature_k;
    let thin = facts.atmosphere == Atmosphere::Thin;
    if thin && facts.water == Water::Ice && teq <= f64::from(tunables.hazy_t_max_k) {
        return Look::Hazy;
    }
    if peak_surface_temperature_k(teq, facts, tunables) > f64::from(tunables.melt_t_k) {
        return Look::Lava;
    }
    if thin {
        return Look::ColdTerrestrial;
    }
    if facts.water == Water::Ice {
        Look::Icy
    } else {
        Look::Airless
    }
}

/// SURF-18: whether a cold terrestrial world draws polar caps: it does
/// when its water is ice.
pub fn has_polar_caps(look: Look, facts: &PlanetFacts) -> bool {
    look == Look::ColdTerrestrial && facts.water == Water::Ice
}

/// SURF-20: how strongly the surface glows at a temperature, 0 at
/// `glow_t_k` rising to 1 at `melt_t_k`.
pub fn glow_strength(surface_t_k: f64, tunables: &Tunables) -> f32 {
    let (from, to) = (f64::from(tunables.glow_t_k), f64::from(tunables.melt_t_k));
    ((surface_t_k - from) / (to - from)).clamp(0.0, 1.0) as f32
}

/// SURF-20: the glow's colour at a temperature, as linear RGB: dull red
/// at `glow_t_k`, orange midway, pale yellow at `melt_t_k` +
/// `heat_ramp_k`. `bake.wesl` runs the same ramp.
pub fn glow_colour(surface_t_k: f64, tunables: &Tunables) -> [f32; 3] {
    let from = f64::from(tunables.glow_t_k);
    let to = f64::from(tunables.melt_t_k + tunables.heat_ramp_k);
    let t = ((surface_t_k - from) / (to - from)).clamp(0.0, 1.0) as f32;
    let (red, orange, yellow) = ([0.30, 0.03, 0.01], [1.0, 0.34, 0.05], [1.0, 0.85, 0.35]);
    let mix = |a: [f32; 3], b: [f32; 3], t: f32| std::array::from_fn(|k| a[k] + (b[k] - a[k]) * t);
    if t < 0.5 {
        mix(red, orange, t * 2.0)
    } else {
        mix(orange, yellow, (t - 0.5) * 2.0)
    }
}

/// SURF-22: the crater density, the share of the crater grid's cells that
/// hold a crater: `crater_density_min` at a star age of 0, rising to
/// `crater_density_max` at `crater_density_full_age_gyr` and above, times
/// the activity's factor. A body with no activity fact counts as dead.
pub fn crater_density(star_age_gyr: f64, activity: Option<Activity>, tunables: &Tunables) -> f32 {
    let share = (star_age_gyr / f64::from(tunables.crater_density_full_age_gyr)).clamp(0.0, 1.0);
    let by_age = tunables.crater_density_min
        + (tunables.crater_density_max - tunables.crater_density_min) * share as f32;
    let factor = match activity.unwrap_or(Activity::Dead) {
        Activity::Dead => tunables.crater_factor_dead,
        Activity::Active => tunables.crater_factor_active,
        Activity::Extreme => tunables.crater_factor_extreme,
    };
    by_age * factor
}

/// SURF-22: the share of the coarse ray grid's cells that hold a fresh
/// crater with rays: `ray_density` on the airless and icy looks of a
/// dead body, and none on any other look or activity.
pub fn ray_density(look: Look, activity: Option<Activity>, tunables: &Tunables) -> f32 {
    if look.carries_rays() && activity.unwrap_or(Activity::Dead) == Activity::Dead {
        tunables.ray_density
    } else {
        0.0
    }
}

/// SURF-25: the knowledge value of a knowledge state.
pub fn knowledge_value(knowledge: Knowledge, tunables: &Tunables) -> f32 {
    match knowledge {
        Knowledge::Surveyed => tunables.knowledge_surveyed,
        Knowledge::Unsurveyed => tunables.knowledge_unsurveyed,
    }
}

/// SURF-25: how many noise octaves a knowledge value gets, between
/// `octaves_min` and `octaves_max`. Fractional: the last octave fades in.
pub fn octaves(knowledge: f32, tunables: &Tunables) -> f32 {
    tunables.octaves_min + (tunables.octaves_max - tunables.octaves_min) * knowledge.clamp(0.0, 1.0)
}

/// SURF-25: which crater scales a knowledge value draws, coarsest first:
/// the coarsest always, each finer one at values above its threshold in
/// `crater_scale_knowledge`. Scales beyond the list are only drawn close
/// in (SURF-34), so they are off here.
pub fn crater_scales_on(knowledge: f32, tunables: &Tunables) -> [bool; 4] {
    let after = tunables.crater_scale_knowledge;
    [
        true,
        knowledge > after[0],
        knowledge > after[1],
        knowledge > after[2],
    ]
}

/// SURF-25: whether a knowledge value shows the telescope frame.
pub fn is_telescope(knowledge: f32, tunables: &Tunables) -> bool {
    knowledge < tunables.telescope_knowledge
}

/// SURF-32: the cubemap face size for a knowledge state, in pixels.
pub fn face_px(knowledge: Knowledge, tunables: &Tunables) -> u32 {
    match knowledge {
        Knowledge::Surveyed => tunables.bake_face_px,
        Knowledge::Unsurveyed => tunables.bake_face_unsurveyed_px,
    }
}

/// SURF-34: whether a globe drawn with this projected radius, in pixels,
/// is close enough in to bake its visible face again finer.
pub fn needs_rebake(projected_radius_px: f32, tunables: &Tunables) -> bool {
    projected_radius_px > tunables.bake_face_px as f32 * tunables.rebake_zoom_factor
}

/// SURF-34: the face size of a close-in bake, in pixels.
pub fn rebake_face_px(tunables: &Tunables) -> u32 {
    tunables.bake_face_px * tunables.rebake_scale
}

/// SURF-31 / SURF-34: how many pieces a close-in face is baked in, so
/// each holds about as many texels as a face of the ordinary bake and a
/// frame bakes no more than it would for a cube.
pub fn rebake_pieces(tunables: &Tunables) -> u32 {
    tunables.rebake_scale * tunables.rebake_scale
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(
        kind: WorldKind,
        atmosphere: Atmosphere,
        water: Water,
        locked: bool,
        activity: Option<Activity>,
        teq: f64,
    ) -> PlanetFacts {
        PlanetFacts {
            kind,
            gravity_g: 1.0,
            equilibrium_temperature_k: teq,
            atmosphere,
            water,
            locked,
            activity,
        }
    }

    fn rocky(atmosphere: Atmosphere, water: Water, locked: bool, teq: f64) -> PlanetFacts {
        facts(
            WorldKind::Rocky,
            atmosphere,
            water,
            locked,
            Some(Activity::Dead),
            teq,
        )
    }

    fn moon(atmosphere: Atmosphere, water: Water, activity: Activity, teq: f64) -> PlanetFacts {
        facts(
            WorldKind::Moon,
            atmosphere,
            water,
            true,
            Some(activity),
            teq,
        )
    }

    fn look(facts: &PlanetFacts) -> Look {
        choose_look(facts, None, &Tunables::default())
    }

    /// SURF-70 / SURF-10 to SURF-19: every row is selected by a fact set
    /// and missed by one that just fails it, in the table's order.
    #[test]
    fn every_look_is_selected_by_its_row_and_missed_just_outside_it() {
        use Atmosphere::*;
        let t = Tunables::default();
        let giant = |kind| facts(kind, Envelope, Water::None, false, None, 120.0);
        assert_eq!(look(&giant(WorldKind::GasGiant)), Look::GasGiant);
        assert_eq!(look(&giant(WorldKind::IceGiant)), Look::IceGiant);
        assert_eq!(look(&giant(WorldKind::SubNeptune)), Look::SubNeptune);
        // Volcanic by activity, whatever the atmosphere; active misses.
        assert_eq!(
            look(&moon(Thin, Water::Ice, Activity::Extreme, 90.0)),
            Look::Volcanic
        );
        assert_eq!(
            look(&moon(Thin, Water::Ice, Activity::Active, 90.0)),
            Look::Hazy
        );
        // Dense hides a hot surface: dense comes before lava.
        assert_eq!(look(&rocky(Dense, Water::None, true, 1_500.0)), Look::Dense);
        assert_eq!(
            look(&rocky(Airless, Water::None, true, 1_500.0)),
            Look::Lava
        );
        // Hazy needs thin, ice and a cold enough Teq.
        let hazy = rocky(Thin, Water::Ice, false, t.hazy_t_max_k as f64);
        assert_eq!(look(&hazy), Look::Hazy);
        let warm = rocky(Thin, Water::Ice, false, t.hazy_t_max_k as f64 + 1.0);
        assert_eq!(look(&warm), Look::ColdTerrestrial);
        let dry = rocky(Thin, Water::None, false, 100.0);
        assert_eq!(look(&dry), Look::ColdTerrestrial);
        // Lava: the peak surface temperature above the melt point.
        let melt = t.melt_t_k as f64;
        let factor = t.locked_peak_factor as f64;
        let just_melts = rocky(Airless, Water::None, true, melt / factor + 1.0);
        assert_eq!(look(&just_melts), Look::Lava);
        let just_solid = rocky(Airless, Water::None, true, melt / factor - 1.0);
        assert_eq!(look(&just_solid), Look::Airless);
        let spinning = rocky(Airless, Water::None, false, melt + 1.0);
        assert_eq!(look(&spinning), Look::Lava);
        let spinning_cool = rocky(Airless, Water::None, false, melt - 1.0);
        assert_eq!(look(&spinning_cool), Look::Airless);
        // A thin atmosphere melts at Teq alone: no temperature map.
        assert_eq!(
            look(&rocky(Thin, Water::None, true, melt + 1.0)),
            Look::Lava
        );
        assert_eq!(
            look(&rocky(Thin, Water::None, true, melt / factor + 1.0)),
            Look::ColdTerrestrial
        );
        // Icy needs airless and ice; airless is the rest.
        assert_eq!(look(&rocky(Airless, Water::Ice, false, 100.0)), Look::Icy);
        assert_eq!(
            look(&rocky(Airless, Water::None, false, 100.0)),
            Look::Airless
        );
        assert_eq!(
            look(&rocky(Airless, Water::Surface, false, 100.0)),
            Look::Airless
        );
    }

    /// SURF-70: the Sol bodies take the looks the spec names.
    #[test]
    fn the_sol_bodies_take_their_looks() {
        use Atmosphere::*;
        let t = Tunables::default();
        let io = moon(Airless, Water::None, Activity::Extreme, 110.0);
        assert_eq!(look(&io), Look::Volcanic);
        let titan = moon(Thin, Water::Ice, Activity::Dead, 94.0);
        assert_eq!(look(&titan), Look::Hazy);
        let triton = moon(Thin, Water::Ice, Activity::Active, 38.0);
        assert_eq!(look(&triton), Look::Hazy);
        let venus = facts(
            WorldKind::Rocky,
            Dense,
            Water::None,
            true,
            Some(Activity::Active),
            327.0,
        );
        assert_eq!(look(&venus), Look::Dense);
        let mars = rocky(Thin, Water::Ice, false, 210.0);
        assert_eq!(look(&mars), Look::ColdTerrestrial);
        assert!(has_polar_caps(Look::ColdTerrestrial, &mars));
        let the_moon = moon(Airless, Water::None, Activity::Dead, 270.0);
        assert_eq!(look(&the_moon), Look::Airless);
        let europa = moon(Airless, Water::Ice, Activity::Active, 110.0);
        assert_eq!(look(&europa), Look::Icy);
        let jupiter = facts(
            WorldKind::GasGiant,
            Envelope,
            Water::None,
            false,
            None,
            110.0,
        );
        assert_eq!(look(&jupiter), Look::GasGiant);
        let neptune = facts(
            WorldKind::IceGiant,
            Envelope,
            Water::None,
            false,
            None,
            47.0,
        );
        assert_eq!(look(&neptune), Look::IceGiant);
        let earth = facts(
            WorldKind::Rocky,
            Dense,
            Water::Surface,
            false,
            Some(Activity::Active),
            255.0,
        );
        assert_eq!(look(&earth), Look::Dense);
        assert_eq!(choose_look(&earth, Some(Look::Lava), &t), Look::Lava);
        // SURF-63: the override's surface temperature is raised; the
        // facts' own Teq stays for everything else.
        assert_eq!(
            surface_teq_k(&earth, Some(Look::Lava), &t),
            f64::from(t.earth_override_t_k)
        );
        assert_eq!(surface_teq_k(&earth, None, &t), 255.0);
        let hot = rocky(Airless, Water::None, true, 1_500.0);
        assert_eq!(surface_teq_k(&hot, Some(Look::Lava), &t), 1_500.0);
    }

    /// SURF-70: a compact system's innermost rocky planet at Teq 1,500 K,
    /// locked, is lava; the same not locked at 1,000 K with no water is
    /// airless and glows; at 1,500 K with a dense atmosphere it is dense.
    #[test]
    fn a_compact_systems_inner_planet_melts_glows_or_hides_under_clouds() {
        let t = Tunables::default();
        let hot = rocky(Atmosphere::Airless, Water::None, true, 1_500.0);
        assert_eq!(look(&hot), Look::Lava);
        let warm = rocky(Atmosphere::Airless, Water::None, false, 1_000.0);
        assert_eq!(look(&warm), Look::Airless);
        let peak = peak_surface_temperature_k(1_000.0, &warm, &t);
        assert_eq!(peak, 1_000.0);
        assert!(glow_strength(peak, &t) > 0.0 && glow_strength(peak, &t) < 1.0);
        let dense = rocky(Atmosphere::Dense, Water::None, true, 1_500.0);
        assert_eq!(look(&dense), Look::Dense);
    }

    /// SURF-21: a locked airless planet peaks at `locked_peak_factor` × Teq
    /// at the substellar point, falls with the fourth root of the cosine,
    /// and keeps the night floor past 90°; a moon locked to its planet, a
    /// free planet or one with an atmosphere is Teq everywhere.
    #[test]
    fn the_surface_temperature_follows_locking_and_the_angle_from_the_star() {
        let t = Tunables::default();
        let locked = rocky(Atmosphere::Airless, Water::None, true, 1_000.0);
        assert!(has_temperature_map(&locked));
        let peak = peak_surface_temperature_k(1_000.0, &locked, &t);
        assert!((peak - 1_414.0).abs() < 0.5, "{peak}");
        let at = |cos: f64| surface_temperature_k(1_000.0, true, cos, &t);
        assert!((at(0.5) - 1_414.0 * 0.5_f64.powf(0.25)).abs() < 0.5);
        assert_eq!(at(0.0), f64::from(t.night_floor_k));
        assert_eq!(at(-0.7), f64::from(t.night_floor_k));
        for body in [
            moon(Atmosphere::Airless, Water::None, Activity::Dead, 1_000.0),
            rocky(Atmosphere::Airless, Water::None, false, 1_000.0),
            rocky(Atmosphere::Thin, Water::None, true, 1_000.0),
        ] {
            assert!(!has_temperature_map(&body));
            assert_eq!(peak_surface_temperature_k(1_000.0, &body, &t), 1_000.0);
        }
        assert!(!locked_to_star(&moon(
            Atmosphere::Airless,
            Water::None,
            Activity::Dead,
            100.0
        )));
    }

    /// SURF-20: the glow's strength rises from nothing at `glow_t_k` to
    /// full at `melt_t_k`, and its colour runs from dull red through
    /// orange to pale yellow at `melt_t_k` + `heat_ramp_k`.
    #[test]
    fn the_glow_rises_with_temperature_from_red_to_yellow() {
        let t = Tunables::default();
        let (glow, melt, ramp) = (
            f64::from(t.glow_t_k),
            f64::from(t.melt_t_k),
            f64::from(t.heat_ramp_k),
        );
        assert_eq!(glow_strength(glow - 1.0, &t), 0.0);
        assert_eq!(glow_strength(glow, &t), 0.0);
        assert!((glow_strength((glow + melt) / 2.0, &t) - 0.5).abs() < 1e-6);
        assert_eq!(glow_strength(melt, &t), 1.0);
        assert_eq!(glow_strength(melt + 500.0, &t), 1.0);
        let red = glow_colour(glow, &t);
        let yellow = glow_colour(melt + ramp, &t);
        let orange = glow_colour((glow + melt + ramp) / 2.0, &t);
        assert!(red[0] > red[1] * 5.0 && red[0] < 0.5, "{red:?}");
        assert!(orange[0] > 0.9 && orange[1] > 0.2 && orange[1] < 0.5);
        assert!(yellow[0] > 0.9 && yellow[1] > 0.7 && yellow[2] > 0.2);
        assert_eq!(glow_colour(melt + ramp + 1_000.0, &t), yellow);
    }

    /// SURF-22: the crater density rises with the star's age to the full
    /// age, and is scaled by the activity: full for dead, half for
    /// active, none for extreme; no activity fact counts as dead.
    #[test]
    fn crater_density_follows_the_stars_age_and_the_activity() {
        let t = Tunables::default();
        let dead = Some(Activity::Dead);
        assert_eq!(crater_density(0.0, dead, &t), t.crater_density_min);
        let full = f64::from(t.crater_density_full_age_gyr);
        assert_eq!(crater_density(full, dead, &t), t.crater_density_max);
        assert_eq!(crater_density(full * 2.0, dead, &t), t.crater_density_max);
        let mid = (t.crater_density_min + t.crater_density_max) / 2.0;
        assert!((crater_density(full / 2.0, dead, &t) - mid).abs() < 1e-6);
        assert_eq!(
            crater_density(full, Some(Activity::Active), &t),
            t.crater_density_max * t.crater_factor_active
        );
        assert_eq!(crater_density(full, Some(Activity::Extreme), &t), 0.0);
        assert_eq!(crater_density(full, None, &t), t.crater_density_max);
    }

    /// SURF-22: rays only on the airless and icy looks of a dead body.
    #[test]
    fn rays_belong_to_dead_airless_and_icy_bodies_only() {
        let t = Tunables::default();
        let dead = Some(Activity::Dead);
        assert_eq!(ray_density(Look::Airless, dead, &t), t.ray_density);
        assert_eq!(ray_density(Look::Icy, dead, &t), t.ray_density);
        assert_eq!(ray_density(Look::Icy, None, &t), t.ray_density);
        assert_eq!(ray_density(Look::Airless, Some(Activity::Active), &t), 0.0);
        assert_eq!(ray_density(Look::Icy, Some(Activity::Extreme), &t), 0.0);
        for look in Look::ALL.into_iter().filter(|l| !l.carries_rays()) {
            assert_eq!(ray_density(look, dead, &t), 0.0, "{look:?}");
        }
    }

    /// SURF-25: the knowledge value sets the octaves between the two
    /// limits and switches the finer crater scales on past each
    /// threshold, the coarsest always on; below `telescope_knowledge` the
    /// picture is the telescope frame.
    #[test]
    fn knowledge_sets_octaves_crater_scales_and_the_telescope_frame() {
        let t = Tunables::default();
        assert_eq!(
            knowledge_value(Knowledge::Surveyed, &t),
            t.knowledge_surveyed
        );
        assert_eq!(
            knowledge_value(Knowledge::Unsurveyed, &t),
            t.knowledge_unsurveyed
        );
        assert_eq!(octaves(0.0, &t), t.octaves_min);
        assert_eq!(octaves(1.0, &t), t.octaves_max);
        assert!((octaves(0.5, &t) - (t.octaves_min + t.octaves_max) / 2.0).abs() < 1e-6);
        assert_eq!(crater_scales_on(0.0, &t), [true, false, false, false]);
        let [a, b, c] = t.crater_scale_knowledge;
        assert_eq!(crater_scales_on(a, &t), [true, false, false, false]);
        assert_eq!(crater_scales_on(a + 0.01, &t), [true, true, false, false]);
        assert_eq!(crater_scales_on(b + 0.01, &t), [true, true, true, false]);
        assert_eq!(crater_scales_on(c + 0.01, &t), [true, true, true, true]);
        assert_eq!(crater_scales_on(1.0, &t), [true, true, true, true]);
        assert!(is_telescope(0.0, &t));
        assert!(is_telescope(t.telescope_knowledge - 0.01, &t));
        assert!(!is_telescope(t.telescope_knowledge, &t));
        assert!(!is_telescope(1.0, &t));
    }

    /// SURF-32: the face size is a plain function of the knowledge state.
    #[test]
    fn the_face_size_follows_the_knowledge_state() {
        let t = Tunables::default();
        assert_eq!(face_px(Knowledge::Surveyed, &t), t.bake_face_px);
        assert_eq!(
            face_px(Knowledge::Unsurveyed, &t),
            t.bake_face_unsurveyed_px
        );
        assert!(t.bake_face_px > t.bake_face_unsurveyed_px);
    }

    /// SURF-61: every authored override names a look, except the reserved
    /// temperate one, which leaves the choice to the facts.
    #[test]
    fn an_override_names_its_look_except_the_reserved_temperate_one() {
        assert_eq!(override_look(LookOverride::Lava), Some(Look::Lava));
        assert_eq!(override_look(LookOverride::GasGiant), Some(Look::GasGiant));
        assert_eq!(override_look(LookOverride::Airless), Some(Look::Airless));
        assert_eq!(override_look(LookOverride::Temperate), None);
    }

    /// SURF-34: the close-in bake is wanted once the projected radius
    /// passes `bake_face_px` × `rebake_zoom_factor`, and is `rebake_scale`
    /// times finer.
    #[test]
    fn the_close_in_bake_is_wanted_past_the_zoom_factor() {
        let t = Tunables::default();
        let edge = t.bake_face_px as f32 * t.rebake_zoom_factor;
        assert!(!needs_rebake(edge, &t));
        assert!(needs_rebake(edge + 1.0, &t));
        assert_eq!(rebake_face_px(&t), t.bake_face_px * t.rebake_scale);
        // Each piece of the close face holds an ordinary face's texels.
        let piece = rebake_face_px(&t).pow(2) / rebake_pieces(&t);
        assert_eq!(piece, t.bake_face_px.pow(2));
    }

    /// SURF-23 / SURF-24: each look's group sets whether it has a halo,
    /// and the rocky looks are the ones that glow (SURF-20).
    #[test]
    fn looks_group_by_what_surrounds_them() {
        assert_eq!(Look::GasGiant.group(), LookGroup::Envelope);
        assert_eq!(Look::SubNeptune.group(), LookGroup::Envelope);
        assert_eq!(Look::Dense.group(), LookGroup::Dense);
        assert_eq!(Look::Hazy.group(), LookGroup::Thin);
        assert_eq!(Look::ColdTerrestrial.group(), LookGroup::Thin);
        for look in [Look::Airless, Look::Icy, Look::Lava, Look::Volcanic] {
            assert_eq!(look.group(), LookGroup::Bare);
            assert!(!look.has_atmosphere());
            assert!(look.is_rocky());
        }
        assert!(Look::Dense.has_atmosphere() && !Look::Dense.is_rocky());
        assert!(Look::Hazy.is_rocky() && Look::ColdTerrestrial.is_rocky());
        let kinds: std::collections::HashSet<u32> =
            Look::ALL.iter().map(|l| l.shader_kind()).collect();
        assert_eq!(kinds.len(), Look::ALL.len(), "one shader kind per look");
    }
}
