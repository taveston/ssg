//! The three type roles (MRK-02) and the one place that names a font file.
//! The **label face** is for names on the map, labels, buttons, headings
//! and the breadcrumb; the **text face** for running prose (nothing uses it yet; `Fonts::face` reaches it); the **data
//! face** for dates, figures, status readouts and stamps. Everything else
//! asks for a role through `Fonts`, never for a font.

use bevy::prelude::*;

/// A type role and weight.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Face {
    Label,
    LabelSemibold,
    Text,
    TextSemibold,
    Data,
}

impl Face {
    /// The font file behind each role, relative to `assets/`: IBM Plex
    /// Sans Condensed for labels, IBM Plex Sans for prose, IBM Plex Mono
    /// for data (MRK-02's type roles table).
    pub fn path(self) -> &'static str {
        match self {
            Face::Label => "fonts/IBMPlexSansCondensed-Regular.ttf",
            Face::LabelSemibold => "fonts/IBMPlexSansCondensed-SemiBold.ttf",
            Face::Text => "fonts/IBMPlexSans-Regular.ttf",
            Face::TextSemibold => "fonts/IBMPlexSans-SemiBold.ttf",
            Face::Data => "fonts/IBMPlexMono-Regular.ttf",
        }
    }
}

/// The loaded font for each role.
#[derive(Resource, Clone)]
pub struct Fonts {
    pub label: Handle<Font>,
    pub label_semibold: Handle<Font>,
    pub text: Handle<Font>,
    pub text_semibold: Handle<Font>,
    pub data: Handle<Font>,
}

impl Fonts {
    fn handle(&self, face: Face) -> &Handle<Font> {
        match face {
            Face::Label => &self.label,
            Face::LabelSemibold => &self.label_semibold,
            Face::Text => &self.text,
            Face::TextSemibold => &self.text_semibold,
            Face::Data => &self.data,
        }
    }

    /// `face` at `size` pixels.
    pub fn face(&self, face: Face, size: f32) -> TextFont {
        TextFont {
            font: self.handle(face).clone().into(),
            font_size: FontSize::Px(size),
            ..default()
        }
    }

    pub fn label(&self, size: f32) -> TextFont {
        self.face(Face::Label, size)
    }

    pub fn label_semibold(&self, size: f32) -> TextFont {
        self.face(Face::LabelSemibold, size)
    }

    pub fn data(&self, size: f32) -> TextFont {
        self.face(Face::Data, size)
    }
}

pub struct FontsPlugin;

impl Plugin for FontsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, load_fonts);
    }
}

fn load_fonts(mut commands: Commands, asset_server: Res<AssetServer>) {
    let load = |face: Face| asset_server.load(face.path());
    commands.insert_resource(Fonts {
        label: load(Face::Label),
        label_semibold: load(Face::LabelSemibold),
        text: load(Face::Text),
        text_semibold: load(Face::TextSemibold),
        data: load(Face::Data),
    });
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use bevy::asset::uuid::Uuid;

    use super::*;

    /// MRK-02: every role resolves to a font file that is there to load,
    /// and the label, text and data faces are different fonts.
    #[test]
    fn every_role_has_its_own_font_file() {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
        for face in [
            Face::Label,
            Face::LabelSemibold,
            Face::Text,
            Face::TextSemibold,
            Face::Data,
        ] {
            assert!(assets.join(face.path()).is_file(), "{face:?} missing");
        }
        let paths = [Face::Label, Face::Text, Face::Data].map(Face::path);
        assert!(paths[0] != paths[1] && paths[1] != paths[2] && paths[0] != paths[2]);
    }

    /// MRK-02: asking `Fonts` for a role gives that role's font.
    #[test]
    fn roles_resolve_through_the_fonts_resource() {
        let handle = |n: u128| Handle::<Font>::Uuid(Uuid::from_u128(n), Default::default());
        let fonts = Fonts {
            label: handle(1),
            label_semibold: handle(2),
            text: handle(3),
            text_semibold: handle(4),
            data: handle(5),
        };
        assert_eq!(fonts.label(12.0).font, handle(1).into());
        assert_eq!(fonts.face(Face::Text, 12.0).font, handle(3).into());
        assert_eq!(fonts.data(12.0).font, handle(5).into());
        assert_eq!(fonts.face(Face::TextSemibold, 9.0).font, handle(4).into());
    }
}
