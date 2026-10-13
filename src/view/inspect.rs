//! Inspection insets (INS-01 to INS-04, MOT-07): one panel per open
//! selection (INS-01, INS-03), anchored to its marker by a leader line and
//! rebuilt whenever the selection changes, showing the contents INS-02 and
//! INS-04 list.
//!
//! The panel is built from an `InsetContent` - plain values gathered from
//! the body - by one helper per section, so what an inset says can be
//! tested apart from how it is laid out.
//!
//! Simplifications: the leader "line" is a short
//! straight connector rather than a drawn diagonal; the rest of the map
//! isn't dimmed; Esc closes the unpinned inset (INS-03), and clicking empty
//! space does nothing.

use bevy::prelude::*;

use super::anchors::BodyAnchors;
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
use super::targets::MapInputBlocker;
use crate::Stage;
use crate::sim::body::{BodyClass, Description, DisplayName, KeyFacts, Knowledge, Ownership};
use crate::sim::orbit::Orbit;

#[derive(Component)]
struct InspectPanelFor {
    body: Entity,
}
#[derive(Component)]
struct LeaderFor(Entity);
/// The inset's Focus button, for the body it would make the frame
/// (CAM-24).
#[derive(Component)]
struct FocusButton(Entity);
/// The inset line MOT-07 asks for, for the body it describes.
#[derive(Component)]
struct PositionNote(Entity);

/// MOT-07: what an inset says for the position of a body in ring mode.
const POSITION_SHOWN_WHEN_PAUSED: &str = "Position: shown when paused";

const PANEL_WIDTH: f32 = 240.0;
const PANEL_PADDING_PX: f32 = 10.0;
const PANEL_ROW_GAP_PX: f32 = 5.0;
/// The panel sits this far above its marker, so the leader meets it near
/// the header rather than at its top edge.
const PANEL_RAISE_PX: f32 = 60.0;
const LEADER_LENGTH: f32 = 28.0;
const LEADER_THICKNESS_PX: f32 = 2.0;
/// A pinned inset is drawn quieter than the live one (INS-03), so the
/// current selection reads first.
const PINNED_LEADER_ALPHA: f32 = 0.4;
const LIVE_LEADER_ALPHA: f32 = 0.8;
const PINNED_BORDER_ALPHA: f32 = 0.5;
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
    pinned: bool,
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
        app.add_systems(
            Update,
            (
                handle_escape,
                rebuild_panels,
                update_position_notes,
                dim_focus_buttons,
                reposition_panels,
            )
                .chain()
                .in_set(Stage::Overlay),
        );
    }
}

/// INS-03: Esc closes the unpinned inset.
fn handle_escape(
    keys: Res<ButtonInput<KeyCode>>,
    bindings: Res<Bindings>,
    mut selection: ResMut<Selection>,
) {
    if pressed(&keys, &bindings.close_inset) {
        selection.close_unpinned();
    }
}

/// INS-01 / INS-03: one inset per open selection, pinned ones first.
#[allow(clippy::too_many_arguments)]
fn rebuild_panels(
    selection: Res<Selection>,
    mut commands: Commands,
    fonts: Res<Fonts>,
    fidelity: Res<FidelityImages>,
    existing_panels: Query<Entity, With<InspectPanelFor>>,
    existing_leaders: Query<Entity, With<LeaderFor>>,
    bodies: InsetBodies,
    satellites: Satellites,
    images: Res<MarkerImages>,
) {
    if !selection.is_changed() {
        return;
    }
    for entity in existing_panels.iter().chain(&existing_leaders) {
        commands.entity(entity).despawn();
    }

    let pinned = selection.pinned.iter().map(|&body| (body, true));
    let primary = selection.primary.map(|body| (body, false));
    for (body, pinned) in pinned.chain(primary) {
        let Some(content) = gather_content(body, pinned, &bodies, &satellites) else {
            continue;
        };
        spawn_panel(&mut commands, (&fonts, &fidelity, &images), &content);
        spawn_leader(&mut commands, body, pinned);
    }
}

