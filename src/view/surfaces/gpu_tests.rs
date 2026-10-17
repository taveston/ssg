//! Tests that bake small faces on the GPU and read the texels back:
//! determinism (SURF-71), the large forms across knowledge states
//! (SURF-72) and the looks' distinguishing features (SURF-13, SURF-15 to
//! SURF-22, SURF-35). They need a GPU: the renderer runs headless, with
//! no window, and the compute pass writes a strip the tests read back.

use std::sync::{Arc, Mutex};

use bevy::app::PluginGroup;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::gpu_readback::{Readback, ReadbackComplete};
use bevy::window::ExitCondition;
use bevy::winit::WinitPlugin;

use super::bake::{BakeKey, Surface, SurfaceBakes, SurfaceInputs, describe_surface};
use super::cube::{FACES, fibonacci_sphere, texel_direction};
use super::star::{StarClass, StarLight};
use super::{SurfaceRenderPlugin, SurfacesPlugin};
use crate::sim::body::Knowledge;
use crate::sim::planet_facts::{Activity, Atmosphere, PlanetFacts, Water, WorldKind};
use crate::view::tunables::{Tunables, TunablesPlugin};

/// The face size the tests bake at: small, so a bake is quick.
const FACE_PX: u32 = 64;
/// Frames to wait for a bake or a read-back before giving up.
const PATIENCE: usize = 600;

/// The bytes a read-back delivered.
#[derive(Resource, Clone, Default)]
struct ReadbackSink(Arc<Mutex<Option<Vec<u8>>>>);

/// A headless app with the renderer and the surfaces, no window.
fn gpu_app() -> App {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .build()
            .disable::<WinitPlugin>()
            .disable::<LogPlugin>()
            .disable::<bevy::audio::AudioPlugin>()
            .disable::<bevy::gilrs::GilrsPlugin>()
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .set(RenderPlugin {
                synchronous_pipeline_compilation: true,
                ..default()
            }),
    )
    .add_plugins((TunablesPlugin, SurfacesPlugin, SurfaceRenderPlugin))
    .init_resource::<ReadbackSink>()
    .init_resource::<crate::view::world::WorldPositions>()
    .init_resource::<crate::view::camera::state::CameraRig>()
    .configure_sets(
        Update,
        (
            crate::Stage::Simulation,
            crate::Stage::Projection,
            crate::Stage::Camera,
            crate::Stage::Grouping,
            crate::Stage::Motion,
            crate::Stage::Screen,
            crate::Stage::Layout,
            crate::Stage::Overlay,
        )
            .chain(),
    );
    // `App::run` would do these before its first update: the renderer
    // makes its device and pipelines in them.
    app.finish();
    app.cleanup();
    app.update();
    app
}

/// A baked strip as texels: `rows` rows of `columns` faces (six for a
/// cube, one for a close face), each `face_px` square, each texel RGBA
/// as floats.
struct Baked {
    face_px: u32,
    rows: u32,
    columns: u32,
    texels: Vec<[f32; 4]>,
}

impl Baked {
    fn texel(&self, column: u32, x: u32, y: u32, row: u32) -> [f32; 4] {
        let width = self.columns * self.face_px;
        let index = (row * self.face_px + y) * width + column * self.face_px + x;
        self.texels[index as usize]
    }

    /// Every inner texel of row 0 (the border left out) with its
    /// direction and value.
    fn inner(&self, row: u32) -> Vec<(Vec3, [f32; 4])> {
        let mut out = Vec::new();
        for face in 0..FACES {
            for y in 1..self.face_px - 1 {
                for x in 1..self.face_px - 1 {
                    let d = texel_direction(face, x, y, self.face_px);
                    out.push((d, self.texel(face, x, y, row)));
                }
            }
        }
        out
    }

    fn luminance(t: [f32; 4]) -> f32 {
        0.3 * t[0] + 0.59 * t[1] + 0.11 * t[2]
    }

