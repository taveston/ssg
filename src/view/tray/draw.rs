//! The tray's panel (MUL-08, MUL-09, MRK-12, CAM-10): spawned once,
//! hidden until a tray opens, refilled when its contents change and
//! placed beside its marker every frame. It sits in its own layer above
//! every marker, label and bracket and beneath the edge arrows and panels
//! (MRK-12), and a press on it never starts a map drag (CAM-10).
//!
//! From the top: the header in small spaced capitals (INS-09), the rows,
//! each with its toggle slot, its symbol or body marker, its name and its
//! tag, then the footer with the key hint. A notch on the edge facing the
//! marker points at it.

use bevy::prelude::*;

use super::TrayState;
use super::contents::{TrayContents, TrayLine};
use super::input::{RowSelect, toggle_row};
use super::place::place_tray;
use crate::view::colors;
use crate::view::fonts::Fonts;
use crate::view::hover::ClickCheck;
use crate::view::inset::inspect::spawn_spaced_capitals;
use crate::view::inset::outline::{COLLAPSED_GLYPH, EXPANDED_GLYPH};
use crate::view::layers;
use crate::view::markers::draw::{BodyLook, MarkerAssets, spawn_list_icon};
use crate::view::markers::{TintedParts, fade_parts};
use crate::view::node_box::node_size;
use crate::view::screen::{MapArea, MapView};
use crate::view::symbols::draw::{Tint, TintSlot};
use crate::view::targets::MapInputBlocker;
use crate::view::tunables::Tunables;

/// The space inside the panel's sides.
const SIDE_PADDING_PX: f32 = 10.0;
/// The space between a row's toggle, icon, name and tag.
const ROW_GAP_PX: f32 = 8.0;
/// The tag's least distance from the name.
const TAG_GAP_PX: f32 = 16.0;
/// One level of indent for a row opened from another (MUL-09).
const INDENT_PX: f32 = 16.0;
/// The toggle's slot, so names line up whether or not a row has one.
const TOGGLE_PX: f32 = 10.0;
/// The outline and the lines between header, rows and footer (INS-09).
const HAIRLINE_PX: f32 = 1.0;
/// The lines between header, rows and footer are the outline's colour
/// mixed this far into the fill, fainter than the outline itself.
const DIVIDER_MIX: f32 = 0.35;
/// The notch: a square this wide turned on its corner, half of it
/// showing past the edge.
const NOTCH_PX: f32 = 10.0;
/// The notch keeps this far from the panel's corners.
const NOTCH_INSET_PX: f32 = 12.0;
/// A dimmed row's parts are drawn at this opacity (MUL-12).
const DIMMED_ALPHA: f32 = 0.4;

/// The tray's root, holding the notch and the panel. A press on it never
/// starts a map drag (CAM-10).
#[derive(Component)]
pub struct TrayRoot;
/// The notch pointing at the marker.
#[derive(Component)]
pub struct TrayNotch;
/// The panel itself: fill, outline, and its three parts.
#[derive(Component)]
pub struct TrayPanel;
/// The header, refilled with its spaced capitals.
#[derive(Component)]
pub struct TrayHeader;
/// The header's text, as one string (MUL-08).
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct TrayHeaderText(pub String);
/// The rows' list, which scrolls when the tray is taller than the map.
#[derive(Component)]
pub struct TrayList;
/// The footer's key hint.
#[derive(Component)]
pub struct TrayFooter;
/// A row's node, for the thing or member it lists.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct TrayRowFor {
    pub entity: Entity,
    pub selected: bool,
    pub dimmed: bool,
}
/// A row's tag (MUL-09).
#[derive(Component)]
pub struct TrayTag;