fn gather_content(
    body: Entity,
    pinned: bool,
    bodies: &InsetBodies,
    satellites: &Satellites,
) -> Option<InsetContent> {
    let (name, description, facts, class, ownership, knowledge, children) =
        bodies.get(body).ok()?;
    let (status, tone, image) = knowledge_status(knowledge);
    let satellites = orbit_entries(children, satellites);
    Some(InsetContent {
        body,
        pinned,
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

fn spawn_leader(commands: &mut Commands, body: Entity, pinned: bool) {
    let alpha = if pinned {
        PINNED_LEADER_ALPHA
    } else {
        LIVE_LEADER_ALPHA
    };
    commands.spawn((
        LeaderFor(body),
        Node {
            position_type: PositionType::Absolute,
            width: px(LEADER_LENGTH),
            height: px(LEADER_THICKNESS_PX),
            display: Display::None,
            ..default()
        },
        BackgroundColor(colors::PANEL_BORDER.with_alpha(alpha)),
    ));
}

/// INS-02: header, class line, image, status, description and facts, the
/// satellite chips, and the disabled planetary-view button.
fn spawn_panel(
    commands: &mut Commands,
    (fonts, fidelity, images): (&Fonts, &FidelityImages, &MarkerImages),
    content: &InsetContent,
) {
    let border = if content.pinned {
        colors::PANEL_BORDER.with_alpha(PINNED_BORDER_ALPHA)
    } else {
        colors::PANEL_BORDER
    };
    commands
        .spawn((
            InspectPanelFor { body: content.body },
            // CAM-10: a press on an inset doesn't start a map drag.
            MapInputBlocker,
            Node {
                position_type: PositionType::Absolute,
                width: px(PANEL_WIDTH),
                padding: UiRect::all(px(PANEL_PADDING_PX)),
                flex_direction: FlexDirection::Column,
                row_gap: px(PANEL_ROW_GAP_PX),
                border: UiRect::all(px(1)),
                display: Display::None,
                ..default()
            },
            BackgroundColor(colors::PANEL),
            BorderColor::all(border),
        ))
        .with_children(|panel| {
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
        });
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

/// INS-02: the name, and the Focus, Pin and Close buttons.
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
                .with_children(|actions| {
                    spawn_actions(actions, fonts, content.body, content.pinned)
                });
        });
}

fn spawn_actions(actions: &mut ChildSpawnerCommands, fonts: &Fonts, body: Entity, pinned: bool) {
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
    // INS-03: Pin keeps the inset open through a new selection.
    actions
        .spawn((
            Text::new(if pinned { "Unpin" } else { "Pin" }),
            font(),
            TextColor(colors::NEUTRAL),
        ))
        .observe(
            move |_click: On<PointerClick>, mut selection: ResMut<Selection>| {
                if pinned {
                    selection.unpin(body);
                } else {
                    selection.pin(body);
                }
            },
        );
    actions
        .spawn((Text::new("Close"), font(), TextColor(colors::FOREIGN)))
        .observe(
            move |_click: On<PointerClick>, mut selection: ResMut<Selection>| {
                selection.close(body);
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

/// INS-01 / INS-06: insets and their leaders follow their bodies' anchors -
/// the marker, or the parent's pill while the body is folded.
fn reposition_panels(
    anchors: Res<BodyAnchors>,
    mut panels: Query<(&InspectPanelFor, &mut Node)>,
    mut leaders: Query<(&LeaderFor, &mut Node), Without<InspectPanelFor>>,
) {
    for (panel_for, mut node) in &mut panels {
        let offset = Vec2::new(LEADER_LENGTH, -PANEL_RAISE_PX);
        place(&mut node, anchors.get(panel_for.body), offset);
    }
    for (leader_for, mut node) in &mut leaders {
        let offset = Vec2::new(0.0, -LEADER_THICKNESS_PX / 2.0);
        place(&mut node, anchors.get(leader_for.0), offset);
    }
}

fn place(node: &mut Node, anchor: Option<Vec2>, offset: Vec2) {
    let Some(anchor) = anchor else {
        node.display = Display::None;
        return;
    };
    node.display = Display::Flex;
    node.left = px(anchor.x + offset.x);
    node.top = px(anchor.y + offset.y);
}

#[cfg(test)]
mod tests {
    use super::*;

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
