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
use super::place::{TrayAnchor, TrayRoom, TraySide, place_tray};
use crate::view::colors;
use crate::view::fonts::Fonts;
use crate::view::hover::ClickCheck;
use crate::view::inset::inspect::spawn_spaced_capitals;
use crate::view::layers;
use crate::view::list_rows::{RowLayout, row_highlight};
use crate::view::markers::draw::{BodyLook, MarkerAssets, spawn_list_icon};
use crate::view::markers::{TintedParts, fade_parts};
use crate::view::node_box::node_size;
use crate::view::screen::{MapArea, MapView};
use crate::view::symbols::draw::{Tint, TintSlot};
use crate::view::targets::MapInputBlocker;
use crate::view::tunables::Tunables;

/// The space inside the panel's sides.
const SIDE_PADDING_PX: f32 = 10.0;
/// The tag's least distance from the name.
const TAG_GAP_PX: f32 = 16.0;
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

/// MUL-09: the tray's rows: `tray_row_px` tall, the indent inside the row
/// so the highlight spans the panel, 8 px between a row's toggle, icon,
/// name and tag, and a toggle slot 10 px wide.
fn row_layout(tunables: &Tunables) -> RowLayout {
    RowLayout {
        height: Some(tunables.tray_row_px),
        pad_y: 0.0,
        side: SIDE_PADDING_PX,
        indent: 16.0,
        indent_inside: true,
        gap: 8.0,
        toggle: 10.0,
        toggle_font: tunables.sidebar_caps_px,
    }
}

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
}
/// A row's tag (MUL-09).
#[derive(Component)]
pub struct TrayTag;

/// MUL-08 / MRK-12 / CAM-10: the tray, hidden until one opens.
pub fn spawn_tray(mut commands: Commands, fonts: Res<Fonts>, tunables: Res<Tunables>) {
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
            spawn_notch(root);
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
            .with_children(|panel| spawn_panel_parts(panel, &fonts, &tunables));
        });
}

/// The notch, a small square turned on its corner in the panel's colours.
fn spawn_notch(root: &mut ChildSpawnerCommands) {
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
}

/// The header or footer: a bar `height` tall with the text inset from the
/// sides, ruled off from the rows by a faint line on `rule`.
fn bar(height: f32, rule: UiRect) -> (Node, BorderColor) {
    let node = Node {
        height: px(height),
        flex_shrink: 0.0,
        align_items: AlignItems::Center,
        padding: UiRect::horizontal(px(SIDE_PADDING_PX)),
        border: rule,
        ..default()
    };
    let divider = colors::mix(colors::PANEL, colors::PANEL_BORDER, DIVIDER_MIX);
    (node, BorderColor::all(divider))
}

/// MUL-08: the panel's header, scrolling list of rows, and footer with
/// the key hint.
fn spawn_panel_parts(panel: &mut ChildSpawnerCommands, fonts: &Fonts, tunables: &Tunables) {
    let rule = UiRect::bottom(px(HAIRLINE_PX));
    panel.spawn((TrayHeader, bar(tunables.tray_header_px, rule)));
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
    let rule = UiRect::top(px(HAIRLINE_PX));
    panel.spawn(bar(tunables.tray_footer_px, rule)).with_child((
        TrayFooter,
        Text::default(),
        TextLayout::linebreak(LineBreak::NoWrap),
        fonts.data(tunables.sidebar_caps_px),
        TextColor(colors::SECONDARY),
    ));
}

/// The parts of the tray that are refilled.
type Parts<'w, 's> = (
    Query<'w, 's, Entity, With<TrayHeader>>,
    Query<'w, 's, Entity, With<TrayList>>,
    Query<'w, 's, &'static mut Text, With<TrayFooter>>,
);

/// Whether two lines draw the same row, whichever of them is selected:
/// the selection only recolours a row (`highlight_tray_rows`).
fn same_but_selection(a: &TrayLine, b: &TrayLine) -> bool {
    let TrayLine {
        row,
        name,
        tag,
        look,
        selected: _,
        dimmed,
    } = a;
    (row, name, tag, look, dimmed) == (&b.row, &b.name, &b.tag, &b.look, &b.dimmed)
}

/// What the panel was last filled with: the header and the lines.
type Filled = Option<(String, Vec<TrayLine>)>;

