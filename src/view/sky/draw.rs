//! Drawing the sky (SKY-16 to SKY-20, SKY-22): two meshes with materials
//! of their own, drawn by the map camera.
//!
//! - The stars are one mesh: a small square per star, every corner
//!   carrying the star's direction. `stars.wesl` turns the direction by
//!   the camera's rotation only (never its position), projects it at
//!   infinity and spreads the square to the star's reach in screen pixels,
//!   then shades a Gaussian in it.
//! - The glow is one triangle over the whole viewport. `glow.wesl` finds
//!   each pixel's direction through the camera and works out the glow and
//!   its dither there.
//!
//! Both are drawn in the camera's opaque pass, which runs before the
//! see-through pass that holds the belt, the orbit rings and every other
//! map layer; they write no depth and add their light to what is there,
//! so they lie beneath everything (SKY-20). Light is added in the
//! display's own (sRGB) levels over the ground, as an opacity would blend
//! there, so the faint values keep the weight they have in mock-up 28.
//! Both clip to the map area: never under the top bar or the sidebar
//! (SKY-18). Neither is a picking target (SKY-19). The sky never changes
//! after it is made, whatever the clock does (SKY-17), and nothing hides
//! it (SKY-22).

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::light::NotShadowCaster;
use bevy::mesh::{Indices, MeshVertexBufferLayoutRef, PrimitiveTopology};
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, BlendComponent, BlendFactor, BlendOperation, BlendState, RenderPipelineDescriptor,
    ShaderType, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;

use super::generate::{Sky, make_sky};
use super::glow::{
    GLOW_CUTOFF_SIGMAS, LANE_CUTOFF_SIGMAS, LANE_FROM, LANE_OCTAVES, LANE_SCALE, LANE_SIGMA_DEG,
    LANE_TO, MOTTLE_OCTAVES, MOTTLE_SCALE,
};
use super::look::{REACH_EXTRA_PX, REACH_SIGMAS, star_opacity, star_sigma_px};
use super::projection::sky_to_render;
use crate::view::colors;
use crate::view::screen::{MapArea, MapView};
use crate::view::tunables::Tunables;

/// The mesh entity that draws the stars.
#[derive(Component)]
pub struct SkyStars;

/// The mesh entity that draws the glow.
#[derive(Component)]
pub struct SkyGlow;

/// The corners of a star's square, in units of its reach.
const CORNERS: [[f32; 2]; 4] = [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]];
/// One triangle that covers the whole viewport, in clip-space x and y.
const COVER: [[f32; 3]; 3] = [[-1.0, -1.0, 0.0], [3.0, -1.0, 0.0], [-1.0, 3.0, 0.0]];

/// What `stars.wesl` reads; field order matches `SkyStars` there.
#[derive(ShaderType, Clone, Copy, Debug, PartialEq)]
pub struct StarsUniform {
    /// The ground colour, sRGB.
    pub ground: Vec4,
    /// The map area in physical pixels: min x, min y, max x, max y.
    pub clip: Vec4,
    /// Physical pixels per logical pixel.
    pub px_scale: f32,
    pub reach_sigmas: f32,
    pub reach_extra_px: f32,
}

/// What `glow.wesl` reads; field order matches `SkyGlow` there. Angles
/// are in radians.
#[derive(ShaderType, Clone, Copy, Debug, PartialEq)]
pub struct GlowUniform {
    pub ground: Vec4,
    pub tone: Vec4,
    pub clip: Vec4,
    /// The band's normal, core and the direction a quarter turn on from
    /// the core, in the sky frame (east, north, up).
    pub normal: Vec4,
    pub core: Vec4,
    pub along: Vec4,
    pub lane_offset: Vec4,
    pub mottle_offset: Vec4,
    pub alpha: f32,
    pub sigma: f32,
    pub cutoff: f32,
    pub core_sigma: f32,
    pub core_floor: f32,
    pub lane_depth: f32,
    pub mottle: f32,
    pub lane_scale: f32,
    pub lane_from: f32,
    pub lane_to: f32,
    pub lane_sigma: f32,
    pub lane_cutoff: f32,
    pub mottle_scale: f32,
    pub dither_levels: f32,
    pub lane_octaves: u32,
    pub mottle_octaves: u32,
    pub dither_seed: u32,
}

