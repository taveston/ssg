//! The inspection sidebar (INS-01 to INS-04, INS-07, INS-09, MOT-07,
//! PLNR-01, PLNR-02, PLNR-05): one panel for the one selection (INS-03),
//! at the right of the window from the top bar to the bottom (INS-07). It
//! slides in and out with the selection (`sidebar.rs` holds the slide;
//! the map area follows it). The details
//! are rebuilt in place whenever the selection changes; the "In orbit"
//! outline below them (`inset/outline.rs`) is built once and refilled. Content
//! taller than the sidebar scrolls inside it, and the wheel over it scrolls
//! it instead of zooming the map.
//!
//! The details are built from an `InsetContent` - plain values gathered
//! from the body (`inset/content.rs`) - by one helper per section, so
//! what an inset says can be tested apart from how it is laid out. Key
//! facts are label-over-value pairs in wrapping rows, and each section
//! below them has a heading in small spaced capitals over a hairline
//! (INS-09). Bevy's text has no letter spacing, so spaced capitals are
//! spawned a letter at a time.
//!
//! A ship the faction commands has a Transfer button that opens the
//! transfer planner (`view/planner/`); while it is open, the image, the
//! description and the key facts give way to the planner's panel, which
//! sits between them and the outline.

use bevy::ecs::system::SystemParam;
use bevy::input::mouse::{AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;

use crate::view::layers;

use super::content::{InsetBodies, InsetContent, InsetImage, gather_content, offers_transfer};
use super::fidelity::FidelityImages;
use super::outline::OutlineRoot;
use crate::sim::group::ShipGroup;
use crate::sim::named_orbit::InNamedOrbit;
use crate::sim::ship::InFlight;
use crate::sim::transfer::plan::Plan;
use crate::view::bindings::{Bindings, pressed};
use crate::view::buttons::{ButtonStyle, ConsoleButton, spawn_button};
use crate::view::camera::requests::FrameRequest;
use crate::view::colors;
use crate::view::fonts::Fonts;
use crate::view::motion_mode::MotionModeState;
use crate::view::multi::OpenTray;
use crate::view::planner::state::PlannerState;
use crate::view::planner::words::TRANSFER;
use crate::view::selection::Selection;
use crate::view::sidebar::SidebarState;
use crate::view::targets::{MapInputBlocker, PointerTargets};
use crate::view::topbar::TOP_BAR_HEIGHT_PX;
use crate::view::tunables::Tunables;
use crate::{Boot, Stage};

/// The sidebar panel, which slides and scrolls (INS-07).
#[derive(Component)]
pub struct InsetPanel;
/// The part of the panel rebuilt for each selection: header, image,
/// description and facts.
#[derive(Component)]
struct InsetDetails;
/// A key fact's label, in small spaced capitals (INS-09); holds the plain
/// text the letters spell.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct FactLabel(pub String);
/// A key fact's value (INS-09).
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct FactValue(pub String);
/// A section's heading, in small spaced capitals (INS-09).
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct SectionHeading(pub String);
/// The hairline above a section (INS-09).
#[derive(Component)]
pub struct Hairline;
/// The inset's Focus button, for the body it would make the frame
/// (CAM-24).
#[derive(Component)]
struct FocusButton(Entity);
/// The inset line MOT-07 asks for, for the body it describes.
#[derive(Component)]
struct PositionNote(Entity);
/// PLNR-01: the inset's Transfer button, for the ship it plans for.
#[derive(Component)]
pub struct TransferButton(pub Entity);
/// PLNR-02: the part of the details that gives way to the planner: the
/// image, the description, the key facts and what follows them.
#[derive(Component)]
struct InsetBody;

/// MOT-07: what an inset says for the position of a body in ring mode.
const POSITION_SHOWN_WHEN_PAUSED: &str = "Position: shown when paused";

const PANEL_PADDING_PX: f32 = 10.0;
const PANEL_ROW_GAP_PX: f32 = 5.0;
/// The hairline along the sidebar's left edge.
const PANEL_EDGE_PX: f32 = 1.0;
/// How far one wheel line scrolls the inset.
const SCROLL_LINE_PX: f32 = 24.0;
const IMAGE_HEIGHT_PX: f32 = 64.0;
/// The fonts and tunables the panel is built from.
type PanelAssets<'a> = (&'a Fonts, &'a Tunables);

/// The orange corner brackets that frame a telescope image (INS-04).
const BRACKET_PX: f32 = 10.0;
const BRACKET_LINE_PX: f32 = 2.0;
/// Small spaced capitals: the size, the gap between letters and the gap
/// between words (INS-09).
const CAPS_LETTER_GAP_PX: f32 = 1.5;
const CAPS_WORD_GAP_PX: f32 = 5.0;
/// Key facts wrap in rows: each takes at least this much of the row, so
/// two short ones share it and a long one has it alone (INS-09).
const FACT_MIN_WIDTH_PERCENT: f32 = 45.0;
const FACT_COLUMN_GAP_PX: f32 = 10.0;
const FACT_ROW_GAP_PX: f32 = 6.0;
/// The hairline between sections, and the space round it (INS-09).
const HAIRLINE_PX: f32 = 1.0;
const SECTION_GAP_PX: f32 = 8.0;

pub struct InspectPlugin;

/// The inset has been rebuilt for a changed selection this frame
/// (`rebuild_inset`); systems that fill what it contains run after it.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct InsetRebuilt;

/// CAM-24: the Focus button is disabled (dimmed, and the camera ignores its
/// click) while its body is in exposure-ring mode.
fn dim_focus_buttons(
    modes: Query<&MotionModeState>,
    mut buttons: Query<(&FocusButton, &mut ConsoleButton)>,
) {
    for (focus, mut button) in &mut buttons {
        let ring = modes.get(focus.0).is_ok_and(|mode| mode.ring_mode);
        if button.disabled != ring {
            button.disabled = ring;
        }
    }
}

impl Plugin for InspectPlugin {
    fn build(&self, app: &mut App) {
        // INS-03: Esc reads whether a tray is open, to close it first.
        app.init_resource::<OpenTray>()
            .add_systems(Startup, spawn_sidebar.in_set(Boot::Overlay))
            .add_systems(
                Update,
                (
                    handle_escape,
                    slide_panel,
                    rebuild_inset.in_set(InsetRebuilt),
                    update_position_notes,
                    dim_focus_buttons,
                    show_transfer_buttons,
                    make_way_for_planner,
                    scroll_inset,
                )
                    .chain()
                    .in_set(Stage::Overlay),
            );
    }
}

/// INS-07: the sidebar: `inset_width_px` wide, from the top bar to the
/// bottom of the window, off the right edge until it slides in. Its
/// details and the outline sit in one scrolling column.
fn spawn_sidebar(mut commands: Commands, tunables: Res<Tunables>, fonts: Res<Fonts>) {
    commands
        .spawn((
            InsetPanel,
            GlobalZIndex(layers::PANELS),
            // CAM-10: a press on the sidebar doesn't start a map drag.
            MapInputBlocker,
            Node {
                position_type: PositionType::Absolute,
                top: px(TOP_BAR_HEIGHT_PX),
                right: px(-tunables.inset_width_px),
                bottom: px(0),
                width: px(tunables.inset_width_px),
                padding: UiRect::all(px(PANEL_PADDING_PX)),
                flex_direction: FlexDirection::Column,
                row_gap: px(PANEL_ROW_GAP_PX),
                border: UiRect::left(px(PANEL_EDGE_PX)),
                overflow: Overflow::scroll_y(),
                display: Display::None,
                ..default()
            },
            ScrollPosition::default(),
            BackgroundColor(colors::PANEL),
            BorderColor::all(colors::PANEL_BORDER),
        ))
        .with_children(|panel| {
            panel.spawn((
                InsetDetails,
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(PANEL_ROW_GAP_PX),
                    ..default()
                },
            ));
            // INS-05 / INS-09: the "In orbit" section; the outline fills
            // itself (`inset/outline.rs`).
            spawn_section_heading(panel, (&fonts, &tunables), "In orbit");
            panel.spawn((
                OutlineRoot,
                Node {
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
            ));
        });
}

/// INS-09: text as small spaced capitals: upper case, one piece per
/// letter so each can be spaced, with a wider gap at each space.
pub fn spaced_capitals(text: &str) -> Vec<Option<char>> {
    text.to_uppercase()
        .chars()
        .map(|c| (!c.is_whitespace()).then_some(c))
        .collect()
}

/// INS-09: a line of small spaced capitals in the secondary colour.
pub fn spawn_spaced_capitals(
    parent: &mut ChildSpawnerCommands,
    (fonts, tunables): PanelAssets,
    text: &str,
    marker: impl Bundle,
) {
    parent
        .spawn((
            marker,
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Baseline,
                column_gap: px(CAPS_LETTER_GAP_PX),
                ..default()
            },
        ))
        .with_children(|line| {
            for letter in spaced_capitals(text) {
                match letter {
                    Some(letter) => {
                        line.spawn((
                            Text::new(letter.to_string()),
                            fonts.label(tunables.sidebar_caps_px),
                            TextColor(colors::SECONDARY),
                        ));
                    }
                    None => {
                        line.spawn(Node {
                            width: px(CAPS_WORD_GAP_PX),
                            ..default()
                        });
                    }
                }
            }
        });
}

