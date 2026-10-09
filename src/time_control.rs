//! Game clock, pause/speed input, and the minimal debug readout.

use bevy::prelude::*;

#[derive(Resource)]
pub struct GameClock {
    pub elapsed_days: f64,
    pub speed_days_per_sec: f64,
    pub paused: bool,
}

impl Default for GameClock {
    fn default() -> Self {
        Self {
            elapsed_days: 0.0,
            // Default: 1 real second = 1 game day.
            speed_days_per_sec: 1.0,
            paused: false,
        }
    }
}

#[derive(Component)]
struct DebugReadout;

pub struct TimeControlPlugin;

impl Plugin for TimeControlPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameClock>()
            .add_systems(Startup, spawn_debug_readout)
            .add_systems(
                Update,
                (tick_clock, handle_time_input, update_debug_readout),
            );
    }
}

fn spawn_debug_readout(mut commands: Commands) {
    commands.spawn((
        DebugReadout,
        Text::new(""),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));
}

fn tick_clock(time: Res<Time>, mut clock: ResMut<GameClock>) {
    if !clock.paused {
        clock.elapsed_days += clock.speed_days_per_sec * time.delta_secs() as f64;
    }
}

fn handle_time_input(keys: Res<ButtonInput<KeyCode>>, mut clock: ResMut<GameClock>) {
    if keys.just_pressed(KeyCode::Space) {
        clock.paused = !clock.paused;
    }
    if keys.just_pressed(KeyCode::Equal) {
        clock.speed_days_per_sec *= 2.0;
    }
    if keys.just_pressed(KeyCode::Minus) {
        clock.speed_days_per_sec = (clock.speed_days_per_sec / 2.0).max(0.0625);
    }
}

fn update_debug_readout(clock: Res<GameClock>, mut query: Query<&mut Text, With<DebugReadout>>) {
    for mut text in &mut query {
        let status = if clock.paused { "PAUSED" } else { "running" };
        *text = Text::new(format!(
            "Day {:.1}   Speed: {:.2}x/day   {}   [Space: pause, +/-: speed]",
            clock.elapsed_days, clock.speed_days_per_sec, status
        ));
    }
}
