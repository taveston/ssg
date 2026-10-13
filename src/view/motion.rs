//! The motion rule on screen (MOT-01, MOT-02, MOT-04, MOT-08, MOT-10 to
//! MOT-12): keeps each orbiting body's tail and mode current, and places
//! the labels of exposure rings. The rings and their tails are drawn in
//! `rings.rs` and hit-tested in `ring_picking.rs` (HOV-02); the rules are
//! in `motion_mode.rs`.
//!
//! MOT-08: applies to everything that orbits, not just satellites -
//! close-in planets get it too. Gates keep their edge-indicator treatment.

use bevy::prelude::*;

use super::layers;

use super::motion_mode::{MotionModeState, TailSettings, motion_state};
use crate::sim::body::{BodyClass, DisplayName};
use crate::sim::clock::GameClock;
use crate::sim::orbit::Orbit;
use crate::view::colors;
use crate::view::fold::SatelliteFoldState;
use crate::view::fonts::Fonts;
use crate::view::scale::km_to_render;
use crate::view::screen::in_viewport;
use crate::view::selection::Selection;
use crate::view::tunables::Tunables;
use crate::view::world::ViewNow;
use crate::{Boot, Stage};

/// The label that sits on an exposure ring at a fixed spot (MOT-02).
#[derive(Component)]
pub struct RingLabelFor(pub Entity);

/// Below this much closing an exposure ring's label isn't drawn.
const MIN_LABEL_ALPHA: f32 = 0.05;
/// Where a ring's label sits relative to its fixed spot on the ring.
const RING_LABEL_OFFSET_PX: Vec2 = Vec2::new(6.0, -6.0);
const RING_LABEL_FONT_PX: f32 = 11.0;

pub struct MotionPlugin;

impl Plugin for MotionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_motion_visuals.in_set(Boot::Overlay))
            .add_systems(
                Update,
                (update_motion_modes, place_ring_labels)
                    .chain()
                    .in_set(Stage::Motion),
            );
    }
}

/// Every orbiting body gets motion state and a ring label (its ring's hit
/// target is `ring_picking.rs`'s). Gates keep their edge-indicator
/// treatment instead.
fn spawn_motion_visuals(
    mut commands: Commands,
    fonts: Res<Fonts>,
    bodies: Query<(Entity, &BodyClass, &DisplayName), With<Orbit>>,
) {
    for (entity, class, name) in &bodies {
        if *class == BodyClass::Gate {
            continue;
        }
        commands.entity(entity).insert(MotionModeState::default());
        commands.spawn((
            RingLabelFor(entity),
            GlobalZIndex(layers::LABELS),
            Node {
                position_type: PositionType::Absolute,
                display: Display::None,
                ..default()
            },
            Text::new(name.0.clone()),
            fonts.label(RING_LABEL_FONT_PX),
            TextColor(colors::NEUTRAL),
        ));
    }
}

/// MOT-01, MOT-02, MOT-04, MOT-10 to MOT-12: each body's tail follows its
/// apparent period at the selected rate, every frame.
fn update_motion_modes(
    clock: Res<GameClock>,
    tunables: Res<Tunables>,
    mut bodies: Query<(&Orbit, &mut MotionModeState)>,
) {
    let settings = TailSettings {
        tail_secs: tunables.tail_secs,
        dot_fade_from_degrees: tunables.dot_fade_from_degrees,
    };
    for (orbit, mut state) in &mut bodies {
        let next = motion_state(
            orbit.period_days,
            clock.rate_days_per_sec(),
            clock.is_paused(),
            settings,
        );
        if *state != next {
            *state = next;
        }
    }
}

/// Orbiting bodies, with what decides how their ring shows.
pub type RingBodies<'w, 's> = Query<
    'w,
    's,
    (
        &'static Orbit,
        &'static MotionModeState,
        Option<&'static ChildOf>,
        Option<&'static SatelliteFoldState>,
    ),
>;

/// Where the point at `angle` on a body's orbit lands on screen.
pub fn ring_point(body: Entity, angle: f32, bodies: &RingBodies, view: &ViewNow) -> Option<Vec2> {
    let (orbit, _, child_of, _) = bodies.get(body).ok()?;
    let centre = match child_of {
        Some(child_of) => view.position(child_of.parent())?,
        None => view.star(),
    };
    let radius = km_to_render(orbit.radius_km);
    let world = centre + Vec3::new(radius * angle.cos(), 0.0, radius * angle.sin());
    view.project(world)
}

/// MOT-02 / MOT-11: how strongly an exposure ring's label shows: it fades
/// in as the tail closes, as the dot and its label fade out, so the label
/// hands over from dot to ring without a jump. A folded body has none.
pub fn ring_label_alpha(state: &MotionModeState, folded_away: bool) -> f32 {
    if folded_away { 0.0 } else { state.closing }
}

/// MOT-02: the label sits on the ring at a fixed spot - the body's start
/// phase - so it doesn't appear to orbit while the body is a ring. A
/// selected body's label is white.
fn place_ring_labels(
    view: ViewNow,
    windows: Query<&Window>,
    selection: Res<Selection>,
    bodies: RingBodies,
    mut labels: Query<(&RingLabelFor, &mut Node, &mut TextColor)>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let viewport = Vec2::new(window.width(), window.height());
    for (label_for, mut node, mut color) in &mut labels {
        let body = label_for.0;
        let placed = bodies.get(body).ok().and_then(|(orbit, mode, _, fold)| {
            let alpha = ring_label_alpha(mode, fold.is_some_and(|f| f.effective_hidden));
            if alpha < MIN_LABEL_ALPHA {
                return None;
            }
            let pos = ring_point(body, orbit.phase0_rad as f32, &bodies, &view)?;
            in_viewport(pos, viewport).then_some((pos, alpha))
        });
        let Some((pos, alpha)) = placed else {
            node.display = Display::None;
            continue;
        };
        node.display = Display::Flex;
        node.left = px(pos.x + RING_LABEL_OFFSET_PX.x);
        node.top = px(pos.y + RING_LABEL_OFFSET_PX.y);
        let base = if selection.is_selected(body) {
            colors::SELECTION_RING
        } else {
            colors::NEUTRAL
        };
        color.0 = base.with_alpha(alpha);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// MOT-02 / MOT-11: the ring's label fades in as the dot fades out,
    /// and a folded body shows neither.
    #[test]
    fn the_ring_label_takes_over_from_the_dot() {
        let state = |closing: f32| MotionModeState {
            tail_degrees: 300.0 + 60.0 * closing,
            closing,
            ring_mode: closing >= 1.0,
        };
        for closing in [0.0, 0.3, 1.0] {
            let s = state(closing);
            assert!((ring_label_alpha(&s, false) + s.dot_alpha() - 1.0).abs() < 1e-6);
        }
        assert_eq!(ring_label_alpha(&state(1.0), true), 0.0);
    }
}
