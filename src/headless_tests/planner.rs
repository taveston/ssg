//! Headless tests of the transfer planner's panel (`docs/spec/planner.md`
//! sections 1 to 5): the inset, the outline and the planner's own
//! plugin on Corvane, driven by clicks on nodes, keys and the clock.

use std::collections::HashMap;

use bevy::input::mouse::AccumulatedMouseScroll;

use super::*;
use crate::view::buttons::ConsoleButton;
use crate::view::camera::requests::PlanetaryViewRequest;
use crate::view::fidelity::FidelityImages;
use crate::view::inspect::{FactLabel, FactValue, InspectPlugin, TransferButton};
use crate::view::outline::{OutlinePlugin, OutlineRow};
use crate::view::planner::PlannerPlugin;
use crate::view::planner::panel::{Choice, Field, NoticeLine, ScheduleButton};
use crate::view::planner::tooltip::{Tooltip, TooltipBox};
use crate::view::planner_state::{PickTarget, PlannerSession, SliderKind};
use crate::view::planner_words::{DELTA_V_TOOLTIP, INTERCEPT_TOOLTIP};
use crate::view::sidebar::SidebarPlugin;

/// Corvane with the inset, its outline and the planner, paused.
fn planner_app() -> App {
    let mut app = headless_app(FRAME);
    app.add_plugins((
        ContentPlugin,
        SimPlugin,
        TunablesPlugin,
        BindingsPlugin,
        SidebarPlugin,
        OutlinePlugin,
        InspectPlugin,
        HoveredPlugin,
        PlannerPlugin,
    ))
    .init_resource::<Selection>()
    .init_resource::<PlannerState>()
    .init_resource::<Frame>()
    .init_resource::<FrameRequest>()
    .init_resource::<PlanetaryViewRequest>()
    .init_resource::<ButtonInput<KeyCode>>()
    .init_resource::<ButtonInput<MouseButton>>()
    .init_resource::<AccumulatedMouseScroll>()
    .insert_resource(FidelityImages {
        surveyed: Handle::default(),
        unsurveyed: Handle::default(),
    })
    .insert_resource(placeholder_fonts());
    run(&mut app, 2);
    pause(&mut app);
    app
}

fn pause(app: &mut App) {
    let mut clock = app.world_mut().resource_mut::<GameClock>();
    if !clock.is_paused() {
        clock.toggle_pause();
    }
}

/// Frames for a pick to be searched, solved and written into the panel.
fn settle(app: &mut App) {
    run(app, 4);
}

fn planner(app: &App) -> &PlannerState {
    app.world().resource::<PlannerState>()
}

fn session(app: &App) -> &PlannerSession {
    planner(app).session.as_ref().expect("an open planner")
}

fn select(app: &mut App, ship: Entity) {
    app.world_mut().resource_mut::<Selection>().select(ship);
    run(app, 3);
}

/// Whether a node and all its ancestors are laid out.
fn shown(app: &App, entity: Entity) -> bool {
    std::iter::successors(Some(entity), |&e| {
        app.world().get::<ChildOf>(e).map(ChildOf::parent)
    })
    .all(|e| {
        app.world()
            .get::<Node>(e)
            .is_none_or(|node| node.display != Display::None)
    })
}

/// The panel's written texts, by field.
fn fields(app: &mut App) -> HashMap<Field, String> {
    let mut query = app.world_mut().query::<(&Field, &Text)>();
    query
        .iter(app.world())
        .map(|(field, text)| (*field, text.0.clone()))
        .collect()
}

/// The shown button whose label is `label`.
fn button(app: &mut App, label: &str) -> Option<Entity> {
    let mut buttons = app
        .world_mut()
        .query_filtered::<(Entity, &Children), With<ConsoleButton>>();
    let found: Vec<(Entity, Vec<Entity>)> = buttons
        .iter(app.world())
        .map(|(e, children)| (e, children.to_vec()))
        .collect();
    found
        .into_iter()
        .find(|(e, children)| {
            shown(app, *e)
                && children
                    .iter()
                    .any(|c| app.world().get::<Text>(*c).is_some_and(|t| t.0 == label))
        })
        .map(|(e, _)| e)
}

