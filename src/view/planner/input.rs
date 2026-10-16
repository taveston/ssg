//! What the player does in the planner's panel (PLNR-02, PLNR-13,
//! PLNR-14, PLNR-23, PLNR-29 to PLNR-31, PLNR-50, PLNR-53): end it, pick
//! a band or an arrival mode, move a slider by pointer or key, apply a
//! preset, schedule the order, and change one. Each action is a function
//! of the planner's state, so it can be tested apart from the pointer
//! that triggers it; orders go to the simulation as messages (ARCH-14).

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::panel::{Choice, Track};
use super::slider::{FAST_STEPS, stepped, value_at};
use super::state::{PlannerSession, PlannerState, SliderKind};
use crate::sim::transfer::order::{Arrival, Order};
use crate::sim::transfer::places::{Band, Place};
use crate::sim::transfer::plan::{Plan, ShipOrder, Status};
use crate::sim::transfer::presets::Preset;
use crate::sim::transfer::ranges::Slider;
use crate::sim::transfer::search::Request;
use crate::view::bindings::{Bindings, held, pressed};
use crate::view::targets::PointerTargets;

/// PLNR-02: the quiet Cancel ends the planner and drops its settings.
pub fn on_planner_cancel(click: On<PointerClick>, mut state: ResMut<PlannerState>) {
    if click.button == PointerButton::Primary {
        state.end();
    }
}

/// PLNR-30: the arrival an Intercept of `destination` passes through: a
/// body's chosen band, or High for a station or ship, whose own band
/// the solver takes (XFER-10).
pub fn intercept_arrival(destination: Place) -> Arrival {
    match destination {
        Place::Body { band, .. } => Arrival::Intercept { band },
        _ => Arrival::Intercept { band: Band::High },
    }
}

/// PLNR-13 / PLNR-14 / PLNR-30: choosing a band starts the tuning again
/// from its preset; choosing Rendezvous or Intercept solves again with
/// the same Depart and Travel time.
pub fn choose(state: &mut PlannerState, choice: Choice) {
    let Some(session) = state.session.as_mut() else {
        return;
    };
    let Some(request) = session.request else {
        return;
    };
    match (choice, request.destination) {
        (Choice::Band(band), Place::Body { body, band: was }) if band != was => {
            let destination = Place::Body { body, band };
            let arrival = match request.arrival {
                Arrival::Rendezvous => Arrival::Rendezvous,
                Arrival::Intercept { .. } => intercept_arrival(destination),
            };
            session.pick(Request {
                destination,
                arrival,
            });
        }
        (Choice::Arrival { intercept }, destination) => {
            let arrival = if intercept {
                intercept_arrival(destination)
            } else {
                Arrival::Rendezvous
            };
            if arrival != request.arrival {
                session.request = Some(Request { arrival, ..request });
            }
        }
        _ => {}
    }
}

/// PLNR-29: a preset sets both sliders and nothing else.
pub fn apply_preset(state: &mut PlannerState, preset: Preset) {
    let Some(session) = state.session.as_mut() else {
        return;
    };
    let Some(presets) = session.search().map(|search| search.presets) else {
        return;
    };
    let setting = match preset {
        Preset::Fastest => presets.fastest,
        Preset::Cheapest => presets.cheapest,
        Preset::NextWindow => presets.next_window,
    };
    if setting.is_some() {
        session.setting = setting;
    }
}

/// PLNR-23: a slider set to `value`; the other slider keeps its setting
/// (PLNR-13).
fn set_slider(session: &mut PlannerSession, kind: SliderKind, value: f64) {
    if let Some(setting) = session.setting.as_mut() {
        match kind {
            SliderKind::Depart => setting.depart_day = value,
            SliderKind::Travel => setting.travel_days = value,
        }
    }
}

/// The slider of `kind` in the current search, and its step limit.
fn slider_of(session: &PlannerSession, kind: SliderKind) -> Option<(Slider, u32)> {
    let found = session.found.as_ref()?;
    let search = found.result.as_ref().ok()?;
    let max_steps = found.context.as_ref()?.tunables.slider_max_steps;
    let slider = match kind {
        SliderKind::Depart => &search.sliders.depart,
        SliderKind::Travel => &search.sliders.travel,
    };
    Some((slider.clone(), max_steps))
}

/// PLNR-23: a press or drag at `fraction` of a track sets that slider to
/// the nearest setting there.
pub fn slide(state: &mut PlannerState, kind: SliderKind, fraction: f32) {
    let Some(session) = state.session.as_mut() else {
        return;
    };
    if let Some((slider, max_steps)) = slider_of(session, kind) {
        set_slider(session, kind, value_at(&slider, fraction, max_steps));
    }
}

/// PLNR-23: Left and Right move the focused slider one step, ten with
/// Shift.
pub fn step(state: &mut PlannerState, kind: SliderKind, steps: i32) {
    let Some(session) = state.session.as_mut() else {
        return;
    };
    let Some((slider, max_steps)) = slider_of(session, kind) else {
        return;
    };
    let Some(setting) = session.setting else {
        return;
    };
    let value = match kind {
        SliderKind::Depart => setting.depart_day,
        SliderKind::Travel => setting.travel_days,
    };
    set_slider(session, kind, stepped(&slider, value, steps, max_steps));
}

