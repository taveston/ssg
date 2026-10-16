//! The label halo (LBL-05): an outline round a text's glyphs in the ground
//! colour, drawn under the text and over everything beneath it, so a ring
//! or line that a label crosses breaks close round the letters and stays
//! whole elsewhere. It follows the glyphs; it is not a box.
//!
//! Any UI text takes it by carrying [`TextHalo`]. Bevy's UI draws one
//! shadow per text; the halo instead draws the text's own glyphs again,
//! in the halo colour, at a ring of small offsets round the text, at the
//! text's own place in the UI stack just below its glyphs. Because the
//! copies are the glyphs Bevy laid out for the text, the halo follows the
//! text's content, place, visibility and fade with no state of its own.
//! The width comes from `label_halo_px` and follows the tunable.

use std::f32::consts::TAU;

use bevy::camera::visibility::InheritedVisibility;
use bevy::color::LinearRgba;
use bevy::math::Affine2;
use bevy::prelude::*;
use bevy::render::sync_world::MainEntity;
use bevy::render::{Extract, ExtractSchedule, RenderApp};
use bevy::text::{PositionedGlyph, TextLayoutInfo};
use bevy::ui::{ComputedStackIndex, ComputedUiTargetCamera, UiGlobalTransform};
use bevy::ui_render::{
    ExtractedGlyph, ExtractedUiItem, ExtractedUiNode, ExtractedUiNodes, RenderUiSystems,
    UiCameraMap, stack_z_offsets,
};

use crate::view::colors;
use crate::view::tunables::Tunables;

/// The halo sits just under its text's glyphs in the UI stack, above the
/// text node's own background and image.
const HALO_Z: f32 = stack_z_offsets::TEXT - 0.001;
/// The outer ring of offsets is spaced at most this far apart, in
/// logical pixels, so the outline has no gaps between the copies.
const MAX_STEP_PX: f32 = 1.0;
/// The fewest copies in a ring, so even a thin halo is round.
const MIN_COPIES: usize = 8;

/// A halo round this text's glyphs: `width_px` logical pixels wide, in
/// `color`, faded with the text (LBL-05). `TextHalo::ground()` is the
/// label halo; its width is kept at `label_halo_px`.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct TextHalo {
    /// How far the outline reaches beyond the glyphs, in logical pixels.
    pub width_px: f32,
    /// The outline's colour, before the text's own fade.
    pub color: Color,
}

impl TextHalo {
    /// The label halo: the ground colour, its width set from the tunables
    /// once spawned (LBL-05).
    pub fn ground() -> Self {
        Self {
            width_px: 0.0,
            color: colors::GROUND,
        }
    }
}

/// The offsets, in logical pixels, at which the glyphs are drawn again to
/// make a halo `width_px` wide: a ring at the full width, spaced so the
/// copies touch, and for a halo wider than a pixel an inner ring at half
/// the width, so small marks such as dots get no gap next to them.
pub fn halo_offsets(width_px: f32) -> Vec<Vec2> {
    if width_px <= 0.0 {
        return Vec::new();
    }
    let ring = |radius: f32| {
        let count = ((TAU * radius / MAX_STEP_PX).ceil() as usize).max(MIN_COPIES);
        (0..count).map(move |i| Vec2::from_angle(TAU * i as f32 / count as f32) * radius)
    };
    let mut offsets: Vec<Vec2> = ring(width_px).collect();
    if width_px > 1.0 {
        offsets.extend(ring(width_px * 0.5));
    }
    offsets
}

/// The colour a halo is drawn in: its own, faded as far as its text is,
/// so the halo fades in and out with the label (LBL-04, LBL-05).
pub fn halo_color(halo: &TextHalo, text: Color) -> Color {
    halo.color.with_alpha(halo.color.alpha() * text.alpha())
}

pub struct HaloPlugin;

impl Plugin for HaloPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, (apply_halo_width, redraw_changed_halos).chain());
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_systems(
                ExtractSchedule,
                extract_text_halos.in_set(RenderUiSystems::ExtractTextShadows),
            );
        }
    }
}

/// Keeps every halo's width at `label_halo_px`: a new halo takes it, and
/// all follow when the tunable changes (LBL-05).
fn apply_halo_width(tunables: Res<Tunables>, mut halos: Query<&mut TextHalo>) {
    let width = tunables.label_halo_px;
    for mut halo in &mut halos {
        if halo.width_px != width {
            halo.width_px = width;
        }
    }
}

/// The UI redraws a text only when something it watches changes; a halo
/// that changes marks its text's colour changed so the text, and so the
/// halo, are drawn again.
fn redraw_changed_halos(
    mut changed: Query<&mut TextColor, (With<TextHalo>, Changed<TextHalo>)>,
    mut removed: RemovedComponents<TextHalo>,
    mut colors: Query<&mut TextColor, Without<TextHalo>>,
) {
    for mut color in &mut changed {
        color.set_changed();
    }
    for entity in removed.read() {
        if let Ok(mut color) = colors.get_mut(entity) {
            color.set_changed();
        }
    }
}