fn press(app: &mut App, keys: &[KeyCode]) {
    {
        let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        for key in keys {
            input.press(*key);
        }
    }
    run(app, 1);
    let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    input.release_all();
    input.clear();
}

/// The key facts shown, label to value.
fn facts(app: &mut App) -> HashMap<String, String> {
    let mut labels = app.world_mut().query::<(&FactLabel, &ChildOf)>();
    let labels: Vec<(String, Entity)> = labels
        .iter(app.world())
        .map(|(l, c)| (l.0.clone(), c.parent()))
        .collect();
    let mut values = app.world_mut().query::<(&FactValue, &ChildOf)>();
    values
        .iter(app.world())
        .filter_map(|(v, c)| {
            let label = labels.iter().find(|(_, p)| *p == c.parent())?;
            Some((label.0.clone(), v.0.clone()))
        })
        .collect()
}

/// The clickable label of the outline entry standing for `entity`: the
/// part after its toggle.
fn row(app: &mut App, entity: Entity) -> Entity {
    let mut rows = app.world_mut().query::<(&OutlineRow, &Children)>();
    rows.iter(app.world())
        .find(|(r, _)| r.0 == entity)
        .map(|(_, children)| children[1])
        .expect("a row")
}

/// Opens the planner for Orison and picks Halden IVa through its outline
/// row.
fn plan_to_halden_iva(app: &mut App) -> (Entity, Entity) {
    plan_to_halden_iva_from(app, 0)
}

/// As `plan_to_halden_iva`, with the depart day `steps` steps on, so an
/// order is still Scheduled for a while.
fn plan_to_halden_iva_from(app: &mut App, steps: i32) -> (Entity, Entity) {
    let orison = body(app, "Orison");
    let iva = body(app, "Halden IVa");
    select(app, orison);
    app.world_mut().resource_mut::<PlannerState>().open(orison);
    run(app, 1);
    let iva_row = row(app, iva);
    click_node(app, iva_row, 1);
    settle(app);
    if steps != 0 {
        crate::view::planner::input::step(
            &mut app.world_mut().resource_mut::<PlannerState>(),
            SliderKind::Depart,
            steps,
        );
        settle(app);
    }
    (orison, iva)
}

/// PLNR-01 / PLNR-02 / PLNR-05 / PLNR-10 / INS-02 / INS-03: a commanded
/// ship's inset lists its delta-v and drive and has a Transfer button; a
/// group and a ship the faction doesn't command have none. Transfer opens
/// the planner choosing, in place of the image and key facts, with the
/// ship still selected. Esc ends the planner first and then closes the
/// inset; Close ends both.
#[test]
fn the_transfer_button_opens_the_planner_in_the_inset() {
    let mut app = planner_app();
    let orison = body(&mut app, "Orison");
    select(&mut app, orison);
    let facts_now = facts(&mut app);
    assert_eq!(facts_now["Delta-v"], "9.9 km/s");
    assert_eq!(facts_now["Drive"], "hydrogen NTR");
    let transfer = button(&mut app, "Transfer").expect("a Transfer button");
    assert!(app.world().get::<TransferButton>(transfer).is_some());
    for other in ["TF-1", "Marten"] {
        let other = body(&mut app, other);
        select(&mut app, other);
        assert_eq!(button(&mut app, "Transfer"), None);
    }

    select(&mut app, orison);
    let transfer = button(&mut app, "Transfer").unwrap();
    click_node(&mut app, transfer, 1);
    settle(&mut app);
    assert!(planner(&app).is_open_for(orison));
    assert_eq!(session(&app).request, None, "choosing");
    assert_eq!(app.world().resource::<Selection>().selected, Some(orison));
    let mut texts = app.world_mut().query::<(Entity, &Text)>();
    let prompt = texts
        .iter(app.world())
        .find(|(_, t)| t.0 == crate::view::planner_words::CHOOSE_PROMPT)
        .map(|(e, _)| e)
        .expect("the prompt");
    assert!(shown(&app, prompt));
    let mut values = app.world_mut().query_filtered::<Entity, With<FactValue>>();
    let value = values.iter(app.world()).next().unwrap();
    assert!(!shown(&app, value), "the key facts give way");
    assert_eq!(button(&mut app, "Transfer"), None);

    press(&mut app, &[KeyCode::Escape]);
    run(&mut app, 2);
    assert!(!planner(&app).is_open());
    assert_eq!(app.world().resource::<Selection>().selected, Some(orison));
    let mut values = app.world_mut().query_filtered::<Entity, With<FactValue>>();
    let value = values.iter(app.world()).next().unwrap();
    assert!(shown(&app, value), "the key facts come back");
    press(&mut app, &[KeyCode::Escape]);
    assert_eq!(app.world().resource::<Selection>().selected, None);

    select(&mut app, orison);
    app.world_mut().resource_mut::<PlannerState>().open(orison);
    let close = button(&mut app, "Close").unwrap();
    click_node(&mut app, close, 1);
    run(&mut app, 2);
    assert!(!planner(&app).is_open());
    assert_eq!(app.world().resource::<Selection>().selected, None);
}

