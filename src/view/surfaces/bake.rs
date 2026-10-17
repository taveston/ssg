//! Baking a body's surface into its cubemap strip (SURF-01, SURF-05,
//! SURF-30 to SURF-34): what a bake is of, the cache of bakes, the
//! scheduler that spreads each bake over frames, and the compute pass
//! that runs `bake.wesl` in the renderer.
//!
//! A bake is described by `BakeParams`, a plain value built from the
//! facts, the star, the seed and the knowledge value and nothing else
//! (`describe_surface`), so two bodies with equal inputs bake equal
//! pictures. `SurfaceBakes` keeps one strip per body and knowledge state,
//! up to `bake_cache_count`, dropping the least recently drawn but never
//! one drawn this frame, and bakes a body again when its description
//! changes. Close in, a body may also hold one **close face**: a single
//! face baked again finer (SURF-34), kept only while it is drawn.
//!
//! Each bake is cut into pieces: a cube's are its six faces, a close
//! face's are bands of rows, each about as many texels as a face of the
//! ordinary bake. Each frame the scheduler issues at most a few pieces
//! as jobs; the render world runs the jobs it can (the pipeline
//! compiled, the image ready) and reports each piece back, so a piece is
//! marked done only once the GPU has it, and one that was skipped is
//! issued again.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use bevy::asset::{RenderAssetUsages, embedded_asset};
use bevy::core_pipeline::schedule::camera_driver;
use bevy::image::{ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::binding_types::{texture_storage_2d, uniform_buffer};
use bevy::render::render_resource::{
    BindGroup, BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries,
    CachedComputePipelineId, CachedPipelineState, ComputePassDescriptor, ComputePipelineDescriptor,
    Extent3d, PipelineCache, ShaderStages, ShaderType, StorageTextureAccess, TextureDimension,
    TextureFormat, TextureUsages, UniformBuffer,
};
use bevy::render::renderer::{RenderContext, RenderDevice, RenderGraph, RenderQueue};
use bevy::render::texture::GpuImage;
use bevy::render::{Render, RenderApp, RenderStartup, RenderSystems};
use bevy::shader::ShaderCacheError;

use super::chooser::{
    Look, choose_look, crater_density, crater_scales_on, glow_strength, has_polar_caps,
    has_temperature_map, knowledge_value, octaves, override_look, peak_surface_temperature_k,
    ray_density, surface_teq_k,
};
use super::cube::{FACES, strip_size};
use super::star::StarLight;
use crate::content::generator::rng::Rng;
use crate::sim::body::Knowledge;
use crate::sim::planet_facts::{Atmosphere, LookOverride, PlanetFacts, Water, WorldKind};
use crate::view::tunables::Tunables;

/// The strip's texel format: albedo and emissive as linear light,
/// height in units of the radius, with enough precision for the
/// shading normal.
pub const STRIP_FORMAT: TextureFormat = TextureFormat::Rgba16Float;
/// The compute shader's workgroup is this many texels square.
const WORKGROUP: u32 = 8;
/// A piece issued this many frames ago with no word back is issued again:
/// the renderer runs a frame behind, so word of a piece takes up to two.
const REISSUE_AFTER_FRAMES: u64 = 2;
/// The surface noise is sampled this far from the origin, in noise
/// units: the seed's offset is drawn from this range.
const SEED_OFFSET_RANGE: (f64, f64) = (10.0, 50.0);

/// What `bake.wesl` reads; field order matches `BakeParams` there.
#[derive(ShaderType, Clone, Copy, Debug, PartialEq)]
pub struct BakeParams {
    pub seed_offset: Vec4,
    pub crater_scales: Vec4,
    pub haze_colour: Vec4,
    pub haze_mix: Vec4,
    pub look: u32,
    pub face_px: u32,
    pub rows: u32,
    /// The face a job bakes, the strip column it writes it to, and the
    /// strip's texel rows it covers, from `first_row` up to `end_row`. A
    /// description holds the whole of face 0; each job sets its own.
    pub face: u32,
    pub column: u32,
    pub first_row: u32,
    pub end_row: u32,
    pub octaves: f32,
    pub crater_density: f32,
    pub ray_density: f32,
    pub ray_full_age: f32,
    pub ray_max_age: f32,
    pub teq: f32,
    pub mapped: u32,
    pub peak_factor: f32,
    pub night_floor: f32,
    pub melt_t: f32,
    pub glow_t: f32,
    pub heat_ramp: f32,
    pub cap_edge: f32,
    pub knowledge: f32,
}

/// The facts a surface is drawn from, as the inset or the map gathers
/// them from a body.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceInputs<'a> {
    pub facts: Option<&'a PlanetFacts>,
    pub look_override: Option<LookOverride>,
    pub star: StarLight,
    pub seed: u64,
    pub knowledge: Knowledge,
}

/// What the seed draws for a surface: where its noise is sampled and the
/// heading of its prime meridian in the sky frame (SURF-41).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SeedDraws {
    pub offset: Vec3,
    pub heading: f32,
}