/// INS-09: a section below the key facts: a hairline, then its heading in
/// small spaced capitals.
fn spawn_section_heading(panel: &mut ChildSpawnerCommands, assets: PanelAssets, title: &str) {
    panel.spawn((
        Hairline,
        Node {
            width: percent(100),
            height: px(HAIRLINE_PX),
            margin: UiRect::vertical(px(SECTION_GAP_PX)),
            ..default()
        },
        BackgroundColor(colors::PANEL_BORDER),
    ));
    spawn_spaced_capitals(panel, assets, title, SectionHeading(title.to_string()));
}

/// INS-09: the key facts as label-over-value pairs in wrapping rows: the
/// label in small spaced capitals in the secondary colour, the value in
/// the data face in the neutral colour.
fn spawn_facts(panel: &mut ChildSpawnerCommands, assets: PanelAssets, facts: &[(String, String)]) {
    let (fonts, tunables) = assets;
    panel
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(FACT_COLUMN_GAP_PX),
            row_gap: px(FACT_ROW_GAP_PX),
            ..default()
        })
        .with_children(|rows| {
            for (label, value) in facts {
                rows.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    min_width: percent(FACT_MIN_WIDTH_PERCENT),
                    flex_grow: 1.0,
                    ..default()
                })
                .with_children(|fact| {
                    spawn_spaced_capitals(fact, assets, label, FactLabel(label.clone()));
                    fact.spawn((
                        FactValue(value.clone()),
                        Text::new(value.clone()),
                        fonts.data(tunables.sidebar_text_px),
                        TextColor(colors::NEUTRAL),
                    ));
                });
            }
        });
}

