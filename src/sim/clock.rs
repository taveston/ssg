//! The game clock (CLK-01, CLK-04).
//!
//! The clock owns its own state: its fields are private and every change
//! goes through a method on it (ARCH-14), so no view module can set a rate
//! or a pause flag behind its back. The readout, the rate buttons and the
//! keys live in `view/topbar.rs`.

use bevy::prelude::*;

/// CLK-03: the slowest rate the player can halve down to. Below this the
/// date stops reading as moving at all.
const MIN_RATE_DAYS_PER_SEC: f64 = 0.0625;

#[derive(Resource)]
pub struct GameClock {
    elapsed_days: f64,
    speed_days_per_sec: f64,
    paused: bool,
}

impl Default for GameClock {
    fn default() -> Self {
        Self {
            elapsed_days: 0.0,
            // CLK-01: 1 real second = 1 game day.
            speed_days_per_sec: 1.0,
            paused: false,
        }
    }
}

impl GameClock {
    /// Simulated days since the campaign epoch.
    pub fn elapsed_days(&self) -> f64 {
        self.elapsed_days
    }

    pub fn rate_days_per_sec(&self) -> f64 {
        self.speed_days_per_sec
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// CLK-04: a paused clock doesn't advance, so nothing orbits.
    pub fn advance(&mut self, real_secs: f64) {
        if !self.paused {
            self.elapsed_days += self.speed_days_per_sec * real_secs;
        }
    }

    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    /// Run at a given rate, resuming if paused - what the top bar's rate
    /// buttons do (CLK-02).
    pub fn run_at(&mut self, days_per_sec: f64) {
        self.paused = false;
        self.speed_days_per_sec = days_per_sec.max(MIN_RATE_DAYS_PER_SEC);
    }

    pub fn double_rate(&mut self) {
        self.speed_days_per_sec *= 2.0;
    }

    pub fn halve_rate(&mut self) {
        self.speed_days_per_sec = (self.speed_days_per_sec / 2.0).max(MIN_RATE_DAYS_PER_SEC);
    }
}

/// CLK-01 / CLK-04: the clock advances with real time, unless paused.
pub fn tick_clock(time: Res<Time>, mut clock: ResMut<GameClock>) {
    clock.advance(time.delta_secs() as f64);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CLK-01: the default is one game day per real second.
    #[test]
    fn default_rate_is_one_day_per_second() {
        let mut clock = GameClock::default();
        assert_eq!(clock.rate_days_per_sec(), 1.0);
        clock.advance(2.0);
        assert!((clock.elapsed_days() - 2.0).abs() < 1e-12);
    }

    /// CLK-01: the rate scales elapsed time, nothing else.
    #[test]
    fn rate_scales_elapsed_days() {
        let mut clock = GameClock::default();
        clock.run_at(5.0);
        clock.advance(2.0);
        assert!((clock.elapsed_days() - 10.0).abs() < 1e-12);
    }

    /// CLK-04: pausing stops the clock, and resuming carries on from where
    /// it stopped rather than catching up.
    #[test]
    fn pause_stops_the_clock_without_losing_time() {
        let mut clock = GameClock::default();
        clock.advance(3.0);
        clock.toggle_pause();
        clock.advance(100.0);
        assert!((clock.elapsed_days() - 3.0).abs() < 1e-12);
        clock.toggle_pause();
        clock.advance(1.0);
        assert!((clock.elapsed_days() - 4.0).abs() < 1e-12);
    }

    /// CLK-03: halving stops at 1/16 d/s; doubling is unbounded.
    #[test]
    fn rate_halves_down_to_one_sixteenth() {
        let mut clock = GameClock::default();
        for _ in 0..10 {
            clock.halve_rate();
        }
        assert_eq!(clock.rate_days_per_sec(), MIN_RATE_DAYS_PER_SEC);
        clock.double_rate();
        assert_eq!(clock.rate_days_per_sec(), 0.125);
    }

    /// CLK-02: picking a rate also resumes, so the buttons can't leave the
    /// clock paused at a rate it isn't running.
    #[test]
    fn choosing_a_rate_resumes() {
        let mut clock = GameClock::default();
        clock.toggle_pause();
        clock.run_at(5.0);
        assert!(!clock.is_paused());
        assert_eq!(clock.rate_days_per_sec(), 5.0);
    }
}
