//! The inspection inset (INS-01 to INS-04, INS-07, MOT-07): one panel for
//! the one selection (INS-03), docked at the right of the map area below
//! the top bar (INS-07). Its contents are rebuilt in place whenever the
//! selection changes; content taller than the map scrolls inside it, and
//! the wheel over it scrolls it instead of zooming the map.
//!
//! The panel is built from an `InsetContent` - plain values gathered from
//! the body - by one helper per section, so what an inset says can be
//! tested apart from how it is laid out.

use bevy::input::mouse::{AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;

use super::bindings::{Bindings, pressed};
use super::camera::requests::FrameRequest;
use super::colors;
use super::date::{date_from_elapsed_days, format_date, format_time_ago};
use super::fidelity::FidelityImages;
use super::fonts::Fonts;
use super::hover::ownership_label;
use super::in_orbit::{OrbitEntry, orbit_entries, spawn_in_orbit_list};
use super::marker_images::MarkerImages;
use super::motion_mode::MotionModeState;
use super::selection::Selection;
use super::targets::{MapInputBlocker, PointerTargets};
use super::topbar::TOP_BAR_HEIGHT_PX;
use super::tunables::Tunables;
use crate::sim::body::{BodyClass, Description, DisplayName, KeyFacts, Knowledge, Ownership};
use crate::sim::orbit::Orbit;
use crate::{Boot, Stage};

/// The dock the inset sits in: the right-hand strip of the map area. It
/// takes no clicks itself, so the map under its empty part stays live.
#[derive(Component)]
struct InsetDock;
/// The inset panel, which scrolls (INS-07).
#[derive(Component)]
struct InsetPanel;
/// The inset's Focus button, for the body it would make the frame
/// (CAM-24).
#[derive(Component)]
struct FocusButton(Entity);
/// The inset line MOT-07 asks for, for the body it describes.
#[derive(Component)]
struct PositionNote(Entity);

/// MOT-07: what an inset says for the position of a body in ring mode.
const POSITION_SHOWN_WHEN_PAUSED: &str = "Position: shown when paused";

const PANEL_PADDING_PX: f32 = 10.0;
const PANEL_ROW_GAP_PX: f32 = 5.0;
/// How far one wheel line scrolls the inset.
const SCROLL_LINE_PX: f32 = 24.0;
const IMAGE_HEIGHT_PX: f32 = 64.0;
/// The orange corner brackets that frame a telescope image (INS-04).
const BRACKET_PX: f32 = 10.0;
const BRACKET_LINE_PX: f32 = 2.0;
const HEADER_FONT_PX: f32 = 15.0;
const BODY_FONT_PX: f32 = 11.0;
const STATUS_FONT_PX: f32 = 10.0;
/// INS-04: the sepia tone of a stale record.
const STALE_TONE: Color = Color::srgb(0.65, 0.55, 0.4);
/// INS-02: "Open planetary view" is disabled until that view exists.
const DISABLED_ALPHA: f32 = 0.4;
const DISABLED_BORDER_ALPHA: f32 = 0.3;

/// INS-02: the class line's readable class name.
pub fn class_label(class: BodyClass) -> &'static str {
    match class {
        BodyClass::Star => "Star",
        BodyClass::Planet => "Planet",
        BodyClass::Moon => "Moon",
        BodyClass::Station => "Station",
        BodyClass::Gate => "Gate",
        BodyClass::SalvageSite => "Salvage site",
        BodyClass::Ship => "Ship",
    }
}

/// INS-04: which image an inset shows, by knowledge state.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InsetImage {
    /// A clean render.
    Surveyed,
    /// A grainy telescope frame, with corner brackets.
    Telescope,
    /// A sepia image.
    Stale,
}

/// INS-04: the colour a status line is drawn in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StatusTone {
    Surveyed,
    Unsurveyed,
    Stale,
}

impl StatusTone {
    fn color(self) -> Color {
        match self {
            StatusTone::Surveyed => colors::OWN,
            // INS-04: unsurveyed is reported in orange.
            StatusTone::Unsurveyed => colors::FOREIGN,
            StatusTone::Stale => STALE_TONE,
        }
    }
}

