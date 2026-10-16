//! The planner panel's nodes: its root, and one helper per section of
//! each shape. The texts are left empty; `write.rs` fills them.

use bevy::prelude::*;

use super::super::input;
use super::super::state::{PlannerState, SliderKind};
use super::super::tooltip::{Stretches, Tooltip};
use super::super::words::{
    CANCEL, CHANGE, CHEAPEST, CHOOSE_PROMPT, DELTA_V, DELTA_V_TOOLTIP, FASTEST, HEADING, HIGH,
    INTERCEPT, INTERCEPT_TOOLTIP, LOW, NEXT_WINDOW, RENDEZVOUS, RENDEZVOUS_TOOLTIP, SCHEDULE,
};
use super::{
    Choice, DeltaFill, Field, NoticeLine, PlannerRoot, PresetButton, ScheduleButton, Segment,
    Shape, Thumb, Track, segment_look,
};
use crate::sim::transfer::places::Band;
use crate::sim::transfer::plan::{Plan, ShipOrder};
use crate::sim::transfer::presets::Preset;
use crate::view::buttons::{ButtonStyle, ConsoleButton, spawn_button};
use crate::view::colors;
use crate::view::fonts::Fonts;
use crate::view::inset::inspect::{InsetPanel, spawn_spaced_capitals};
use crate::view::tunables::Tunables;

/// The panel's root and the notice line, put into the sidebar once it
/// exists: the root after the details, the notice at the very top.
pub fn spawn_root(
    mut commands: Commands,
    panels: Query<Entity, With<InsetPanel>>,
    roots: Query<(), With<PlannerRoot>>,
    fonts: Res<Fonts>,
    tunables: Res<Tunables>,
) {
    let Ok(panel) = panels.single() else {
        return;
    };
    if !roots.is_empty() {
        return;
    }
    let root = commands
        .spawn((
            PlannerRoot,
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(ROW_GAP_PX),
                ..default()
            },
        ))
        .id();
    let notice = commands
        .spawn((
            NoticeLine,
            Node {
                display: Display::None,
                ..default()
            },
            Text::new(""),
            fonts.label(tunables.sidebar_text_px),
            TextColor(colors::SECONDARY),
        ))
        .id();
    commands.entity(panel).insert_children(0, &[notice]);
    commands.entity(panel).insert_children(2, &[root]);
}

const ROW_GAP_PX: f32 = 6.0;
/// The space between buttons in a row.
const BUTTON_GAP_PX: f32 = 6.0;
/// Room above and below a track for the taller thumb.
fn track_margin_px(tunables: &Tunables) -> f32 {
    ((tunables.slider_thumb_h_px - tunables.slider_track_px) / 2.0).max(0.0)
}

/// The fonts and tunables the panel is built from.
pub(super) type Assets<'a> = (&'a Fonts, &'a Tunables);

/// Lays out a view's shape; the texts are written afterwards.
pub(super) fn spawn_view(panel: &mut ChildSpawnerCommands, assets: Assets, shape: &Shape) {
    match shape {
        Shape::Empty => {}
        Shape::Choosing => spawn_choosing(panel, assets),
        Shape::Tuning { .. } => spawn_tuning(panel, assets, shape),
        Shape::Order { ship, scheduled } => spawn_order(panel, assets, *ship, *scheduled),
        Shape::Coasting => {
            spawn_field(panel, assets, Field::Coasting, false);
        }
    }
}

/// The section's hairline and its heading, "TRANSFER" (PLNR-20).
fn spawn_heading(panel: &mut ChildSpawnerCommands, (fonts, tunables): Assets) {
    panel.spawn((
        Node {
            width: percent(100),
            height: px(1),
            margin: UiRect::vertical(px(4)),
            ..default()
        },
        BackgroundColor(colors::PANEL_BORDER),
    ));
    spawn_spaced_capitals(panel, (fonts, tunables), HEADING, ());
}

/// A text written each frame: data face for figures, label face else.
fn spawn_field(
    panel: &mut ChildSpawnerCommands,
    (fonts, tunables): Assets,
    field: Field,
    data: bool,
) -> Entity {
    let font = if data {
        fonts.data(tunables.sidebar_text_px)
    } else {
        fonts.label(tunables.sidebar_text_px)
    };
    panel
        .spawn((field, Text::new(""), font, TextColor(colors::NEUTRAL)))
        .id()
}