/// MUL-08 / MRK-12 / CAM-10: the tray, hidden until one opens.
pub fn spawn_tray(mut commands: Commands, fonts: Res<Fonts>, tunables: Res<Tunables>) {
    let line = |side: UiRect| Node {
        border: side,
        ..default()
    };
    commands
        .spawn((
            TrayRoot,
            MapInputBlocker,
            GlobalZIndex(layers::TRAY),
            Node {
                position_type: PositionType::Absolute,
                display: Display::None,
                ..default()
            },
            Visibility::Hidden,
        ))
        .with_children(|root| {
            root.spawn((
                TrayNotch,
                Node {
                    position_type: PositionType::Absolute,
                    width: px(NOTCH_PX),
                    height: px(NOTCH_PX),
                    border: UiRect::all(px(HAIRLINE_PX)),
                    ..default()
                },
                UiTransform::from_rotation(Rot2::degrees(45.0)),
                BackgroundColor(colors::PANEL),
                BorderColor::all(colors::PANEL_BORDER),
            ));
            root.spawn((
                TrayPanel,
                Node {
                    flex_direction: FlexDirection::Column,
                    min_width: px(tunables.tray_min_width_px),
                    border: UiRect::all(px(HAIRLINE_PX)),
                    ..default()
                },
                BackgroundColor(colors::PANEL),
                BorderColor::all(colors::PANEL_BORDER),
            ))
            .with_children(|panel| {
                panel.spawn((
                    TrayHeader,
                    Node {
                        height: px(tunables.tray_header_px),
                        flex_shrink: 0.0,
                        align_items: AlignItems::Center,
                        padding: UiRect::horizontal(px(SIDE_PADDING_PX)),
                        ..line(UiRect::bottom(px(HAIRLINE_PX)))
                    },
                    BorderColor::all(colors::mix(
                        colors::PANEL,
                        colors::PANEL_BORDER,
                        DIVIDER_MIX,
                    )),
                ));
                panel.spawn((
                    TrayList,
                    Node {
                        flex_direction: FlexDirection::Column,
                        flex_shrink: 1.0,
                        min_height: px(0),
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                    ScrollPosition::default(),
                ));
                panel
                    .spawn((
                        Node {
                            height: px(tunables.tray_footer_px),
                            flex_shrink: 0.0,
                            align_items: AlignItems::Center,
                            padding: UiRect::horizontal(px(SIDE_PADDING_PX)),
                            ..line(UiRect::top(px(HAIRLINE_PX)))
                        },
                        BorderColor::all(colors::mix(
                            colors::PANEL,
                            colors::PANEL_BORDER,
                            DIVIDER_MIX,
                        )),
                    ))
                    .with_child((
                        TrayFooter,
                        Text::default(),
                        TextLayout::linebreak(LineBreak::NoWrap),
                        fonts.data(tunables.sidebar_caps_px),
                        TextColor(colors::SECONDARY),
                    ));
            });
        });
}

/// The parts of the tray that are refilled.
type Parts<'w, 's> = (
    Query<'w, 's, Entity, With<TrayHeader>>,
    Query<'w, 's, Entity, With<TrayList>>,
    Query<'w, 's, &'static mut Text, With<TrayFooter>>,
);

/// MUL-08 / MUL-09: refill the header, rows and footer whenever what the
/// tray says changes: another tray, the marker's things, a row opened or
/// closed, the selection or the planner's picks.
pub fn fill_tray(
    mut commands: Commands,
    (fonts, tunables): (Res<Fonts>, Res<Tunables>),
    contents: Res<TrayContents>,
    (headers, lists, mut footers): Parts,
    mut shown: Local<Option<(String, Vec<TrayLine>, String)>>,
) {
    // The contents change every frame the marker moves; the panel's text
    // only when one of these does.
    let unchanged = shown.as_ref().is_some_and(|(header, lines, hint)| {
        *header == contents.header && *lines == contents.lines && *hint == contents.hint
    });
    if unchanged {
        return;
    }
    let assets = (&*fonts, &*tunables);
    if let Ok(header) = headers.single() {
        let text = contents.header.clone();
        commands
            .entity(header)
            .despawn_related::<Children>()
            .with_children(|header| {
                spawn_spaced_capitals(header, assets, &text, TrayHeaderText(text.clone()));
            });
    }
    if let Ok(list) = lists.single() {
        commands
            .entity(list)
            .despawn_related::<Children>()
            .with_children(|list| {
                for line in &contents.lines {
                    spawn_row(list, assets, line);
                }
            });
    }
    for mut footer in &mut footers {
        footer.0.clone_from(&contents.hint);
    }
    *shown = Some((
        contents.header.clone(),
        contents.lines.clone(),
        contents.hint.clone(),
    ));
}

/// MUL-09: one row: its toggle slot, its symbol at `list_symbol_px` (or
/// its body marker), its name in the neutral colour and its tag at the
/// right in the secondary colour; highlighted while selected. A click on
/// the toggle opens or closes its members; anywhere else on it selects
/// it, or picks it while the planner is open (MUL-10, MUL-12). The
/// release of a drag over it does neither (CAM-10).
fn spawn_row(list: &mut ChildSpawnerCommands, assets: MarkerAssets, line: &TrayLine) {
    let (fonts, tunables) = assets;
    let entity = line.row.entity;
    let dimmed = line.dimmed;
    let look = BodyLook {
        class: line.look.class,
        affiliation: line.look.affiliation,
        record: line.look.record.as_ref(),
        group: None,
    };
    let glyph = match line.row.open {
        Some(true) => EXPANDED_GLYPH,
        Some(false) => COLLAPSED_GLYPH,
        None => "",
    };
    let text = |color: Color| (TextColor(color), Tint::new(color, TintSlot::Text));
    let mut row = list.spawn((
        TrayRowFor {
            entity,
            selected: line.selected,
            dimmed,
        },
        Node {
            height: px(tunables.tray_row_px),
            flex_shrink: 0.0,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(ROW_GAP_PX),
            padding: UiRect {
                left: px(SIDE_PADDING_PX + line.row.depth as f32 * INDENT_PX),
                right: px(SIDE_PADDING_PX),
                ..default()
            },
            ..default()
        },
        BackgroundColor(if line.selected {
            colors::LIST_HIGHLIGHT
        } else {
            Color::NONE
        }),
    ));
    if dimmed {
        row.insert(DimmedRow);
    }
    row.with_children(|row| {
        let mut toggle = row.spawn((
            Node {
                width: px(TOGGLE_PX),
                ..default()
            },
            Text::new(glyph),
            fonts.data(tunables.sidebar_caps_px),
            text(colors::PANEL_BORDER),
        ));
        if line.row.open.is_some() {
            toggle.observe(
                move |mut click: On<PointerClick>,
                      check: ClickCheck,
                      mut state: ResMut<TrayState>| {
                    click.propagate(false);
                    if check.selects(&click) {
                        toggle_row(&mut state, entity);
                    }
                },
            );
        }
        spawn_list_icon(row, assets, look, tunables.list_marker_px / 2.0);
        row.spawn((
            Text::new(line.name.clone()),
            TextLayout::linebreak(LineBreak::NoWrap),
            fonts.label(tunables.sidebar_text_px),
            text(colors::NEUTRAL),
            Node {
                flex_grow: 1.0,
                margin: UiRect::right(px(TAG_GAP_PX)),
                ..default()
            },
        ));
        row.spawn((
            TrayTag,
            Text::new(line.tag.clone()),
            TextLayout::linebreak(LineBreak::NoWrap),
            fonts.data(tunables.sidebar_caps_px),
            text(colors::SECONDARY),
        ));
    })
    .observe(
        move |click: On<PointerClick>, check: ClickCheck, mut select: RowSelect| {
            if check.selects(&click) {
                select.clicked(entity, dimmed);
            }
        },
    );
}

/// On a row that can't be picked while the planner is open (MUL-12).
#[derive(Component)]
pub struct DimmedRow;

/// MUL-12: a dimmed row's parts are drawn faint.
pub fn dim_rows(
    rows: Query<Entity, Added<DimmedRow>>,
    children: Query<&Children>,
    mut parts: TintedParts,
) {
    for row in &rows {
        fade_parts(row, DIMMED_ALPHA, &children, &mut parts);
    }
}

/// The tray's root and notch, placed each frame.
type Placed<'w, 's> = (
    Query<
        'w,
        's,
        (
            &'static mut Node,
            &'static mut Visibility,
            &'static ComputedNode,
        ),
        (With<TrayRoot>, Without<TrayNotch>, Without<TrayPanel>),
    >,
    Query<'w, 's, &'static mut Node, (With<TrayNotch>, Without<TrayPanel>)>,
    Query<'w, 's, &'static mut Node, With<TrayPanel>>,
);

/// MUL-08 / MUL-10: show the tray while one is open and its marker is
/// drawn, beside the marker and inside the map area, following it as the
/// camera and clock move; hide it otherwise. Its size is last frame's
/// layout, so a tray that has just opened waits a frame unseen.
pub fn place_tray_panel(
    contents: Res<TrayContents>,
    tunables: Res<Tunables>,
    map: MapView,
    (mut roots, mut notches, mut panels): Placed,
) {
    let Ok((mut node, mut visibility, computed)) = roots.single_mut() else {
        return;
    };
    let (Some(anchor), Some(viewport)) = (contents.anchor, map.viewport()) else {
        if node.display != Display::None {
            node.display = Display::None;
        }
        return;
    };
    let size = node_size(computed);
    let area = MapArea::of(viewport).rect();
    let place = place_tray(anchor, size, area, tunables.tray_gap_px, NOTCH_INSET_PX);
    // Nodes are written only when they move, so a still tray costs no
    // layout.
    node.set_if_neq(Node {
        display: Display::Flex,
        left: px(place.top_left.x),
        top: px(place.top_left.y),
        ..node.clone()
    });
    let measured = size.x > 0.0 && size.y > 0.0;
    visibility.set_if_neq(if measured {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    });
    for mut panel in &mut panels {
        panel.set_if_neq(Node {
            max_height: px(place.max_height),
            ..panel.clone()
        });
    }
    for mut notch in &mut notches {
        let edge = if place.left_side { size.x } else { 0.0 };
        notch.set_if_neq(Node {
            left: px(edge - NOTCH_PX / 2.0),
            top: px(place.notch_y - NOTCH_PX / 2.0),
            ..notch.clone()
        });
    }
}