/// The material the stars are drawn with.
#[derive(AsBindGroup, Asset, TypePath, Clone, Debug)]
pub struct SkyStarsMaterial {
    #[uniform(0)]
    pub stars: StarsUniform,
}

/// The material the glow is drawn with.
#[derive(AsBindGroup, Asset, TypePath, Clone, Debug)]
pub struct SkyGlowMaterial {
    #[uniform(0)]
    pub glow: GlowUniform,
}

const STARS_SHADER: &str = "embedded://orphan_worlds/view/sky/stars.wesl";
const GLOW_SHADER: &str = "embedded://orphan_worlds/view/sky/glow.wesl";

impl Material for SkyStarsMaterial {
    fn vertex_shader() -> ShaderRef {
        STARS_SHADER.into()
    }

    fn fragment_shader() -> ShaderRef {
        STARS_SHADER.into()
    }

    /// SKY-20: drawn in the opaque pass, before every see-through layer.
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Opaque
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let vertices = layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(1),
            Mesh::ATTRIBUTE_UV_1.at_shader_location(2),
            Mesh::ATTRIBUTE_COLOR.at_shader_location(3),
        ])?;
        descriptor.vertex.buffers = vec![vertices];
        draw_beneath(descriptor);
        Ok(())
    }
}

impl Material for SkyGlowMaterial {
    fn vertex_shader() -> ShaderRef {
        GLOW_SHADER.into()
    }

    fn fragment_shader() -> ShaderRef {
        GLOW_SHADER.into()
    }

    /// SKY-20: drawn in the opaque pass, before every see-through layer.
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Opaque
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let vertices = layout
            .0
            .get_layout(&[Mesh::ATTRIBUTE_POSITION.at_shader_location(0)])?;
        descriptor.vertex.buffers = vec![vertices];
        draw_beneath(descriptor);
        Ok(())
    }
}

/// SKY-20: the sky adds its light to what is drawn, writes no depth and
/// culls no faces, so whatever the map draws after it lies on top and is
/// never hidden by it.
pub fn draw_beneath(descriptor: &mut RenderPipelineDescriptor) {
    let add = BlendState {
        color: BlendComponent {
            src_factor: BlendFactor::One,
            dst_factor: BlendFactor::One,
            operation: BlendOperation::Add,
        },
        alpha: BlendComponent {
            src_factor: BlendFactor::Zero,
            dst_factor: BlendFactor::One,
            operation: BlendOperation::Add,
        },
    };
    if let Some(fragment) = descriptor.fragment.as_mut() {
        for target in fragment.targets.iter_mut().flatten() {
            target.blend = Some(add);
        }
    }
    if let Some(depth) = descriptor.depth_stencil.as_mut() {
        depth.depth_write_enabled = Some(false);
    }
    descriptor.primitive.cull_mode = None;
}

/// SKY-18: the map area (logical pixels) in the physical pixels the
/// shaders clip against.
pub fn clip_rect(area: &MapArea, scale: f32) -> Vec4 {
    let rect = area.rect();
    Vec4::new(rect.min.x, rect.min.y, rect.max.x, rect.max.y) * scale
}

/// A colour's sRGB channels, opaque.
fn srgb4(colour: Color) -> Vec4 {
    let c = colour.to_srgba();
    Vec4::new(c.red, c.green, c.blue, 1.0)
}

/// A sky-frame vector for a shader.
fn vec4_of(v: bevy::math::DVec3) -> Vec4 {
    v.as_vec3().extend(0.0)
}

/// The stars' mesh: four corners per star, each with the star's
/// direction in render space, its corner, its softness and opacity
/// (SKY-09, SKY-10) and its colour (SKY-12).
pub fn stars_mesh(sky: &Sky, tunables: &Tunables) -> Mesh {
    let count = sky.field.len() + sky.band_stars.len();
    let mut positions = Vec::with_capacity(4 * count);
    let mut corners = Vec::with_capacity(4 * count);
    let mut looks = Vec::with_capacity(4 * count);
    let mut colours = Vec::with_capacity(4 * count);
    let mut indices = Vec::with_capacity(6 * count);
    for (i, star) in sky.stars().enumerate() {
        let direction = sky_to_render(star.direction).to_array();
        let m = star.magnitude as f32;
        let look = [star_sigma_px(m, tunables), star_opacity(m, tunables)];
        let [r, g, b] = star.colour;
        for corner in CORNERS {
            positions.push(direction);
            corners.push(corner);
            looks.push(look);
            colours.push([r, g, b, 1.0]);
        }
        let base = 4 * i as u32;
        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, corners)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_1, looks)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colours)
    .with_inserted_indices(Indices::U32(indices))
}