/// The seed's draws, fixed by the seed.
pub fn seed_draws(seed: u64) -> SeedDraws {
    let mut rng = Rng::new(seed);
    let (lo, hi) = SEED_OFFSET_RANGE;
    let mut draw = || rng.uniform(lo, hi) as f32;
    let offset = Vec3::new(draw(), draw(), draw());
    let heading = rng.uniform(0.0, std::f64::consts::TAU) as f32;
    SeedDraws { offset, heading }
}

/// SURF-05: the facts of a natural body that has none: an airless rock.
pub const NO_FACTS: PlanetFacts = PlanetFacts {
    kind: WorldKind::Rocky,
    gravity_g: 1.0,
    equilibrium_temperature_k: 200.0,
    atmosphere: Atmosphere::Airless,
    water: Water::None,
    locked: false,
    activity: None,
};

/// A body's surface as a plain description: its look, what its bake
/// reads, and the sizes of its strip.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Surface {
    pub look: Look,
    pub params: BakeParams,
    /// The knowledge value the picture is drawn at (SURF-25).
    pub knowledge: f32,
    /// The hottest surface temperature on the body, in K (SURF-21).
    pub peak_t_k: f32,
    /// Whether the body is locked to its star, so its prime meridian
    /// faces it (SURF-41).
    pub locked_to_star: bool,
    /// Whether a moon is locked to its planet, so its prime meridian
    /// faces that.
    pub locked_to_parent: bool,
    pub draws: SeedDraws,
}

impl Surface {
    /// The strip's face size in pixels.
    pub fn face_px(&self) -> u32 {
        self.params.face_px
    }

    /// How many rows of faces the strip holds: two when the body glows.
    pub fn rows(&self) -> u32 {
        self.params.rows
    }
}

/// SURF-01 / SURF-30: a body's surface from its facts, its star, its
/// seed and its knowledge state, and nothing else. The bake's face size
/// is the knowledge state's (SURF-32) unless `face_px` says otherwise,
/// as a test's small bake does; `fine_scales` turns the finest crater
/// scale on, as the close-in bake does (SURF-34).
pub fn describe_surface(
    inputs: &SurfaceInputs,
    face_px: Option<u32>,
    fine_scales: bool,
    tunables: &Tunables,
) -> Surface {
    let facts = inputs.facts.unwrap_or(&NO_FACTS);
    let look_override = inputs.look_override.and_then(override_look);
    let look = choose_look(facts, look_override, tunables);
    let knowledge = knowledge_value(inputs.knowledge, tunables);
    let teq = surface_teq_k(facts, look_override, tunables);
    let mapped = has_temperature_map(facts);
    let peak_t_k = peak_surface_temperature_k(teq, facts, tunables) as f32;
    let glows = matches!(look, Look::Lava | Look::Volcanic)
        || (look.is_rocky() && glow_strength(f64::from(peak_t_k), tunables) > 0.0);
    let [_, c2, c3, c4] = crater_scales_on(knowledge, tunables);
    let on = |on: bool| if on { 1.0 } else { 0.0 };
    let draws = seed_draws(inputs.seed);
    let face_px = face_px.unwrap_or_else(|| super::chooser::face_px(inputs.knowledge, tunables));
    let rows = if glows { 2 } else { 1 };
    let haze = tunables.hazy_colour;
    let mix = tunables.sub_neptune_haze_colour;
    let params = BakeParams {
        seed_offset: draws.offset.extend(0.0),
        crater_scales: Vec4::new(on(c2), on(c3), on(c4), on(fine_scales)),
        haze_colour: Vec4::new(haze[0], haze[1], haze[2], 0.0),
        haze_mix: Vec4::new(mix[0], mix[1], mix[2], tunables.sub_neptune_haze_mix),
        look: look.shader_kind(),
        face_px,
        rows,
        face: 0,
        column: 0,
        first_row: 0,
        end_row: face_px * rows,
        octaves: octaves(knowledge, tunables),
        crater_density: crater_density(inputs.star.age_gyr, facts.activity, tunables),
        ray_density: ray_density(look, facts.activity, tunables),
        ray_full_age: tunables.ray_full_age_gyr,
        ray_max_age: tunables.ray_max_age_gyr,
        teq: teq as f32,
        mapped: u32::from(mapped),
        peak_factor: tunables.locked_peak_factor,
        night_floor: tunables.night_floor_k,
        melt_t: tunables.melt_t_k,
        glow_t: tunables.glow_t_k,
        heat_ramp: tunables.heat_ramp_k,
        cap_edge: if has_polar_caps(look, facts) {
            tunables.cap_latitude_deg.to_radians().cos()
        } else {
            2.0
        },
        knowledge,
    };
    Surface {
        look,
        params,
        knowledge,
        peak_t_k,
        locked_to_star: super::chooser::locked_to_star(facts),
        locked_to_parent: facts.locked && facts.kind == WorldKind::Moon,
        draws,
    }
}

/// Which bake: a body's, in a knowledge state (SURF-33).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BakeKey {
    pub body: Entity,
    pub knowledge: Knowledge,
}

/// What a bake holds of a body's surface: the whole cube, or one face
/// baked again close in (SURF-34).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Part {
    Cube,
    Close(u32),
}

/// One piece of a bake: the face it is of, the strip column it is
/// written to, the strip's texel rows it covers, and how far along it is.
#[derive(Debug, Clone, Copy)]
struct Piece {
    face: u32,
    column: u32,
    rows: (u32, u32),
    done: bool,
    /// The frame the piece was last issued on, 0 for never.
    issued: u64,
}

