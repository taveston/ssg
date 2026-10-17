//! Drawing a globe from its bake each frame (SURF-20, SURF-23, SURF-24,
//! SURF-25, SURF-40 to SURF-42): the material `globe.wesl` draws with,
//! and the plain rules that fill it: which way the body is turned
//! (SURF-41), where its star's light comes from and in what colour
//! (SURF-23), and what each look's atmosphere adds (SURF-24, with the
//! dense look's halo warmed by the heat under it, SURF-20). The inset
//! and the map build a `GlobeMaterial` from these for each globe they
//! draw.

use bevy::asset::embedded_asset;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

use super::bake::Surface;
use super::chooser::{Look, LookGroup, glow_colour, glow_strength};
use super::star::StarClass;
use crate::view::tunables::Tunables;

/// What `globe.wesl` reads; field order matches `Globe` there.
#[derive(ShaderType, Clone, Copy, Debug, PartialEq)]
pub struct GlobeUniform {
    pub view_from_body_x: Vec4,
    pub view_from_body_y: Vec4,
    pub view_from_body_z: Vec4,
    /// The direction toward the star, in the view frame.
    pub light: Vec4,
    /// The star's light, linear RGB times `star_light_strength`.
    pub light_colour: Vec4,
    /// The atmosphere's colour, linear RGB, and in w the halo's strength.
    pub halo_colour: Vec4,
    /// The colour behind the globe, display space.
    pub ground: Vec4,
    pub disc_fraction: f32,
    pub wrap: f32,
    pub ambient: f32,
    pub halo_width: f32,
    pub terminator_width: f32,
    pub telescope: u32,
    pub telescope_block: f32,
    pub desaturate: f32,
    pub grain: f32,
    pub grain_seed: u32,
    pub face_px: u32,
    pub rows: u32,
    pub plumes: u32,
    pub node_px: f32,
    pub opaque_radius: f32,
    /// SURF-34: the face drawn from the close-in strip instead of the
    /// cube's, `NO_CLOSE_FACE` for none, and that strip's face size.
    pub close_face: u32,
    pub close_px: u32,
}

/// The close face of a globe with none: no face of the cube has it.
pub const NO_CLOSE_FACE: u32 = super::cube::FACES;

/// The material that draws one globe from its strip, and from its close
/// face's strip where it has one (SURF-34).
#[derive(AsBindGroup, Asset, TypePath, Clone, Debug)]
pub struct GlobeMaterial {
    #[uniform(0)]
    pub globe: GlobeUniform,
    #[texture(1)]
    #[sampler(2)]
    pub strip: Handle<Image>,
    #[texture(3)]
    #[sampler(4)]
    pub close: Option<Handle<Image>>,
}

impl UiMaterial for GlobeMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://orphan_worlds/view/surfaces/globe.wesl".into()
    }
}

/// The globe's shader and material, for the renderer.
pub struct GlobeMaterialPlugin;

impl Plugin for GlobeMaterialPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "globe.wesl");
        app.add_plugins(UiMaterialPlugin::<GlobeMaterial>::default());
    }
}

/// The atmosphere colours of the looks that have one, display space, from
/// mock-up 29, and how strong each halo is.
const THIN_COLOUR: [f32; 3] = [0.78, 0.66, 0.52];
const THIN_STRENGTH: f32 = 0.38;
const DENSE_COLOUR: [f32; 3] = [0.98, 0.82, 0.55];
const DENSE_STRENGTH: f32 = 1.0;
const ICE_GIANT_COLOUR: [f32; 3] = [0.45, 0.75, 0.95];
const ICE_GIANT_STRENGTH: f32 = 0.9;
const GAS_GIANT_COLOUR: [f32; 3] = [0.92, 0.82, 0.62];
const GAS_GIANT_STRENGTH: f32 = 0.8;

/// What a look adds round its surface (SURF-20, SURF-23, SURF-24).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LookShading {
    /// How far light wraps past the terminator.
    pub wrap: f32,
    /// What the night side keeps of the albedo.
    pub ambient: f32,
    /// The halo's colour, linear RGB, and strength; 0 for none.
    pub halo_colour: [f32; 3],
    pub halo_strength: f32,
    /// The halo falls off over this much of the radius.
    pub halo_width: f32,
    /// SURF-13: two plumes over the limb.
    pub plumes: bool,
}

fn to_linear(display: [f32; 3]) -> [f32; 3] {
    display.map(|c| c.powf(2.2))
}

fn mix3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    std::array::from_fn(|k| a[k] + (b[k] - a[k]) * t)
}

