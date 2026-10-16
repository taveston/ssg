//! Headless tests of the transfer planner's panel (`docs/spec/planner.md`
//! sections 1 to 5): the inset, the outline and the planner's own
//! plugin on Corvane, driven by clicks on nodes, keys and the clock.

use std::collections::HashMap;

use bevy::input::mouse::AccumulatedMouseScroll;

use super::*;
use crate::sim::transfer::grid::Setting;
use crate::view::buttons::ConsoleButton;
use crate::view::camera::requests::PlanetaryViewRequest;
use crate::view::inset::fidelity::FidelityImages;
use crate::view::inset::inspect::{FactLabel, FactValue, InspectPlugin, TransferButton};
use crate::view::inset::outline::{OutlinePlugin, OutlineRow};
use crate::view::planner::PlannerPlugin;
use crate::view::planner::panel::{Choice, Field, NoticeLine, ScheduleButton};
use crate::view::planner::state::{PickTarget, PlannerSession, SliderKind};
use crate::view::planner::tooltip::TooltipBox;
use crate::view::planner::words::{
    CHEAPEST_TOOLTIP, DELTA_V_TOOLTIP, DEPART_STEADY_TOOLTIP, DEPART_TOOLTIP,
    FASTEST_DISABLED_TOOLTIP, FASTEST_TOOLTIP, HIGH_TOOLTIP, INTERCEPT_TOOLTIP, LOW_TOOLTIP,
    NEXT_WINDOW_TOOLTIP, RENDEZVOUS_TOOLTIP, TRAVEL_FIXED_TOOLTIP, TRAVEL_LAPS_TOOLTIP,
    TRAVEL_TOOLTIP,
};
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
        .find(|(_, t)| t.0 == crate::view::planner::words::CHOOSE_PROMPT)
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

/// The search's snapshot.
pub(super) fn context(app: &App) -> &crate::sim::transfer::search::SearchContext {
    session(app)
        .found
        .as_ref()
        .and_then(|found| found.context.as_ref())
        .expect("a search")
}

/// Whether the solver accepts `setting` for the open planner's request.
fn accepted(app: &App, setting: Setting) -> bool {
    context(app)
        .solve(session(app).request.unwrap(), setting)
        .is_ok()
}

/// PLNR-13 / PLNR-33: changing Rendezvous to Intercept keeps the
/// settings, except a thumb the change leaves on a setting the solver
/// rejects: on Halden IVa's Soonest some travel times can be met but not
/// intercepted, and from one of them the Travel time thumb moves to an
/// accepted setting while Depart, still accepted, stays.
#[test]
fn an_arrival_change_moves_a_thumb_off_a_setting_it_rejects() {
    use crate::sim::transfer::search::Request;
    use crate::view::planner::input::intercept_arrival;
    let mut app = planner_app();
    plan_to_halden_iva(&mut app);
    let rendezvous = session(&app).request.unwrap();
    let intercept = Request {
        arrival: intercept_arrival(rendezvous.destination),
        ..rendezvous
    };
    let soonest_column = session(&app).search().unwrap().grid.rows[0].clone();
    let setting = soonest_column
        .iter()
        .filter(|sample| sample.accepted())
        .map(|sample| sample.setting)
        .find(|&setting| context(&app).solve(intercept, setting).is_err())
        .expect("a travel time Halden IVa can be met at but not intercepted");
    put_thumbs(&mut app, setting);
    assert_eq!(
        session(&app).setting,
        Some(setting),
        "accepted as a Rendezvous"
    );

    let option = option(&mut app, Choice::Arrival { intercept: true });
    click_node(&mut app, option, 1);
    settle(&mut app);
    assert_eq!(session(&app).request, Some(intercept));
    let moved = session(&app).setting.unwrap();
    assert_eq!(moved.depart_day, setting.depart_day, "Depart stays");
    assert_ne!(moved.travel_days, setting.travel_days, "Travel time moves");
    assert!(accepted(&app, moved), "{moved:?}");
    // It moves to the nearest accepted travel time: every setting closer
    // to the old one is rejected.
    let moved_by = (moved.travel_days - setting.travel_days).abs();
    let max_steps = context(&app).tunables.slider_max_steps;
    let travels = session(&app)
        .search()
        .unwrap()
        .sliders
        .travel
        .settings(max_steps);
    for travel_days in travels {
        let closer = Setting {
            travel_days,
            ..moved
        };
        if (travel_days - setting.travel_days).abs() < moved_by {
            assert!(!accepted(&app, closer), "{closer:?} is nearer");
        }
    }
}