/// A label on the left and a figure on the right.
fn spawn_row(
    panel: &mut ChildSpawnerCommands,
    assets: Assets,
    label: Result<&'static str, Field>,
    value: Field,
    tooltip: Option<&'static str>,
) {
    // Only a fixed label carries a tooltip (PLNR-91).
    let (fonts, tunables) = assets;
    panel
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Baseline,
            ..default()
        })
        .with_children(|row| {
            match label {
                Ok(text) => {
                    let mut label = row.spawn((
                        Text::new(text),
                        fonts.label(tunables.sidebar_text_px),
                        TextColor(colors::SECONDARY),
                    ));
                    if let Some(tip) = tooltip {
                        label.insert(Tooltip(tip));
                    }
                }
                Err(field) => {
                    spawn_field(row, assets, field, false);
                }
            }
            spawn_field(row, assets, value, true);
        });
}

/// PLNR-10 / PLNR-12: the prompt, the hovered place's line and Cancel.
fn spawn_choosing(panel: &mut ChildSpawnerCommands, assets: Assets) {
    let (fonts, tunables) = assets;
    spawn_heading(panel, assets);
    panel.spawn((
        Text::new(CHOOSE_PROMPT),
        fonts.label(tunables.sidebar_text_px),
        TextColor(colors::NEUTRAL),
    ));
    spawn_field(panel, assets, Field::Hover, false);
    spawn_button_row(panel, |row| spawn_quiet_cancel(row, assets));
}

/// PLNR-02: the quiet Cancel that ends the planner.
fn spawn_quiet_cancel(row: &mut ChildSpawnerCommands, (fonts, tunables): Assets) {
    spawn_button(
        row,
        fonts,
        CANCEL,
        tunables.sidebar_text_px,
        ConsoleButton::new(ButtonStyle::Quiet),
    )
    .observe(input::on_planner_cancel);
}

fn spawn_button_row(
    panel: &mut ChildSpawnerCommands,
    buttons: impl FnOnce(&mut ChildSpawnerCommands),
) {
    panel
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(BUTTON_GAP_PX),
            row_gap: px(BUTTON_GAP_PX),
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(buttons);
}

/// PLNR-20: the tuning panel, top to bottom.
fn spawn_tuning(panel: &mut ChildSpawnerCommands, assets: Assets, shape: &Shape) {
    let Shape::Tuning {
        bands,
        depart,
        travel,
        travel_note,
        gap,
        arrival,
        speed,
        delta_v,
        presets,
        arrival_choice,
    } = shape
    else {
        return;
    };
    spawn_heading(panel, assets);
    spawn_field(panel, assets, Field::Route, false);
    spawn_field(panel, assets, Field::Place, false);
    if *bands {
        spawn_band_choice(panel, assets);
    }
    if let Some(track) = depart {
        spawn_depart(panel, assets, *track);
    }
    if let Some(track) = travel {
        spawn_travel(panel, assets, *track, *travel_note);
    }
    if *gap {
        spawn_field(panel, assets, Field::Gap, false);
    }
    if *arrival {
        spawn_row(
            panel,
            assets,
            Err(Field::ArrivalLabel),
            Field::ArrivalValue,
            None,
        );
    }
    if *speed {
        spawn_row(
            panel,
            assets,
            Err(Field::SpeedLabel),
            Field::SpeedValue,
            None,
        );
    }
    if *delta_v {
        spawn_delta_v(panel, assets);
    }
    spawn_presets(panel, assets, presets);
    if *arrival_choice {
        spawn_arrival_choice(panel, assets);
    }
    spawn_schedule(panel, assets);
}

/// PLNR-14: the High / Low band choice.
fn spawn_band_choice(panel: &mut ChildSpawnerCommands, assets: Assets) {
    spawn_choice(
        panel,
        assets,
        [
            (Choice::Band(Band::High), HIGH, None),
            (Choice::Band(Band::Low), LOW, None),
        ],
    );
}

/// PLNR-21 / PLNR-23: the Depart row and its track.
fn spawn_depart(panel: &mut ChildSpawnerCommands, assets: Assets, track: bool) {
    spawn_row(
        panel,
        assets,
        Err(Field::DepartLabel),
        Field::DepartValue,
        None,
    );
    if track {
        spawn_track(panel, assets, SliderKind::Depart);
    }
}

