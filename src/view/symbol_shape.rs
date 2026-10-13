//! Ship and station symbol parts as vector geometry (SYM-02 to SYM-08,
//! SYM-17). Each part is a UI node carrying a `SymbolPart`: which shape it
//! is, its colour and the size it is drawn at. A UI material evaluates the
//! shape per screen pixel in `symbol_shape.wesl`, so outlines are exactly
//! `symbol_stroke_px` wide and as sharp as the orbit rings at any size or
//! UI scale; nothing is a bitmap scaled to fit.
//!
//! Callers only write `SymbolPart`; `sync_materials` gives each new part
//! its material and copies later colour changes (fades) into it.

use bevy::asset::embedded_asset;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

use super::symbols::FrameShape;
use super::tunables::Tunables;

/// One part a symbol is built from.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum SymbolMask {
    /// The frame's outline, solid or dashed (SYM-02, SYM-08).
    Frame { shape: FrameShape, dashed: bool },
    /// The frame's opaque inside (SYM-03).
    Fill(FrameShape),
    /// Filled triangle: warship (SYM-05).
    Triangle,
    /// Outline drop: fleet supply (SYM-05).
    Drop,
    /// Outline crate: cargo (SYM-05).
    Crate,
    /// Outline unit box, with both diagonals when `crossed` (SYM-05,
    /// SYM-06).
    UnitBox { crossed: bool },
}

/// A symbol part on a node `size_px` across, drawn in `color`.
#[derive(Component, Clone, Copy, PartialEq, Debug)]
pub struct SymbolPart {
    pub mask: SymbolMask,
    pub color: Color,
    pub size_px: f32,
}

/// The values the shader reads; field order matches `SymbolShape` in
/// `symbol_shape.wesl`.
#[derive(ShaderType, Clone, Copy, PartialEq, Debug)]
pub struct ShapeUniform {
    pub color: Vec4,
    pub mask: u32,
    pub shape: u32,
    pub dashed: u32,
    pub crossed: u32,
    pub stroke_px: f32,
    pub size_px: f32,
}

/// The material that draws one symbol part.
#[derive(AsBindGroup, Asset, TypePath, Clone, Debug)]
pub struct SymbolShapeMaterial {
    #[uniform(0)]
    pub shape: ShapeUniform,
}

impl UiMaterial for SymbolShapeMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://orphan_worlds/view/symbol_shape.wesl".into()
    }
}

fn shape_code(shape: FrameShape) -> u32 {
    match shape {
        FrameShape::Circle => 0,
        FrameShape::Square => 1,
        FrameShape::Diamond => 2,
        FrameShape::Quatrefoil => 3,
    }
}

/// The shader's view of a part, with the outline `stroke_px` wide.
pub fn shape_uniform(part: &SymbolPart, stroke_px: f32) -> ShapeUniform {
    let (mask, shape, dashed, crossed) = match part.mask {
        SymbolMask::Frame { shape, dashed } => (0, shape_code(shape), dashed, false),
        SymbolMask::Fill(shape) => (1, shape_code(shape), false, false),
        SymbolMask::Triangle => (2, 0, false, false),
        SymbolMask::Drop => (3, 0, false, false),
        SymbolMask::Crate => (4, 0, false, false),
        SymbolMask::UnitBox { crossed } => (5, 0, false, crossed),
    };
    ShapeUniform {
        color: part.color.to_linear().to_vec4(),
        mask,
        shape,
        dashed: dashed as u32,
        crossed: crossed as u32,
        stroke_px,
        size_px: part.size_px,
    }
}

pub struct SymbolShapePlugin;

impl Plugin for SymbolShapePlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "symbol_shape.wesl");
        app.add_plugins(UiMaterialPlugin::<SymbolShapeMaterial>::default())
            .add_systems(PostUpdate, sync_materials);
    }
}

/// Parts added or recoloured since the last frame, with their material.
type ChangedParts<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static SymbolPart,
        Option<&'static MaterialNode<SymbolShapeMaterial>>,
    ),
    Changed<SymbolPart>,
>;

/// Gives each new part its own material (parts fade one by one), and
/// copies a changed part into its material.
fn sync_materials(
    mut commands: Commands,
    tunables: Res<Tunables>,
    mut materials: ResMut<Assets<SymbolShapeMaterial>>,
    parts: ChangedParts,
) {
    for (entity, part, node) in &parts {
        let shape = shape_uniform(part, tunables.symbol_stroke_px);
        if let Some(mut material) = node.and_then(|n| materials.get_mut(&n.0)) {
            material.shape = shape;
            continue;
        }
        let handle = materials.add(SymbolShapeMaterial { shape });
        commands.entity(entity).insert(MaterialNode(handle));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SYM-02 / SYM-08: a part's shape, dash and stroke reach the shader,
    /// with the outline `symbol_stroke_px` wide.
    #[test]
    fn a_part_reaches_the_shader_with_its_shape_and_stroke() {
        let part = SymbolPart {
            mask: SymbolMask::Frame {
                shape: FrameShape::Diamond,
                dashed: true,
            },
            color: Color::WHITE,
            size_px: 22.0,
        };
        let uniform = shape_uniform(&part, 1.5);
        assert_eq!((uniform.mask, uniform.shape, uniform.dashed), (0, 2, 1));
        assert_eq!((uniform.stroke_px, uniform.size_px), (1.5, 22.0));
        assert_eq!(uniform.color, Vec4::ONE);
        let crossed = SymbolPart {
            mask: SymbolMask::UnitBox { crossed: true },
            ..part
        };
        assert_eq!(shape_uniform(&crossed, 1.5).crossed, 1);
    }
}
