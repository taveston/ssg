use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::prelude::*;

mod camera;
mod colors;
mod date;
mod fidelity;
mod fonts;
mod grouping;
mod hover_inspect;
mod magnifier;
mod markers;
mod motion;
mod orbits;
mod scale;
mod scene;
mod system_data;
mod time_control;
mod topbar;
mod tunables;
mod zoom_stops;

use camera::CameraPlugin;
use fidelity::FidelityPlugin;
use fonts::FontsPlugin;
use grouping::GroupingPlugin;
use hover_inspect::HoverInspectPlugin;
use magnifier::MagnifierPlugin;
use markers::MarkersPlugin;
use motion::MotionPlugin;
use orbits::OrbitsPlugin;
use scene::ScenePlugin;
use system_data::SystemDataPlugin;
use time_control::TimeControlPlugin;
use topbar::TopBarPlugin;
use tunables::TunablesPlugin;
use zoom_stops::ZoomStopsPlugin;

/// The per-frame pipeline. Grouping needs the camera settled and bodies
/// moved; motion needs to know what's folded; the marker overlay needs
/// both. Plugins tag their `Update` systems with these instead of reaching
/// across module boundaries with `.after()`.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Stage {
    Camera,
    Orbits,
    Grouping,
    Motion,
    Markers,
    Magnifier,
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
        .insert_resource(ClearColor(colors::GROUND))
        .configure_sets(
            Update,
            (
                Stage::Camera,
                Stage::Orbits,
                Stage::Grouping,
                Stage::Motion,
                Stage::Markers,
                Stage::Magnifier,
            )
                .chain(),
        )
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_plugins(TunablesPlugin)
        .add_plugins(FontsPlugin)
        .add_plugins(FidelityPlugin)
        .add_plugins(SystemDataPlugin)
        .add_plugins(TimeControlPlugin)
        .add_plugins(ScenePlugin)
        .add_plugins(ZoomStopsPlugin)
        .add_plugins(OrbitsPlugin)
        .add_plugins(GroupingPlugin)
        .add_plugins(MotionPlugin)
        .add_plugins(HoverInspectPlugin)
        .add_plugins(MarkersPlugin)
        .add_plugins(MagnifierPlugin)
        .add_plugins(CameraPlugin)
        .add_plugins(TopBarPlugin)
        .run();
}