/// PLNR-03 / PLNR-10 / PLNR-11 / PLNR-12 / PLNR-13 / PLNR-20 / INS-01:
/// while choosing, hovering a moon shows its Fastest line; a click on its
/// outline entry picks its High band instead of selecting it, and the
/// sliders start at the Fastest setting. The planned ship and a group
/// can't be picked.
#[test]
fn a_click_picks_a_place_and_tunes_from_its_fastest_setting() {
    let mut app = planner_app();
    let orison = body(&mut app, "Orison");
    let iva = body(&mut app, "Halden IVa");
    let tf1 = body(&mut app, "TF-1");
    select(&mut app, orison);
    app.world_mut().resource_mut::<PlannerState>().open(orison);
    run(&mut app, 2);
    assert!(planner(&app).unpickable.contains(&orison));
    assert!(planner(&app).unpickable.contains(&tf1));
    let group_hover = app.world_mut().spawn((MarkerFor(tf1), Hovered)).id();
    run(&mut app, 2);
    let hovered = app
        .world()
        .resource::<crate::view::hovered::HoveredBodies>();
    assert!(
        !hovered.contains(tf1),
        "no hover mark on what can't be picked"
    );
    app.world_mut().despawn(group_hover);

    let hover = app.world_mut().spawn((MarkerFor(iva), Hovered)).id();
    settle(&mut app);
    let line = fields(&mut app)[&Field::Hover].clone();
    assert!(line.starts_with("Fastest: arrives "), "{line}");
    assert!(line.ends_with(" km/s"), "{line}");
    app.world_mut().despawn(hover);

    app.world_mut()
        .resource_mut::<PlannerState>()
        .pick(PickTarget::Body(tf1));
    settle(&mut app);
    assert_eq!(session(&app).request, None, "a group can't be picked");

    let iva_row = row(&mut app, iva);
    click_node(&mut app, iva_row, 1);
    settle(&mut app);
    assert_eq!(app.world().resource::<Selection>().selected, Some(orison));
    let session = session(&app);
    assert_eq!(
        session.request.unwrap().destination,
        Place::Body {
            body: iva,
            band: Band::High
        }
    );
    let starting = session.search().unwrap().presets.starting();
    assert_eq!(session.setting, starting);
    let texts = fields(&mut app);
    assert_eq!(texts[&Field::Route], "Orison → Halden IVa");
    assert_eq!(texts[&Field::Place], "High orbit, Halden IV · hydrogen NTR");
    assert!(texts[&Field::DeltaVValue].ends_with("of 9.9 km/s"));
}

/// The choice option `choice`.
fn option(app: &mut App, choice: Choice) -> Entity {
    let mut options = app.world_mut().query::<(Entity, &Choice)>();
    options
        .iter(app.world())
        .find(|(_, c)| **c == choice)
        .map(|(e, _)| e)
        .expect("the option")
}