/// SURF-31: a cube's pieces: its six faces, each whole, each in its own
/// column.
fn cube_pieces(params: &BakeParams) -> Vec<Piece> {
    let height = params.face_px * params.rows;
    (0..FACES)
        .map(|face| Piece {
            face,
            column: face,
            rows: (0, height),
            done: false,
            issued: 0,
        })
        .collect()
}

/// SURF-31 / SURF-34: a close face's pieces: `bands` bands of its rows,
/// all in the strip's one column, so a face many times finer than the
/// ordinary bake's still bakes a little each frame.
fn band_pieces(face: u32, params: &BakeParams, bands: u32) -> Vec<Piece> {
    let height = params.face_px * params.rows;
    let band = height.div_ceil(bands.max(1)).max(1);
    (0..height.div_ceil(band))
        .map(|at| Piece {
            face,
            column: 0,
            rows: (at * band, ((at + 1) * band).min(height)),
            done: false,
            issued: 0,
        })
        .collect()
}

/// One body's bake: its strip and how far along it is.
#[derive(Debug, Clone)]
pub struct BakeEntry {
    /// What the strip was baked from; a change means a new bake.
    pub params: BakeParams,
    pub strip: Handle<Image>,
    pieces: Vec<Piece>,
    /// SURF-31: every piece is in the strip; until then the body draws
    /// its disc marker.
    pub ready: bool,
    /// The frame the bake was last drawn or asked for (SURF-33).
    last_used: u64,
}

impl BakeEntry {
    /// A bake not yet begun, with a new strip as wide as its pieces'
    /// columns.
    fn new(
        params: &BakeParams,
        pieces: Vec<Piece>,
        images: &mut Assets<Image>,
        frame: u64,
    ) -> Self {
        let columns = pieces.iter().map(|p| p.column + 1).max().unwrap_or(1);
        Self {
            params: *params,
            strip: images.add(strip_image(params.face_px, columns, params.rows)),
            pieces,
            ready: false,
            last_used: frame,
        }
    }
}

/// One piece to bake this frame, for the render world: its strip, the
/// parameters with the piece's face, column and rows, and which piece it
/// is, to report back.
#[derive(Debug, Clone)]
pub struct BakeJob {
    pub strip: Handle<Image>,
    pub params: BakeParams,
    pub piece: u32,
}

/// The pieces the render world wants baked this frame (SURF-31).
#[derive(Resource, ExtractResource, Clone, Default, Debug)]
#[extract_app(RenderApp)]
pub struct PendingBakeJobs(pub Vec<BakeJob>);

/// One piece baked: its strip and which piece.
type PieceWord = (AssetId<Image>, u32);

/// Word from the render world of the pieces it baked: each strip and
/// piece, as the compute pass ran them.
#[derive(Resource, Clone, Default)]
pub struct BakedFaces(Arc<Mutex<Vec<PieceWord>>>);

impl BakedFaces {
    /// The render world's word that it baked a piece (for a cube, piece
    /// `n` is face `n`); a headless test stands in for it.
    pub fn report(&self, strip: AssetId<Image>, piece: u32) {
        self.0.lock().unwrap().push((strip, piece));
    }

    fn drain(&self) -> Vec<PieceWord> {
        std::mem::take(&mut *self.0.lock().unwrap())
    }
}

/// SURF-33 / SURF-34: the bakes kept, by body and knowledge state, and
/// each body's close face while it is drawn.
#[derive(Resource, Default, Debug)]
pub struct SurfaceBakes {
    entries: HashMap<(BakeKey, Part), BakeEntry>,
    /// Counts the frames, for recency. Requests made during a frame are
    /// stamped with it; the scheduler moves it on once it has evicted.
    frame: u64,
}

/// A strip for a surface: `columns` faces wide (six for a cube, one for
/// a close face), one row per set of layers, written by the compute
/// pass, sampled by the globe pass and readable back for the tests.
pub fn strip_image(face_px: u32, columns: u32, rows: u32) -> Image {
    let (width, height) = strip_size(face_px, columns, rows);
    let mut image = Image::new_uninit(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        STRIP_FORMAT,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
        | TextureUsages::STORAGE_BINDING
        | TextureUsages::COPY_SRC
        | TextureUsages::COPY_DST;
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor::linear());
    image
}

impl SurfaceBakes {
    /// SURF-30 / SURF-33: asks for a body's bake, starting it if there is
    /// none or its inputs changed, and marks it as drawn this frame.
    /// Starting one drops the body's other bakes (another knowledge
    /// state's, or stale inputs', and its close face), so a body holds
    /// one bake and no piece is spent on a picture nothing shows. Returns
    /// the entry; `ready` says whether the strip can be drawn.
    pub fn request(
        &mut self,
        key: BakeKey,
        params: &BakeParams,
        images: &mut Assets<Image>,
    ) -> &BakeEntry {
        let frame = self.frame;
        let slot = (key, Part::Cube);
        let kept = self
            .entries
            .get(&slot)
            .is_some_and(|entry| entry.params == *params);
        if !kept {
            self.entries.retain(|(other, _), _| other.body != key.body);
        }
        let entry = self
            .entries
            .entry(slot)
            .or_insert_with(|| BakeEntry::new(params, cube_pieces(params), images, frame));
        entry.last_used = frame;
        entry
    }