    fn mean_luminance(&self, row: u32) -> f32 {
        let inner = self.inner(row);
        inner.iter().map(|(_, t)| Self::luminance(*t)).sum::<f32>() / inner.len() as f32
    }

    fn mean_colour(&self) -> [f32; 3] {
        let inner = self.inner(0);
        let n = inner.len() as f32;
        std::array::from_fn(|k| inner.iter().map(|(_, t)| t[k]).sum::<f32>() / n)
    }

    fn max_abs_height(&self) -> f32 {
        self.inner(0)
            .iter()
            .map(|(_, t)| t[3].abs())
            .fold(0.0, f32::max)
    }

    fn mean_abs_height(&self) -> f32 {
        let inner = self.inner(0);
        inner.iter().map(|(_, t)| t[3].abs()).sum::<f32>() / inner.len() as f32
    }

    /// SURF-26: the albedo's luminance averaged over each cap of
    /// `large_form_deg` across, centred on the Fibonacci sphere's
    /// directions.
    fn large_forms(&self, tunables: &Tunables) -> Vec<f32> {
        let inner = self.inner(0);
        let cos_cap = (tunables.large_form_deg / 2.0).to_radians().cos();
        fibonacci_sphere(tunables.large_form_samples)
            .into_iter()
            .map(|centre| {
                let (sum, count) = inner
                    .iter()
                    .filter(|(d, _)| d.dot(centre) >= cos_cap)
                    .fold((0.0, 0), |(sum, count), (_, t)| {
                        (sum + Self::luminance(*t), count + 1)
                    });
                sum / count.max(1) as f32
            })
            .collect()
    }
}

/// A half-precision float as a float.
fn f16_to_f32(h: u16) -> f32 {
    let sign = if h & 0x8000 != 0 { -1.0 } else { 1.0 };
    let exponent = i32::from((h >> 10) & 0x1f);
    let mantissa = f32::from(h & 0x3ff);
    match exponent {
        0 => sign * mantissa * 2f32.powi(-24),
        31 => {
            if mantissa == 0.0 {
                sign * f32::INFINITY
            } else {
                f32::NAN
            }
        }
        _ => sign * (1.0 + mantissa / 1024.0) * 2f32.powi(exponent - 15),
    }
}

/// The strip's bytes as texels: rows padded to the copy alignment.
fn decode(bytes: &[u8], face_px: u32, rows: u32, columns: u32) -> Baked {
    let width = columns * face_px;
    let height = rows * face_px;
    let row_bytes = (width * 8).div_ceil(256) * 256;
    let mut texels = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        for x in 0..width {
            let at = (y * row_bytes + x * 8) as usize;
            let channel = |k: usize| {
                f16_to_f32(u16::from_le_bytes([
                    bytes[at + 2 * k],
                    bytes[at + 2 * k + 1],
                ]))
            };
            texels.push([channel(0), channel(1), channel(2), channel(3)]);
        }
    }
    Baked {
        face_px,
        rows,
        columns,
        texels,
    }
}

/// Bakes `inputs` at the test's face size for a body of its own and
/// reads the strip back.
fn bake(app: &mut App, inputs: &SurfaceInputs, knowledge_face: Option<u32>) -> Baked {
    let tunables = app.world().resource::<Tunables>().clone();
    let surface = describe_surface(inputs, knowledge_face.or(Some(FACE_PX)), false, &tunables);
    let body = app.world_mut().spawn_empty().id();
    let key = BakeKey {
        body,
        knowledge: inputs.knowledge,
    };
    let strip = run_bake(app, |bakes, images| {
        let entry = bakes.request(key, &surface.params, images);
        (entry.strip.clone(), entry.ready)
    });
    read_back(app, strip, &surface, FACES)
}

/// Asks for a bake each frame, as a globe being drawn does, until it is
/// ready, and returns its strip.
fn run_bake(
    app: &mut App,
    mut ask: impl FnMut(&mut SurfaceBakes, &mut Assets<Image>) -> (Handle<Image>, bool),
) -> Handle<Image> {
    let mut frames = 0;
    loop {
        let (strip, ready) = {
            let world = app.world_mut();
            world.resource_scope(|world, mut bakes: Mut<SurfaceBakes>| {
                let mut images = world.resource_mut::<Assets<Image>>();
                ask(&mut bakes, &mut images)
            })
        };
        if ready {
            return strip;
        }
        app.update();
        frames += 1;
        assert!(frames < PATIENCE, "the bake never finished");
    }
}