/// PLNR-13 / PLNR-14 / PLNR-29 / PLNR-30: High is selected first;
/// choosing Low starts again from its preset, as does a click on a
/// band's ring, which picks that band; choosing Intercept keeps
/// both sliders and solves an Intercept; a preset sets both sliders.
#[test]
fn the_band_choice_and_arrival_mode_retune_the_leg() {
    use crate::view::planner::panel::Segment;
    let mut app = planner_app();
    let (_, iva) = plan_to_halden_iva(&mut app);
    let high = option(&mut app, Choice::Band(Band::High));
    assert!(app.world().get::<Segment>(high).unwrap().selected);

    let low = option(&mut app, Choice::Band(Band::Low));
    click_node(&mut app, low, 1);
    settle(&mut app);
    let session_now = session(&app);
    assert_eq!(
        session_now.request.unwrap().destination,
        Place::Body {
            body: iva,
            band: Band::Low
        }
    );
    assert_eq!(
        session_now.setting,
        session_now.search().unwrap().presets.starting()
    );

    let high_ring = band_orbit(&mut app, iva, Band::High);
    app.world_mut()
        .resource_mut::<PlannerState>()
        .pick(PickTarget::Orbit(high_ring));
    settle(&mut app);
    assert_eq!(
        session(&app).request.unwrap().destination,
        Place::Body {
            body: iva,
            band: Band::High
        },
        "a band's ring picks that band"
    );
    let low = option(&mut app, Choice::Band(Band::Low));
    click_node(&mut app, low, 1);
    settle(&mut app);

    crate::view::planner::input::step(
        &mut app.world_mut().resource_mut::<PlannerState>(),
        SliderKind::Travel,
        5,
    );
    settle(&mut app);
    let moved = session(&app).setting;
    let intercept = option(&mut app, Choice::Arrival { intercept: true });
    click_node(&mut app, intercept, 1);
    settle(&mut app);
    let session_now = session(&app);
    assert_eq!(session_now.setting, moved, "the sliders stay");
    assert!(matches!(
        session_now.request.unwrap().arrival,
        Arrival::Intercept { .. }
    ));
    let leg = session_now.solved_leg().expect("an intercept");
    assert_eq!(leg.arrival_speed_kms, leg.intercept_speed_kms);
    assert_eq!(fields(&mut app)[&Field::ArrivalLabel], "Intercepts");

    let cheapest = button(&mut app, "Cheapest").expect("Cheapest");
    click_node(&mut app, cheapest, 1);
    settle(&mut app);
    let session_now = session(&app);
    assert_eq!(
        session_now.setting,
        session_now.search().unwrap().presets.cheapest
    );
}