/// INS-04: the status line, its tone and the image for a knowledge state.
/// The stale line gives the time since imaging rather than the date
/// (F4-10, open for triage).
pub fn knowledge_status(knowledge: &Knowledge) -> (String, StatusTone, InsetImage) {
    match knowledge {
        Knowledge::Surveyed { imaged_day } => (
            format!(
                "SURVEYED · {}",
                format_date(&date_from_elapsed_days(*imaged_day))
            ),
            StatusTone::Surveyed,
            InsetImage::Surveyed,
        ),
        Knowledge::Unsurveyed => (
            "TELESCOPE FRAME · UNSURVEYED".to_string(),
            StatusTone::Unsurveyed,
            InsetImage::Telescope,
        ),
        Knowledge::Stale { last_imaged_day } => (
            format!("LAST IMAGED · {}", format_time_ago(-*last_imaged_day)),
            StatusTone::Stale,
            InsetImage::Stale,
        ),
    }
}

/// Everything an inset shows, as plain values (INS-02).
struct InsetContent {
    body: Entity,
    name: String,
    class_line: String,
    status: String,
    tone: StatusTone,
    image: InsetImage,
    description: String,
    facts: Vec<(String, String)>,
    satellites: Vec<OrbitEntry>,
}

/// The satellites an inset lists (INS-05).
type Satellites<'w, 's> = Query<
    'w,
    's,
    (
        &'static DisplayName,
        &'static BodyClass,
        &'static Ownership,
        &'static Orbit,
    ),
>;

type InsetBodies<'w, 's> = Query<
    'w,
    's,
    (
        &'static DisplayName,
        &'static Description,
        &'static KeyFacts,
        &'static BodyClass,
        &'static Ownership,
        &'static Knowledge,
        Option<&'static Children>,
    ),
>;

pub struct InspectPlugin;

/// The Focus button's text colour: neutral grey while the body is in
/// exposure-ring mode, when it can't be a frame (CAM-24).
pub fn focus_button_color(ring_mode: bool) -> Color {
    if ring_mode {
        colors::ORBIT_LINE
    } else {
        colors::OWN
    }
}

/// CAM-24: the Focus button is disabled (dimmed, and the camera ignores its
/// click) while its body is in exposure-ring mode.
fn dim_focus_buttons(
    modes: Query<&MotionModeState>,
    mut buttons: Query<(&FocusButton, &mut TextColor)>,
) {
    for (button, mut color) in &mut buttons {
        let ring = modes.get(button.0).is_ok_and(|mode| mode.ring_mode);
        color.0 = focus_button_color(ring);
    }
}

impl Plugin for InspectPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_dock.in_set(Boot::Overlay))
            .add_systems(
                Update,
                (
                    handle_escape,
                    rebuild_inset,
                    update_position_notes,
                    dim_focus_buttons,
                    scroll_inset,
                )
                    .chain()
                    .in_set(Stage::Overlay),
            );
    }
}

