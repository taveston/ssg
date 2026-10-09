//! Breadcrumb, date, and pause/speed buttons. Replaces iteration 1's debug
//! readout; the keyboard shortcuts (Space, +, -) still work alongside these.

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;

use crate::colors;
use crate::date::{date_from_elapsed_days, format_date};
use crate::fonts::Fonts;
use crate::time_control::GameClock;

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
        app.add_systems(Startup, spawn_top_bar)
            .add_systems(Update, (update_date_text, update_button_styles, update_fps_text));
    }
}

fn button_node() -> Node {
    Node {
        padding: UiRect::axes(px(12), px(5)),
        border: UiRect::all(px(1)),
        margin: UiRect::left(px(6)),
        ..default()
    }
}

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
        ))
        .with_children(|bar| {
            bar.spawn((
                Text::new("Cluster / Corvane system"),
                fonts.label(14.0),
                TextColor(colors::NEUTRAL),
            ));

            bar.spawn((DateText, Text::new(""), fonts.mono(14.0), TextColor(colors::NEUTRAL)));

            bar.spawn((
                PauseButton,
                button_node(),
                BackgroundColor(Color::NONE),
                BorderColor::all(colors::PANEL_BORDER),
            ))
            .with_children(|b| {
                b.spawn((Text::new("Pause"), fonts.label(13.0), TextColor(colors::NEUTRAL)));
            })
            .observe(on_pause_clicked);

            for (label, speed) in [("1 d/s", 1.0_f64), ("5 d/s", 5.0_f64)] {
                bar.spawn((
                    SpeedButton(speed),
                    button_node(),
                    BackgroundColor(Color::NONE),
                    BorderColor::all(colors::PANEL_BORDER),
                ))
                .with_children(|b| {
                    b.spawn((Text::new(label), fonts.label(13.0), TextColor(colors::NEUTRAL)));
                })
                .observe(move |_click: On<PointerClick>, mut clock: ResMut<GameClock>| {
                    clock.paused = false;
                    clock.speed_days_per_sec = speed;
                });
            }

            bar.spawn((FpsText, Text::new(""), fonts.mono(13.0), TextColor(colors::PANEL_BORDER)));
        });
}

fn on_pause_clicked(_click: On<PointerClick>, mut clock: ResMut<GameClock>) {
    clock.paused = !clock.paused;
}

fn update_date_text(clock: Res<GameClock>, mut query: Query<&mut Text, With<DateText>>) {
    for mut text in &mut query {
        let date = date_from_elapsed_days(clock.elapsed_days);
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
        background.0 = if clock.paused {
            colors::OWN.with_alpha(0.3)
        } else {
            Color::NONE
        };
    }
    for (speed_button, mut background) in &mut speed_buttons {
        let active = !clock.paused && (clock.speed_days_per_sec - speed_button.0).abs() < 0.01;
        background.0 = if active {
            colors::OWN.with_alpha(0.3)
        } else {
            Color::NONE
        };
    }
}
