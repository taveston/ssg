//! How a stroke of the frame cue is drawn (REF-01, REF-05, REF-06): a UI
//! node laid along the stroke and turned to its direction, filled by the
//! material in `stroke.wesl`, which draws the dots or the solid line, the
//! ticks across it and the arrowhead per screen pixel, faded along its
//! length. Callers write [`CueStroke`] and place the node with
//! [`stroke_node_box`]; the material follows the look.

use bevy::asset::embedded_asset;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

use super::geometry::{MAX_TICKS, Stroke, StrokeStyle};

/// A tick across an arm is this wide.
pub const TICK_WIDTH_PX: f32 = 1.0;
/// An arrowhead is this wide for each pixel of its length.
pub const ARROW_WIDTH_PER_LENGTH: f32 = 0.75;
/// Room round a stroke inside its node, so its edges soften unclipped.
const PAD_PX: f32 = 1.0;

/// One stroke of the cue as drawn this frame, in `color` with the
/// stroke's own opacity, or nothing while `stroke` is `None`.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct CueStroke {
    pub color: Color,
    pub stroke: Option<Stroke>,
}

/// The node a stroke is drawn in: its top-left and size before turning,
/// and the turn about its centre (clockwise on screen, y down).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrokeBox {
    pub min: Vec2,
    pub size: Vec2,
    pub rotation: Rot2,
}

/// Half the height a stroke needs across its line: its dots or line, its
/// ticks and its arrowhead.
fn half_height(stroke: &Stroke) -> f32 {
    let line = match stroke.style {
        StrokeStyle::Dotted { dot_px, .. } => dot_px / 2.0,
        StrokeStyle::Solid { width_px, arrow_px } => {
            (width_px / 2.0).max(arrow_px * ARROW_WIDTH_PER_LENGTH / 2.0)
        }
    };
    line.max(stroke.tick_px / 2.0) + PAD_PX
}

/// The node that holds `stroke`: as long as the stroke with a pixel to
/// spare at each end, centred on its middle and turned to its direction.
pub fn stroke_node_box(stroke: &Stroke) -> StrokeBox {
    let size = Vec2::new(stroke.length + 2.0 * PAD_PX, 2.0 * half_height(stroke));
    let centre = stroke.at(stroke.length / 2.0);
    StrokeBox {
        min: centre - size / 2.0,
        size,
        rotation: Rot2::radians(stroke.dir.y.atan2(stroke.dir.x)),
    }
}

/// The values the shader reads; field order matches `CueStroke` in
/// `stroke.wesl`. Lengths are logical pixels along the stroke from its
/// start; `ticks` packs four tick distances in each vector.
#[derive(ShaderType, Clone, Copy, Debug, PartialEq)]
pub struct CueStrokeUniform {
    pub color: Vec4,
    pub node_size: Vec2,
    pub length_px: f32,
    pub alpha_start: f32,
    pub alpha_end: f32,
    pub width_px: f32,
    /// 0 for a solid line.
    pub pitch_px: f32,
    pub tick_px: f32,
    pub tick_width_px: f32,
    pub tick_count: u32,
    pub arrow_px: f32,
    pub arrow_half_px: f32,
    pub ticks: [Vec4; MAX_TICKS / 4],
}

impl Default for CueStrokeUniform {
    fn default() -> Self {
        Self {
            color: Vec4::ZERO,
            node_size: Vec2::ONE,
            length_px: 0.0,
            alpha_start: 0.0,
            alpha_end: 0.0,
            width_px: 0.0,
            pitch_px: 0.0,
            tick_px: 0.0,
            tick_width_px: TICK_WIDTH_PX,
            tick_count: 0,
            arrow_px: 0.0,
            arrow_half_px: 0.0,
            ticks: [Vec4::ZERO; MAX_TICKS / 4],
        }
    }
}

