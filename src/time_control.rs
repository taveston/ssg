//! Game clock and its pause/speed keyboard shortcuts. The on-screen readout
//! lives in `topbar.rs`.

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

pub struct TimeControlPlugin;

impl Plugin for TimeControlPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameClock>()
            .add_systems(Update, (tick_clock, handle_time_input));
    }
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
