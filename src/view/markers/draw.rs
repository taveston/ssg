//! Builds a body's marker (MRK-03, MRK-04, MRK-11, MRK-13, SYM-13): its
//! box sized by class, its shape (a body marker with its class detail, a
//! hollow circle, a glyph or a symbol), and its label with the name and a
//! symbol's second line. A station's marker also carries a back frame and
//! a count, shown while something is docked at it (MUL-03). Lists and
//! exposure-ring identity marks copy the shape at their own size (INS-05,
//! MOT-14). `markers/mod.rs` places and fades the markers each frame.

use bevy::prelude::*;

use super::{
    BodyDetail, BodyRim, MarkerDrawn, MarkerHalfSize, MarkerLabel, MarkerShape, SecondLineFor,
    body_fill, marker_half_size, marker_shape, symbol_settings,
};
use crate::sim::body::{Affiliation, BodyClass, DisplayName};
use crate::sim::clock::GameClock;
use crate::sim::track::ShipRecord;
use crate::view::colors;
use crate::view::fonts::Fonts;
use crate::view::halo::TextHalo;
use crate::view::hover;
use crate::view::layers;
use crate::view::multi::{BackFrame, back_offset};
use crate::view::multi_markers::draw::{StationMarker, blank_look, spawn_back_frame, spawn_count};
use crate::view::ship_groups::GroupFaces;
use crate::view::symbols::draw::{
    AmplifierStyle, Tint, TintSlot, amplifier_gap, amplifier_reach, spawn_symbol_parts,
};
use crate::view::symbols::{
    GroupFace, SecondLineFacts, label_second_line, symbol_look, with_group,
};
use crate::view::targets::MarkerFor;
use crate::view::tunables::Tunables;

/// Outline width of hollow markers.
const HOLLOW_BORDER_PX: f32 = 1.5;
/// Glyph markers are drawn a little larger than their box so the glyph's
/// own side bearings don't shrink it.
const GLYPH_SCALE: f32 = 1.8;
const LABEL_FONT_PX: f32 = 12.0;
const SECOND_LINE_FONT_PX: f32 = 10.0;

/// What a marker needs from its body.
type SpawnBodies<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static BodyClass,
        &'static Affiliation,
        &'static DisplayName,
        Option<&'static ShipRecord>,
    ),
>;

pub(super) fn spawn_markers(
    mut commands: Commands,
    (fonts, tunables): (Res<Fonts>, Res<Tunables>),
    clock: Res<GameClock>,
    bodies: SpawnBodies,
    groups: GroupFaces,
) {
    for (body, class, affiliation, name, record) in &bodies {
        let marker = BodyLook {
            class: *class,
            affiliation: *affiliation,
            record,
            group: groups.face(body),
        };
        let assets = (&*fonts, &*tunables);
        let members = groups.member_classes(body);
        // SYM-13: a symbol always has a second line, empty while nothing
        // applies, so a countdown can appear on it later (PLNR-66).
        let label = MarkerText {
            name: &name.0,
            second_line: record.map(|r| {
                let facts = SecondLineFacts {
                    members: &members,
                    ..SecondLineFacts::on_day(clock.elapsed_days())
                };
                label_second_line(*affiliation, r, facts).unwrap_or_default()
            }),
        };
        spawn_marker(&mut commands, assets, body, marker, label);
    }
}

/// Who a marker stands for, as far as its look goes.
#[derive(Clone, Copy)]
pub struct BodyLook<'a> {
    pub class: BodyClass,
    pub affiliation: Affiliation,
    pub record: Option<&'a ShipRecord>,
    /// A group's count and task-force mark (SYM-09); `None` for a ship.
    pub group: Option<GroupFace>,
}

impl BodyLook<'_> {
    fn shape(&self) -> MarkerShape {
        marker_shape(self.class, self.record.is_some())
    }

    /// The marker's and label's colour: a symbol's by affiliation and
    /// command (SYM-02); any other body's the neutral white, whoever owns
    /// it, as the map doesn't show a body's ownership (MRK-01, MRK-10).
    pub fn color(&self, tunables: &Tunables) -> Color {
        match self.record {
            Some(record) => symbol_look(self.affiliation, record, symbol_settings(tunables)).color,
            None => colors::NEUTRAL,
        }
    }
}

