use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::prelude::*;

mod camera;
mod colors;
mod date;
mod fidelity;
mod fonts;
mod hover_inspect;
mod markers;
mod orbits;
mod scale;
mod scene;
mod system_data;
mod time_control;
mod topbar;

use camera::CameraPlugin;
use fidelity::FidelityPlugin;
use fonts::FontsPlugin;
use hover_inspect::HoverInspectPlugin;
use markers::MarkersPlugin;
use orbits::OrbitsPlugin;
use scene::ScenePlugin;
use system_data::SystemDataPlugin;
use time_control::TimeControlPlugin;
use topbar::TopBarPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Orphan Worlds".to_string(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(colors::GROUND))
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_plugins(FontsPlugin)
        .add_plugins(FidelityPlugin)
        .add_plugins(SystemDataPlugin)
        .add_plugins(TimeControlPlugin)
        .add_plugins(ScenePlugin)
        .add_plugins(OrbitsPlugin)
        .add_plugins(HoverInspectPlugin)
        .add_plugins(MarkersPlugin)
        .add_plugins(CameraPlugin)
        .add_plugins(TopBarPlugin)
        .run();
}
