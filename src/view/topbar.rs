//! The top bar and the clock controls (CLK-02, CLK-05): the breadcrumb's
//! slot (filled by `breadcrumb.rs`), date,
//! the Pause, 1 d/s and 5 d/s buttons with the active rate highlighted, and
//! the clock keys (CLK-05: Space, `.` and `,`). Every change goes through the clock's
//! own API (ARCH-14). A debug frames-per-second readout sits at the end,
//! hidden until its binding toggles it (CLK-06).

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;

use crate::sim::clock::GameClock;
use crate::view::bindings::{Bindings, pressed};
use crate::view::breadcrumb::spawn_breadcrumb_root;
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
                toggle_fps_readout,
            ),
        );
    }
}

/// The top bar's height. Edge arrows stay below it (EDGE-01).
pub const TOP_BAR_HEIGHT_PX: f32 = 54.0;
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
            // CLK-06: hidden by default.
            bar.spawn((
                FpsText,
                Node {
                    display: Display::None,
                    ..default()
                },
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

#[cfg(test)]
mod tests {
    use bevy::asset::uuid::Uuid;
    use bevy::ecs::system::RunSystemOnce;

    use super::*;
    use crate::view::breadcrumb::BreadcrumbRoot;

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

    fn placeholder_fonts() -> Fonts {
        let handle = |n: u128| Handle::<Font>::Uuid(Uuid::from_u128(n), Default::default());
        Fonts {
            label: handle(1),
            label_semibold: handle(2),
            mono: handle(3),
        }
    }

    /// MRK-02: the top bar's date readout is in the mono font; it has a
    /// slot for the breadcrumb (CLK-02).
    #[test]
    fn the_date_is_mono_and_there_is_a_breadcrumb() {
        let handle = |n: u128| Handle::<Font>::Uuid(Uuid::from_u128(n), Default::default());
        let fonts = Fonts {
            label: handle(1),
            label_semibold: handle(2),
            mono: handle(3),
        };
        let mut world = World::new();
        world.insert_resource(fonts.clone());
        world.run_system_once(spawn_top_bar).unwrap();

        let mut dates = world.query_filtered::<&TextFont, With<DateText>>();
        assert_eq!(dates.single(&world).unwrap().font, fonts.mono(1.0).font);
        let mut roots = world.query_filtered::<(), With<BreadcrumbRoot>>();
        assert_eq!(roots.iter(&world).count(), 1);
    }
}