/// INS-07: where the sidebar's right edge sits for a slide: flush with
/// the window once open, its whole width off the edge when hidden.
pub fn sidebar_right_px(slide: f32, width_px: f32) -> f32 {
    -(1.0 - slide.clamp(0.0, 1.0)) * width_px
}

/// INS-07: the panel follows the slide, and is not drawn at all while
/// fully hidden.
fn slide_panel(
    sidebar: Res<SidebarState>,
    tunables: Res<Tunables>,
    mut panels: Query<&mut Node, With<InsetPanel>>,
) {
    for mut node in &mut panels {
        let right = px(sidebar_right_px(sidebar.slide, tunables.inset_width_px));
        let display = if sidebar.visible() {
            Display::Flex
        } else {
            Display::None
        };
        if node.right != right || node.display != display {
            node.right = right;
            node.display = display;
        }
    }
}

/// INS-03: Esc closes the inset. While a multi-marker's tray is open, Esc
/// closes the tray first and leaves the selection (MUL-10); while the
/// transfer planner is open, Esc ends the planner before the inset closes
/// (PLNR-02, PLNR-03).
fn handle_escape(
    keys: Res<ButtonInput<KeyCode>>,
    bindings: Res<Bindings>,
    mut tray: ResMut<OpenTray>,
    mut planner: ResMut<PlannerState>,
    mut selection: ResMut<Selection>,
) {
    if !pressed(&keys, &bindings.close_inset) {
        return;
    }
    if tray.0.take().is_none() && !planner.end() {
        selection.clear();
    }
}