    /// SURF-34: asks for a body's close face: `face` of its cube baked
    /// again from `params` (finer, with the finer crater scales) in
    /// `bands` pieces, and marks it as drawn this frame. A body holds one
    /// close face: asking for another face, or with other inputs, drops
    /// the old one. The body's cube bake is left as it is. A close face
    /// not asked for during a frame is dropped at the frame's end.
    pub fn request_close(
        &mut self,
        key: BakeKey,
        face: u32,
        params: &BakeParams,
        bands: u32,
        images: &mut Assets<Image>,
    ) -> &BakeEntry {
        let frame = self.frame;
        let slot = (key, Part::Close(face));
        let kept = self
            .entries
            .get(&slot)
            .is_some_and(|entry| entry.params == *params);
        if !kept {
            self.entries
                .retain(|(other, part), _| other.body != key.body || *part == Part::Cube);
        }
        let entry = self.entries.entry(slot).or_insert_with(|| {
            BakeEntry::new(params, band_pieces(face, params, bands), images, frame)
        });
        entry.last_used = frame;
        entry
    }

    /// A body's bake as it stands, if one was asked for.
    #[cfg(test)]
    pub fn entry(&self, key: BakeKey) -> Option<&BakeEntry> {
        self.entries.get(&(key, Part::Cube))
    }

    /// A body's close face as it stands, with the face it is of, if one
    /// was asked for (SURF-34).
    #[cfg(test)]
    pub fn close_entry(&self, key: BakeKey) -> Option<(u32, &BakeEntry)> {
        self.entries
            .iter()
            .find_map(|((other, part), entry)| match part {
                Part::Close(face) if *other == key => Some((*face, entry)),
                _ => None,
            })
    }

    /// How many bakes are kept, close faces included.
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// SURF-31: marks the pieces the render world baked; a bake with
    /// every piece in is ready.
    fn take_word(&mut self, baked: &[PieceWord]) {
        for &(strip, piece) in baked {
            if let Some(entry) = self.entries.values_mut().find(|e| e.strip.id() == strip)
                && let Some(done) = entry.pieces.get_mut(piece as usize)
            {
                done.done = true;
                entry.ready = entry.pieces.iter().all(|p| p.done);
            }
        }
    }

    /// SURF-33 / SURF-34: drops every close face not drawn this frame,
    /// then the least recently drawn cube bakes beyond `keep`. A bake
    /// drawn this frame is never dropped, so however many globes are
    /// drawn at once each keeps its bake and finishes; the cache shrinks
    /// back to `keep` once fewer are drawn.
    fn evict(&mut self, keep: usize) {
        let now = self.frame;
        self.entries
            .retain(|(_, part), entry| *part == Part::Cube || entry.last_used >= now);
        let cubes = |entries: &HashMap<(BakeKey, Part), BakeEntry>| {
            entries
                .keys()
                .filter(|(_, part)| *part == Part::Cube)
                .count()
        };
        while cubes(&self.entries) > keep.max(1) {
            let Some(oldest) = self
                .entries
                .iter()
                .filter(|((_, part), entry)| *part == Part::Cube && entry.last_used < now)
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(slot, _)| *slot)
            else {
                break;
            };
            self.entries.remove(&oldest);
        }
    }

    /// SURF-31: the pieces to bake this frame, at most `per_frame`: cube
    /// bakes before close faces, so a body has a picture before it has a
    /// finer one, and within each the most recently asked-for first; each
    /// piece not yet done and not issued within the last frames.
    fn issue(&mut self, per_frame: usize) -> Vec<BakeJob> {
        let frame = self.frame;
        let mut pending: Vec<(Part, &mut BakeEntry)> = self
            .entries
            .iter_mut()
            .filter(|(_, entry)| !entry.ready)
            .map(|((_, part), entry)| (*part, entry))
            .collect();
        pending
            .sort_by_key(|(part, entry)| (*part != Part::Cube, std::cmp::Reverse(entry.last_used)));
        let mut jobs = Vec::new();
        for (_, entry) in pending {
            for (at, piece) in entry.pieces.iter_mut().enumerate() {
                if jobs.len() >= per_frame {
                    return jobs;
                }
                let waiting = piece.issued != 0 && frame - piece.issued < REISSUE_AFTER_FRAMES;
                if piece.done || waiting {
                    continue;
                }
                piece.issued = frame;
                jobs.push(BakeJob {
                    strip: entry.strip.clone(),
                    params: BakeParams {
                        face: piece.face,
                        column: piece.column,
                        first_row: piece.rows.0,
                        end_row: piece.rows.1,
                        ..entry.params
                    },
                    piece: at as u32,
                });
            }
        }
        jobs
    }
}

/// SURF-31: how many pieces to bake each frame so a cube bake takes at
/// most `bake_frames` frames.
pub fn pieces_per_frame(bake_frames: u32) -> usize {
    FACES.div_ceil(bake_frames.max(1)) as usize
}