/// PLNR-23 / PLNR-24 / PLNR-87: a focused slider takes Left and Right a
/// step at a time, ten with Shift. Moving it solves the leg but doesn't
/// search again; the Travel time track follows the depart day. The
/// search is redone once the day changes, and a depart day in the past
/// moves to today.
#[test]
fn sliders_move_in_steps_and_the_search_waits_for_the_day() {
    use crate::sim::transfer::ranges::Slider;
    let mut app = planner_app();
    plan_to_halden_iva(&mut app);
    let searched_on = session(&app)
        .found
        .as_ref()
        .unwrap()
        .context
        .as_ref()
        .unwrap()
        .today;
    let (Slider::Range { start, end }, before) = (
        session(&app).search().unwrap().sliders.travel.clone(),
        session(&app).setting.unwrap(),
    ) else {
        panic!("a travel range");
    };
    let step = crate::sim::transfer::ranges::slider_step_days(end - start, 400);

    app.world_mut().resource_mut::<PlannerState>().slider_focus = Some(SliderKind::Travel);
    press(&mut app, &[KeyCode::ArrowRight]);
    settle(&mut app);
    let after = session(&app).setting.unwrap();
    assert!((after.travel_days - before.travel_days - step).abs() < 1e-9);
    press(&mut app, &[KeyCode::ShiftLeft, KeyCode::ArrowRight]);
    settle(&mut app);
    let back = session(&app).setting.unwrap();
    assert!((back.travel_days - after.travel_days - 10.0 * step).abs() < 1e-9);
    assert_eq!(session(&app).solved_for, Some(back), "the leg is solved");
    let found = session(&app).found.as_ref().unwrap();
    assert_eq!(
        found.context.as_ref().unwrap().today,
        searched_on,
        "no new search"
    );

    app.world_mut().resource_mut::<PlannerState>().slider_focus = Some(SliderKind::Depart);
    press(&mut app, &[KeyCode::ArrowRight]);
    settle(&mut app);
    let moved = session(&app).setting.unwrap();
    assert!(moved.depart_day > back.depart_day);
    assert_eq!(session(&app).travel_track_day, Some(moved.depart_day));

    // Back to today, then let the clock run: the depart day keeps up.
    crate::view::planner::input::step(
        &mut app.world_mut().resource_mut::<PlannerState>(),
        SliderKind::Depart,
        -1,
    );
    settle(&mut app);
    app.world_mut().resource_mut::<GameClock>().run_at(0.5);
    run(&mut app, 10);
    pause(&mut app);
    run(&mut app, 2);
    let today = day(&app);
    assert!(today > searched_on);
    assert!((session(&app).setting.unwrap().depart_day - today).abs() < 1e-9);
    assert_eq!(
        session(&app)
            .found
            .as_ref()
            .unwrap()
            .context
            .as_ref()
            .unwrap()
            .today,
        searched_on,
        "still the same day"
    );
    app.world_mut().resource_mut::<GameClock>().run_at(5.0);
    run(&mut app, 15);
    pause(&mut app);
    run(&mut app, 2);
    let searched_again = session(&app)
        .found
        .as_ref()
        .unwrap()
        .context
        .as_ref()
        .unwrap()
        .today;
    assert!(searched_again.floor() > searched_on.floor(), "a new day");
}

/// PLNR-27 / PLNR-31: a Rendezvous at Iver is over Orison's budget, so
/// Schedule is disabled and the reason line says what it needs.
#[test]
fn schedule_is_disabled_over_budget() {
    let mut app = planner_app();
    let orison = body(&mut app, "Orison");
    let iver = body(&mut app, "Iver");
    select(&mut app, orison);
    app.world_mut().resource_mut::<PlannerState>().open(orison);
    run(&mut app, 1);
    app.world_mut()
        .resource_mut::<PlannerState>()
        .pick(PickTarget::Body(iver));
    settle(&mut app);
    let mut schedule = app
        .world_mut()
        .query_filtered::<&ConsoleButton, With<ScheduleButton>>();
    assert!(schedule.single(app.world()).unwrap().disabled);
    let reason = fields(&mut app)[&Field::Reason].clone();
    assert!(
        reason.starts_with("Needs ") && reason.ends_with(", has 9.9"),
        "{reason}"
    );
}