/// What PLNR-01 reads of a ship: whether it is a group, parked, in
/// flight, and its plan.
pub type OrderableShip = (
    Has<ShipGroup>,
    Has<InNamedOrbit>,
    Has<InFlight>,
    Option<&'static Plan>,
);

/// PLNR-01: whether a ship may be given an order now: not a group,
/// parked in a named orbit and not in flight, with no active plan.
pub fn may_take_order((group, parked, flying, plan): (bool, bool, bool, Option<&Plan>)) -> bool {
    offers_transfer(group, parked && !flying, plan.map(|p| p.status))
}

/// PLNR-01: the Transfer button shows while its ship may be given an
/// order, and not while the planner is open for it.
fn show_transfer_buttons(
    planner: Res<PlannerState>,
    ships: Query<OrderableShip>,
    mut buttons: Query<(&TransferButton, &mut Node)>,
) {
    for (button, mut node) in &mut buttons {
        let shown = ships.get(button.0).is_ok_and(may_take_order) && !planner.is_open_for(button.0);
        let display = if shown { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
    }
}

/// PLNR-02: while the planner is open, the image, description and key
/// facts give way to it; they come back when it ends.
fn make_way_for_planner(planner: Res<PlannerState>, mut bodies: Query<&mut Node, With<InsetBody>>) {
    let display = if planner.is_open() {
        Display::None
    } else {
        Display::Flex
    };
    for mut node in &mut bodies {
        if node.display != display {
            node.display = display;
        }
    }
}

/// INS-01 / INS-03: whenever the selection changes, the details are
/// replaced in place; the outline below them stays (INS-05). With nothing
/// selected the details are emptied and the sidebar slides out. A ship's
/// details are also rebuilt when its facts change, as when an order
/// reserves delta-v (PLNR-05).
fn rebuild_inset(
    selection: Res<Selection>,
    mut commands: Commands,
    style: InsetStyle,
    bodies: InsetBodies,
    details: Query<Entity, With<InsetDetails>>,
    mut shown_facts: Local<Vec<(String, String)>>,
) {
    let InsetStyle {
        fonts,
        fidelity,
        tunables,
    } = &style;
    let content = selection
        .selected
        .and_then(|body| gather_content(body, &bodies, tunables));
    let facts = content
        .as_ref()
        .map(|c| c.facts.clone())
        .unwrap_or_default();
    if !selection.is_changed() && facts == *shown_facts {
        return;
    }
    *shown_facts = facts;
    let Ok(details) = details.single() else {
        return;
    };
    let mut details = commands.entity(details);
    details.despawn_related::<Children>();
    if let Some(content) = content {
        details.with_children(|panel| fill_panel(panel, fidelity, (fonts, tunables), &content));
    }
}

/// What the inset's details are drawn with: the fonts, the knowledge
/// images and the tunables.
#[derive(SystemParam)]
struct InsetStyle<'w> {
    fonts: Res<'w, Fonts>,
    fidelity: Res<'w, FidelityImages>,
    tunables: Res<'w, Tunables>,
}

