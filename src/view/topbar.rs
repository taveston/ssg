//! The top bar and the clock controls (CLK-02, CLK-05): breadcrumb, date,
//! the Pause, 1 d/s and 5 d/s buttons with the active rate highlighted, and
//! the clock keys (CLK-05: Space, `.` and `,`). Every change goes through the clock's
//! own API (ARCH-14).
//!
//! The FPS readout is not specced (F4-12) and stays until triaged.

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;

use crate::sim::clock::GameClock;
use crate::view::bindings::{Bindings, pressed};
use crate::view::colors;
use crate::view::date::{date_from_elapsed_days, format_date};
use crate::view::fonts::Fonts;
use crate::view::targets::MapInputBlocker;

#[derive(Component)]
struct DateText;

#[derive(Component)]
struct FpsText;

#[derive(Component)]
struct PauseButton;

#[derive(Component)]
struct SpeedButton(f64);

pub struct TopBarPlugin;

impl Plugin for TopBarPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_top_bar).add_systems(
            Update,
            (
                handle_clock_keys,
                update_date_text,
                update_button_styles,
                update_fps_text,
            ),
        );
    }
}

const BAR_FONT_PX: f32 = 14.0;
/// CLK-02: the active rate's highlight.
const ACTIVE_ALPHA: f32 = 0.3;
/// A rate button is active when the clock runs within this of its rate.
const RATE_MATCH_TOLERANCE: f64 = 0.01;
const BUTTON_FONT_PX: f32 = 13.0;

/// A top-bar button: an outlined box with a label, highlighted by
/// `update_button_styles` when active.
fn spawn_button<'a>(
    bar: &'a mut ChildSpawnerCommands,
    fonts: &Fonts,
    label: &str,
    marker: impl Bundle,
) -> EntityCommands<'a> {
    let mut button = bar.spawn((
        marker,
        Node {
            padding: UiRect::axes(px(12), px(5)),
            border: UiRect::all(px(1)),
            margin: UiRect::left(px(6)),
            ..default()
        },
        BackgroundColor(Color::NONE),
        BorderColor::all(colors::PANEL_BORDER),
    ));
    button.with_children(|b| {
        b.spawn((
            Text::new(label),
            fonts.label(BUTTON_FONT_PX),
            TextColor(colors::NEUTRAL),
        ));
    });
    button
}

/// CLK-02: breadcrumb, date, Pause, 1 d/s and 5 d/s.
fn spawn_top_bar(mut commands: Commands, fonts: Res<Fonts>) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(0),
                left: px(0),
                width: percent(100),
                padding: UiRect::axes(px(14), px(8)),
                align_items: AlignItems::Center,
                column_gap: px(14),
                ..default()
            },
            BackgroundColor(colors::PANEL),
            // CAM-10: a press on the bar doesn't start a map drag.
            MapInputBlocker,
        ))
        .with_children(|bar| {
            bar.spawn((
                Text::new("Cluster / Corvane system"),
                fonts.label(BAR_FONT_PX),
                TextColor(colors::NEUTRAL),
            ));
            bar.spawn((
                DateText,
                Text::new(""),
                fonts.mono(BAR_FONT_PX),
                TextColor(colors::NEUTRAL),
            ));
            spawn_button(bar, &fonts, "Pause", PauseButton).observe(on_pause_clicked);
            for (label, speed) in [("1 d/s", 1.0_f64), ("5 d/s", 5.0_f64)] {
                spawn_button(bar, &fonts, label, SpeedButton(speed)).observe(
                    move |_click: On<PointerClick>, mut clock: ResMut<GameClock>| {
                        clock.run_at(speed);
                    },
                );
            }
            // Not specced (F4-12), kept until triaged.
            bar.spawn((
                FpsText,
                Text::new(""),
                fonts.mono(BUTTON_FONT_PX),
                TextColor(colors::PANEL_BORDER),
            ));
        });
}

/// CLK-05: Space pauses and resumes, `.` doubles the rate and `,` halves it
/// (down to 1/16 d/s), through the bindings (CAM-18).
fn handle_clock_keys(
    keys: Res<ButtonInput<KeyCode>>,
    bindings: Res<Bindings>,
    mut clock: ResMut<GameClock>,
) {
    if pressed(&keys, &bindings.pause) {
        clock.toggle_pause();
    }
    if pressed(&keys, &bindings.rate_up) {
        clock.double_rate();
    }
    if pressed(&keys, &bindings.rate_down) {
        clock.halve_rate();
    }
}

fn on_pause_clicked(_click: On<PointerClick>, mut clock: ResMut<GameClock>) {
    clock.toggle_pause();
}

fn update_date_text(clock: Res<GameClock>, mut query: Query<&mut Text, With<DateText>>) {
    for mut text in &mut query {
        let date = date_from_elapsed_days(clock.elapsed_days());
        *text = Text::new(format_date(&date));
    }
}

fn update_fps_text(diagnostics: Res<DiagnosticsStore>, mut query: Query<&mut Text, With<FpsText>>) {
    let Some(fps) = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
    else {
        return;
    };
    for mut text in &mut query {
        *text = Text::new(format!("{fps:.0} fps"));
    }
}

fn update_button_styles(
    clock: Res<GameClock>,
    mut pause_buttons: Query<&mut BackgroundColor, (With<PauseButton>, Without<SpeedButton>)>,
    mut speed_buttons: Query<(&SpeedButton, &mut BackgroundColor), Without<PauseButton>>,
) {
    for mut background in &mut pause_buttons {
        background.0 = if clock.is_paused() {
            colors::OWN.with_alpha(ACTIVE_ALPHA)
        } else {
            Color::NONE
        };
    }
    for (speed_button, mut background) in &mut speed_buttons {
        let active = !clock.is_paused()
            && (clock.rate_days_per_sec() - speed_button.0).abs() < RATE_MATCH_TOLERANCE;
        background.0 = if active {
            colors::OWN.with_alpha(ACTIVE_ALPHA)
        } else {
            Color::NONE
        };
    }
}