/// What a marker's label says (SYM-13).
struct MarkerText<'a> {
    name: &'a str,
    second_line: Option<String>,
}

/// The fonts and tunables a marker is built from.
pub type MarkerAssets<'a> = (&'a Fonts, &'a Tunables);

/// MRK-03 / MRK-04: the marker's box, sized by class and shaped by
/// rounding and borders.
fn marker_node(shape: MarkerShape, half: f32) -> Node {
    let mut node = Node {
        position_type: PositionType::Absolute,
        width: px(half * 2.0),
        height: px(half * 2.0),
        display: Display::None,
        ..default()
    };
    if shape == MarkerShape::HollowCircle {
        node.border = UiRect::all(px(HOLLOW_BORDER_PX));
    }
    if matches!(shape, MarkerShape::Disc | MarkerShape::HollowCircle) {
        node.border_radius = BorderRadius::MAX;
    }
    node
}

/// The shape itself, spawned on or under `marker`, at `half` pixels.
fn spawn_shape(marker: &mut EntityCommands, assets: MarkerAssets, look: BodyLook, half: f32) {
    let (fonts, tunables) = assets;
    let color = look.color(tunables);
    match look.shape() {
        MarkerShape::Disc => {
            // MRK-13: an opaque ground-colour inside a neutral rim;
            // `update_markers` switches the fill when it grows (MRK-11).
            let fill = body_fill(false, tunables);
            marker.insert((BackgroundColor(fill), Tint::new(fill, TintSlot::Background)));
            marker.with_children(|parent| {
                spawn_body_detail(parent, look.class, color, tunables);
                parent.spawn((
                    BodyRim,
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(0),
                        top: px(0),
                        width: percent(100),
                        height: percent(100),
                        border: UiRect::all(px(tunables.body_rim_px)),
                        border_radius: BorderRadius::MAX,
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                    BorderColor::all(color),
                    Tint::new(color, TintSlot::Border),
                    Pickable::IGNORE,
                ));
            });
        }
        MarkerShape::HollowCircle => {
            marker.insert((
                BackgroundColor(Color::NONE),
                BorderColor::all(color),
                Tint::new(color, TintSlot::Border),
            ));
        }
        MarkerShape::Glyph(glyph) => {
            marker.with_children(|parent| {
                parent.spawn((
                    Text::new(glyph),
                    fonts.label(half * GLYPH_SCALE),
                    TextColor(color),
                    Tint::new(color, TintSlot::Text),
                    Pickable::IGNORE,
                ));
            });
        }
        MarkerShape::Symbol => {
            if let Some(record) = look.record {
                let symbol = with_group(
                    symbol_look(look.affiliation, record, symbol_settings(tunables)),
                    look.group,
                );
                let style = AmplifierStyle {
                    gap: amplifier_gap(tunables.bracket_margin_px, tunables.amplifier_gap_px),
                    track_mix: tunables.condition_track_mix,
                };
                // SYM-13: the label keeps clear of the amplifiers too.
                marker.insert(amplifier_reach(&symbol, half * 2.0, style.gap));
                marker.with_children(|parent| {
                    spawn_symbol_parts(parent, fonts, &symbol, half * 2.0, style);
                });
            }
        }
    }
}