/// Reads a baked strip of `surface`'s, `columns` faces wide, back from
/// the GPU.
fn read_back(app: &mut App, strip: Handle<Image>, surface: &Surface, columns: u32) -> Baked {
    let mut frames = 0;
    let sink = app.world().resource::<ReadbackSink>().clone();
    *sink.0.lock().unwrap() = None;
    let reader = app
        .world_mut()
        .spawn(Readback::texture(strip))
        .observe(move |event: On<ReadbackComplete>| {
            *sink.0.lock().unwrap() = Some(event.data.clone());
        })
        .id();
    let bytes = loop {
        app.update();
        frames += 1;
        if let Some(bytes) = app
            .world()
            .resource::<ReadbackSink>()
            .0
            .lock()
            .unwrap()
            .take()
        {
            break bytes;
        }
        assert!(frames < PATIENCE, "the read-back never came");
    };
    app.world_mut().entity_mut(reader).despawn();
    decode(&bytes, surface.face_px(), surface.rows(), columns)
}

fn facts(
    kind: WorldKind,
    atmosphere: Atmosphere,
    water: Water,
    locked: bool,
    activity: Activity,
    teq: f64,
) -> PlanetFacts {
    PlanetFacts {
        kind,
        gravity_g: 1.0,
        equilibrium_temperature_k: teq,
        atmosphere,
        water,
        locked,
        activity: Some(activity),
    }
}

fn inputs<'a>(facts: &'a PlanetFacts, seed: u64, knowledge: Knowledge) -> SurfaceInputs<'a> {
    SurfaceInputs {
        facts: Some(facts),
        look_override: None,
        star: StarLight {
            class: StarClass::G,
            age_gyr: 4.6,
        },
        seed,
        knowledge,
    }
}

fn surveyed(facts: &PlanetFacts, seed: u64) -> SurfaceInputs<'_> {
    inputs(facts, seed, Knowledge::Surveyed)
}

/// How many texels of two bakes differ.
fn differing(a: &Baked, b: &Baked) -> usize {
    a.texels
        .iter()
        .zip(&b.texels)
        .filter(|(p, q)| p.iter().zip(q.iter()).any(|(x, y)| (x - y).abs() > 1e-4))
        .count()
}

/// Bakes `face` of `inputs` on its own as a close face, at the test's
/// face size, in a few bands, with the finer crater scales on or off,
/// and reads it back.
fn bake_close(app: &mut App, inputs: &SurfaceInputs, face: u32, fine: bool) -> Baked {
    let tunables = app.world().resource::<Tunables>().clone();
    let surface = describe_surface(inputs, Some(FACE_PX), fine, &tunables);
    let key = BakeKey {
        body: app.world_mut().spawn_empty().id(),
        knowledge: inputs.knowledge,
    };
    // Five bands, so the last is short.
    let strip = run_bake(app, |bakes, images| {
        let entry = bakes.request_close(key, face, &surface.params, 5, images);
        (entry.strip.clone(), entry.ready)
    });
    read_back(app, strip, &surface, 1)
}

/// SURF-34 / SURF-31: a close face baked on its own, in bands, into a
/// strip one face wide holds the same texels as that face of the cube
/// baked at the same size, every row of every layer; with the finer
/// crater scales on it differs.
#[test]
fn a_close_face_baked_in_bands_matches_the_cubes_face() {
    let mut app = gpu_app();
    let rock = facts(
        WorldKind::Rocky,
        Atmosphere::Airless,
        Water::None,
        false,
        Activity::Dead,
        900.0,
    );
    let inputs = surveyed(&rock, 7);
    let cube = bake(&mut app, &inputs, None);
    assert_eq!(cube.rows, 2, "a glowing rock bakes both layers");
    let close = bake_close(&mut app, &inputs, 4, false);
    let mut compared = 0;
    for row in 0..cube.rows {
        for y in 0..FACE_PX {
            for x in 0..FACE_PX {
                assert_eq!(close.texel(0, x, y, row), cube.texel(4, x, y, row));
                compared += 1;
            }
        }
    }
    assert_eq!(compared, close.texels.len());
    let fine = bake_close(&mut app, &inputs, 4, true);
    assert!(differing(&close, &fine) > 0, "the finest craters show");
}

