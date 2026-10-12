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

/// Barlow for labels and UI; JetBrains Mono for dates, figures and status
/// readouts (MRK-02). Paths are relative to `assets/`.
const LABEL_FONT: &str = "fonts/Barlow-Regular.ttf";
const LABEL_SEMIBOLD_FONT: &str = "fonts/Barlow-SemiBold.ttf";
const MONO_FONT: &str = "fonts/JetBrainsMono.ttf";

fn load_fonts(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(Fonts {
        label: asset_server.load(LABEL_FONT),
        label_semibold: asset_server.load(LABEL_SEMIBOLD_FONT),
        mono: asset_server.load(MONO_FONT),
    });
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    /// MRK-02: labels load Barlow and readouts JetBrains Mono, and the
    /// files are there to load.
    #[test]
    fn labels_are_barlow_and_readouts_are_jetbrains_mono() {
        assert!(LABEL_FONT.contains("Barlow") && LABEL_SEMIBOLD_FONT.contains("Barlow"));
        assert!(MONO_FONT.contains("JetBrainsMono"));
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
        for font in [LABEL_FONT, LABEL_SEMIBOLD_FONT, MONO_FONT] {
            assert!(assets.join(font).is_file(), "{font} missing");
        }
    }
}
