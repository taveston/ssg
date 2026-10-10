//! Orphan Worlds: assembles the domain plugins and orders them (ARCH-01,
//! ARCH-04). Simulation in `sim/`, content in `content/`, everything drawn
//! in `view/`; nothing else lives here.

use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::prelude::*;

mod content;
mod sim;
mod view;

/// The startup order: the world is spawned from content, then the view
/// derives what it needs from it (transforms, fold state, zoom stops), then
/// the overlay is built on top.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Boot {
    World,
    Derive,
    Overlay,
}

/// The per-frame pipeline. The simulation runs first; then the camera
/// settles, simulated positions become transforms, grouping decides what is
/// folded, motion what is a ring, and the overlay is laid out and drawn on
/// top. Plugins tag their systems with these instead of reaching across
/// module boundaries with `.after()`.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Stage {
    Simulation,
    Camera,
    Projection,
    Grouping,
    Motion,
    Screen,
    Layout,
    Overlay,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Orphan Worlds".to_string(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(view::colors::GROUND))
        .configure_sets(Startup, (Boot::World, Boot::Derive, Boot::Overlay).chain())
        .configure_sets(
            Update,
            (
                Stage::Simulation,
                Stage::Camera,
                Stage::Projection,
                Stage::Grouping,
                Stage::Motion,
                Stage::Screen,
                Stage::Layout,
                Stage::Overlay,
            )
                .chain(),
        )
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_plugins((content::ContentPlugin, sim::SimPlugin, view::ViewPlugin))
        .run();
}