/// Whether the preset button labelled `label` is drawn selected.
fn preset_selected(app: &mut App, label: &str) -> bool {
    let preset = button(app, label).expect("the preset");
    app.world().get::<ConsoleButton>(preset).unwrap().selected
}

/// PLNR-23 / PLNR-29 / PLNR-33 / PLNR-87: a focused slider takes Left and
/// Right a step at a time, ten with Shift, and never comes to rest on a
/// setting the solver rejects; a press on a rejected stretch of the
/// Travel time track lands on an accepted setting. Moving a slider solves
/// the leg but doesn't search again, and the Travel time track follows
/// the depart day. The preset the sliders start on is drawn selected
/// until a slider moves off it.
#[test]
fn sliders_step_and_never_rest_on_a_rejected_setting() {
    use crate::sim::transfer::ranges::Slider;
    let mut app = planner_app();
    plan_to_halden_iva(&mut app);
    assert!(preset_selected(&mut app, "Fastest"));
    let searched = session(&app).found.as_ref().unwrap().key;
    let Slider::Range { start, end } = session(&app).search().unwrap().sliders.travel.clone()
    else {
        panic!("a travel range");
    };
    let step = crate::sim::transfer::ranges::slider_step_days(end - start, 400);
    let before = session(&app).setting.unwrap();

    app.world_mut().resource_mut::<PlannerState>().slider_focus = Some(SliderKind::Travel);
    press(&mut app, &[KeyCode::ArrowRight]);
    settle(&mut app);
    let after = session(&app).setting.unwrap();
    assert!(after.travel_days >= before.travel_days + step - 1e-9);
    assert!(accepted(&app, after));
    let one_on = Setting {
        travel_days: before.travel_days + step,
        ..before
    };
    if accepted(&app, one_on) {
        assert!((after.travel_days - one_on.travel_days).abs() < 1e-9);
    }
    assert!(
        !preset_selected(&mut app, "Fastest"),
        "moved off the preset"
    );
    press(&mut app, &[KeyCode::ShiftLeft, KeyCode::ArrowRight]);
    settle(&mut app);
    let ten_on = session(&app).setting.unwrap();
    assert!(ten_on.travel_days >= after.travel_days + 10.0 * step - 1e-9);
    assert!(accepted(&app, ten_on));
    assert_eq!(session(&app).solved_for, Some(ten_on), "the leg is solved");

    // A press in the middle of a rejected stretch of the Travel time
    // track.
    let track = session(&app).travel_track.clone();
    let rejected = track
        .iter()
        .find(|sample| sample.result.is_err())
        .expect("Halden IVa has rejected travel times")
        .setting;
    let fraction = ((rejected.travel_days - start) / (end - start)) as f32;
    crate::view::planner::input::slide(
        &mut app.world_mut().resource_mut::<PlannerState>(),
        SliderKind::Travel,
        fraction,
    );
    settle(&mut app);
    let landed = session(&app).setting.unwrap();
    assert_ne!(landed.travel_days, rejected.travel_days);
    assert!(accepted(&app, landed), "{landed:?}");
    assert!(session(&app).solved_leg().is_some());

    app.world_mut().resource_mut::<PlannerState>().slider_focus = Some(SliderKind::Depart);
    press(&mut app, &[KeyCode::ArrowRight]);
    settle(&mut app);
    let moved = session(&app).setting.unwrap();
    assert_eq!(moved.depart_day, 1.0, "the next whole day");
    assert!(accepted(&app, moved));
    assert_eq!(session(&app).travel_track_day, Some(moved.depart_day));
    assert_eq!(
        session(&app).found.as_ref().unwrap().key,
        searched,
        "no new search"
    );
}