/// MUL-08 / MUL-09: refill the header and rows whenever what they say
/// changes: another tray, the marker's things, a row opened or closed, or
/// the planner's picks. A new selected row only moves the highlight
/// (`highlight_tray_rows`); the footer's hint is rewritten when it
/// changes.
pub fn fill_tray(
    mut commands: Commands,
    (fonts, tunables): (Res<Fonts>, Res<Tunables>),
    contents: Res<TrayContents>,
    (headers, lists, mut footers): Parts,
    mut shown: Local<Filled>,
) {
    for mut footer in &mut footers {
        if footer.0 != contents.hint {
            footer.0.clone_from(&contents.hint);
        }
    }
    // The contents change every frame the marker moves; the rows only when
    // what they say does.
    let unchanged = shown.as_ref().is_some_and(|(header, lines)| {
        *header == contents.header
            && lines.len() == contents.lines.len()
            && lines
                .iter()
                .zip(&contents.lines)
                .all(|(a, b)| same_but_selection(a, b))
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
    *shown = Some((contents.header.clone(), contents.lines.clone()));
}

/// MUL-09 / MUL-10: the selected row is highlighted as in the outline,
/// the highlight moving with the selection without rebuilding the rows.
pub fn highlight_tray_rows(
    contents: Res<TrayContents>,
    mut rows: Query<(&TrayRowFor, &mut BackgroundColor)>,
) {
    if !contents.is_changed() {
        return;
    }
    let cursor = contents
        .lines
        .iter()
        .find(|line| line.selected)
        .map(|line| line.row.entity);
    for (row, mut background) in &mut rows {
        background.set_if_neq(row_highlight(cursor == Some(row.entity)));
    }
}

/// MUL-09: one row: its toggle slot, its symbol at `list_symbol_px` (or
/// its body marker), its name in the neutral colour and its tag at the
/// right in the secondary colour; highlighted while selected. A click on
/// the toggle opens or closes its members; anywhere else on it selects
/// it, or picks it while the planner is open (MUL-10, MUL-12). The
/// release of a drag over it does neither (CAM-10).
fn spawn_row(list: &mut ChildSpawnerCommands, assets: MarkerAssets, line: &TrayLine) {
    let entity = line.row.entity;
    let dimmed = line.dimmed;
    let layout = row_layout(assets.1);
    let mut row = list.spawn((
        TrayRowFor { entity },
        layout.row(line.row.depth, line.selected),
    ));
    if dimmed {
        row.insert(DimmedRow);
    }
    row.with_children(|row| spawn_row_parts(row, assets, &layout, line))
        .observe(
            move |click: On<PointerClick>, check: ClickCheck, mut select: RowSelect| {
                if check.selects(&click) {
                    select.clicked(entity, dimmed);
                }
            },
        );
}

/// MUL-09: what a row holds, left to right: the toggle, which opens or
/// closes its members when it has any, the icon, the name and the tag.
fn spawn_row_parts(
    row: &mut ChildSpawnerCommands,
    assets: MarkerAssets,
    layout: &RowLayout,
    line: &TrayLine,
) {
    let (fonts, tunables) = assets;
    let entity = line.row.entity;
    let toggle = layout.spawn_toggle(row, fonts, line.row.open);
    if line.row.open.is_some() {
        row.commands().entity(toggle).observe(
            move |mut click: On<PointerClick>, check: ClickCheck, mut state: ResMut<TrayState>| {
                click.propagate(false);
                if check.selects(&click) {
                    toggle_row(&mut state, entity);
                }
            },
        );
    }
    let look = BodyLook {
        class: line.look.class,
        affiliation: line.look.affiliation,
        record: line.look.record.as_ref(),
        group: None,
    };
    spawn_list_icon(row, assets, look, tunables.list_marker_px / 2.0);
    let text = |color: Color| (TextColor(color), Tint::new(color, TintSlot::Text));
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
    let (Some(marker), Some(viewport)) = (contents.anchor, map.viewport()) else {
        if node.display != Display::None {
            node.display = Display::None;
        }
        return;
    };
    let size = node_size(computed);
    let anchor = TrayAnchor {
        marker,
        parent: contents.parent,
    };
    let room = TrayRoom {
        size,
        area: MapArea::of(viewport).rect(),
        gap: tunables.tray_gap_px,
        notch_inset: NOTCH_INSET_PX,
    };
    let place = place_tray(anchor, room);
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
    // The notch's centre, on the edge facing the marker.
    let at = match place.side {
        TraySide::Right => Vec2::new(0.0, place.notch),
        TraySide::Left => Vec2::new(size.x, place.notch),
        TraySide::Below => Vec2::new(place.notch, 0.0),
        TraySide::Above => Vec2::new(place.notch, size.y.min(place.max_height)),
    };
    for mut notch in &mut notches {
        notch.set_if_neq(Node {
            left: px(at.x - NOTCH_PX / 2.0),
            top: px(at.y - NOTCH_PX / 2.0),
            ..notch.clone()
        });
    }
}