/// INS-07: the dock at the right edge of the map area,
/// `inset_dock_margin_px` below the top bar and from the right edge,
/// `inset_width_px` wide and as tall as the map area allows. The panel in
/// it never moves; content taller than the dock scrolls.
fn spawn_dock(mut commands: Commands, tunables: Res<Tunables>) {
    let margin = tunables.inset_dock_margin_px;
    commands
        .spawn((
            InsetDock,
            Node {
                position_type: PositionType::Absolute,
                top: px(TOP_BAR_HEIGHT_PX + margin),
                right: px(margin),
                bottom: px(margin),
                width: px(tunables.inset_width_px),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                display: Display::None,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .with_children(|dock| {
            dock.spawn((
                InsetPanel,
                // CAM-10: a press on the inset doesn't start a map drag.
                MapInputBlocker,
                Node {
                    max_height: percent(100),
                    padding: UiRect::all(px(PANEL_PADDING_PX)),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(PANEL_ROW_GAP_PX),
                    border: UiRect::all(px(1)),
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
                ScrollPosition::default(),
                BackgroundColor(colors::PANEL),
                BorderColor::all(colors::PANEL_BORDER),
            ));
        });
}

/// INS-03: Esc closes the inset.
fn handle_escape(
    keys: Res<ButtonInput<KeyCode>>,
    bindings: Res<Bindings>,
    mut selection: ResMut<Selection>,
) {
    if pressed(&keys, &bindings.close_inset) {
        selection.clear();
    }
}

/// The inset's nodes.
type InsetNodes<'w, 's> = (
    Query<'w, 's, &'static mut Node, With<InsetDock>>,
    Query<'w, 's, (Entity, &'static mut ScrollPosition), With<InsetPanel>>,
);

/// INS-01 / INS-03: whenever the selection changes, the inset's contents
/// are replaced in place, scrolled back to the top; with nothing selected
/// the dock is hidden.
#[allow(clippy::too_many_arguments)]
fn rebuild_inset(
    selection: Res<Selection>,
    mut commands: Commands,
    fonts: Res<Fonts>,
    fidelity: Res<FidelityImages>,
    bodies: InsetBodies,
    satellites: Satellites,
    images: Res<MarkerImages>,
    (mut docks, mut panels): InsetNodes,
) {
    if !selection.is_changed() {
        return;
    }
    let content = selection
        .selected
        .and_then(|body| gather_content(body, &bodies, &satellites));
    for mut dock in &mut docks {
        dock.display = if content.is_some() {
            Display::Flex
        } else {
            Display::None
        };
    }
    let Ok((panel, mut scroll)) = panels.single_mut() else {
        return;
    };
    *scroll = ScrollPosition::default();
    let mut panel = commands.entity(panel);
    panel.despawn_related::<Children>();
    if let Some(content) = content {
        panel.with_children(|panel| fill_panel(panel, (&fonts, &fidelity, &images), &content));
    }
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

fn gather_content(
    body: Entity,
    bodies: &InsetBodies,
    satellites: &Satellites,
) -> Option<InsetContent> {
    let (name, description, facts, class, ownership, knowledge, children) =
        bodies.get(body).ok()?;
    let (status, tone, image) = knowledge_status(knowledge);
    let satellites = orbit_entries(children, satellites);
    Some(InsetContent {
        body,
        name: name.0.clone(),
        class_line: format!("{} · {}", class_label(*class), ownership_label(*ownership)),
        status,
        tone,
        image,
        description: description.0.clone(),
        facts: facts.0.clone(),
        satellites,
    })
}

/// INS-02: header, class line, image, status, description and facts, the
/// position note (MOT-07), the satellite list (INS-05), and the disabled
/// planetary-view button.
fn fill_panel(
    panel: &mut ChildSpawnerCommands,
    (fonts, fidelity, images): (&Fonts, &FidelityImages, &MarkerImages),
    content: &InsetContent,
) {
    spawn_header(panel, fonts, content);
    spawn_text(
        panel,
        &content.class_line,
        fonts.label(BODY_FONT_PX),
        colors::PANEL_BORDER,
    );
    spawn_image(panel, fidelity, content.image);
    spawn_text(
        panel,
        &content.status,
        fonts.mono(STATUS_FONT_PX),
        content.tone.color(),
    );
    spawn_text(
        panel,
        &content.description,
        fonts.label(BODY_FONT_PX),
        colors::NEUTRAL,
    );
    for (key, value) in &content.facts {
        let line = format!("{key}: {value}");
        spawn_text(panel, &line, fonts.mono(BODY_FONT_PX), colors::NEUTRAL);
    }
    spawn_position_note(panel, fonts, content.body);
    spawn_in_orbit_list(panel, fonts, images, &content.satellites);
    spawn_planetary_view_button(panel, fonts);
}

/// MOT-07: the line that stands in for a position while the body is an
/// exposure ring. Shown and hidden by `update_position_notes`.
fn spawn_position_note(panel: &mut ChildSpawnerCommands, fonts: &Fonts, body: Entity) {
    panel.spawn((
        PositionNote(body),
        Node {
            display: Display::None,
            ..default()
        },
        Text::new(POSITION_SHOWN_WHEN_PAUSED),
        fonts.mono(BODY_FONT_PX),
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
fn spawn_header(panel: &mut ChildSpawnerCommands, fonts: &Fonts, content: &InsetContent) {
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
                fonts.label_semibold(HEADER_FONT_PX),
                TextColor(colors::NEUTRAL),
            ));
            header
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: px(8),
                    ..default()
                })
                .with_children(|actions| spawn_actions(actions, fonts, content.body));
        });
}

fn spawn_actions(actions: &mut ChildSpawnerCommands, fonts: &Fonts, body: Entity) {
    let font = || fonts.label(BODY_FONT_PX);
    actions
        .spawn((
            FocusButton(body),
            Text::new("Focus"),
            font(),
            TextColor(colors::OWN),
        ))
        // ZOOM-16: the inset's Focus button makes the inset's object the
        // frame and moves to it, as a double-click does.
        .observe(
            move |_click: On<PointerClick>,
                  mut selection: ResMut<Selection>,
                  mut request: ResMut<FrameRequest>| {
                selection.select(body);
                request.0 = Some(body);
            },
        );
    // INS-03: Close clears the selection and closes the inset.
    actions
        .spawn((Text::new("Close"), font(), TextColor(colors::FOREIGN)))
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
        InsetImage::Stale => fidelity.stale.clone(),
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
        BorderColor::all(colors::FOREIGN),
    ));
}

/// INS-02: "Open planetary view", disabled until that view exists.
fn spawn_planetary_view_button(panel: &mut ChildSpawnerCommands, fonts: &Fonts) {
    panel
        .spawn((
            Node {
                padding: UiRect::all(px(6)),
                border: UiRect::all(px(1)),
                ..default()
            },
            BorderColor::all(colors::PANEL_BORDER.with_alpha(DISABLED_BORDER_ALPHA)),
        ))
        .with_children(|button| {
            spawn_text(
                button,
                "Open planetary view",
                fonts.label(BODY_FONT_PX),
                colors::PANEL_BORDER.with_alpha(DISABLED_ALPHA),
            );
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// INS-07: the inset is docked at the right of the map area, the
    /// margin below the top bar and from the edge, `inset_width_px` wide;
    /// it is placed once and nothing moves it afterwards.
    #[test]
    fn the_inset_is_docked_at_the_right() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = World::new();
        world.init_resource::<Tunables>();
        world.run_system_once(spawn_dock).unwrap();
        let mut docks = world.query_filtered::<&Node, With<InsetDock>>();
        let dock = docks.single(&world).unwrap();
        assert_eq!(dock.top, px(TOP_BAR_HEIGHT_PX + 12.0));
        assert_eq!((dock.right, dock.bottom), (px(12.0), px(12.0)));
        assert_eq!(dock.width, px(320.0));
        assert_eq!(dock.position_type, PositionType::Absolute);
        let mut panels = world.query_filtered::<&Node, With<InsetPanel>>();
        let panel = panels.single(&world).unwrap();
        assert_eq!(panel.overflow, Overflow::scroll_y(), "long content scrolls");
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

    /// CAM-24: the Focus button looks disabled only in ring mode.
    #[test]
    fn focus_button_is_dimmed_in_ring_mode() {
        assert_eq!(focus_button_color(false), colors::OWN);
        assert_ne!(focus_button_color(true), colors::OWN);
    }

    /// MOT-07: the inset says "Position: shown when paused" while its body
    /// is an exposure ring, and hides the line once it is a dot again.
    #[test]
    fn position_note_shows_only_in_ring_mode() {
        let mut world = World::new();
        let body = world
            .spawn(MotionModeState {
                ring_mode: true,
                crossfade_t: 1.0,
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

    /// INS-02: classes read as words, not enum names.
    #[test]
    fn class_names_are_readable() {
        assert_eq!(class_label(BodyClass::SalvageSite), "Salvage site");
        assert_eq!(class_label(BodyClass::Planet), "Planet");
        assert!(
            class_label(BodyClass::Ship)
                .chars()
                .all(|c| c.is_alphabetic() || c == ' ')
        );
    }

    /// INS-04: surveyed is a clean render with the survey date.
    #[test]
    fn surveyed_shows_the_date() {
        let (status, tone, image) = knowledge_status(&Knowledge::Surveyed { imaged_day: 0.0 });
        assert_eq!(status, "SURVEYED · 14 Mar 2341");
        assert_eq!(tone, StatusTone::Surveyed);
        assert_eq!(image, InsetImage::Surveyed);
    }

    /// INS-04: unsurveyed is a telescope frame, in orange.
    #[test]
    fn unsurveyed_is_a_telescope_frame_in_orange() {
        let (status, tone, image) = knowledge_status(&Knowledge::Unsurveyed);
        assert_eq!(status, "TELESCOPE FRAME · UNSURVEYED");
        assert_eq!(tone.color(), colors::FOREIGN);
        assert_eq!(image, InsetImage::Telescope);
    }

    /// INS-04: stale is a sepia image with the last-imaged line (which
    /// gives the time since, not the date: F4-10).
    #[test]
    fn stale_is_sepia_and_last_imaged() {
        let (status, tone, image) = knowledge_status(&Knowledge::Stale {
            last_imaged_day: -1_006.0,
        });
        assert_eq!(status, "LAST IMAGED · 2 Y 9 M AGO");
        assert_eq!(tone, StatusTone::Stale);
        assert_eq!(image, InsetImage::Stale);
    }
}
