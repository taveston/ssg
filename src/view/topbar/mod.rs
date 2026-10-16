//! The top bar and the clock controls (CLK-02, CLK-05): the breadcrumb's
//! slot (filled by `topbar/breadcrumb.rs`), the date, the Pause, 1 d/s and 5 d/s
//! buttons with the active rate highlighted, and the clock keys (CLK-05:
//! Space, `.` and `,`). Every change goes through the clock's own API
//! (ARCH-14). A debug frames-per-second readout sits at the end,
//! hidden until its binding toggles it (CLK-06).

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;

use super::layers;

use crate::sim::clock::GameClock;
use crate::view::bindings::{Bindings, pressed};
use crate::view::buttons::{ButtonStyle, ConsoleButton, spawn_button};
use crate::view::colors;
use crate::view::date::{date_from_elapsed_days, format_date};
use crate::view::fonts::Fonts;
use crate::view::targets::MapInputBlocker;
use crate::view::topbar::breadcrumb::spawn_breadcrumb_root;

pub mod breadcrumb;

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
                toggle_fps_readout,
            ),
        );
    }
}

/// The top bar's height. Edge arrows stay below it (EDGE-01).
pub const TOP_BAR_HEIGHT_PX: f32 = 54.0;
const BAR_FONT_PX: f32 = 14.0;
/// A rate button is active when the clock runs within this of its rate.
const RATE_MATCH_TOLERANCE: f64 = 0.01;
const BUTTON_FONT_PX: f32 = 13.0;

/// A top-bar button, secondary until `update_button_styles` makes the
/// active one primary (CLK-02).
fn spawn_bar_button<'a>(
    bar: &'a mut ChildSpawnerCommands,
    fonts: &Fonts,
    label: &str,
    marker: impl Bundle,
) -> EntityCommands<'a> {
    let mut button = spawn_button(
        bar,
        fonts,
        label,
        BUTTON_FONT_PX,
        ConsoleButton::new(ButtonStyle::Secondary),
    );
    button.insert(marker);
    button
}