/// PLNR-22 / PLNR-23: the Travel time row, its track if it has one, and
/// its note if it has one.
fn spawn_travel(panel: &mut ChildSpawnerCommands, assets: Assets, track: bool, note: bool) {
    spawn_row(
        panel,
        assets,
        Ok(super::super::words::TRAVEL_TIME),
        Field::TravelValue,
        None,
    );
    if track {
        spawn_track(panel, assets, SliderKind::Travel);
    }
    if note {
        spawn_field(panel, assets, Field::TravelNote, false);
    }
}

/// PLNR-30: the Rendezvous / Intercept choice, each with its tooltip.
fn spawn_arrival_choice(panel: &mut ChildSpawnerCommands, assets: Assets) {
    spawn_choice(
        panel,
        assets,
        [
            (
                Choice::Arrival { intercept: false },
                RENDEZVOUS,
                Some(RENDEZVOUS_TOOLTIP),
            ),
            (
                Choice::Arrival { intercept: true },
                INTERCEPT,
                Some(INTERCEPT_TOOLTIP),
            ),
        ],
    );
}

/// PLNR-14 / PLNR-30: a two-way choice, the first option first.
fn spawn_choice(
    panel: &mut ChildSpawnerCommands,
    (fonts, tunables): Assets,
    options: [(Choice, &'static str, Option<&'static str>); 2],
) {
    spawn_button_row(panel, |row| {
        for (choice, label, tooltip) in options {
            let look = segment_look(false, tunables);
            let mut option = row.spawn((
                choice,
                Segment { selected: false },
                Node {
                    padding: UiRect::axes(px(12), px(5)),
                    border: UiRect::all(px(tunables.button_stroke_px)),
                    ..default()
                },
                BackgroundColor(look.background),
                BorderColor::all(look.border),
            ));
            option.with_children(|b| {
                b.spawn((
                    Text::new(label),
                    fonts.label(tunables.sidebar_text_px),
                    TextColor(look.text),
                ));
            });
            if let Some(text) = tooltip {
                option.insert(Tooltip(text));
            }
            option.observe(
                move |click: On<PointerClick>, mut state: ResMut<PlannerState>| {
                    if click.button == PointerButton::Primary {
                        input::choose(&mut state, choice);
                    }
                },
            );
        }
    });
}

/// PLNR-23 / PLNR-28 / PLNR-35: a slider's track, with its thumb, and
/// the tooltips of its rejected stretches.
fn spawn_track(panel: &mut ChildSpawnerCommands, (_, tunables): Assets, kind: SliderKind) {
    let margin = track_margin_px(tunables);
    panel
        .spawn((
            Track(kind),
            Node {
                width: percent(100),
                height: px(tunables.slider_track_px),
                margin: UiRect::vertical(px(margin)),
                ..default()
            },
            BackgroundGradient(vec![]),
            Stretches::default(),
        ))
        .with_children(|track| {
            track.spawn((
                Thumb(kind),
                Node {
                    position_type: PositionType::Absolute,
                    width: px(tunables.slider_thumb_w_px),
                    height: px(tunables.slider_thumb_h_px),
                    top: px(-margin),
                    left: percent(0),
                    margin: UiRect::left(px(-tunables.slider_thumb_w_px / 2.0)),
                    ..default()
                },
                BackgroundColor(colors::NEUTRAL),
            ));
        })
        .observe(
            move |press: On<PointerPress>, mut slide: input::SliderInput| {
                if press.button == PointerButton::Primary {
                    slide.focus(kind);
                    slide.slide_to(kind, press.pointer.position.x);
                }
            },
        )
        .observe(
            move |drag: On<PointerDrag>, mut slide: input::SliderInput| {
                if drag.button == PointerButton::Primary {
                    slide.slide_to(kind, drag.pointer.position.x);
                }
            },
        );
}