/// INS-07: the wheel over the inset scrolls it, within its content.
fn scroll_inset(
    scroll: Res<AccumulatedMouseScroll>,
    targets: PointerTargets,
    mut panels: Query<(Entity, &mut ScrollPosition, &ComputedNode), With<InsetPanel>>,
) {
    let Ok((panel, mut position, computed)) = panels.single_mut() else {
        return;
    };
    if scroll.delta.y == 0.0 || targets.panel() != Some(panel) {
        return;
    }
    let lines = match scroll.unit {
        MouseScrollUnit::Line => SCROLL_LINE_PX,
        MouseScrollUnit::Pixel => 1.0,
    };
    let max = ((computed.content_size() - computed.size()) * computed.inverse_scale_factor()).y;
    position.y = scrolled(position.y, -scroll.delta.y * lines, max);
}

/// INS-07: a scroll offset moved by `delta`, kept between the top and the
/// end of the content.
pub fn scrolled(offset: f32, delta: f32, max: f32) -> f32 {
    (offset + delta).clamp(0.0, max.max(0.0))
}

/// INS-02: header, class line, image, description and facts, the
/// position note (MOT-07) and the disabled planetary-view button.
fn fill_panel(
    panel: &mut ChildSpawnerCommands,
    fidelity: &FidelityImages,
    assets: PanelAssets,
    content: &InsetContent,
) {
    let (fonts, tunables) = assets;
    spawn_header(panel, assets, content);
    spawn_text(
        panel,
        &content.class_line,
        fonts.label(tunables.sidebar_text_px),
        colors::PANEL_BORDER,
    );
    panel
        .spawn((
            InsetBody,
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(PANEL_ROW_GAP_PX),
                ..default()
            },
        ))
        .with_children(|body| {
            spawn_image(body, fidelity, content.image);
            spawn_text(
                body,
                &content.description,
                fonts.label(tunables.sidebar_text_px),
                colors::NEUTRAL,
            );
            spawn_facts(body, assets, &content.facts);
            spawn_position_note(body, assets, content.body);
            spawn_planetary_view_button(body, assets);
        });
}

/// MOT-07: the line that stands in for a position while the body is an
/// exposure ring. Shown and hidden by `update_position_notes`.
fn spawn_position_note(
    panel: &mut ChildSpawnerCommands,
    (fonts, tunables): PanelAssets,
    body: Entity,
) {
    panel.spawn((
        PositionNote(body),
        Node {
            display: Display::None,
            ..default()
        },
        Text::new(POSITION_SHOWN_WHEN_PAUSED),
        fonts.data(tunables.sidebar_text_px),
        TextColor(colors::NEUTRAL),
    ));
}

/// MOT-07: a body in exposure-ring mode has no readable position, so its
/// inset says where to get one.
fn update_position_notes(
    modes: Query<&MotionModeState>,
    mut notes: Query<(&PositionNote, &mut Node)>,
) {
    for (note, mut node) in &mut notes {
        let ring = modes.get(note.0).is_ok_and(|mode| mode.ring_mode);
        node.display = if ring { Display::Flex } else { Display::None };
    }
}

fn spawn_text(parent: &mut ChildSpawnerCommands, text: &str, font: TextFont, color: Color) {
    parent.spawn((Text::new(text), font, TextColor(color)));
}

/// INS-02: the name, and the Focus and Close buttons.
fn spawn_header(panel: &mut ChildSpawnerCommands, assets: PanelAssets, content: &InsetContent) {
    let (fonts, tunables) = assets;
    panel
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|header| {
            header.spawn((
                Text::new(content.name.clone()),
                fonts.label_semibold(tunables.sidebar_header_px),
                TextColor(colors::NEUTRAL),
            ));
            header
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: px(8),
                    ..default()
                })
                .with_children(|actions| {
                    spawn_actions(actions, assets, content.body, content.commanded_ship)
                });
        });
}