/// MRK-13: the class detail inside or round a body marker: radial ticks
/// outside the star's rim, a centre dot in a planet, nothing for a moon.
fn spawn_body_detail(
    parent: &mut ChildSpawnerCommands,
    class: BodyClass,
    color: Color,
    tunables: &Tunables,
) {
    let full = || Node {
        position_type: PositionType::Absolute,
        left: px(0),
        top: px(0),
        width: percent(100),
        height: percent(100),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    };
    let part = |node: Node| {
        (
            node,
            BackgroundColor(color),
            Tint::new(color, TintSlot::Background),
            Pickable::IGNORE,
        )
    };
    match class {
        BodyClass::Planet => {
            let dot = tunables.planet_dot_px;
            parent
                .spawn((BodyDetail, full(), Pickable::IGNORE))
                .with_children(|holder| {
                    holder.spawn(part(Node {
                        width: px(dot),
                        height: px(dot),
                        border_radius: BorderRadius::MAX,
                        ..default()
                    }));
                });
        }
        BodyClass::Star => {
            let count = tunables.star_tick_count.max(1);
            let (tick, width) = (tunables.star_tick_px, tunables.body_rim_px);
            for i in 0..count {
                // Each tick hangs above the top of a full-size holder,
                // which is turned about the marker's centre.
                let angle = std::f32::consts::TAU * i as f32 / count as f32;
                parent
                    .spawn((
                        BodyDetail,
                        full(),
                        UiTransform::from_rotation(Rot2::radians(angle)),
                        Pickable::IGNORE,
                    ))
                    .with_children(|holder| {
                        holder.spawn(part(Node {
                            position_type: PositionType::Absolute,
                            top: px(-tick),
                            left: percent(50),
                            margin: UiRect::left(px(-width / 2.0)),
                            width: px(width),
                            height: px(tick),
                            ..default()
                        }));
                    });
            }
        }
        _ => {}
    }
}

fn spawn_marker(
    commands: &mut Commands,
    assets: MarkerAssets,
    body: Entity,
    look: BodyLook,
    text: MarkerText,
) {
    let (_, tunables) = assets;
    let half = marker_half_size(look.class, tunables);
    let shape = look.shape();
    let mut marker = commands.spawn((
        MarkerFor(body),
        MarkerHalfSize(Vec2::splat(half)),
        MarkerDrawn::default(),
        shape,
        marker_node(shape, half),
        // MRK-12: bodies are re-layered by depth each frame.
        GlobalZIndex(if shape == MarkerShape::Disc {
            layers::BODIES
        } else {
            layers::SYMBOLS
        }),
        // Hover and click go through the marker's hit area instead
        // (MRK-08, `markers/picking.rs`), so the node itself takes none.
        Pickable::IGNORE,
    ));
    if shape == MarkerShape::Disc {
        marker.insert(disc_gap(tunables));
    }
    let station = look.class == BodyClass::Station && shape == MarkerShape::Symbol;
    if station {
        // MUL-03: the back frame goes under the station's own frame, so it
        // is spawned first; it and the count show while things are docked.
        let back = (half * 2.0, back_offset(tunables.back_frame_offset_px));
        marker
            .insert((StationMarker, BackFrame(None)))
            .with_children(|parent| spawn_back_frame(parent, &blank_look(), back, false));
    }
    spawn_shape(&mut marker, assets, look, half);
    if station {
        let (fonts, _) = assets;
        marker.with_children(|parent| {
            spawn_count(parent, fonts, &blank_look(), tunables.multi_count_px, false);
        });
    }
    let color = look.color(tunables);
    marker.with_children(|parent| spawn_label(parent, assets, (body, half, color), text));
}