/// The glow's mesh: one triangle covering the viewport.
fn cover_mesh() -> Mesh {
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, COVER.to_vec())
}

/// The stars' uniform before the map area is known.
fn stars_uniform() -> StarsUniform {
    StarsUniform {
        ground: srgb4(colors::GROUND),
        clip: Vec4::ZERO,
        px_scale: 1.0,
        reach_sigmas: REACH_SIGMAS,
        reach_extra_px: REACH_EXTRA_PX,
    }
}

/// The glow's uniform for a sky (SKY-13 to SKY-15), before the map area
/// is known.
pub fn glow_uniform(sky: &Sky, tunables: &Tunables) -> GlowUniform {
    let radians = |deg: f32| deg.to_radians();
    let band = &sky.band;
    GlowUniform {
        ground: srgb4(colors::GROUND),
        tone: srgb4(tunables.sky_tone.into()),
        clip: Vec4::ZERO,
        normal: vec4_of(band.normal),
        core: vec4_of(band.core),
        along: vec4_of(band.along()),
        lane_offset: Vec3::from_array(sky.noise.lanes).extend(0.0),
        mottle_offset: Vec3::from_array(sky.noise.mottle).extend(0.0),
        alpha: tunables.sky_glow_alpha,
        sigma: radians(tunables.sky_glow_sigma_deg),
        cutoff: radians(GLOW_CUTOFF_SIGMAS * tunables.sky_glow_sigma_deg),
        core_sigma: radians(tunables.sky_core_sigma_deg),
        core_floor: tunables.sky_core_floor,
        lane_depth: tunables.sky_lane_depth,
        mottle: tunables.sky_glow_mottle,
        lane_scale: LANE_SCALE,
        lane_from: LANE_FROM,
        lane_to: LANE_TO,
        lane_sigma: radians(LANE_SIGMA_DEG),
        lane_cutoff: radians(LANE_CUTOFF_SIGMAS * LANE_SIGMA_DEG),
        mottle_scale: MOTTLE_SCALE,
        dither_levels: tunables.sky_dither_levels,
        lane_octaves: LANE_OCTAVES,
        mottle_octaves: MOTTLE_OCTAVES,
        dither_seed: sky.noise.dither,
    }
}

/// SKY-01 / SKY-22: make the sky from the default seed when the game
/// starts and put it beneath the map, always on.
pub fn spawn_sky(
    mut commands: Commands,
    tunables: Res<Tunables>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut stars: ResMut<Assets<SkyStarsMaterial>>,
    mut glows: ResMut<Assets<SkyGlowMaterial>>,
) {
    let sky = make_sky(tunables.sky_default_seed, &tunables);
    let beneath = (
        Transform::default(),
        Visibility::Visible,
        NoFrustumCulling,
        NotShadowCaster,
        Pickable::IGNORE,
    );
    commands.spawn((
        SkyStars,
        Mesh3d(meshes.add(stars_mesh(&sky, &tunables))),
        MeshMaterial3d(stars.add(SkyStarsMaterial {
            stars: stars_uniform(),
        })),
        beneath.clone(),
    ));
    commands.spawn((
        SkyGlow,
        Mesh3d(meshes.add(cover_mesh())),
        MeshMaterial3d(glows.add(SkyGlowMaterial {
            glow: glow_uniform(&sky, &tunables),
        })),
        beneath,
    ));
}