/// SURF-23 / SURF-24 / SURF-20: what a look's surroundings add: the wrap
/// and ambient of its group, its halo in its atmosphere colour at its
/// group's width, with the hazy and sub-Neptune factors, and the dense
/// look's halo warmed toward the glow colour by the glow's strength at
/// the body's peak temperature.
pub fn look_shading(look: Look, peak_t_k: f32, tunables: &Tunables) -> LookShading {
    let group = look.group();
    let wrap = if look.has_atmosphere() {
        tunables.wrap_light
    } else {
        0.0
    };
    let ambient = match group {
        LookGroup::Bare => tunables.ambient_light_bare,
        LookGroup::Thin => tunables.ambient_light_thin,
        LookGroup::Dense | LookGroup::Envelope => tunables.ambient_light_dense,
    };
    let (colour, strength, width) = match look {
        Look::ColdTerrestrial => (THIN_COLOUR, THIN_STRENGTH, tunables.halo_width_thin),
        Look::Hazy => (
            tunables.hazy_colour,
            DENSE_STRENGTH,
            tunables.halo_width_dense * tunables.hazy_halo_factor,
        ),
        Look::Dense => {
            let glow = glow_strength(f64::from(peak_t_k), tunables);
            let warmed = mix3(
                DENSE_COLOUR,
                glow_colour(f64::from(peak_t_k), tunables),
                glow,
            );
            (warmed, DENSE_STRENGTH, tunables.halo_width_dense)
        }
        Look::IceGiant => (
            ICE_GIANT_COLOUR,
            ICE_GIANT_STRENGTH,
            tunables.halo_width_envelope,
        ),
        Look::SubNeptune => (
            mix3(
                ICE_GIANT_COLOUR,
                tunables.sub_neptune_haze_colour,
                tunables.sub_neptune_haze_mix,
            ),
            ICE_GIANT_STRENGTH,
            tunables.halo_width_envelope * tunables.sub_neptune_halo_factor,
        ),
        Look::GasGiant => (
            GAS_GIANT_COLOUR,
            GAS_GIANT_STRENGTH,
            tunables.halo_width_envelope,
        ),
        Look::Airless | Look::Icy | Look::Lava | Look::Volcanic => ([0.0; 3], 0.0, 1.0),
    };
    LookShading {
        wrap,
        ambient,
        halo_colour: to_linear(colour),
        halo_strength: strength,
        halo_width: width,
        plumes: look == Look::Volcanic,
    }
}

/// SURF-41: the body frame in render space: its poles on the orbital
/// plane's normal (render +Y) and its prime meridian (body +Z) at a
/// heading about it: toward `faces` when the body is locked to what it
/// faces, else the seed's fixed heading.
pub fn body_rotation(surface: &Surface, faces: Option<Vec3>) -> Quat {
    let heading = match faces {
        Some(toward) if surface.locked_to_star || surface.locked_to_parent => {
            toward.x.atan2(toward.z)
        }
        _ => surface.draws.heading,
    };
    Quat::from_rotation_y(heading)
}

/// SURF-45: the body frame to the view frame of a camera turned by
/// `camera_rotation`, as a matrix the shader applies to body directions.
pub fn view_from_body(camera_rotation: Quat, body_rotation: Quat) -> Mat3 {
    Mat3::from_quat(camera_rotation.inverse() * body_rotation)
}

/// SURF-23: the direction toward the star in the view frame, from the
/// body's and the star's positions in render space.
pub fn light_in_view(camera_rotation: Quat, body: Vec3, star: Vec3) -> Vec3 {
    let toward = (star - body).try_normalize().unwrap_or(Vec3::Z);
    camera_rotation.inverse() * toward
}

/// SURF-23: the star's light as the shader multiplies it: its class's
/// colour at `star_light_strength`.
pub fn light_colour(class: StarClass, tunables: &Tunables) -> Vec4 {
    let c = class.light_colour(tunables);
    Vec4::new(c[0], c[1], c[2], 1.0) * tunables.star_light_strength
}

/// How a globe is drawn: where it sits in its node and how it is seen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlobeView {
    pub view_from_body: Mat3,
    pub light: Vec3,
    pub light_colour: Vec4,
    /// The colour behind the globe, display space.
    pub ground: [f32; 3],
    /// The disc's radius as a share of half the node's width.
    pub disc_fraction: f32,
    pub node_px: f32,
    /// Past this radius, in disc units, the node is transparent.
    pub opaque_radius: f32,
    /// SURF-25: the grain's seed, so the telescope frame's grain is fixed
    /// per body.
    pub grain_seed: u32,
    /// SURF-34: the face drawn from a close-in bake, and that bake's face
    /// size, while one is ready.
    pub close: Option<CloseFace>,
}

/// SURF-34: a face of the globe drawn from its close-in bake.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CloseFace {
    pub face: u32,
    pub face_px: u32,
}