/// Draws each halo: the text's glyphs again in the halo colour at each
/// offset, just under the text. Runs for the texts the UI redraws this
/// frame, beside Bevy's own text shadows.
#[allow(clippy::type_complexity)]
fn extract_text_halos(
    mut commands: Commands,
    extracted: ResMut<ExtractedUiNodes>,
    texts: Extract<
        Query<(
            Entity,
            &ComputedNode,
            &ComputedStackIndex,
            &UiGlobalTransform,
            &ComputedUiTargetCamera,
            &InheritedVisibility,
            Option<&CalculatedClip>,
            &TextLayoutInfo,
            &TextColor,
            &TextHalo,
        )>,
    >,
    camera_map: Extract<UiCameraMap>,
) {
    let extracted = extracted.into_inner();
    let mut camera_mapper = camera_map.get_mapper();
    let changed: Vec<MainEntity> = extracted.changed.iter().copied().collect();
    for main_entity in changed {
        let Ok((entity, node, stack, transform, target, visible, clip, layout, color, halo)) =
            texts.get(main_entity.entity())
        else {
            continue;
        };
        if !visible.get() || node.is_empty() || layout.glyphs.is_empty() {
            continue;
        }
        let Some(camera) = camera_mapper.map(target) else {
            continue;
        };
        let tint: LinearRgba = halo_color(halo, color.0).into();
        if tint.alpha <= 0.0 {
            continue;
        }
        let group = &mut extracted
            .uinodes
            .entry(entity.into())
            .or_insert_with(|| (camera, Default::default()))
            .1;
        for offset in halo_offsets(halo.width_px) {
            let node_transform = Affine2::from(*transform)
                * Affine2::from_translation(
                    node.content_box().min + offset / node.inverse_scale_factor(),
                );
            for glyphs in glyph_runs(&layout.glyphs) {
                group.insert(
                    commands.spawn_empty().id(),
                    ExtractedUiNode {
                        transform: node_transform,
                        z_order: stack.0 as f32 + HALO_Z,
                        image: glyphs[0].atlas_info.texture,
                        clip: clip.cloned(),
                        item: ExtractedUiItem::Glyphs {
                            glyphs: glyphs
                                .iter()
                                .map(|glyph| ExtractedGlyph {
                                    color: tint,
                                    translation: glyph.position,
                                    rect: glyph.atlas_info.rect,
                                })
                                .collect(),
                        },
                    },
                );
            }
        }
    }
}

/// The text's glyphs split into runs that share one atlas texture, since
/// each drawn item samples one texture.
fn glyph_runs(glyphs: &[PositionedGlyph]) -> impl Iterator<Item = &[PositionedGlyph]> {
    glyphs.chunk_by(|a, b| a.atlas_info.texture == b.atlas_info.texture)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// LBL-05: the copies reach exactly the halo's width and no further,
    /// spaced so the outline has no gaps; no width, no copies.
    #[test]
    fn halo_copies_ring_the_text_at_its_width_without_gaps() {
        for width in [1.0, 2.0, 3.0] {
            let offsets = halo_offsets(width);
            let reach = offsets.iter().map(|o| o.length()).fold(0.0, f32::max);
            assert!(
                (reach - width).abs() < 1e-4,
                "reaches {reach}, want {width}"
            );
            let outer: Vec<Vec2> = offsets
                .iter()
                .copied()
                .filter(|o| (o.length() - width).abs() < 1e-4)
                .collect();
            assert!(outer.len() >= MIN_COPIES);
            for (i, a) in outer.iter().enumerate() {
                let b = outer[(i + 1) % outer.len()];
                assert!(a.distance(b) <= MAX_STEP_PX + 1e-4, "gap {}", a.distance(b));
            }
        }
        assert!(halo_offsets(0.0).is_empty());
    }

    /// LBL-04 / LBL-05: the halo is the ground colour, and fades with its
    /// text.
    #[test]
    fn the_label_halo_is_the_ground_colour_and_fades_with_its_text() {
        let halo = TextHalo::ground();
        assert_eq!(halo.color, colors::GROUND);
        let faded = halo_color(&halo, Color::WHITE.with_alpha(0.25));
        assert!((faded.alpha() - 0.25).abs() < 1e-6);
        assert_eq!(faded.with_alpha(1.0), colors::GROUND.with_alpha(1.0));
    }

    /// LBL-05: a halo takes its width from `label_halo_px`, follows the
    /// tunable, and a change to it has its text drawn again.
    #[test]
    fn halo_width_follows_the_tunable_and_redraws_its_text() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Tunables>()
            .add_plugins(HaloPlugin);
        let text = app
            .world_mut()
            .spawn((Text::new("Europa"), TextColor::WHITE, TextHalo::ground()))
            .id();
        app.update();
        let width = |app: &App| app.world().get::<TextHalo>(text).unwrap().width_px;
        assert_eq!(width(&app), Tunables::default().label_halo_px);
        let color_changed = |app: &App| {
            app.world()
                .get_entity(text)
                .unwrap()
                .get_ref::<TextColor>()
                .unwrap()
                .last_changed()
        };

        app.update();
        let before = color_changed(&app);
        app.world_mut().resource_mut::<Tunables>().label_halo_px = 3.0;
        app.update();
        assert_eq!(width(&app), 3.0);
        assert!(color_changed(&app).is_newer_than(before, app.world().read_change_tick()));
    }
}