/// SURF-31 / SURF-33: takes word of the pieces baked, drops the bakes
/// beyond the cache and the close faces not drawn this frame, then moves
/// on a frame and issues its pieces.
pub fn advance_bakes(
    mut bakes: ResMut<SurfaceBakes>,
    baked: Res<BakedFaces>,
    tunables: Res<Tunables>,
    mut jobs: ResMut<PendingBakeJobs>,
) {
    let word = baked.drain();
    bakes.take_word(&word);
    bakes.evict(tunables.bake_cache_count);
    bakes.frame += 1;
    jobs.0 = bakes.issue(pieces_per_frame(tunables.bake_frames));
}

/// The bake's compute pipeline in the render world.
#[derive(Resource)]
struct BakePipeline {
    layout: BindGroupLayoutDescriptor,
    pipeline: CachedComputePipelineId,
}

/// The path the shader is embedded at.
const BAKE_SHADER: &str = "embedded://orphan_worlds/view/surfaces/bake.wesl";

fn init_bake_pipeline(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    pipeline_cache: Res<PipelineCache>,
) {
    let layout = BindGroupLayoutDescriptor::new(
        "surface bake",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                uniform_buffer::<BakeParams>(false),
                texture_storage_2d(STRIP_FORMAT, StorageTextureAccess::WriteOnly),
            ),
        ),
    );
    let pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("surface bake".into()),
        layout: vec![layout.clone()],
        shader: asset_server.load(BAKE_SHADER),
        entry_point: Some("bake".into()),
        ..default()
    });
    commands.insert_resource(BakePipeline { layout, pipeline });
}

/// This frame's jobs with their bind groups, for the compute pass: each
/// job's strip, parameters and piece.
#[derive(Resource, Default)]
struct BakeBindGroups(Vec<(AssetId<Image>, BakeParams, u32, BindGroup)>);

/// Set once the bake pipeline's failure to compile has been logged, so
/// the error is said once, not every frame a face waits.
#[derive(Resource, Default)]
struct BakePipelineFailed(bool);

/// Binds each job's parameters and strip; a job whose strip the GPU
/// doesn't hold yet is left for the next frame.
fn prepare_bake_bind_groups(
    mut commands: Commands,
    pipeline: Res<BakePipeline>,
    pipeline_cache: Res<PipelineCache>,
    render_device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    jobs: Res<PendingBakeJobs>,
) {
    let mut bound = Vec::new();
    for job in &jobs.0 {
        let Some(gpu_image) = gpu_images.get(&job.strip) else {
            continue;
        };
        let mut uniform = UniformBuffer::from(job.params);
        uniform.write_buffer(&render_device, &queue);
        let bind_group = render_device.create_bind_group(
            "surface bake",
            &pipeline_cache.get_bind_group_layout(&pipeline.layout),
            &BindGroupEntries::sequential((&uniform, &gpu_image.texture_view)),
        );
        bound.push((job.strip.id(), job.params, job.piece, bind_group));
    }
    commands.insert_resource(BakeBindGroups(bound));
}

/// SURF-31: runs this frame's bake jobs, one dispatch per piece, and
/// reports each piece baked. Nothing runs until the pipeline has
/// compiled.
fn run_bakes(
    mut render_context: RenderContext,
    pipeline_cache: Res<PipelineCache>,
    pipeline: Res<BakePipeline>,
    bound: Res<BakeBindGroups>,
    baked: Res<BakedFaces>,
    mut failed: ResMut<BakePipelineFailed>,
) {
    if bound.0.is_empty() {
        return;
    }
    let Some(compute) = pipeline_cache.get_compute_pipeline(pipeline.pipeline) else {
        // A pipeline that failed to compile never bakes: the faces would
        // be issued again every few frames with nothing to show, so say
        // why, once. A shader still loading is not a failure.
        if let CachedPipelineState::Err(error) =
            pipeline_cache.get_compute_pipeline_state(pipeline.pipeline)
            && !matches!(error, ShaderCacheError::ShaderNotLoaded(_))
            && !failed.0
        {
            failed.0 = true;
            error!("the surface bake shader failed to compile; no globe will draw: {error}");
        }
        return;
    };
    let mut pass = render_context
        .command_encoder()
        .begin_compute_pass(&ComputePassDescriptor {
            label: Some("surface bake"),
            ..default()
        });
    pass.set_pipeline(compute);
    for (strip, params, piece, bind_group) in &bound.0 {
        pass.set_bind_group(0, bind_group, &[]);
        pass.dispatch_workgroups(
            params.face_px.div_ceil(WORKGROUP),
            (params.end_row - params.first_row).div_ceil(WORKGROUP),
            1,
        );
        baked.report(*strip, *piece);
    }
}

/// The bake's shader and compute pass, for the renderer. The cache and
/// the scheduler are `SurfacesPlugin`'s, so headless apps can run them.
pub struct BakeRenderPlugin;

