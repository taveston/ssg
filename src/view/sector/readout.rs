//! The truth readout (SECT-08): two lines at the top left of the map
//! area, saying that the sector view shows everything, not what anyone
//! knows, and naming the sector's seed and its number of gated systems.
//! Shown only with the sector view.

use bevy::prelude::*;

use crate::content::start::LoadedSector;
use crate::view::colors;
use crate::view::fonts::Fonts;
use crate::view::inset::inspect::spawn_spaced_capitals;
use crate::view::layers;
use crate::view::map_layer::MapLayer;
use crate::view::screen::MapView;
use crate::view::tunables::Tunables;

/// SECT-08: the first line, as small spaced capitals.
pub const TRUTH_LINE: &str = "Truth view · shows everything, not what anyone knows";

/// The space between the readout's two lines.
const LINE_GAP_PX: f32 = 3.0;

/// The readout's root.
#[derive(Component)]
pub struct TruthReadout;

/// The readout's second line.
#[derive(Component)]
pub struct SeedLine;

/// SECT-08: the second line: "Seed *seed* · *N* gated systems".
pub fn seed_line(seed: u64, gated: usize) -> String {
    format!("Seed {seed} · {gated} gated systems")
}

/// SECT-08: the readout, hidden until placed: the truth line in small
/// spaced capitals over the seed line in the data face, both in the
/// secondary colour.
pub fn spawn_readout(
    mut commands: Commands,
    sector: Option<Res<LoadedSector>>,
    (fonts, tunables): (Res<Fonts>, Res<Tunables>),
) {
    let Some(loaded) = sector else {
        return;
    };
    let sector = &loaded.sector;
    let line = seed_line(sector.settings.seed, sector.gated.len());
    commands
        .spawn((
            TruthReadout,
            GlobalZIndex(layers::EDGE),
            Pickable::IGNORE,
            Node {
                position_type: PositionType::Absolute,
                flex_direction: FlexDirection::Column,
                row_gap: px(LINE_GAP_PX),
                display: Display::None,
                ..default()
            },
        ))
        .with_children(|readout| {
            spawn_spaced_capitals(readout, (&fonts, &tunables), TRUTH_LINE, Pickable::IGNORE);
            readout.spawn((
                SeedLine,
                Text::new(line),
                TextLayout::linebreak(LineBreak::NoWrap),
                fonts.data(tunables.sidebar_text_px),
                TextColor(colors::SECONDARY),
                Pickable::IGNORE,
            ));
        });
}

/// SECT-08: the readout sits `plot_margin_px` in from the map area's top
/// left, while the sector view is shown.
pub fn show_readout(
    layer: Res<MapLayer>,
    map: MapView,
    tunables: Res<Tunables>,
    mut readouts: Query<&mut Node, With<TruthReadout>>,
) {
    let area = map.area();
    for mut node in &mut readouts {
        let Some(area) = area.as_ref().filter(|_| layer.is_sector()) else {
            if node.display != Display::None {
                node.display = Display::None;
            }
            continue;
        };
        node.display = Display::Flex;
        node.left = px(area.offset.x + tunables.plot_margin_px);
        node.top = px(area.offset.y + tunables.plot_margin_px);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SECT-08: the second line names the seed and the number of gated
    /// systems.
    #[test]
    fn the_seed_line_names_the_seed_and_the_gated_count() {
        assert_eq!(seed_line(1234, 30), "Seed 1234 · 30 gated systems");
        assert_eq!(
            seed_line(u64::MAX, 8),
            format!("Seed {} · 8 gated systems", u64::MAX)
        );
        assert_eq!(
            TRUTH_LINE.to_uppercase(),
            "TRUTH VIEW · SHOWS EVERYTHING, NOT WHAT ANYONE KNOWS"
        );
    }
}