/// PLNR-21 / PLNR-24 / PLNR-87: a depart day is a whole day, so as the
/// clock runs a Depart position keeps its day: the search is not made
/// again, Soonest moves on to the clock, and the grid keeps the columns
/// it had for the days still to come. Once the clock reaches the thumb's
/// day, the thumb moves to Soonest and the travel time stays.
#[test]
fn positions_keep_their_day_as_the_clock_runs() {
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
    // Back to Soonest, then on to day 3.
    for steps in [-1000, 3] {
        crate::view::planner::input::step(
            &mut app.world_mut().resource_mut::<PlannerState>(),
            SliderKind::Depart,
            steps,
        );
        settle(&mut app);
    }
    let setting = session(&app).setting.unwrap();
    assert_eq!(setting.depart_day, 3.0);
    let key = session(&app).found.as_ref().unwrap().key;
    let grid = session(&app).search().unwrap().grid.clone();
    assert!(day(&app) < 1.0);

    run_until(&mut app, 1.0, 1.5);
    pause(&mut app);
    run(&mut app, 2);
    let now = day(&app);
    let search = session(&app).search().unwrap().clone();
    assert_eq!(
        session(&app).found.as_ref().unwrap().key,
        key,
        "no new search"
    );
    assert_eq!(
        session(&app).setting.unwrap(),
        setting,
        "the thumb keeps its day"
    );
    assert!(search.soonest() >= 1.0 && search.soonest() <= now);
    assert_eq!(context(&app).today, search.soonest());
    let days = search.grid.depart_days();
    assert!(
        !days.contains(&grid.depart_days()[0]),
        "the old Soonest left"
    );
    for day in days.iter().skip(1).filter(|d| grid.column(**d).is_some()) {
        assert_eq!(
            search.grid.column(*day),
            grid.column(*day),
            "day {day} kept"
        );
    }
    let positions = search.sliders.depart.settings(400);
    assert!(positions.contains(&3.0));
    assert!(!positions.contains(&1.0), "day 1 has come");

    run_until(&mut app, 1.0, 3.2);
    pause(&mut app);
    run(&mut app, 2);
    let moved = session(&app).setting.unwrap();
    let soonest = session(&app).search().unwrap().soonest();
    assert!(soonest >= 3.0);
    assert_eq!(moved.depart_day, soonest, "to Soonest");
    assert_eq!(
        moved.travel_days, setting.travel_days,
        "the travel time stays"
    );
}

/// PLNR-50: Schedule at Soonest solves the order again from the clock,
/// so it departs the moment Schedule is pressed, not when the planner
/// last solved Soonest.
#[test]
fn schedule_from_soonest_solves_from_the_clock() {
    let mut app = planner_app();
    let (orison, _) = plan_to_halden_iva(&mut app);
    let soonest = session(&app).search().unwrap().soonest();
    assert_eq!(session(&app).setting.unwrap().depart_day, soonest);
    run_until(&mut app, 0.05, soonest + 0.2);
    pause(&mut app);
    run(&mut app, 1);
    let pressed = day(&app);
    assert!(pressed.floor() == soonest.floor() && pressed > soonest + 0.1);
    let schedule = button(&mut app, "Schedule").expect("Schedule");
    click_node(&mut app, schedule, 1);
    settle(&mut app);
    assert!(!planner(&app).is_open());
    let leg = plan(&app, orison).leg();
    assert_eq!(leg.depart_day, pressed);
}