impl Plugin for BakeRenderPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "bake.wesl");
        app.init_resource::<BakedFaces>()
            .init_resource::<PendingBakeJobs>()
            .add_plugins(ExtractResourcePlugin::<PendingBakeJobs>::default());
        let baked = app.world().resource::<BakedFaces>().clone();
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .insert_resource(baked)
            .init_resource::<BakeBindGroups>()
            .init_resource::<BakePipelineFailed>()
            .add_systems(RenderStartup, init_bake_pipeline)
            .add_systems(
                Render,
                prepare_bake_bind_groups.in_set(RenderSystems::PrepareBindGroups),
            )
            .add_systems(RenderGraph, run_bakes.before(camera_driver));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::planet_facts::Activity;
    use crate::view::surfaces::star::StarClass;

    fn facts() -> PlanetFacts {
        PlanetFacts {
            kind: WorldKind::Rocky,
            gravity_g: 0.9,
            equilibrium_temperature_k: 1_000.0,
            atmosphere: Atmosphere::Airless,
            water: Water::None,
            locked: true,
            activity: Some(Activity::Dead),
        }
    }

    fn inputs(facts: &PlanetFacts, seed: u64, knowledge: Knowledge) -> SurfaceInputs<'_> {
        SurfaceInputs {
            facts: Some(facts),
            look_override: None,
            star: StarLight {
                class: StarClass::K,
                age_gyr: 5.0,
            },
            seed,
            knowledge,
        }
    }

    /// SURF-01 / SURF-30: the description depends on the facts, the star,
    /// the seed and the knowledge state only: equal inputs give equal
    /// bakes, and each input changes it.
    #[test]
    fn the_surface_is_fixed_by_its_inputs() {
        let t = Tunables::default();
        let f = facts();
        let a = describe_surface(&inputs(&f, 7, Knowledge::Surveyed), None, false, &t);
        let b = describe_surface(&inputs(&f, 7, Knowledge::Surveyed), None, false, &t);
        assert_eq!(a, b);
        assert_eq!(a.look, Look::Lava);
        assert_eq!(a.face_px(), t.bake_face_px);
        assert_eq!(a.rows(), 2, "a lava world glows");
        assert!(a.locked_to_star && !a.locked_to_parent);
        let other_seed = describe_surface(&inputs(&f, 8, Knowledge::Surveyed), None, false, &t);
        assert_ne!(a.params.seed_offset, other_seed.params.seed_offset);
        assert_ne!(a.draws.heading, other_seed.draws.heading);
        let unsurveyed = describe_surface(&inputs(&f, 7, Knowledge::Unsurveyed), None, false, &t);
        assert_eq!(unsurveyed.face_px(), t.bake_face_unsurveyed_px);
        assert_eq!(unsurveyed.params.octaves, t.octaves_min);
        assert_eq!(
            unsurveyed.params.crater_scales,
            Vec4::new(0.0, 0.0, 0.0, 0.0)
        );
        assert_eq!(a.params.crater_scales, Vec4::new(1.0, 1.0, 1.0, 0.0));
        let close = describe_surface(&inputs(&f, 7, Knowledge::Surveyed), Some(64), true, &t);
        assert_eq!((close.face_px(), close.params.crater_scales.w), (64, 1.0));
        let cold = PlanetFacts {
            equilibrium_temperature_k: 200.0,
            ..f
        };
        let dark = describe_surface(&inputs(&cold, 7, Knowledge::Surveyed), None, false, &t);
        assert_eq!((dark.look, dark.rows()), (Look::Airless, 1));
        assert_eq!(dark.params.ray_density, t.ray_density);
        let mut older = inputs(&cold, 7, Knowledge::Surveyed);
        older.star.age_gyr = 8.0;
        let old = describe_surface(&older, None, false, &t);
        assert!(old.params.crater_density > dark.params.crater_density);
    }

    /// SURF-05: a body without facts is an airless rock; SURF-18: a cold
    /// terrestrial world with ice has caps down to the cap latitude;
    /// SURF-63: a lava override on a cool world takes the override's
    /// temperature.
    #[test]
    fn facts_missing_caps_and_the_override_temperature_shape_the_bake() {
        let t = Tunables::default();
        let mut none = inputs(&NO_FACTS, 1, Knowledge::Surveyed);
        none.facts = None;
        let bare = describe_surface(&none, None, false, &t);
        assert_eq!(bare.look, Look::Airless);
        assert!(bare.params.cap_edge > 1.0, "no caps");
        let mars = PlanetFacts {
            kind: WorldKind::Rocky,
            gravity_g: 0.38,
            equilibrium_temperature_k: 210.0,
            atmosphere: Atmosphere::Thin,
            water: Water::Ice,
            locked: false,
            activity: Some(Activity::Dead),
        };
        let capped = describe_surface(&inputs(&mars, 3, Knowledge::Surveyed), None, false, &t);
        assert_eq!(capped.look, Look::ColdTerrestrial);
        let edge = t.cap_latitude_deg.to_radians().cos();
        assert!((capped.params.cap_edge - edge).abs() < 1e-6);
        let earth = PlanetFacts {
            atmosphere: Atmosphere::Dense,
            water: Water::Surface,
            equilibrium_temperature_k: 255.0,
            ..mars
        };
        let mut overridden = inputs(&earth, 3, Knowledge::Surveyed);
        overridden.look_override = Some(LookOverride::Lava);
        let lava = describe_surface(&overridden, None, false, &t);
        assert_eq!(lava.look, Look::Lava);
        assert_eq!(lava.params.teq, t.earth_override_t_k);
        assert_eq!(lava.rows(), 2);
    }

    fn params(face_px: u32) -> BakeParams {
        let t = Tunables::default();
        let f = facts();
        describe_surface(
            &inputs(&f, 7, Knowledge::Surveyed),
            Some(face_px),
            false,
            &t,
        )
        .params
    }

    /// SURF-31 / SURF-33: a request starts a bake that is not ready; the
    /// faces are issued a few per frame, reissued only if no word comes
    /// back, and the bake is ready once every face is reported; a changed
    /// description starts it over; the cache keeps the most recently
    /// drawn bakes and drops the rest.
    #[test]
    fn bakes_spread_over_frames_and_are_kept_by_recency() {
        let mut images = Assets::<Image>::default();
        let mut bakes = SurfaceBakes::default();
        let key = |index: u32| BakeKey {
            body: Entity::from_raw_u32(index).unwrap(),
            knowledge: Knowledge::Surveyed,
        };
        let p = params(64);
        bakes.frame = 1;
        let strip = bakes.request(key(1), &p, &mut images).strip.clone();
        assert!(!bakes.entry(key(1)).unwrap().ready);
        assert_eq!(pieces_per_frame(6), 1);
        assert_eq!(pieces_per_frame(2), 3);
        assert_eq!(pieces_per_frame(0), 6);
        let jobs = bakes.issue(1);
        assert_eq!(jobs.len(), 1);
        assert_eq!((jobs[0].params.face, jobs[0].strip.id()), (0, strip.id()));
        // Issued again only after the renderer has had its frames.
        bakes.frame = 2;
        assert_eq!(bakes.issue(1)[0].params.face, 1);
        bakes.frame = 3;
        assert_eq!(bakes.issue(1)[0].params.face, 0, "face 0 had no word back");
        bakes.take_word(&[(strip.id(), 0), (strip.id(), 1)]);
        bakes.frame = 4;
        let jobs = bakes.issue(6);
        let faces: Vec<u32> = jobs.iter().map(|j| j.params.face).collect();
        assert_eq!(faces, [2, 3, 4, 5]);
        // Each face is written whole, to its own column of the strip.
        for job in &jobs {
            assert_eq!(job.piece, job.params.face);
            assert_eq!(job.params.column, job.params.face);
            assert_eq!((job.params.first_row, job.params.end_row), (0, 64 * 2));
        }
        let ready = |bakes: &SurfaceBakes| bakes.entry(key(1)).unwrap().ready;
        assert!(!ready(&bakes));
        bakes.take_word(&[(strip.id(), 2), (strip.id(), 3), (strip.id(), 4)]);
        assert!(!ready(&bakes));
        bakes.take_word(&[(strip.id(), 5)]);
        assert!(ready(&bakes));
        assert!(bakes.issue(6).is_empty());
        // The same description keeps the bake; a new one starts over.
        bakes.frame = 5;
        assert!(bakes.request(key(1), &p, &mut images).ready);
        let finer = params(128);
        assert!(!bakes.request(key(1), &finer, &mut images).ready);
        assert_ne!(bakes.entry(key(1)).unwrap().strip.id(), strip.id());
        // Recency: the body drawn longest ago goes first.
        for index in 2..=5 {
            bakes.frame = 10 + u64::from(index);
            bakes.request(key(index), &p, &mut images);
        }
        bakes.frame = 20;
        bakes.request(key(1), &p, &mut images);
        bakes.evict(3);
        assert_eq!(bakes.len(), 3);
        assert!(bakes.entry(key(1)).is_some());
        assert!(bakes.entry(key(5)).is_some());
        assert!(bakes.entry(key(2)).is_none());
    }

    /// SURF-33: a bake drawn this frame is never dropped, however many
    /// are drawn at once, so each finishes; once fewer are drawn the
    /// cache shrinks back to its size, least recently drawn first.
    #[test]
    fn bakes_drawn_this_frame_are_never_dropped() {
        let mut images = Assets::<Image>::default();
        let mut bakes = SurfaceBakes::default();
        let key = |index: u32| BakeKey {
            body: Entity::from_raw_u32(index).unwrap(),
            knowledge: Knowledge::Surveyed,
        };
        let p = params(64);
        // Five globes drawn every frame through a cache of two.
        let mut strips = Vec::new();
        for frame in 1..=40 {
            bakes.frame = frame;
            strips = (1..=5)
                .map(|index| bakes.request(key(index), &p, &mut images).strip.id())
                .collect();
            bakes.evict(2);
            assert_eq!(bakes.len(), 5);
            let jobs = bakes.issue(1);
            let word: Vec<PieceWord> = jobs.iter().map(|j| (j.strip.id(), j.piece)).collect();
            bakes.take_word(&word);
        }
        for index in 1..=5 {
            let entry = bakes.entry(key(index)).unwrap();
            assert!(entry.ready, "body {index} finished");
            assert_eq!(
                entry.strip.id(),
                strips[index as usize - 1],
                "never restarted"
            );
        }
        // Then only two are drawn: the others go, oldest first.
        bakes.frame = 41;
        bakes.request(key(4), &p, &mut images);
        bakes.request(key(5), &p, &mut images);
        bakes.evict(2);
        assert_eq!(bakes.len(), 2);
        assert!(bakes.entry(key(4)).is_some() && bakes.entry(key(5)).is_some());
    }

    /// SURF-34: a close face is one face baked in bands into a strip one
    /// face wide, after any cube bake; it leaves the body's cube bake
    /// alone, a body holds one at a time, and it goes as soon as a frame
    /// passes without it being drawn.
    #[test]
    fn a_close_face_bakes_in_bands_beside_the_cube_and_lives_while_drawn() {
        let mut images = Assets::<Image>::default();
        let mut bakes = SurfaceBakes::default();
        let key = BakeKey {
            body: Entity::from_raw_u32(1).unwrap(),
            knowledge: Knowledge::Surveyed,
        };
        let cube = params(16);
        let close = params(64);
        bakes.frame = 1;
        let cube_strip = bakes.request(key, &cube, &mut images).strip.id();
        let entry = bakes.request_close(key, 4, &close, 16, &mut images);
        let image = images.get(entry.strip.id()).unwrap();
        assert_eq!(image.texture_descriptor.size.width, 64, "one face wide");
        assert_eq!(image.texture_descriptor.size.height, 64 * 2);
        assert_eq!(bakes.len(), 2);
        // The cube's six faces come first, then the close face's bands.
        let jobs = bakes.issue(100);
        assert_eq!(jobs.len(), 6 + 16);
        assert!(jobs[..6].iter().all(|j| j.strip.id() == cube_strip));
        let bands = &jobs[6..];
        let mut next_row = 0;
        for (at, job) in bands.iter().enumerate() {
            assert_eq!(job.piece, at as u32);
            assert_eq!((job.params.face, job.params.column), (4, 0));
            assert_eq!(job.params.first_row, next_row);
            next_row = job.params.end_row;
        }
        assert_eq!(next_row, 64 * 2, "the bands cover every row");
        let word: Vec<PieceWord> = bands.iter().map(|j| (j.strip.id(), j.piece)).collect();
        bakes.take_word(&word);
        assert!(bakes.close_entry(key).unwrap().1.ready);
        // Another face replaces it; the cube stays.
        bakes.request_close(key, 2, &close, 16, &mut images);
        assert_eq!(bakes.close_entry(key).unwrap().0, 2);
        assert_eq!(bakes.len(), 2);
        assert_eq!(bakes.entry(key).unwrap().strip.id(), cube_strip);
        // Not drawn for a frame: it goes, the cube stays.
        bakes.evict(8);
        bakes.frame = 2;
        bakes.request(key, &cube, &mut images);
        bakes.evict(8);
        assert!(bakes.close_entry(key).is_none());
        assert!(bakes.entry(key).is_some());
    }

    /// SURF-33: a body's bake in a new knowledge state replaces its old
    /// one: the old entry is dropped, so no face is spent on it and the
    /// cache counts the body once; asking for the new state again keeps
    /// the new bake.
    #[test]
    fn a_knowledge_change_drops_the_bodys_old_bake() {
        let mut images = Assets::<Image>::default();
        let mut bakes = SurfaceBakes::default();
        let body = Entity::from_raw_u32(1).unwrap();
        let key = |knowledge| BakeKey { body, knowledge };
        let p = params(64);
        bakes.frame = 1;
        let old = bakes
            .request(key(Knowledge::Unsurveyed), &p, &mut images)
            .strip
            .clone();
        bakes.frame = 2;
        bakes.request(key(Knowledge::Surveyed), &params(128), &mut images);
        assert_eq!(bakes.len(), 1);
        assert!(bakes.entry(key(Knowledge::Unsurveyed)).is_none());
        let jobs = bakes.issue(6);
        assert!(jobs.iter().all(|job| job.strip.id() != old.id()));
        assert_eq!(jobs.len(), 6);
        bakes.frame = 3;
        let new = bakes
            .request(key(Knowledge::Surveyed), &params(128), &mut images)
            .strip
            .clone();
        assert_eq!(bakes.len(), 1);
        assert_eq!(new.id(), jobs[0].strip.id(), "the same bake goes on");
        // Another body's bake is untouched.
        let other = BakeKey {
            body: Entity::from_raw_u32(2).unwrap(),
            knowledge: Knowledge::Surveyed,
        };
        bakes.request(other, &p, &mut images);
        bakes.request(key(Knowledge::Unsurveyed), &p, &mut images);
        assert_eq!(bakes.len(), 2);
        assert!(bakes.entry(other).is_some());
    }

    /// SURF-30 / SURF-35: a strip is six faces wide, one row per set of
    /// layers, in the strip format, and can be written by the compute
    /// pass, sampled and read back.
    #[test]
    fn the_strip_is_six_faces_wide_and_bindable_every_way() {
        let image = strip_image(64, FACES, 2);
        assert_eq!(image.texture_descriptor.size.width, 384);
        assert_eq!(image.texture_descriptor.size.height, 128);
        assert_eq!(image.texture_descriptor.format, STRIP_FORMAT);
        let usage = image.texture_descriptor.usage;
        assert!(usage.contains(TextureUsages::STORAGE_BINDING));
        assert!(usage.contains(TextureUsages::TEXTURE_BINDING));
        assert!(usage.contains(TextureUsages::COPY_SRC));
    }
}