/// PLNR-27: the Delta-v row with its tooltip, the bar and the line.
fn spawn_delta_v(panel: &mut ChildSpawnerCommands, assets: Assets) {
    let (_, tunables) = assets;
    spawn_row(
        panel,
        assets,
        Ok(DELTA_V),
        Field::DeltaVValue,
        Some(DELTA_V_TOOLTIP),
    );
    panel
        .spawn((
            Node {
                width: percent(100),
                height: px(tunables.delta_bar_px),
                ..default()
            },
            BackgroundColor(colors::PANEL_BORDER.with_alpha(HAIRLINE_ALPHA)),
        ))
        .with_children(|bar| {
            bar.spawn((
                DeltaFill,
                Node {
                    width: percent(0),
                    height: percent(100),
                    ..default()
                },
                BackgroundColor(colors::OWN),
            ));
        });
    spawn_field(panel, assets, Field::DeltaVLine, false);
}

/// The Delta-v bar's empty part is the inset's hairline colour, faint so
/// the fill reads against it.
const HAIRLINE_ALPHA: f32 = 0.25;

/// PLNR-29: the presets, as secondary buttons in a row.
fn spawn_presets(panel: &mut ChildSpawnerCommands, (fonts, tunables): Assets, presets: &[Preset]) {
    if presets.is_empty() {
        return;
    }
    spawn_button_row(panel, |row| {
        for &preset in presets {
            let label = match preset {
                Preset::Fastest => FASTEST,
                Preset::Cheapest => CHEAPEST,
                Preset::NextWindow => NEXT_WINDOW,
            };
            spawn_button(
                row,
                fonts,
                label,
                tunables.sidebar_text_px,
                ConsoleButton::new(ButtonStyle::Secondary),
            )
            .insert(PresetButton(preset))
            .observe(
                move |click: On<PointerClick>, mut state: ResMut<PlannerState>| {
                    if click.button == PointerButton::Primary {
                        input::apply_preset(&mut state, preset);
                    }
                },
            );
        }
    });
}

/// PLNR-31 / PLNR-02: Schedule, the quiet Cancel, and the reason line.
fn spawn_schedule(panel: &mut ChildSpawnerCommands, assets: Assets) {
    let (fonts, tunables) = assets;
    spawn_button_row(panel, |row| {
        spawn_button(
            row,
            fonts,
            SCHEDULE,
            tunables.sidebar_text_px,
            ConsoleButton::new(ButtonStyle::Primary).disabled(),
        )
        .insert(ScheduleButton)
        .observe(input::on_schedule);
        spawn_quiet_cancel(row, assets);
    });
    spawn_field(panel, assets, Field::Reason, false);
}

/// PLNR-51: an order: its heading, Departs, the arrival row, its delta-v
/// and, while Scheduled, Cancel and Change.
fn spawn_order(panel: &mut ChildSpawnerCommands, assets: Assets, ship: Entity, scheduled: bool) {
    let (fonts, tunables) = assets;
    panel.spawn((
        Node {
            width: percent(100),
            height: px(1),
            margin: UiRect::vertical(px(4)),
            ..default()
        },
        BackgroundColor(colors::PANEL_BORDER),
    ));
    spawn_field(panel, assets, Field::OrderHeading, false);
    spawn_row(
        panel,
        assets,
        Err(Field::OrderDepartsLabel),
        Field::OrderDepartsValue,
        None,
    );
    spawn_row(
        panel,
        assets,
        Err(Field::OrderArrivalLabel),
        Field::OrderArrivalValue,
        None,
    );
    spawn_row(panel, assets, Ok(DELTA_V), Field::OrderDeltaV, None);
    if !scheduled {
        return;
    }
    spawn_button_row(panel, |row| {
        spawn_button(
            row,
            fonts,
            CANCEL,
            tunables.sidebar_text_px,
            ConsoleButton::new(ButtonStyle::Secondary),
        )
        .observe(
            move |click: On<PointerClick>, mut orders: MessageWriter<ShipOrder>| {
                if click.button == PointerButton::Primary {
                    orders.write(ShipOrder::Cancel { ship });
                }
            },
        );
        spawn_button(
            row,
            fonts,
            CHANGE,
            tunables.sidebar_text_px,
            ConsoleButton::new(ButtonStyle::Secondary),
        )
        .observe(
            move |click: On<PointerClick>, plans: Query<&Plan>, mut state: ResMut<PlannerState>| {
                if click.button == PointerButton::Primary
                    && let Ok(plan) = plans.get(ship)
                {
                    input::change_order(&mut state, ship, plan);
                }
            },
        );
    });
}