/// PLNR-05 / PLNR-31 / PLNR-50 / PLNR-51 / PLNR-52 / PLNR-53: Schedule
/// gives Orison a Scheduled one-leg plan that reserves its delta-v, and
/// the inset shows the order with Cancel and Change in place of
/// Transfer. Change opens the planner on the order, counting what it
/// reserved; Schedule replaces it. Cancel releases the reservation and
/// brings Transfer back.
#[test]
fn an_order_is_scheduled_changed_and_cancelled() {
    let mut app = planner_app();
    let (orison, iva) = plan_to_halden_iva_from(&mut app, 8);
    let schedule = button(&mut app, "Schedule").expect("Schedule");
    assert!(!app.world().get::<ConsoleButton>(schedule).unwrap().disabled);
    let leg_kms = session(&app).solved_leg().unwrap().delta_v_kms;
    click_node(&mut app, schedule, 1);
    settle(&mut app);
    assert!(!planner(&app).is_open());
    let first = plan(&app, orison).clone();
    assert_eq!(first.status, Status::Scheduled);
    assert_eq!(
        first.leg().destination,
        Place::Body {
            body: iva,
            band: Band::High
        }
    );
    assert!((account(&app, orison).reserved_kms - leg_kms).abs() < 1e-9);
    assert_eq!(fields(&mut app)[&Field::OrderHeading], "→ Halden IVa");
    assert!(facts(&mut app)["Delta-v"].contains("reserved"));
    assert_eq!(button(&mut app, "Transfer"), None);

    let change = button(&mut app, "Change").expect("Change");
    click_node(&mut app, change, 1);
    settle(&mut app);
    let session_now = session(&app);
    assert_eq!(session_now.changing, Some(leg_kms));
    assert_eq!(
        session_now.setting.unwrap().depart_day,
        first.leg().depart_day
    );
    let context = session_now
        .found
        .as_ref()
        .unwrap()
        .context
        .as_ref()
        .unwrap();
    assert!(
        (context.available_kms - 9.9).abs() < 1e-9,
        "the reservation counts"
    );
    crate::view::planner::input::step(
        &mut app.world_mut().resource_mut::<PlannerState>(),
        SliderKind::Depart,
        3,
    );
    settle(&mut app);
    let new_depart = session(&app).setting.unwrap().depart_day;
    let schedule = button(&mut app, "Schedule").unwrap();
    click_node(&mut app, schedule, 1);
    settle(&mut app);
    let second = plan(&app, orison).clone();
    assert_eq!(second.status, Status::Scheduled);
    assert_eq!(second.leg().depart_day, new_depart);
    assert!((account(&app, orison).reserved_kms - second.reserved_kms()).abs() < 1e-9);

    let cancel = button(&mut app, "Cancel").expect("Cancel");
    click_node(&mut app, cancel, 1);
    settle(&mut app);
    assert_eq!(plan(&app, orison).status, Status::Failed);
    assert_eq!(account(&app, orison).reserved_kms, 0.0);
    assert!(button(&mut app, "Transfer").is_some());
}

/// PLNR-53: an order that departs while it is being changed closes the
/// planner at once, and a line at the top of the inset says so until the
/// selection changes.
#[test]
fn an_order_that_departs_while_changed_closes_the_planner() {
    let mut app = planner_app();
    let (orison, _) = plan_to_halden_iva_from(&mut app, 8);
    let schedule = button(&mut app, "Schedule").unwrap();
    click_node(&mut app, schedule, 1);
    settle(&mut app);
    let depart = plan(&app, orison).leg().depart_day;
    let change = button(&mut app, "Change").unwrap();
    click_node(&mut app, change, 1);
    settle(&mut app);
    assert!(planner(&app).is_open());
    run_until(&mut app, 1.0, depart + 0.01);
    run(&mut app, 2);
    assert!(!planner(&app).is_open());
    let mut notice = app
        .world_mut()
        .query_filtered::<(Entity, &Text), With<NoticeLine>>();
    let (line, text) = notice.single(app.world()).unwrap();
    assert_eq!(text.0, "Departed while you were changing it");
    assert!(shown(&app, line));
    let other = body(&mut app, "Ardent");
    select(&mut app, other);
    assert_eq!(planner(&app).notice, None);
}