/// SURF-71 / SURF-01: baking one face twice for the same inputs gives the
/// same texels, pixel for pixel; another seed gives different ones.
#[test]
fn the_same_inputs_bake_the_same_texels_and_another_seed_differs() {
    let mut app = gpu_app();
    let rock = facts(
        WorldKind::Rocky,
        Atmosphere::Airless,
        Water::None,
        true,
        Activity::Dead,
        400.0,
    );
    let first = bake(&mut app, &surveyed(&rock, 7), None);
    let again = bake(&mut app, &surveyed(&rock, 7), None);
    assert_eq!(first.texels.len(), again.texels.len());
    assert_eq!(
        differing(&first, &again),
        0,
        "the same inputs, the same picture"
    );
    assert!(first.mean_luminance(0) > 0.01, "something was baked");
    let other = bake(&mut app, &surveyed(&rock, 8), None);
    let changed = differing(&first, &other);
    assert!(
        changed > first.texels.len() / 2,
        "another seed changes the picture: {changed} texels differ"
    );
}

/// SURF-72 / SURF-26: the unsurveyed and the surveyed pictures of one
/// body agree in their large forms: the albedo averaged over each of the
/// caps differs by at most the tolerance.
#[test]
fn the_large_forms_persist_between_knowledge_states() {
    let mut app = gpu_app();
    let tunables = app.world().resource::<Tunables>().clone();
    let rock = facts(
        WorldKind::Rocky,
        Atmosphere::Airless,
        Water::None,
        false,
        Activity::Dead,
        300.0,
    );
    let seen = bake(&mut app, &surveyed(&rock, 11), Some(FACE_PX));
    let unseen = bake(
        &mut app,
        &inputs(&rock, 11, Knowledge::Unsurveyed),
        Some(FACE_PX),
    );
    let a = seen.large_forms(&tunables);
    let b = unseen.large_forms(&tunables);
    assert_eq!(a.len(), tunables.large_form_samples);
    let worst = a
        .iter()
        .zip(&b)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f32::max);
    assert!(
        worst <= tunables.large_form_tolerance,
        "the caps differ by up to {worst}"
    );
    // The two pictures do differ in their detail.
    assert!(differing(&seen, &unseen) > 0);
}