/// SKY-18: keep both materials clipped to this frame's map area, so the
/// sky follows the sidebar's slide as the map does. A material is only
/// written when its clip changes.
pub fn clip_sky_to_map(
    map: MapView,
    stars: Query<&MeshMaterial3d<SkyStarsMaterial>>,
    glows: Query<&MeshMaterial3d<SkyGlowMaterial>>,
    mut star_materials: ResMut<Assets<SkyStarsMaterial>>,
    mut glow_materials: ResMut<Assets<SkyGlowMaterial>>,
) {
    let Some(area) = map.area() else {
        return;
    };
    let scale = map.scale_factor();
    let clip = clip_rect(&area, scale);
    for handle in &stars {
        let stale = star_materials
            .get(&handle.0)
            .is_some_and(|m| m.stars.clip != clip || m.stars.px_scale != scale);
        if stale && let Some(mut material) = star_materials.get_mut(&handle.0) {
            material.stars.clip = clip;
            material.stars.px_scale = scale;
        }
    }
    for handle in &glows {
        let stale = glow_materials
            .get(&handle.0)
            .is_some_and(|m| m.glow.clip != clip);
        if stale && let Some(mut material) = glow_materials.get_mut(&handle.0) {
            material.glow.clip = clip;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::mesh::VertexAttributeValues;
    use bevy::render::render_resource::{
        ColorTargetState, ColorWrites, CompareFunction, DepthStencilState, FragmentState,
        TextureFormat,
    };

    /// SKY-20: the sky's pipelines add light beneath the map and write no
    /// depth, and its materials draw in the opaque pass, which runs before
    /// the see-through pass that draws every other map layer (the belt,
    /// rings, tails and the frame cue are all see-through or overlay).
    #[test]
    fn the_sky_draws_beneath_every_other_layer() {
        let mut descriptor = RenderPipelineDescriptor {
            fragment: Some(FragmentState {
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Rgba8UnormSrgb,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(CompareFunction::GreaterEqual),
                stencil: default(),
                bias: default(),
            }),
            ..default()
        };
        draw_beneath(&mut descriptor);
        let target = descriptor.fragment.unwrap().targets[0].clone().unwrap();
        let blend = target.blend.unwrap();
        assert_eq!(
            blend.color.dst_factor,
            BlendFactor::One,
            "adds to what is there"
        );
        assert_eq!(blend.alpha.src_factor, BlendFactor::Zero);
        assert_eq!(
            descriptor.depth_stencil.unwrap().depth_write_enabled,
            Some(false)
        );
        let sky = make_sky(1, &few_stars());
        let stars = SkyStarsMaterial {
            stars: stars_uniform(),
        };
        let glow = SkyGlowMaterial {
            glow: glow_uniform(&sky, &few_stars()),
        };
        assert_eq!(stars.alpha_mode(), AlphaMode::Opaque);
        assert_eq!(glow.alpha_mode(), AlphaMode::Opaque);
    }

    fn few_stars() -> Tunables {
        Tunables {
            sky_field_stars: 50,
            sky_band_stars: 70,
            ..Tunables::default()
        }
    }

    /// SKY-09 / SKY-10 / SKY-12: every star's square carries its render
    /// direction, its softness and opacity from its magnitude, and its
    /// colour.
    #[test]
    fn the_star_mesh_carries_each_star_at_its_direction() {
        let t = few_stars();
        let sky = make_sky(5, &t);
        let mesh = stars_mesh(&sky, &t);
        assert_eq!(mesh.count_vertices(), 4 * 120);
        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("no positions");
        };
        let Some(VertexAttributeValues::Float32x2(looks)) = mesh.attribute(Mesh::ATTRIBUTE_UV_1)
        else {
            panic!("no looks");
        };
        for (i, star) in sky.stars().enumerate() {
            let m = star.magnitude as f32;
            for corner in 0..4 {
                let v = 4 * i + corner;
                assert_eq!(positions[v], sky_to_render(star.direction).to_array());
                assert_eq!(looks[v], [star_sigma_px(m, &t), star_opacity(m, &t)]);
            }
        }
    }

    /// SKY-18: the clip rectangle is the map area, below the top bar and
    /// left of the sidebar, in physical pixels.
    #[test]
    fn the_clip_is_the_map_area_in_physical_pixels() {
        let area = MapArea::of(Vec2::new(960.0, 720.0));
        let clip = clip_rect(&area, 1.25);
        assert_eq!(clip.x, 0.0);
        assert_eq!(clip.y, area.offset.y * 1.25);
        assert_eq!(clip.z, 960.0 * 1.25);
        assert_eq!(clip.w, 720.0 * 1.25);
    }
}