impl From<&CueStroke> for CueStrokeUniform {
    fn from(look: &CueStroke) -> Self {
        let Some(stroke) = &look.stroke else {
            return Self::default();
        };
        let (width_px, pitch_px, arrow_px) = match stroke.style {
            StrokeStyle::Dotted { dot_px, pitch_px } => (dot_px, pitch_px, 0.0),
            StrokeStyle::Solid { width_px, arrow_px } => (width_px, 0.0, arrow_px),
        };
        let mut ticks = [Vec4::ZERO; MAX_TICKS / 4];
        let count = stroke.ticks.len().min(MAX_TICKS);
        for (i, &tick) in stroke.ticks.iter().take(count).enumerate() {
            ticks[i / 4][i % 4] = tick;
        }
        Self {
            color: look.color.to_linear().to_vec4(),
            node_size: stroke_node_box(stroke).size,
            length_px: stroke.length,
            alpha_start: stroke.alpha_start,
            alpha_end: stroke.alpha_end,
            width_px,
            pitch_px,
            tick_px: stroke.tick_px,
            tick_width_px: TICK_WIDTH_PX,
            tick_count: count as u32,
            arrow_px,
            arrow_half_px: arrow_px * ARROW_WIDTH_PER_LENGTH / 2.0,
            ticks,
        }
    }
}

/// The material that draws one stroke.
#[derive(AsBindGroup, Asset, TypePath, Clone, Debug)]
pub struct CueStrokeMaterial {
    #[uniform(0)]
    pub stroke: CueStrokeUniform,
}

impl UiMaterial for CueStrokeMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://orphan_worlds/view/frame_cue/stroke.wesl".into()
    }
}

/// The cue's material, which needs a renderer; the cue itself is
/// `FrameCuePlugin`.
pub struct CueStrokeMaterialPlugin;

impl Plugin for CueStrokeMaterialPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "stroke.wesl");
        app.add_plugins(UiMaterialPlugin::<CueStrokeMaterial>::default())
            .add_systems(PostUpdate, sync_stroke_materials);
    }
}

/// Strokes changed since the last frame, with their material.
type ChangedStrokes<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static CueStroke,
        Option<&'static MaterialNode<CueStrokeMaterial>>,
    ),
    Changed<CueStroke>,
>;

/// Gives each new stroke its material, and copies a changed look into it.
fn sync_stroke_materials(
    mut commands: Commands,
    mut materials: ResMut<Assets<CueStrokeMaterial>>,
    strokes: ChangedStrokes,
) {
    for (entity, look, node) in &strokes {
        let stroke = CueStrokeUniform::from(look);
        if let Some(mut material) = node.and_then(|n| materials.get_mut(&n.0)) {
            material.stroke = stroke;
            continue;
        }
        let handle = materials.add(CueStrokeMaterial { stroke });
        commands.entity(entity).insert(MaterialNode(handle));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arm() -> Stroke {
        Stroke {
            start: Vec2::new(100.0, 100.0),
            dir: Vec2::Y,
            length: 50.0,
            alpha_start: 1.0,
            alpha_end: 0.45,
            style: StrokeStyle::Dotted {
                dot_px: 2.0,
                pitch_px: 5.0,
            },
            ticks: vec![10.0, 20.0, 30.0, 40.0, 45.0],
            tick_px: 9.0,
        }
    }

    /// REF-01: a stroke's node is centred on the stroke's middle, as long
    /// as the stroke plus its padding, and turned to its direction.
    #[test]
    fn a_stroke_node_lies_along_the_stroke() {
        let node = stroke_node_box(&arm());
        assert_eq!(node.size, Vec2::new(52.0, 11.0));
        assert_eq!(node.min + node.size / 2.0, Vec2::new(100.0, 125.0));
        assert!((node.rotation.as_radians() - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
    }

    /// REF-05: the shader gets every tick, four to a vector, and the dot
    /// pitch; a hidden stroke draws nothing.
    #[test]
    fn the_uniform_carries_the_ticks_and_dots() {
        let look = CueStroke {
            color: Color::WHITE,
            stroke: Some(arm()),
        };
        let uniform = CueStrokeUniform::from(&look);
        assert_eq!(uniform.tick_count, 5);
        assert_eq!(uniform.ticks[0], Vec4::new(10.0, 20.0, 30.0, 40.0));
        assert_eq!(uniform.ticks[1].x, 45.0);
        assert_eq!((uniform.width_px, uniform.pitch_px), (2.0, 5.0));
        let hidden = CueStroke {
            color: Color::WHITE,
            stroke: None,
        };
        assert_eq!(CueStrokeUniform::from(&hidden).length_px, 0.0);
    }
}
