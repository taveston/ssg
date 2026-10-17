//! Which click on the map is the second click of a double-click (ZOOM-16,
//! ZOOM-17, GRP-07, HOV-02, HOV-05).
//!
//! Bevy counts clicks per entity: a second click counts as a double only
//! if it lands on the same node as the first. On the map that node can
//! move between the clicks. A first click with nothing selected opens the
//! inset, which narrows the map and slides everything on it sideways
//! (INS-07), so the second click lands on something else, or on empty
//! map. So the map keeps its own record: a second press soon after a
//! click on a map target, and where that click's press was (the pointer
//! kept still while the map moved), completes a double-click on the first
//! click's target, whatever the second click now lands on. Two quick
//! clicks in different places are two single clicks. A click on empty map
//! leaves nothing to double, and a click right after a double-click
//! starts afresh, so a triple-click frames one thing only.
//!
//! Its own module so the click handlers in `hover.rs` and on the
//! multi-markers share one record.

use std::time::Duration;

use bevy::picking::PickingSettings;
use bevy::prelude::*;

use super::bindings::Bindings;
use super::multi::TrayFor;

/// The map's record of recent clicks, timed on the real clock
/// (`Time<Real>`): the last select-button press, the last single click on
/// a map target, and what the click being handled asked for.
#[derive(Resource, Default, Debug)]
pub struct MapClicks {
    press: Option<Press>,
    last: Option<LastClick>,
    /// The tray the click being handled asked for, if any.
    tray: Option<TrayFor>,
    /// The last click completed a double-click, so the next one is a
    /// single click whatever Bevy counts.
    after_double: bool,
}

/// A select-button press: when, and where on screen.
#[derive(Clone, Copy, Debug)]
struct Press {
    at: Duration,
    position: Vec2,
}

/// A single click on a map target that a second click may double: the
/// body a double-click on that target acts on, the tray the click asked
/// for, where its press was, and when the click ended.
#[derive(Clone, Copy, Debug)]
struct LastClick {
    body: Entity,
    tray: Option<TrayFor>,
    pressed_at: Vec2,
    released_at: Duration,
}

/// A double-click completed through the record: the body it acts on, and
/// the tray its first click opened, to ask for again.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Double {
    pub body: Entity,
    pub tray: Option<TrayFor>,
}

impl MapClicks {
    /// A select-button press at `now`, at `position` on screen. Only the
    /// first press after a click can double it: a second press since then,
    /// on a panel or starting a drag, ends the chance.
    pub fn pressed(&mut self, now: Duration, position: Vec2) {
        if let (Some(last), Some(before)) = (self.last, self.press)
            && before.at > last.released_at
        {
            self.last = None;
        }
        self.press = Some(Press { at: now, position });
    }

    /// A click on the map starts being handled. Returns whether it follows
    /// a completed double-click, and so is a single click whatever Bevy
    /// counts.
    pub fn begin(&mut self) -> bool {
        self.tray = None;
        std::mem::take(&mut self.after_double)
    }

    /// The click being handled asked for `tray`.
    pub fn asked_for_tray(&mut self, tray: TrayFor) {
        self.tray = Some(tray);
    }

    /// A single click on a map target ended at `now`; a second press soon
    /// after, in the same place, makes a double-click on `body`. A click
    /// with no press on record can't be doubled.
    pub fn remember(&mut self, body: Entity, now: Duration) {
        self.last = self.press.map(|press| LastClick {
            body,
            tray: self.tray,
            pressed_at: press.position,
            released_at: now,
        });
    }

    /// The click just handled can't start a double-click: it was on empty
    /// map.
    pub fn forget(&mut self) {
        self.last = None;
    }

    /// The click just handled completed a double-click on its own target,
    /// as Bevy counts it: the next click starts afresh.
    pub fn doubled(&mut self) {
        self.last = None;
        self.after_double = true;
    }

    /// If the click now ending started with a press within `interval` of
    /// the end of the last single click on a map target, and within
    /// `reach_px` of that click's press, this click completes a
    /// double-click on that click's target. The record is then used up and
    /// the next click starts afresh. The press must come after the
    /// remembered click ended, so the remembered click itself, seen by a
    /// second handler, never doubles itself.
    pub fn take_double(&mut self, interval: Duration, reach_px: f32) -> Option<Double> {
        let last = self.last?;
        let press = self.press?;
        let double = press.at > last.released_at
            && press.at - last.released_at <= interval
            && press.position.distance(last.pressed_at) < reach_px;
        if !double {
            return None;
        }
        self.doubled();
        Some(Double {
            body: last.body,
            tray: last.tray,
        })
    }
}