/// CLK-02: breadcrumb, date, Pause, 1 d/s and 5 d/s.
fn spawn_top_bar(mut commands: Commands, fonts: Res<Fonts>) {
    commands
        .spawn((
            GlobalZIndex(layers::PANELS),
            Node {
                position_type: PositionType::Absolute,
                top: px(0),
                left: px(0),
                width: percent(100),
                height: px(TOP_BAR_HEIGHT_PX),
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
            spawn_breadcrumb_root(bar);
            bar.spawn((
                DateText,
                Text::new(""),
                fonts.data(BAR_FONT_PX),
                TextColor(colors::NEUTRAL),
            ));
            spawn_bar_button(bar, &fonts, "Pause", PauseButton).observe(on_pause_clicked);
            for (label, speed) in [("1 d/s", 1.0_f64), ("5 d/s", 5.0_f64)] {
                spawn_bar_button(bar, &fonts, label, SpeedButton(speed)).observe(
                    move |_click: On<PointerClick>, mut clock: ResMut<GameClock>| {
                        clock.run_at(speed);
                    },
                );
            }
            // CLK-06: hidden by default.
            bar.spawn((
                FpsText,
                Node {
                    display: Display::None,
                    ..default()
                },
                Text::new(""),
                fonts.data(BUTTON_FONT_PX),
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

/// CLK-06: the FPS binding shows the readout, or hides it again.
fn toggle_fps_readout(
    keys: Res<ButtonInput<KeyCode>>,
    bindings: Res<Bindings>,
    mut readouts: Query<&mut Node, With<FpsText>>,
) {
    if !pressed(&keys, &bindings.toggle_fps) {
        return;
    }
    for mut node in &mut readouts {
        node.display = match node.display {
            Display::None => Display::Flex,
            _ => Display::None,
        };
    }
}

/// CLK-02: the active rate (or Pause, while paused) is the bar's one
/// primary button; the others are secondary.
fn update_button_styles(
    clock: Res<GameClock>,
    mut pause_buttons: Query<&mut ConsoleButton, (With<PauseButton>, Without<SpeedButton>)>,
    mut speed_buttons: Query<(&SpeedButton, &mut ConsoleButton), Without<PauseButton>>,
) {
    let style = |active: bool| {
        if active {
            ButtonStyle::Primary
        } else {
            ButtonStyle::Secondary
        }
    };
    for mut button in &mut pause_buttons {
        button.style = style(clock.is_paused());
    }
    for (speed_button, mut button) in &mut speed_buttons {
        let active = !clock.is_paused()
            && (clock.rate_days_per_sec() - speed_button.0).abs() < RATE_MATCH_TOLERANCE;
        button.style = style(active);
    }
}

#[cfg(test)]
mod tests {
    use bevy::asset::uuid::Uuid;
    use bevy::ecs::system::RunSystemOnce;

    use super::*;
    use crate::view::topbar::breadcrumb::BreadcrumbRoot;

    fn fps_display(world: &mut World) -> Display {
        let mut readouts = world.query_filtered::<&Node, With<FpsText>>();
        readouts.single(world).unwrap().display
    }

    fn press_fps_key(world: &mut World) {
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(KeyCode::F3);
        world.insert_resource(keys);
        world.run_system_once(toggle_fps_readout).unwrap();
    }

    /// CLK-06: the FPS readout is hidden at start, and F3 shows and hides it.
    #[test]
    fn the_fps_readout_starts_hidden_and_f3_toggles_it() {
        let mut world = World::new();
        world.insert_resource(placeholder_fonts());
        world.insert_resource(Bindings::default());
        world.insert_resource(ButtonInput::<KeyCode>::default());
        world.run_system_once(spawn_top_bar).unwrap();
        assert_eq!(fps_display(&mut world), Display::None);

        world.run_system_once(toggle_fps_readout).unwrap();
        assert_eq!(fps_display(&mut world), Display::None, "no key, no change");
        press_fps_key(&mut world);
        assert_eq!(fps_display(&mut world), Display::Flex);
        press_fps_key(&mut world);
        assert_eq!(fps_display(&mut world), Display::None);
    }

    fn primary_count(world: &mut World) -> usize {
        let mut buttons = world.query::<&ConsoleButton>();
        buttons
            .iter(world)
            .filter(|b| b.style == ButtonStyle::Primary)
            .count()
    }

    /// CLK-02: the active rate is the bar's only primary button; while
    /// paused, Pause is.
    #[test]
    fn only_the_active_rate_is_a_primary_button() {
        let mut world = World::new();
        world.insert_resource(placeholder_fonts());
        world.insert_resource(GameClock::default());
        world.run_system_once(spawn_top_bar).unwrap();

        world.resource_mut::<GameClock>().run_at(5.0);
        world.run_system_once(update_button_styles).unwrap();
        assert_eq!(primary_count(&mut world), 1);
        let mut speeds = world.query::<(&SpeedButton, &ConsoleButton)>();
        for (speed, button) in speeds.iter(&world) {
            let expected = if speed.0 == 5.0 {
                ButtonStyle::Primary
            } else {
                ButtonStyle::Secondary
            };
            assert_eq!(button.style, expected);
        }

        world.resource_mut::<GameClock>().toggle_pause();
        world.run_system_once(update_button_styles).unwrap();
        assert_eq!(primary_count(&mut world), 1);
        let mut pause = world.query_filtered::<&ConsoleButton, With<PauseButton>>();
        assert_eq!(pause.single(&world).unwrap().style, ButtonStyle::Primary);
    }

    fn placeholder_fonts() -> Fonts {
        let handle = |n: u128| Handle::<Font>::Uuid(Uuid::from_u128(n), Default::default());
        Fonts {
            label: handle(1),
            label_semibold: handle(2),
            text: handle(4),
            text_semibold: handle(5),
            data: handle(3),
        }
    }

    /// MRK-02: the top bar's date readout is in the data face; it has a
    /// slot for the breadcrumb (CLK-02).
    #[test]
    fn the_date_is_in_the_data_face_and_there_is_a_breadcrumb() {
        let handle = |n: u128| Handle::<Font>::Uuid(Uuid::from_u128(n), Default::default());
        let fonts = Fonts {
            label: handle(1),
            label_semibold: handle(2),
            text: handle(4),
            text_semibold: handle(5),
            data: handle(3),
        };
        let mut world = World::new();
        world.insert_resource(fonts.clone());
        world.run_system_once(spawn_top_bar).unwrap();

        let mut dates = world.query_filtered::<&TextFont, With<DateText>>();
        assert_eq!(dates.single(&world).unwrap().font, fonts.data(1.0).font);
        let mut roots = world.query_filtered::<(), With<BreadcrumbRoot>>();
        assert_eq!(roots.iter(&world).count(), 1);
    }
}
