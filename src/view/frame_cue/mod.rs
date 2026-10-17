//! The frame cue (REF-01 to REF-10): a dotted crosshair in the orbital
//! plane through the frame body, with ticks at the scale bar's distance,
//! an "N" on the north arm and a solid vertical axis along the plane's
//! normal. It shows which way north lies, that the view is a plane seen
//! at a tilt, and which body is the frame, after the orbit rings have
//! faded. It is drawn in the frame violet (MRK-01) and is never a hover
//! or click target.
//!
//! - `geometry.rs`: where each stroke lies on screen, as plain functions.
//! - `draw.rs`: the nodes, and the system that lays them out each frame.
//! - `stroke.rs` and `stroke.wesl`: how a stroke is drawn, as a UI
//!   material turned along it.
//!
//! The strokes are UI nodes so they take their place in the draw order
//! (MRK-12, REF-09): the arms, ticks and lower half under every natural
//! body, the upper half above them and below the symbols.

use bevy::prelude::*;

use super::markers::MarkersPlaced;
use super::plot_marks::{ScaleBarKm, ScaleBarMeasured};
use crate::{Boot, Stage};

pub mod draw;
pub mod geometry;
pub mod stroke;

/// The cue's nodes and layout; its material is
/// `stroke::CueStrokeMaterialPlugin`, which needs a renderer.
pub struct FrameCuePlugin;

impl Plugin for FrameCuePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ScaleBarKm>()
            .init_resource::<crate::view::map_layer::MapLayer>()
            .add_systems(Startup, draw::spawn_frame_cue.in_set(Boot::Overlay))
            .add_systems(
                Update,
                draw::draw_frame_cue
                    .after(MarkersPlaced)
                    .after(ScaleBarMeasured)
                    .in_set(Stage::Overlay),
            );
    }
}