fn spawn_actions(
    actions: &mut ChildSpawnerCommands,
    (fonts, tunables): PanelAssets,
    body: Entity,
    commanded_ship: bool,
) {
    spawn_button(
        actions,
        fonts,
        "Focus",
        tunables.sidebar_text_px,
        ConsoleButton::new(ButtonStyle::Secondary),
    )
    .insert(FocusButton(body))
    // ZOOM-16: the inset's Focus button makes the inset's object the
    // frame and moves to it, as a double-click does. It is disabled
    // while the object is in exposure-ring mode (CAM-24).
    .observe(
        move |_click: On<PointerClick>,
              modes: Query<&MotionModeState>,
              mut selection: ResMut<Selection>,
              mut request: ResMut<FrameRequest>| {
            if modes.get(body).is_ok_and(|mode| mode.ring_mode) {
                return;
            }
            selection.select(body);
            request.0 = Some(body);
        },
    );
    // PLNR-01 / PLNR-02: Transfer opens the planner for the ship, which
    // stays selected. Shown while the ship may be given an order
    // (`show_transfer_buttons`).
    if commanded_ship {
        spawn_button(
            actions,
            fonts,
            TRANSFER,
            tunables.sidebar_text_px,
            ConsoleButton::new(ButtonStyle::Secondary),
        )
        .insert(TransferButton(body))
        .observe(
            move |click: On<PointerClick>, mut planner: ResMut<PlannerState>| {
                if click.button == PointerButton::Primary {
                    planner.open(body);
                }
            },
        );
    }
    // INS-03 / PLNR-02: Close clears the selection and closes the inset,
    // which ends the planner.
    spawn_button(
        actions,
        fonts,
        "Close",
        tunables.sidebar_text_px,
        ConsoleButton::new(ButtonStyle::Quiet),
    )
    .observe(
        |_click: On<PointerClick>, mut selection: ResMut<Selection>| {
            selection.clear();
        },
    );
}

/// INS-04: the knowledge-state image; a telescope frame gets orange corner
/// brackets.
fn spawn_image(panel: &mut ChildSpawnerCommands, fidelity: &FidelityImages, image: InsetImage) {
    let handle = match image {
        InsetImage::Surveyed => fidelity.surveyed.clone(),
        InsetImage::Telescope => fidelity.unsurveyed.clone(),
    };
    panel
        .spawn(Node {
            width: percent(100),
            height: px(IMAGE_HEIGHT_PX),
            ..default()
        })
        .with_children(|frame| {
            frame.spawn((
                ImageNode::new(handle),
                Node {
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
            ));
            if image == InsetImage::Telescope {
                for (top, right) in [(true, true), (true, false), (false, true), (false, false)] {
                    spawn_corner_bracket(frame, top, right);
                }
            }
        });
}

fn spawn_corner_bracket(frame: &mut ChildSpawnerCommands, top: bool, right: bool) {
    let line = |on: bool| if on { px(BRACKET_LINE_PX) } else { px(0) };
    let edge = |on: bool| if on { px(0) } else { Val::Auto };
    frame.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: px(BRACKET_PX),
            height: px(BRACKET_PX),
            top: edge(top),
            bottom: edge(!top),
            left: edge(!right),
            right: edge(right),
            border: UiRect {
                top: line(top),
                bottom: line(!top),
                left: line(!right),
                right: line(right),
            },
            ..default()
        },
        BorderColor::all(colors::WARNING),
    ));
}