/// SURF-40: what the globe pass reads for a surface seen a given way.
pub fn globe_uniform(surface: &Surface, view: &GlobeView, tunables: &Tunables) -> GlobeUniform {
    let look = look_shading(surface.look, surface.peak_t_k, tunables);
    let m = view.view_from_body;
    let telescope = super::chooser::is_telescope(surface.knowledge, tunables);
    let [hr, hg, hb] = look.halo_colour;
    GlobeUniform {
        view_from_body_x: m.x_axis.extend(0.0),
        view_from_body_y: m.y_axis.extend(0.0),
        view_from_body_z: m.z_axis.extend(0.0),
        light: view.light.extend(0.0),
        light_colour: view.light_colour,
        halo_colour: Vec4::new(hr, hg, hb, look.halo_strength),
        ground: Vec4::new(view.ground[0], view.ground[1], view.ground[2], 1.0),
        disc_fraction: view.disc_fraction,
        wrap: look.wrap,
        ambient: look.ambient,
        halo_width: look.halo_width,
        terminator_width: tunables.terminator_width,
        telescope: u32::from(telescope),
        telescope_block: tunables.telescope_block,
        desaturate: tunables.telescope_desaturate,
        grain: tunables.telescope_grain / 255.0,
        grain_seed: view.grain_seed,
        face_px: surface.face_px(),
        rows: surface.rows(),
        plumes: u32::from(look.plumes),
        node_px: view.node_px,
        opaque_radius: view.opaque_radius,
        close_face: view.close.map_or(NO_CLOSE_FACE, |close| close.face),
        close_px: view.close.map_or(0, |close| close.face_px),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::body::Knowledge;
    use crate::sim::planet_facts::{Activity, Atmosphere, PlanetFacts, Water, WorldKind};
    use crate::view::camera::rules::camera_rotation;
    use crate::view::surfaces::bake::{SurfaceInputs, describe_surface};
    use crate::view::surfaces::star::StarLight;

    fn surface(facts: &PlanetFacts, seed: u64) -> Surface {
        let inputs = SurfaceInputs {
            facts: Some(facts),
            look_override: None,
            star: StarLight {
                class: StarClass::M,
                age_gyr: 3.0,
            },
            seed,
            knowledge: Knowledge::Surveyed,
        };
        describe_surface(&inputs, None, false, &Tunables::default())
    }

    fn facts(kind: WorldKind, atmosphere: Atmosphere, locked: bool) -> PlanetFacts {
        PlanetFacts {
            kind,
            gravity_g: 1.0,
            equilibrium_temperature_k: 300.0,
            atmosphere,
            water: Water::None,
            locked,
            activity: Some(Activity::Dead),
        }
    }

    /// SURF-41: a locked planet's prime meridian faces its star, a locked
    /// moon's its planet, and a free body keeps its seed's heading; the
    /// poles stay on the orbital plane's normal.
    #[test]
    fn locked_bodies_face_what_locks_them_and_free_ones_keep_their_heading() {
        let toward = Vec3::new(0.6, 0.0, -0.8);
        let planet = surface(&facts(WorldKind::Rocky, Atmosphere::Airless, true), 1);
        let turned = body_rotation(&planet, Some(toward));
        assert!((turned * Vec3::Z - toward).length() < 1e-5);
        assert!((turned * Vec3::Y - Vec3::Y).length() < 1e-6, "poles stay");
        let moon = surface(&facts(WorldKind::Moon, Atmosphere::Airless, true), 1);
        assert!(moon.locked_to_parent && !moon.locked_to_star);
        assert!((body_rotation(&moon, Some(toward)) * Vec3::Z - toward).length() < 1e-5);
        let free = surface(&facts(WorldKind::Rocky, Atmosphere::Airless, false), 1);
        let kept = body_rotation(&free, Some(toward));
        let expected = Quat::from_rotation_y(free.draws.heading);
        assert!((kept * Vec3::Z - expected * Vec3::Z).length() < 1e-5);
        assert!((kept * Vec3::Z - toward).length() > 0.01);
        let other = surface(&facts(WorldKind::Rocky, Atmosphere::Airless, false), 2);
        assert_ne!(other.draws.heading, free.draws.heading);
    }

    /// SURF-23 / SURF-45: the light comes from the body toward the star,
    /// seen through the camera's turn; a point in front of the camera
    /// faces +Z in the view frame.
    #[test]
    fn the_light_and_the_body_are_seen_through_the_cameras_turn() {
        let camera = camera_rotation(0.0, 0.0);
        // Straight down: the star off to the east (+X) lights from the
        // screen's right.
        let light = light_in_view(camera, Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0));
        assert!((light - Vec3::X).length() < 1e-5, "{light}");
        // The orbital normal is toward the camera, so a body's north pole
        // points at the viewer.
        let body = Quat::IDENTITY;
        let m = view_from_body(camera, body);
        assert!((m * Vec3::Y - Vec3::Z).length() < 1e-5, "{}", m * Vec3::Y);
        // Tilted to the horizon and turned a quarter: north is up the
        // screen.
        let tilted = camera_rotation(0.0, std::f32::consts::FRAC_PI_2);
        let m = view_from_body(tilted, body);
        assert!((m * Vec3::Y - Vec3::Y).length() < 1e-5);
        let same = light_in_view(camera, Vec3::new(5.0, 0.0, 0.0), Vec3::new(5.0, 0.0, 0.0));
        assert!(same.is_normalized(), "a star on the body still lights");
    }

    /// SURF-23 / SURF-24 / SURF-20: looks with an atmosphere wrap the
    /// light and draw a halo at their group's width, the bare ones
    /// neither; the hazy and sub-Neptune halos take their factors; the
    /// dense halo warms with the heat under it; only the volcanic look
    /// has plumes.
    #[test]
    fn each_look_adds_what_its_surroundings_give() {
        let t = Tunables::default();
        for look in [Look::Airless, Look::Icy, Look::Lava, Look::Volcanic] {
            let s = look_shading(look, 300.0, &t);
            assert_eq!(
                (s.wrap, s.halo_strength, s.ambient),
                (0.0, 0.0, t.ambient_light_bare)
            );
            assert_eq!(s.plumes, look == Look::Volcanic);
        }
        let thin = look_shading(Look::ColdTerrestrial, 300.0, &t);
        assert_eq!(
            (thin.wrap, thin.halo_width),
            (t.wrap_light, t.halo_width_thin)
        );
        assert!(thin.halo_strength > 0.0 && thin.ambient == t.ambient_light_thin);
        let hazy = look_shading(Look::Hazy, 100.0, &t);
        assert!((hazy.halo_width - t.halo_width_dense * t.hazy_halo_factor).abs() < 1e-6);
        assert_eq!(hazy.halo_colour, to_linear(t.hazy_colour));
        let sub = look_shading(Look::SubNeptune, 100.0, &t);
        let giant = look_shading(Look::IceGiant, 100.0, &t);
        assert!((sub.halo_width - giant.halo_width * t.sub_neptune_halo_factor).abs() < 1e-6);
        assert_ne!(sub.halo_colour, giant.halo_colour);
        assert_eq!(giant.ambient, t.ambient_light_dense);
        let cool = look_shading(Look::Dense, 300.0, &t);
        let hot = look_shading(Look::Dense, f64::from(t.melt_t_k) as f32, &t);
        assert_eq!(cool.halo_colour, to_linear(DENSE_COLOUR));
        let glow = to_linear(glow_colour(f64::from(t.melt_t_k), &t));
        for (hot, glow) in hot.halo_colour.iter().zip(&glow) {
            assert!((hot - glow).abs() < 1e-6);
        }
        assert!(!hot.plumes);
    }

    /// SURF-25 / SURF-40: the uniform carries the view, the look's
    /// surroundings and the telescope frame for an unsurveyed body.
    #[test]
    fn the_uniform_carries_the_view_and_the_knowledge_state() {
        let t = Tunables::default();
        let f = facts(WorldKind::Rocky, Atmosphere::Thin, false);
        let seen = surface(&f, 4);
        let view = GlobeView {
            view_from_body: Mat3::IDENTITY,
            light: Vec3::X,
            light_colour: light_colour(StarClass::M, &t),
            ground: [0.05, 0.07, 0.09],
            disc_fraction: 0.88,
            node_px: 300.0,
            opaque_radius: 10.0,
            grain_seed: 9,
            close: None,
        };
        let u = globe_uniform(&seen, &view, &t);
        assert_eq!(u.telescope, 0);
        assert_eq!(u.close_face, NO_CLOSE_FACE);
        let close = CloseFace {
            face: 4,
            face_px: 4096,
        };
        let near = globe_uniform(
            &seen,
            &GlobeView {
                close: Some(close),
                ..view
            },
            &t,
        );
        assert_eq!((near.close_face, near.close_px), (4, 4096));
        assert_eq!(u.light, Vec4::new(1.0, 0.0, 0.0, 0.0));
        assert_eq!(u.light_colour.x, t.star_light_strength);
        assert_eq!((u.face_px, u.rows), (t.bake_face_px, 1));
        assert_eq!(u.halo_width, t.halo_width_thin);
        let mut unseen = seen;
        unseen.knowledge = t.knowledge_unsurveyed;
        let u = globe_uniform(&unseen, &view, &t);
        assert_eq!(u.telescope, 1);
        assert_eq!(u.grain, t.telescope_grain / 255.0);
        assert_eq!(u.grain_seed, 9);
    }
}