/// What moving a slider by pointer needs: the planner and the tracks'
/// geometry.
#[derive(SystemParam)]
pub struct SliderInput<'w, 's> {
    state: ResMut<'w, PlannerState>,
    tracks: Query<
        'w,
        's,
        (
            &'static Track,
            &'static ComputedNode,
            &'static UiGlobalTransform,
        ),
    >,
}

impl SliderInput<'_, '_> {
    /// PLNR-23: a slider pressed has focus, and takes the arrow keys.
    pub fn focus(&mut self, kind: SliderKind) {
        self.state.slider_focus = Some(kind);
    }

    /// PLNR-23: the slider of `kind` set to the point of its track under
    /// the pointer at `x` (logical pixels).
    pub fn slide_to(&mut self, kind: SliderKind, x: f32) {
        let Some((_, node, transform)) = self.tracks.iter().find(|(track, ..)| track.0 == kind)
        else {
            return;
        };
        let scale = node.inverse_scale_factor();
        let width = node.size().x * scale;
        if width <= 0.0 {
            return;
        }
        let left = transform.translation.x * scale - width / 2.0;
        slide(&mut self.state, kind, (x - left) / width);
    }
}

/// PLNR-23: the focused slider takes Left and Right, a step at a time or
/// ten with Shift; a press on the map takes its focus away.
pub fn slider_keys(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    bindings: Res<Bindings>,
    targets: PointerTargets,
    mut state: ResMut<PlannerState>,
) {
    if mouse.just_pressed(bindings.select_button) && targets.panel().is_none() {
        state.slider_focus = None;
    }
    let Some(kind) = state.slider_focus else {
        return;
    };
    let size = if held(&keys, &bindings.slider_fast) {
        FAST_STEPS
    } else {
        1
    };
    if pressed(&keys, &bindings.slider_down) {
        step(&mut state, kind, -size);
    }
    if pressed(&keys, &bindings.slider_up) {
        step(&mut state, kind, size);
    }
}

/// PLNR-31 / PLNR-50 / PLNR-53: the order the panel would schedule: only
/// for a leg that can be flown within the budget.
pub fn order_to_schedule(state: &PlannerState) -> Option<(Entity, Order, bool)> {
    let session = state.session.as_ref()?;
    let request = session.request?;
    let setting = session.setting?;
    let context = session.found.as_ref()?.context.as_ref()?;
    let leg = session.solved_leg()?;
    if session.solved_for != Some(setting) || leg.delta_v_kms > context.available_kms {
        return None;
    }
    Some((
        session.ship,
        context.order(request, setting),
        session.changing.is_some(),
    ))
}

/// PLNR-50 / PLNR-53: Schedule gives the ship its order, replacing the
/// one being changed, and closes the planner.
pub fn on_schedule(
    click: On<PointerClick>,
    mut state: ResMut<PlannerState>,
    mut orders: MessageWriter<ShipOrder>,
) {
    if click.button != PointerButton::Primary {
        return;
    }
    let Some((ship, order, changing)) = order_to_schedule(&state) else {
        return;
    };
    if changing {
        orders.write(ShipOrder::Cancel { ship });
    }
    orders.write(ShipOrder::Transfer { ship, order });
    state.end();
}

/// PLNR-53: the order a Scheduled plan was given: its destination,
/// arrival mode, depart day and requested travel time (less any trim the
/// solver added, PHAS-15).
pub fn order_of(plan: &Plan) -> Order {
    let leg = plan.leg();
    Order {
        destination: leg.destination,
        arrival: leg.arrival,
        depart_day: leg.depart_day,
        travel_days: leg.travel_days - leg.trim_days.unwrap_or(0.0),
    }
}

/// PLNR-53: Change opens the planner tuning the Scheduled order, which
/// stays in force until Schedule replaces it.
pub fn change_order(state: &mut PlannerState, ship: Entity, plan: &Plan) {
    if plan.status == Status::Scheduled {
        state.start(PlannerSession::changing(
            ship,
            &order_of(plan),
            plan.reserved_kms(),
        ));
    }
}

/// PLNR-53: if the order being changed departs or fails meanwhile, the
/// planner closes at once and says what happened.
pub fn watch_changed_order(mut state: ResMut<PlannerState>, plans: Query<&Plan>) {
    let Some(session) = state.session.as_ref() else {
        return;
    };
    if session.changing.is_none() {
        return;
    }
    let status = plans.get(session.ship).map(|plan| plan.status).ok();
    let notice = match status {
        Some(Status::Scheduled) => return,
        Some(Status::Failed) | None => super::words::ORDER_FAILED,
        Some(_) => super::words::DEPARTED_WHILE_CHANGING,
    };
    state.end();
    state.notice = Some(notice);
}

/// PLNR-02 / PLNR-53: the planner ends when its ship is no longer
/// selected (Close, or another selection), and the notice goes when the
/// selection changes.
pub fn end_with_selection(
    selection: Res<crate::view::selection::Selection>,
    mut state: ResMut<PlannerState>,
) {
    if let Some(session) = &state.session
        && selection.selected != Some(session.ship)
    {
        state.end();
    }
    if selection.is_changed() && !selection.is_added() {
        state.notice = None;
    }
}
