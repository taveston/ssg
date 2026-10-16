//! Which click on the map is the second click of a double-click (ZOOM-16,
//! ZOOM-17, GRP-07, HOV-02, HOV-05).
//!
//! Bevy counts clicks per entity: a second click counts as a double only
//! if it lands on the same node as the first. On the map that node can
//! move between the clicks. A first click with nothing selected opens the
//! inset, which narrows the map and slides everything on it sideways
//! (INS-07), so the second click lands on something else, or on empty
//! map. So the map keeps its own record: a press soon enough after a
//! click on a map target makes that click a double-click on the first
//! click's target, wherever on the map the second click lands. A click on
//! empty map leaves nothing to double, so a click after it is a single
//! click on whatever it hits.
//!
//! Its own module so the click handlers in `hover.rs` and on the
//! multi-markers share one record.

use std::time::Duration;

use bevy::picking::PickingSettings;
use bevy::prelude::*;

use super::bindings::Bindings;

/// When the select button last went down, and the last single click on a
/// map target, both on the real clock (`Time<Real>`).
#[derive(Resource, Default, Debug)]
pub struct MapClicks {
    pressed_at: Option<Duration>,
    last: Option<LastClick>,
}

/// A single click on a map target: the body a double-click on that target
/// would act on, and when the click ended.
#[derive(Clone, Copy, Debug)]
struct LastClick {
    body: Entity,
    released_at: Duration,
}

impl MapClicks {
    /// A single click on a map target ended at `now`; a press soon after
    /// makes a double-click on `body`.
    pub fn remember(&mut self, body: Entity, now: Duration) {
        self.last = Some(LastClick {
            body,
            released_at: now,
        });
    }

    /// The click just handled can't start a double-click: it was on empty
    /// map, or was itself the second click of one.
    pub fn forget(&mut self) {
        self.last = None;
    }

    /// A select-button press at `now`. Only the first press after a click
    /// can double it: a second press since then, on a panel or starting a
    /// drag, ends the chance.
    pub fn pressed(&mut self, now: Duration) {
        if let (Some(last), Some(before)) = (self.last, self.pressed_at)
            && before > last.released_at
        {
            self.last = None;
        }
        self.pressed_at = Some(now);
    }

    /// If the click now ending started with a press within `interval` of
    /// the last single click on a map target, the body that click stood
    /// for: this click completes a double-click on it. The record is then
    /// used up, so a third click starts afresh. The press must come after
    /// the remembered click ended, so the remembered click itself, seen
    /// by a second handler, never doubles itself.
    pub fn take_double(&mut self, interval: Duration) -> Option<Entity> {
        let last = self.last?;
        let pressed = self.pressed_at?;
        let double = pressed > last.released_at && pressed - last.released_at <= interval;
        if double {
            self.last = None;
        }
        double.then_some(last.body)
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

/// Notes when the select button goes down, before Bevy's picking sends
/// the click events of this frame.
pub fn record_press(
    buttons: Res<ButtonInput<MouseButton>>,
    bindings: Res<Bindings>,
    time: Res<Time<Real>>,
    mut clicks: ResMut<MapClicks>,
) {
    if buttons.just_pressed(bindings.select_button) {
        clicks.pressed(time.elapsed());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INTERVAL: Duration = Duration::from_millis(500);

    fn ms(ms: u64) -> Duration {
        Duration::from_millis(ms)
    }

    /// ZOOM-16: a press soon after a click on a target doubles that click,
    /// once; a press too late, or none since, doesn't.
    #[test]
    fn a_press_soon_after_a_click_doubles_that_click_once() {
        let mut world = World::new();
        let body = world.spawn_empty().id();
        let mut clicks = MapClicks::default();
        clicks.pressed(ms(1000));
        clicks.remember(body, ms(1080));
        assert_eq!(clicks.take_double(INTERVAL), None, "the same click");
        clicks.pressed(ms(1300));
        assert_eq!(clicks.take_double(INTERVAL), Some(body));
        assert_eq!(clicks.take_double(INTERVAL), None, "used up");

        clicks.remember(body, ms(2000));
        clicks.pressed(ms(2600));
        assert_eq!(clicks.take_double(INTERVAL), None, "too late");

        clicks.remember(body, ms(3000));
        clicks.forget();
        clicks.pressed(ms(3100));
        assert_eq!(clicks.take_double(INTERVAL), None, "forgotten");
    }

    /// ZOOM-16: a press between the click and the next one, on a panel or
    /// to drag, means the next click doubles nothing.
    #[test]
    fn a_press_in_between_ends_the_double_click() {
        let mut world = World::new();
        let body = world.spawn_empty().id();
        let mut clicks = MapClicks::default();
        clicks.pressed(ms(1000));
        clicks.remember(body, ms(1080));
        clicks.pressed(ms(1150));
        clicks.pressed(ms(1300));
        assert_eq!(clicks.take_double(INTERVAL), None);
    }
}