/// The looks' distinguishing features, read from small bakes: a glow
/// where hot and a molten, flat sea on the dayside of a lava world
/// (SURF-15, SURF-20, SURF-21); rays only on a dead world (SURF-22); the
/// volcanic world's glowing pits (SURF-13); no relief under a cloud deck
/// or envelope, the hazy deck in its own palette (SURF-10, SURF-11,
/// SURF-12, SURF-16, SURF-17); polar caps on a cold world with ice
/// (SURF-18); ice brighter and smoother than rock (SURF-19); and no seam
/// between faces (SURF-35).
#[test]
fn each_look_bakes_its_distinguishing_features() {
    let mut app = gpu_app();
    let tunables = app.world().resource::<Tunables>().clone();
    let rocky = |atmosphere, water, locked, activity, teq| {
        facts(WorldKind::Rocky, atmosphere, water, locked, activity, teq)
    };

    // SURF-15 / SURF-21: a locked airless world at Teq 1,500 K melts on
    // its dayside (+Z of the body frame): the emissive row glows there,
    // not on the far side, and the sea is flat.
    let lava = rocky(
        Atmosphere::Airless,
        Water::None,
        true,
        Activity::Dead,
        1_500.0,
    );
    let baked = bake(&mut app, &surveyed(&lava, 3), None);
    assert_eq!(baked.rows, 2);
    let (mut day, mut night, mut sea_height, mut rock_height) = (0.0, 0.0, 0.0, 0.0);
    let (mut sea, mut rock) = (0, 0);
    for (d, t) in baked.inner(1) {
        if d.z > 0.9 {
            day += Baked::luminance(t);
        } else if d.z < -0.5 {
            night += Baked::luminance(t);
        }
    }
    let heights: Vec<(Vec3, [f32; 4])> = baked.inner(0);
    for ((d, emissive), (_, base)) in baked.inner(1).into_iter().zip(&heights) {
        if d.z > 0.9 && Baked::luminance(emissive) > 0.4 {
            sea_height += base[3].abs();
            sea += 1;
        } else if d.z < -0.5 {
            rock_height += base[3].abs();
            rock += 1;
        }
    }
    assert!(day > 0.0 && night < day * 0.05, "lit {day}, dark {night}");
    assert!(sea > 20, "a molten sea on the dayside: {sea} texels");
    let (sea_mean, rock_mean) = (sea_height / sea as f32, rock_height / rock as f32);
    assert!(
        sea_mean < 0.25 * rock_mean,
        "the sea is flat: {sea_mean} against {rock_mean} on rock"
    );

    // SURF-20: a hot but solid airless world glows all over; a cold one
    // has no emissive row.
    let warm = rocky(
        Atmosphere::Airless,
        Water::None,
        false,
        Activity::Dead,
        1_000.0,
    );
    let glowing = bake(&mut app, &surveyed(&warm, 3), None);
    assert_eq!(glowing.rows, 2);
    assert!(glowing.mean_luminance(1) > 0.01);
    let cold = rocky(
        Atmosphere::Airless,
        Water::None,
        false,
        Activity::Dead,
        300.0,
    );
    let dark = bake(&mut app, &surveyed(&cold, 3), None);
    assert_eq!(dark.rows, 1);

    // SURF-22: rays brighten a dead world's surface; an active one, with
    // the same seed, has none of those bright streaks.
    let active = rocky(
        Atmosphere::Airless,
        Water::None,
        false,
        Activity::Active,
        300.0,
    );
    let unrayed = bake(&mut app, &surveyed(&active, 3), None);
    let bright = |b: &Baked| {
        b.inner(0)
            .iter()
            .filter(|(_, t)| Baked::luminance(*t) > 0.5)
            .count()
    };
    let (rayed_bright, unrayed_bright) = (bright(&dark), bright(&unrayed));
    assert!(
        rayed_bright > 50 && rayed_bright > 4 * unrayed_bright,
        "rays: {rayed_bright} bright texels dead, {unrayed_bright} active"
    );

    // SURF-13: a volcanic world's pits glow orange, and it has no rays.
    let io = facts(
        WorldKind::Moon,
        Atmosphere::Airless,
        Water::None,
        true,
        Activity::Extreme,
        110.0,
    );
    let volcanic = bake(&mut app, &surveyed(&io, 5), None);
    assert_eq!(volcanic.rows, 2);
    let hottest = volcanic
        .inner(1)
        .into_iter()
        .map(|(_, t)| t)
        .max_by(|a, b| Baked::luminance(*a).total_cmp(&Baked::luminance(*b)))
        .unwrap();
    assert!(hottest[0] > 0.5 && hottest[0] > hottest[1] && hottest[1] > hottest[2]);

    // SURF-10 to SURF-12, SURF-16, SURF-17: no relief under a deck or an
    // envelope; the hazy deck is redder than the dense one.
    let dense = rocky(
        Atmosphere::Dense,
        Water::None,
        false,
        Activity::Active,
        700.0,
    );
    let deck = bake(&mut app, &surveyed(&dense, 9), None);
    assert_eq!(deck.max_abs_height(), 0.0);
    let titan = facts(
        WorldKind::Moon,
        Atmosphere::Thin,
        Water::Ice,
        true,
        Activity::Dead,
        94.0,
    );
    let hazy = bake(&mut app, &surveyed(&titan, 9), None);
    assert_eq!(hazy.max_abs_height(), 0.0);
    let (deck_colour, hazy_colour) = (deck.mean_colour(), hazy.mean_colour());
    assert!(hazy_colour[0] / hazy_colour[2] > deck_colour[0] / deck_colour[2]);
    for kind in [
        WorldKind::GasGiant,
        WorldKind::IceGiant,
        WorldKind::SubNeptune,
    ] {
        let giant = facts(
            kind,
            Atmosphere::Envelope,
            Water::None,
            false,
            Activity::Dead,
            100.0,
        );
        let envelope = bake(&mut app, &surveyed(&giant, 2), None);
        assert_eq!(envelope.max_abs_height(), 0.0, "{kind:?}");
        assert_eq!(envelope.rows, 1);
    }
    let gas = facts(
        WorldKind::GasGiant,
        Atmosphere::Envelope,
        Water::None,
        false,
        Activity::Dead,
        100.0,
    );
    let ice = facts(
        WorldKind::IceGiant,
        Atmosphere::Envelope,
        Water::None,
        false,
        Activity::Dead,
        60.0,
    );
    let (gas_colour, ice_colour) = (
        bake(&mut app, &surveyed(&gas, 2), None).mean_colour(),
        bake(&mut app, &surveyed(&ice, 2), None).mean_colour(),
    );
    assert!(gas_colour[0] > gas_colour[2] && ice_colour[2] > ice_colour[0]);

    // SURF-18: polar caps where the water is ice, none without.
    let mars = rocky(Atmosphere::Thin, Water::Ice, false, Activity::Dead, 210.0);
    let dry = rocky(Atmosphere::Thin, Water::None, false, Activity::Dead, 210.0);
    let polar = |b: &Baked| {
        let cos_cap = tunables.cap_latitude_deg.to_radians().cos();
        let (sum, n) = b
            .inner(0)
            .iter()
            .filter(|(d, _)| d.y.abs() > cos_cap + 0.02)
            .fold((0.0, 0), |(s, n), (_, t)| (s + Baked::luminance(*t), n + 1));
        sum / n as f32
    };
    let capped = bake(&mut app, &surveyed(&mars, 4), None);
    let bare = bake(&mut app, &surveyed(&dry, 4), None);
    assert!(
        polar(&capped) > 0.5 && polar(&bare) < 0.3,
        "{} {}",
        polar(&capped),
        polar(&bare)
    );
    assert!(capped.max_abs_height() > 0.0);

    // SURF-19: ice is brighter than rock, and its craters shallower.
    let icy = rocky(
        Atmosphere::Airless,
        Water::Ice,
        false,
        Activity::Dead,
        100.0,
    );
    let frozen = bake(&mut app, &surveyed(&icy, 3), None);
    assert!(frozen.mean_luminance(0) > dark.mean_luminance(0) * 1.5);
    assert!(frozen.mean_abs_height() < dark.mean_abs_height());

    // SURF-35: across a face's edge the border texel of one face and the
    // last inner texel of its neighbour are baked for nearly the same
    // direction, so they sit clearly closer than texels a whole step
    // apart inside a face. A border copied from its own face's edge, or
    // a neighbour turned the wrong way, would be a step or more apart.
    let b = &dark;
    let w = b.face_px;
    let (mut seam, mut inside) = (0.0, 0.0);
    for y in 1..w - 1 {
        // +X (face 0) at s = -1 meets +Z (face 4) at s = +1.
        let across =
            Baked::luminance(b.texel(0, 0, y, 0)) - Baked::luminance(b.texel(4, w - 2, y, 0));
        let within =
            Baked::luminance(b.texel(4, w - 2, y, 0)) - Baked::luminance(b.texel(4, w - 3, y, 0));
        seam += across.abs();
        inside += within.abs();
    }
    assert!(seam <= 0.75 * inside, "seam {seam} inside {inside}");
}
