use bevy::prelude::*;

mod camera;
mod orbits;
mod scale;
mod scene;
mod system_data;
mod time_control;

use camera::CameraPlugin;
use orbits::OrbitsPlugin;
use scene::ScenePlugin;
use system_data::SystemDataPlugin;
use time_control::TimeControlPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Orphan Worlds".to_string(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.01, 0.01, 0.03)))
        .insert_resource(GlobalAmbientLight {
            brightness: 200.0,
            ..default()
        })
        .add_plugins(SystemDataPlugin)
        .add_plugins(TimeControlPlugin)
        .add_plugins(OrbitsPlugin)
        .add_plugins(ScenePlugin)
        .add_plugins(CameraPlugin)
        .run();
}