/// A Soonest setting of the open planner's leg, and a later time on the
/// same day, for which `changes` holds between the leg solved at Soonest
/// and the leg solved then. The time is checked a little later too, so
/// running the clock to it may overshoot.
pub(super) fn soonest_change(
    app: &App,
    changes: impl Fn(&Result<SolvedLeg, Infeasible>, &Result<SolvedLeg, Infeasible>) -> bool,
) -> Option<(Setting, f64)> {
    let search = session(app).search().unwrap();
    let context = context(app);
    let request = session(app).request.unwrap();
    let soonest = search.soonest();
    let max_steps = context.tunables.slider_max_steps;
    search
        .sliders
        .travel
        .settings(max_steps)
        .into_iter()
        .flat_map(|travel_days| {
            [0.3, 0.5, 0.7].map(|fraction| {
                let at = Setting {
                    depart_day: soonest,
                    travel_days,
                };
                (at, soonest + fraction)
            })
        })
        .find(|&(at, then)| {
            let first = context.solve(request, at);
            [then, then + 0.02].iter().all(|&day| {
                let later = Setting {
                    depart_day: day,
                    ..at
                };
                changes(&first, &context.solve(request, later))
            })
        })
}

/// How much more than the leg at Soonest needs the ship is given, in
/// km/s, when the leg solved later must be over its budget.
const BUDGET_GAP_KMS: f64 = 0.05;

/// Puts the open planner's thumbs on `setting` and lets it be solved.
fn put_thumbs(app: &mut App, setting: Setting) {
    let mut state = app.world_mut().resource_mut::<PlannerState>();
    state.session.as_mut().unwrap().setting = Some(setting);
    settle(app);
}