/// The longest gap from the end of one click to the press of the next
/// that still makes a double-click: Bevy's own setting, so the map agrees
/// with Bevy's click count.
pub fn double_click_interval(settings: Option<&PickingSettings>) -> Duration {
    settings.map_or_else(
        || PickingSettings::default().multi_click_interval,
        |settings| settings.multi_click_interval,
    )
}

/// Notes when and where the select button goes down, before Bevy's
/// picking sends the click events of this frame.
pub fn record_press(
    buttons: Res<ButtonInput<MouseButton>>,
    bindings: Res<Bindings>,
    time: Res<Time<Real>>,
    windows: Query<&Window>,
    mut clicks: ResMut<MapClicks>,
) {
    if !buttons.just_pressed(bindings.select_button) {
        return;
    }
    if let Some(position) = windows.single().ok().and_then(Window::cursor_position) {
        clicks.pressed(time.elapsed(), position);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INTERVAL: Duration = Duration::from_millis(500);
    const REACH: f32 = 4.0;
    const HERE: Vec2 = Vec2::new(300.0, 200.0);

    fn ms(ms: u64) -> Duration {
        Duration::from_millis(ms)
    }

    fn double(body: Entity) -> Option<Double> {
        Some(Double { body, tray: None })
    }

    /// ZOOM-16: a press soon after a click on a target, in the same place,
    /// doubles that click, once; a press too late, or none since, doesn't.
    #[test]
    fn a_press_soon_after_a_click_doubles_that_click_once() {
        let mut world = World::new();
        let body = world.spawn_empty().id();
        let mut clicks = MapClicks::default();
        clicks.pressed(ms(1000), HERE);
        clicks.remember(body, ms(1080));
        assert_eq!(clicks.take_double(INTERVAL, REACH), None, "the same click");
        clicks.pressed(ms(1300), HERE + Vec2::new(2.0, 1.0));
        assert_eq!(clicks.take_double(INTERVAL, REACH), double(body));
        assert_eq!(clicks.take_double(INTERVAL, REACH), None, "used up");

        clicks.remember(body, ms(2000));
        clicks.pressed(ms(2600), HERE);
        assert_eq!(clicks.take_double(INTERVAL, REACH), None, "too late");

        clicks.remember(body, ms(3000));
        clicks.forget();
        clicks.pressed(ms(3100), HERE);
        assert_eq!(clicks.take_double(INTERVAL, REACH), None, "forgotten");
    }

    /// ZOOM-16: two quick presses in different places are two clicks.
    #[test]
    fn a_quick_press_elsewhere_is_no_double_click() {
        let mut world = World::new();
        let body = world.spawn_empty().id();
        let mut clicks = MapClicks::default();
        clicks.pressed(ms(1000), HERE);
        clicks.remember(body, ms(1080));
        clicks.pressed(ms(1200), HERE + Vec2::new(0.0, 40.0));
        assert_eq!(clicks.take_double(INTERVAL, REACH), None);
    }

    /// ZOOM-16: a press between the click and the next one, on a panel or
    /// to drag, means the next click doubles nothing.
    #[test]
    fn a_press_in_between_ends_the_double_click() {
        let mut world = World::new();
        let body = world.spawn_empty().id();
        let mut clicks = MapClicks::default();
        clicks.pressed(ms(1000), HERE);
        clicks.remember(body, ms(1080));
        clicks.pressed(ms(1150), HERE);
        clicks.pressed(ms(1300), HERE);
        assert_eq!(clicks.take_double(INTERVAL, REACH), None);
    }

    /// MUL-07 / ZOOM-16: a completed double-click carries the tray its
    /// first click asked for, and the click after it starts afresh.
    #[test]
    fn a_double_click_keeps_its_first_clicks_tray_and_ends_the_run() {
        let mut world = World::new();
        let body = world.spawn_empty().id();
        let orbit = world.spawn_empty().id();
        let mut clicks = MapClicks::default();
        clicks.pressed(ms(1000), HERE);
        assert!(!clicks.begin());
        clicks.asked_for_tray(TrayFor::Orbit(orbit));
        clicks.remember(body, ms(1080));
        clicks.pressed(ms(1200), HERE);
        assert!(!clicks.begin());
        assert_eq!(
            clicks.take_double(INTERVAL, REACH),
            Some(Double {
                body,
                tray: Some(TrayFor::Orbit(orbit)),
            })
        );
        assert!(clicks.begin(), "the next click is a single click");
        assert!(!clicks.begin());
    }
}