/// SYM-13: the label beside the marker, in its colour: the name, and for
/// a symbol a second line in the secondary text colour. It starts to the
/// right; `markers/labels.rs` places it from then on (LBL-01). It is a hover and
/// click target for its body, and wins over an orbit ring (HOV-02).
fn spawn_label(
    parent: &mut ChildSpawnerCommands,
    (fonts, tunables): MarkerAssets,
    (body, half, color): (Entity, f32, Color),
    text: MarkerText,
) {
    parent
        .spawn((
            MarkerLabel,
            MarkerFor(body),
            // MRK-12: labels sit above every marker, not just their own.
            GlobalZIndex(layers::LABELS),
            Node {
                position_type: PositionType::Absolute,
                left: px(half * 2.0 + tunables.bracket_margin_px + tunables.label_gap_px),
                top: px(0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .with_children(|label| {
            label.spawn((
                Text::new(text.name),
                // The label sits beside a marker a few pixels wide; it must
                // not wrap to that width.
                TextLayout::linebreak(LineBreak::NoWrap),
                fonts.label(LABEL_FONT_PX),
                TextColor(color),
                Tint::new(color, TintSlot::Text),
                TextHalo::ground(),
            ));
            if let Some(line) = text.second_line {
                label.spawn((
                    SecondLineFor(body),
                    Text::new(line),
                    TextLayout::linebreak(LineBreak::NoWrap),
                    fonts.data(SECOND_LINE_FONT_PX),
                    TextColor(colors::PANEL_BORDER),
                    Tint::new(colors::PANEL_BORDER, TintSlot::Text),
                    TextHalo::ground(),
                ));
            }
        })
        .observe(hover::on_marker_over)
        .observe(hover::on_marker_out)
        .observe(hover::on_marker_click);
}

/// INS-05: a small copy of a body's marker, for lists: a symbol at
/// `list_symbol_px` for a ship or station, or the body's shape at
/// `body_half` pixels.
pub fn spawn_list_icon(
    parent: &mut ChildSpawnerCommands,
    assets: MarkerAssets,
    look: BodyLook,
    body_half: f32,
) {
    let (_, tunables) = assets;
    let half = match look.shape() {
        MarkerShape::Symbol => tunables.list_symbol_px / 2.0,
        _ => body_half,
    };
    spawn_icon(parent, assets, look, half);
}

/// A copy of a body's marker at `half` pixels, laid out in the flow of its
/// parent rather than placed on the map: a symbol, or a body marker with
/// its class detail (MRK-13). Lists (INS-05) and exposure-ring identity
/// marks (MOT-14) are built from it.
pub fn spawn_icon(
    parent: &mut ChildSpawnerCommands,
    assets: MarkerAssets,
    look: BodyLook,
    half: f32,
) {
    let mut node = marker_node(look.shape(), half);
    node.position_type = PositionType::Relative;
    node.display = Display::Flex;
    let mut icon = parent.spawn(node);
    spawn_shape(&mut icon, assets, look, half);
}

/// MRK-11: a natural body's ring of ground colour, hard against the
/// outside of its rim, so overlapping discs stay distinct.
pub fn disc_gap(tunables: &Tunables) -> Outline {
    Outline::new(px(tunables.body_gap_px), px(0), colors::GROUND)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The class details spawned for a body marker of `class`.
    fn details_of(class: BodyClass) -> (usize, usize) {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = World::new();
        world
            .run_system_once(move |mut commands: Commands| {
                commands.spawn(Node::default()).with_children(|parent| {
                    spawn_body_detail(parent, class, colors::NEUTRAL, &Tunables::default());
                });
            })
            .unwrap();
        let holders = world
            .query_filtered::<(), With<BodyDetail>>()
            .iter(&world)
            .count();
        let parts = world
            .query::<&Tint>()
            .iter(&world)
            .filter(|tint| tint.color == colors::NEUTRAL)
            .count();
        (holders, parts)
    }

    /// MRK-13: the star has `star_tick_count` ticks, a planet one centre
    /// dot, a moon nothing beyond its circle.
    #[test]
    fn body_markers_show_their_class_in_their_shape() {
        let ticks = Tunables::default().star_tick_count as usize;
        assert_eq!(details_of(BodyClass::Star), (ticks, ticks));
        assert_eq!(details_of(BodyClass::Planet), (1, 1));
        assert_eq!(details_of(BodyClass::Moon), (0, 0));
    }

    /// MRK-11: a disc carries its ground-coloured gap hard against its
    /// rim, whether or not it is hovered or selected.
    #[test]
    fn a_disc_is_ringed_by_a_ground_gap() {
        let tunables = Tunables::default();
        let gap = disc_gap(&tunables);
        assert_eq!(gap.color, colors::GROUND);
        assert_eq!(gap.width, px(tunables.body_gap_px));
        assert_eq!(gap.offset, px(0));
    }

    /// MRK-10: the star, planets and moons are drawn in the neutral colour
    /// whoever owns them.
    #[test]
    fn natural_bodies_are_neutral_whoever_owns_them() {
        let tunables = Tunables::default();
        for class in [BodyClass::Star, BodyClass::Planet, BodyClass::Moon] {
            for affiliation in [Affiliation::Own, Affiliation::Hostile, Affiliation::Neutral] {
                let look = BodyLook {
                    class,
                    affiliation,
                    record: None,
                    group: None,
                };
                assert_eq!(look.color(&tunables), colors::NEUTRAL);
            }
        }
    }
}
