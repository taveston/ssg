//! Headless tests of a shared orbit's identity mark (MOT-14, MUL-12): what
//! it shows when the orbit holds one station, and what a click on it picks
//! while the transfer planner is open.

use super::*;
use crate::view::multi::{OpenTray, TrayFor};
use crate::view::planner::state::PickTarget;
use crate::view::rings::marks::{IdentityFront, MarkShows, RingMarkFor, RingMarksPlugin};
use crate::view::tray::TrayPlugin;

/// The identity mark drawn for the ring of `band`.
fn mark_of(app: &mut App, band: Entity) -> (Entity, MarkShows) {
    let mut marks = app
        .world_mut()
        .query::<(Entity, &RingMarkFor, &MarkShows)>();
    let found: Vec<(Entity, MarkShows)> = marks
        .iter(app.world())
        .filter(|(_, m, _)| m.owner == band)
        .map(|(e, _, shows)| (e, shows.clone()))
        .collect();
    assert_eq!(found.len(), 1, "one mark per ring");
    found[0].clone()
}

/// MOT-14 / MUL-03: Iver's High band holds only Iver Yard, with Corvid and
/// the Pellam trader docked there. Its identity mark is the station's
/// marker: the yard's symbol in front of a back frame, counting the two
/// docked ships.
#[test]
fn a_shared_orbit_holding_one_station_shows_the_station_marker() {
    use crate::view::multi_markers::draw::{MultiCount, MultiPart};
    let mut app = camera_app_with(|app| {
        app.insert_resource(placeholder_fonts()).add_plugins((
            GroupingPlugin,
            MotionPlugin,
            HoveredPlugin,
            SharedOrbitPlugin,
            RingMarksPlugin,
        ));
    });
    let yard = body(&mut app, "Iver Yard");
    let band = app.world().get::<InNamedOrbit>(yard).unwrap().0;
    let (root, shows) = mark_of(&mut app, band);
    let MarkShows::Multi(IdentityFront::Station(_), look) = shows else {
        panic!("the mark is the station's marker, not {shows:?}");
    };
    assert_eq!(look.count, "2", "the docked count");
    let parts = descendants(&mut app, root);
    let frames = parts
        .iter()
        .filter(|&&e| app.world().get::<MultiPart>(e).is_some())
        .count();
    let count: Vec<String> = parts
        .iter()
        .filter(|&&e| app.world().get::<MultiCount>(e).is_some())
        .filter_map(|&e| app.world().get::<Text>(e).map(|t| t.0.clone()))
        .collect();
    assert_eq!(
        (frames, count),
        (2, vec!["2".to_string()]),
        "one back frame and the count"
    );
}

/// MUL-12 / PLNR-11 / HOV-05: with the planner open and Halden IV's High
/// band an exposure ring, a click on the band's identity mark picks the
/// band, as a click on its ring does, and opens the mark's tray.
#[test]
fn with_the_planner_open_a_shared_orbits_mark_picks_its_band() {
    let mut app = symbols_app_with(|app| {
        app.add_plugins((SharedOrbitPlugin, RingMarksPlugin, TrayPlugin));
    });
    let halden = body(&mut app, "Halden IV");
    let orison = body(&mut app, "Orison");
    let high = app.world().get::<InNamedOrbit>(orison).unwrap().0;
    let mut clock = app.world_mut().resource_mut::<GameClock>();
    clock.toggle_pause();
    clock.run_at(4.0);
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(halden);
    run(&mut app, 60 + eased_frames());
    app.world_mut().resource_mut::<Selection>().select(orison);
    app.world_mut().resource_mut::<PlannerState>().open(orison);
    let (mark, _) = mark_of(&mut app, high);
    click_node(&mut app, mark, 1);
    run(&mut app, 2);
    let clicked = app
        .world_mut()
        .resource_mut::<PlannerState>()
        .clicked
        .take();
    assert_eq!(clicked, Some(PickTarget::Orbit(high)));
    assert_eq!(
        app.world().resource::<OpenTray>().0,
        Some(TrayFor::Orbit(high))
    );
}
