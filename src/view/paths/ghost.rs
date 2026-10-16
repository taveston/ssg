//! A ghost's look (PLNR-63): a circle the size of its object's marker,
//! outlined in dashes with an empty centre, drawn per screen pixel by a UI
//! material in `ghost.wesl`, so its dashes keep their length in pixels at
//! any size. Callers write `GhostLook`; the material follows it.

use bevy::asset::embedded_asset;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

/// What a ghost looks like: its colour (opacity included), its size
/// across, its outline's width and its dash length, in pixels.
#[derive(Component, Clone, Copy, PartialEq, Debug)]
pub struct GhostLook {
    pub color: Color,
    pub size_px: f32,
    pub width_px: f32,
    pub dash_px: f32,
}

/// The values the shader reads; field order matches `Ghost` in
/// `ghost.wesl`.
#[derive(ShaderType, Clone, Copy, PartialEq, Debug)]
pub struct GhostUniform {
    pub color: Vec4,
    pub size_px: f32,
    pub width_px: f32,
    pub dash_px: f32,
}

impl From<GhostLook> for GhostUniform {
    fn from(look: GhostLook) -> Self {
        Self {
            color: look.color.to_linear().to_vec4(),
            size_px: look.size_px,
            width_px: look.width_px,
            dash_px: look.dash_px,
        }
    }
}

/// The material that draws one ghost.
#[derive(AsBindGroup, Asset, TypePath, Clone, Debug)]
pub struct GhostMaterial {
    #[uniform(0)]
    pub ghost: GhostUniform,
}

impl UiMaterial for GhostMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://orphan_worlds/view/paths/ghost.wesl".into()
    }
}

pub struct GhostMaterialPlugin;

impl Plugin for GhostMaterialPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "ghost.wesl");
        app.add_plugins(UiMaterialPlugin::<GhostMaterial>::default())
            .add_systems(PostUpdate, sync_ghost_materials);
    }
}

/// Ghosts added or changed since the last frame, with their material.
type ChangedGhosts<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static GhostLook,
        Option<&'static MaterialNode<GhostMaterial>>,
    ),
    Changed<GhostLook>,
>;

/// Gives each new ghost its material, and copies a changed look into it.
fn sync_ghost_materials(
    mut commands: Commands,
    mut materials: ResMut<Assets<GhostMaterial>>,
    ghosts: ChangedGhosts,
) {
    for (entity, look, node) in &ghosts {
        let ghost = GhostUniform::from(*look);
        if let Some(mut material) = node.and_then(|n| materials.get_mut(&n.0)) {
            material.ghost = ghost;
            continue;
        }
        let handle = materials.add(GhostMaterial { ghost });
        commands.entity(entity).insert(MaterialNode(handle));
    }
}