/// INS-02: "Open planetary view", a secondary button disabled until that
/// view exists.
fn spawn_planetary_view_button(panel: &mut ChildSpawnerCommands, (fonts, tunables): PanelAssets) {
    panel
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            ..default()
        })
        .with_children(|row| {
            spawn_button(
                row,
                fonts,
                "Open planetary view",
                tunables.sidebar_text_px,
                ConsoleButton::new(ButtonStyle::Secondary).disabled(),
            );
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// INS-07: the sidebar runs from the top bar to the bottom of the
    /// window, `inset_width_px` wide, scrolls, and slides in from off the
    /// right edge as the slide runs.
    #[test]
    fn the_sidebar_spans_the_right_edge_and_slides_in() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = World::new();
        world.init_resource::<Tunables>();
        world.init_resource::<SidebarState>();
        world.insert_resource(Fonts {
            label: Handle::default(),
            label_semibold: Handle::default(),
            text: Handle::default(),
            text_semibold: Handle::default(),
            data: Handle::default(),
        });
        world.run_system_once(spawn_sidebar).unwrap();
        world.run_system_once(slide_panel).unwrap();
        let mut panels = world.query_filtered::<&Node, With<InsetPanel>>();
        let panel = panels.single(&world).unwrap();
        assert_eq!(panel.top, px(TOP_BAR_HEIGHT_PX));
        assert_eq!(panel.bottom, px(0));
        assert_eq!(panel.width, px(320.0));
        assert_eq!(panel.position_type, PositionType::Absolute);
        assert_eq!(panel.overflow, Overflow::scroll_y(), "long content scrolls");
        assert_eq!((panel.right, panel.display), (px(-320.0), Display::None));

        world.resource_mut::<SidebarState>().slide = 0.25;
        world.run_system_once(slide_panel).unwrap();
        let panel = panels.single(&world).unwrap();
        assert_eq!((panel.right, panel.display), (px(-240.0), Display::Flex));
        world.resource_mut::<SidebarState>().slide = 1.0;
        world.run_system_once(slide_panel).unwrap();
        assert_eq!(panels.single(&world).unwrap().right, px(0.0));
        assert_eq!(sidebar_right_px(2.0, 320.0), 0.0);
    }

    /// INS-07: scrolling stops at the top and at the end of the content,
    /// and content that fits doesn't scroll at all.
    #[test]
    fn the_inset_scrolls_within_its_content() {
        assert_eq!(scrolled(0.0, 24.0, 100.0), 24.0);
        assert_eq!(scrolled(90.0, 24.0, 100.0), 100.0);
        assert_eq!(scrolled(10.0, -24.0, 100.0), 0.0);
        assert_eq!(scrolled(0.0, 24.0, -40.0), 0.0);
    }

    /// CAM-24: the Focus button is disabled only while its body is in
    /// ring mode.
    #[test]
    fn focus_button_is_disabled_in_ring_mode() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = World::new();
        let body = world.spawn(MotionModeState::default()).id();
        let button = world
            .spawn((
                FocusButton(body),
                ConsoleButton::new(ButtonStyle::Secondary),
            ))
            .id();
        world.run_system_once(dim_focus_buttons).unwrap();
        assert!(!world.get::<ConsoleButton>(button).unwrap().disabled);
        world.get_mut::<MotionModeState>(body).unwrap().ring_mode = true;
        world.run_system_once(dim_focus_buttons).unwrap();
        assert!(world.get::<ConsoleButton>(button).unwrap().disabled);
    }

    /// MOT-07: the inset says "Position: shown when paused" while its body
    /// is an exposure ring, and hides the line once it is a dot again.
    #[test]
    fn position_note_shows_only_in_ring_mode() {
        let mut world = World::new();
        let body = world
            .spawn(MotionModeState {
                tail_degrees: 360.0,
                closing: 1.0,
                ring_mode: true,
            })
            .id();
        let note = world
            .spawn((
                PositionNote(body),
                Node {
                    display: Display::None,
                    ..default()
                },
            ))
            .id();
        let mut update = IntoSystem::into_system(update_position_notes);
        update.initialize(&mut world);
        update.run((), &mut world).unwrap();
        assert_eq!(world.get::<Node>(note).unwrap().display, Display::Flex);

        world.get_mut::<MotionModeState>(body).unwrap().ring_mode = false;
        update.run((), &mut world).unwrap();
        assert_eq!(world.get::<Node>(note).unwrap().display, Display::None);
        assert_eq!(POSITION_SHOWN_WHEN_PAUSED, "Position: shown when paused");
    }

    /// INS-09: spaced capitals are upper case, one letter at a time, with
    /// a gap in place of each space.
    #[test]
    fn spaced_capitals_are_upper_case_letters_with_gaps() {
        assert_eq!(
            spaced_capitals("Mean radius"),
            [
                Some('M'),
                Some('E'),
                Some('A'),
                Some('N'),
                None,
                Some('R'),
                Some('A'),
                Some('D'),
                Some('I'),
                Some('U'),
                Some('S')
            ]
        );
        assert!(spaced_capitals("").is_empty());
    }
}