/// PLNR-50: Schedule at Soonest solves the leg again from the clock; when
/// that leg is rejected, or needs more than the ship has, no order is
/// made, the planner stays open with its thumbs at the new Soonest, and
/// the reason line says why. Halden IVa's legs change within a day: for
/// the rejected case a setting accepted and affordable at Soonest but
/// rejected later is found; for the over-budget case, one that costs
/// more later, with Orison's budget set between the two costs.
#[test]
fn schedule_from_soonest_makes_no_order_when_the_leg_solved_again_fails() {
    for over_budget in [false, true] {
        let mut app = planner_app();
        let (orison, _) = plan_to_halden_iva(&mut app);
        let budget = context(&app).available_kms;
        // Over budget: a leg that costs more later in the day, with the
        // ship's budget then set between the two costs.
        let found = soonest_change(&app, |first, later| match (first, later) {
            (Ok(first), Ok(later)) if over_budget => {
                first.delta_v_kms + 2.0 * BUDGET_GAP_KMS < later.delta_v_kms.min(budget)
            }
            (Ok(first), Err(_)) if !over_budget => first.delta_v_kms <= budget,
            _ => false,
        });
        let (setting, then) =
            found.unwrap_or_else(|| panic!("{over_budget}: a leg that fails later the same day"));
        if over_budget {
            let request = session(&app).request.unwrap();
            let first = context(&app).solve(request, setting).unwrap();
            let squeezed = first.delta_v_kms + BUDGET_GAP_KMS;
            let mut account = app.world_mut().get_mut::<DeltaV>(orison).unwrap();
            account.delta_v_kms = squeezed + account.reserved_kms + account.spent_kms;
            settle(&mut app);
            let available = context(&app).available_kms;
            assert!((available - squeezed).abs() < 1e-9, "searched again");
        }
        put_thumbs(&mut app, setting);
        assert_eq!(session(&app).setting, Some(setting), "{over_budget}");
        assert!(
            crate::view::planner::input::order_to_schedule(planner(&app)).is_some(),
            "Schedule is enabled at Soonest"
        );
        run_until(&mut app, 0.05, then);
        pause(&mut app);
        run(&mut app, 1);
        let pressed = day(&app);
        assert_eq!(pressed.floor(), setting.depart_day.floor(), "the same day");
        let schedule = button(&mut app, "Schedule").expect("Schedule");
        click_node(&mut app, schedule, 1);
        settle(&mut app);
        assert!(planner(&app).is_open(), "{over_budget}: the planner stays");
        assert!(
            app.world().get::<Plan>(orison).is_none(),
            "{over_budget}: no order"
        );
        assert_eq!(
            session(&app).setting.unwrap().depart_day,
            pressed,
            "the thumbs are at the new Soonest"
        );
        let reason = fields(&mut app)[&Field::Reason].clone();
        if over_budget {
            assert!(reason.starts_with("Needs "), "{reason}");
        } else {
            assert!(session(&app).leg.as_ref().unwrap().is_err());
            assert!(
                !reason.is_empty() && !reason.starts_with("Needs "),
                "{reason}"
            );
        }
    }
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

/// PLNR-96 / PLNR-103: no Rendezvous at Iver is within Orison's budget,
/// so Fastest is disabled, and its tooltip says why and points to
/// Cheapest. Iver's leave days differ in cost, so the Depart label
/// explains windows.
#[test]
fn a_disabled_fastest_says_why() {
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
    let fastest = button(&mut app, "Fastest").expect("Fastest, disabled");
    assert!(app.world().get::<ConsoleButton>(fastest).unwrap().disabled);
    assert_tooltip(&mut app, fastest, FASTEST_DISABLED_TOOLTIP);
    let depart = slider_label(&mut app, SliderKind::Depart);
    assert_tooltip(&mut app, depart, DEPART_TOOLTIP);
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
/// by a phasing leg: it leaves today at Soonest, its travel time counts
/// laps, there
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
    assert_eq!(texts[&Field::DepartLabel], "Depart");
    assert_eq!(texts[&Field::DepartValue], "today");
    let soonest = session(&app).search().unwrap().soonest();
    assert_eq!(session(&app).setting.unwrap().depart_day, soonest);
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

/// The tooltip box: whether it shows, and its text.
fn tooltip_box(app: &mut App) -> (Display, String) {
    let mut boxes = app
        .world_mut()
        .query_filtered::<(&Node, &Children), With<TooltipBox>>();
    let (node, children) = boxes.single(app.world()).unwrap();
    let text = app.world().get::<Text>(children[0]).unwrap().0.clone();
    (node.display, text)
}

/// Rests the mouse on `node` until just before `tooltip_delay_ms` has
/// passed and then until just after it, and returns the tooltip box at
/// each time; then moves the mouse off.
fn rest_on(app: &mut App, node: Entity) -> ((Display, String), (Display, String)) {
    use bevy::picking::backend::HitData;
    use bevy::picking::hover::HoverMap;
    use bevy::picking::pointer::PointerId;

    let camera = app.world_mut().spawn_empty().id();
    let mut map = HoverMap::default();
    map.0.insert(
        PointerId::Mouse,
        [(node, HitData::new(camera, 0.0, None, None))]
            .into_iter()
            .collect(),
    );
    app.insert_resource(map);
    run(app, 12);
    let before = tooltip_box(app);
    run(app, 20);
    let after = tooltip_box(app);
    app.insert_resource(HoverMap::default());
    run(app, 2);
    assert_eq!(tooltip_box(app).0, Display::None, "gone once off it");
    (before, after)
}

/// The text node a control shows its name in: the label itself, or the
/// text inside a button.
fn text_of(app: &App, control: Entity) -> Entity {
    if app.world().get::<Text>(control).is_some() {
        return control;
    }
    app.world().get::<Children>(control).unwrap()[0]
}

/// The shown label of `kind`'s slider row.
fn slider_label(app: &mut App, kind: SliderKind) -> Entity {
    use crate::view::planner::panel::SliderLabel;
    let mut labels = app.world_mut().query::<(Entity, &SliderLabel)>();
    labels
        .iter(app.world())
        .find(|(_, l)| l.0 == kind)
        .map(|(e, _)| e)
        .expect("the slider's label")
}

/// The label with text `text`.
fn label(app: &mut App, text: &str) -> Entity {
    let mut labels = app.world_mut().query::<(Entity, &Text)>();
    labels
        .iter(app.world())
        .find(|(_, t)| t.0 == text)
        .map(|(e, _)| e)
        .expect("the label")
}

/// Asserts that resting the mouse on `control`'s name shows `text` once
/// `tooltip_delay_ms` has passed, and not before.
fn assert_tooltip(app: &mut App, control: Entity, text: &str) {
    let name = text_of(app, control);
    let (before, after) = rest_on(app, name);
    assert_eq!(before.0, Display::None, "not yet: {text}");
    assert_eq!(after, (Display::Flex, text.to_string()));
}

/// PLNR-91 / PLNR-94 / PLNR-95 / PLNR-97 to PLNR-102 / PLNR-106:
/// resting the pointer on the Depart and Travel time labels, the Delta-v
/// label, High, Low, each preset shown, Rendezvous and Intercept shows
/// that control's tooltip once `tooltip_delay_ms` has passed, and not
/// before.
#[test]
fn each_control_shows_its_tooltip_after_the_delay() {
    let mut app = planner_app();
    plan_to_halden_iva(&mut app);
    run(&mut app, 2);
    let depart = slider_label(&mut app, SliderKind::Depart);
    // Every day to Halden IVa costs about the same (PLNR-106).
    assert_tooltip(&mut app, depart, DEPART_STEADY_TOOLTIP);
    let travel = slider_label(&mut app, SliderKind::Travel);
    assert_tooltip(&mut app, travel, TRAVEL_TOOLTIP);
    let delta_v = label(&mut app, "Delta-v");
    assert_tooltip(&mut app, delta_v, DELTA_V_TOOLTIP);
    for (choice, text) in [
        (Choice::Band(Band::High), HIGH_TOOLTIP),
        (Choice::Band(Band::Low), LOW_TOOLTIP),
        (Choice::Arrival { intercept: false }, RENDEZVOUS_TOOLTIP),
        (Choice::Arrival { intercept: true }, INTERCEPT_TOOLTIP),
    ] {
        let control = option(&mut app, choice);
        assert_tooltip(&mut app, control, text);
    }
    let mut presets = 0;
    for (name, text) in [
        ("Fastest", FASTEST_TOOLTIP),
        ("Cheapest", CHEAPEST_TOOLTIP),
        ("Next window", NEXT_WINDOW_TOOLTIP),
    ] {
        if let Some(control) = button(&mut app, name) {
            assert_tooltip(&mut app, control, text);
            presets += 1;
        }
    }
    assert!(presets >= 2, "the presets shown");
}

/// PLNR-91: while the pointer rests where it is, a shown tooltip box is
/// left alone: its node isn't written again each frame.
#[test]
fn a_resting_tooltip_box_is_not_rewritten() {
    use bevy::picking::backend::HitData;
    use bevy::picking::hover::HoverMap;
    use bevy::picking::pointer::PointerId;

    let mut app = planner_app();
    plan_to_halden_iva(&mut app);
    run(&mut app, 2);
    let delta_v = label(&mut app, "Delta-v");
    let camera = app.world_mut().spawn_empty().id();
    let mut map = HoverMap::default();
    map.0.insert(
        PointerId::Mouse,
        [(delta_v, HitData::new(camera, 0.0, None, None))]
            .into_iter()
            .collect(),
    );
    app.insert_resource(map);
    run(&mut app, 40);
    assert_eq!(tooltip_box(&mut app).0, Display::Flex);
    let mut boxes = app.world_mut().query_filtered::<Entity, With<TooltipBox>>();
    let tip = boxes.single(app.world()).unwrap();
    let written = |app: &App| {
        app.world()
            .entity(tip)
            .get_change_ticks::<Node>()
            .unwrap()
            .changed
    };
    let before = written(&app);
    run(&mut app, 5);
    assert_eq!(written(&app), before);
}

/// PLNR-105 / PLNR-106: Ardent is met by phasing, so the Travel time
/// label speaks of laps, and since every day costs the same the Depart
/// label says only what Depart is.
#[test]
fn a_phasing_legs_labels_speak_of_laps_and_not_of_windows() {
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
    let travel = slider_label(&mut app, SliderKind::Travel);
    assert_tooltip(&mut app, travel, TRAVEL_LAPS_TOOLTIP);
    let depart = slider_label(&mut app, SliderKind::Depart);
    assert_tooltip(&mut app, depart, DEPART_STEADY_TOOLTIP);
}

/// PLNR-104: RFA Tolland's drive is low-thrust, so its Travel time label
/// says the drive sets the travel time.
#[test]
fn a_low_thrust_drives_travel_time_says_the_drive_sets_it() {
    let mut app = planner_app();
    let tolland = body(&mut app, "RFA Tolland");
    let iva = body(&mut app, "Halden IVa");
    select(&mut app, tolland);
    app.world_mut().resource_mut::<PlannerState>().open(tolland);
    run(&mut app, 1);
    app.world_mut()
        .resource_mut::<PlannerState>()
        .pick(PickTarget::Body(iva));
    settle(&mut app);
    assert_eq!(
        fields(&mut app)[&Field::TravelNote],
        "Travel time is fixed by this drive"
    );
    let travel = slider_label(&mut app, SliderKind::Travel);
    assert_tooltip(&mut app, travel, TRAVEL_FIXED_TOOLTIP);
}

/// PLNR-35 / PLNR-92: resting the pointer on a rejected stretch of the
/// Travel time track shows, after `tooltip_delay_ms`, the solver's reason
/// there as a sentence; on an accepted stretch it shows nothing.
#[test]
fn a_rejected_stretch_of_a_track_says_why() {
    use crate::view::planner::panel::Track;
    use crate::view::planner::tooltip::Stretches;
    use bevy::picking::backend::HitData;
    use bevy::picking::hover::HoverMap;
    use bevy::picking::pointer::PointerId;

    let mut app = planner_app();
    plan_to_halden_iva(&mut app);
    run(&mut app, 2);
    let mut tracks = app.world_mut().query::<(Entity, &Track, &Stretches)>();
    let (track, stretch) = tracks
        .iter(app.world())
        .find(|(_, t, _)| t.0 == SliderKind::Travel)
        .map(|(e, _, s)| (e, s.0.first().cloned().expect("a rejected stretch")))
        .unwrap();
    assert_eq!(stretch.text, "Path passes too close to Halden IV");
    let camera = app.world_mut().spawn_empty().id();
    let hover_at = |app: &mut App, fraction: f32| {
        let at = Vec3::new(fraction - 0.5, 0.0, 0.0);
        let mut map = HoverMap::default();
        map.0.insert(
            PointerId::Mouse,
            [(track, HitData::new(camera, 0.0, Some(at), None))]
                .into_iter()
                .collect(),
        );
        app.insert_resource(map);
    };
    hover_at(&mut app, (stretch.from + stretch.to) / 2.0);
    run(&mut app, 12);
    assert_eq!(tooltip_box(&mut app).0, Display::None, "not yet");
    run(&mut app, 20);
    assert_eq!(tooltip_box(&mut app), (Display::Flex, stretch.text.clone()));
    let thumb = session(&app).setting.unwrap().travel_days;
    let search = session(&app).search().unwrap();
    let crate::sim::transfer::ranges::Slider::Range { start, end } = search.sliders.travel else {
        panic!("a travel range");
    };
    hover_at(&mut app, ((thumb - start) / (end - start)) as f32);
    run(&mut app, 32);
    assert_eq!(tooltip_box(&mut app).0, Display::None, "accepted there");
}