/// PLNR-40 / PLNR-41 / PLNR-42: Ardent shares Orison's band, so it is met
/// by a phasing leg: it departs now, its travel time counts laps, there
/// is no Rendezvous / Intercept choice and only Fastest and Cheapest, and
/// a Gap line says how far it is. Orison's own band is already reached.
#[test]
fn a_target_in_the_ships_own_orbit_is_met_by_phasing() {
    let mut app = planner_app();
    let orison = body(&mut app, "Orison");
    let ardent = body(&mut app, "Ardent");
    select(&mut app, orison);
    app.world_mut().resource_mut::<PlannerState>().open(orison);
    run(&mut app, 1);
    app.world_mut()
        .resource_mut::<PlannerState>()
        .pick(PickTarget::Body(ardent));
    settle(&mut app);
    let texts = fields(&mut app);
    assert_eq!(texts[&Field::DepartLabel], "Departs");
    assert_eq!(texts[&Field::DepartValue], "now");
    assert!(
        texts[&Field::TravelValue].contains(" lap"),
        "{}",
        texts[&Field::TravelValue]
    );
    let gap = &texts[&Field::Gap];
    assert!(gap.starts_with("Gap ") && (gap.ends_with(" ahead") || gap.ends_with(" behind")));
    let mut options = app.world_mut().query::<&Choice>();
    assert!(
        options
            .iter(app.world())
            .all(|c| !matches!(c, Choice::Arrival { .. }))
    );
    assert_eq!(button(&mut app, "Next window"), None);

    let high = app.world().get::<InNamedOrbit>(orison).unwrap().0;
    app.world_mut()
        .resource_mut::<PlannerState>()
        .pick(PickTarget::Orbit(high));
    settle(&mut app);
    assert_eq!(fields(&mut app)[&Field::Reason], "Already there");
    let mut schedule = app
        .world_mut()
        .query_filtered::<&ConsoleButton, With<ScheduleButton>>();
    assert!(schedule.single(app.world()).unwrap().disabled);
}

/// PLNR-54: after its Intercept of Iver, Orison coasts; its inset says
/// where from and what it intercepted, and has no Transfer button.
#[test]
fn a_coasting_ship_says_what_it_intercepted() {
    let mut app = planner_app();
    let orison = body(&mut app, "Orison");
    let iver = body(&mut app, "Iver");
    let order = cheapest_intercept_of_iver(&mut app, orison, iver);
    schedule(&mut app, orison, order).unwrap();
    let end = plan(&app, orison).leg().crossing_end_day;
    run_until(&mut app, 20.0, end + 1.0);
    pause(&mut app);
    select(&mut app, orison);
    run(&mut app, 2);
    assert_eq!(
        fields(&mut app)[&Field::Coasting],
        "Coasting · intercepted Iver, from Halden IV"
    );
    assert_eq!(button(&mut app, "Transfer"), None);
}

/// PLNR-91 / PLNR-94: resting the pointer on the Delta-v label or on
/// Intercept shows its tooltip once `tooltip_delay_ms` has passed, and
/// not before.
#[test]
fn tooltips_show_after_the_delay() {
    use bevy::picking::backend::HitData;
    use bevy::picking::hover::HoverMap;
    use bevy::picking::pointer::PointerId;

    let mut app = planner_app();
    plan_to_halden_iva(&mut app);
    for text in [DELTA_V_TOOLTIP, INTERCEPT_TOOLTIP] {
        let mut labels = app.world_mut().query::<(Entity, &Tooltip)>();
        let label = labels
            .iter(app.world())
            .find(|(_, t)| t.0 == text)
            .map(|(e, _)| e)
            .expect("a label with the tooltip");
        let camera = app.world_mut().spawn_empty().id();
        let mut map = HoverMap::default();
        map.0.insert(
            PointerId::Mouse,
            [(label, HitData::new(camera, 0.0, None, None))]
                .into_iter()
                .collect(),
        );
        app.insert_resource(map);
        let tooltip_box = |app: &mut App| {
            let mut boxes = app
                .world_mut()
                .query_filtered::<(&Node, &Children), With<TooltipBox>>();
            let (node, children) = boxes.single(app.world()).unwrap();
            let text = app.world().get::<Text>(children[0]).unwrap().0.clone();
            (node.display, text)
        };
        run(&mut app, 12);
        assert_eq!(tooltip_box(&mut app).0, Display::None, "not yet");
        run(&mut app, 20);
        assert_eq!(tooltip_box(&mut app), (Display::Flex, text.to_string()));
        app.insert_resource(HoverMap::default());
        run(&mut app, 2);
        assert_eq!(tooltip_box(&mut app).0, Display::None);
    }
}
