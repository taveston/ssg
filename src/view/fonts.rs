//! Loads the two OFL fonts from `assets/fonts/` once and shares the
//! handles as a resource. Barlow for labels and UI, JetBrains Mono for
//! dates, figures, and status readouts (MRK-02).

use bevy::prelude::*;

#[derive(Resource, Clone)]
pub struct Fonts {
    pub label: Handle<Font>,
    pub label_semibold: Handle<Font>,
    pub mono: Handle<Font>,
}

impl Fonts {
    pub fn label(&self, size: f32) -> TextFont {
        TextFont {
            font: self.label.clone().into(),
            font_size: FontSize::Px(size),
            ..default()
        }
    }

    pub fn label_semibold(&self, size: f32) -> TextFont {
        TextFont {
            font: self.label_semibold.clone().into(),
            font_size: FontSize::Px(size),
            ..default()
        }
    }

    pub fn mono(&self, size: f32) -> TextFont {
        TextFont {
            font: self.mono.clone().into(),
            font_size: FontSize::Px(size),
            ..default()
        }
    }
}

pub struct FontsPlugin;

impl Plugin for FontsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, load_fonts);
    }
}

fn load_fonts(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(Fonts {
        label: asset_server.load("fonts/Barlow-Regular.ttf"),
        label_semibold: asset_server.load("fonts/Barlow-SemiBold.ttf"),
        mono: asset_server.load("fonts/JetBrainsMono.ttf"),
    });
}
