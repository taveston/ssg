//! Headless tests of the simulation and systems as data (ARCH-13,
//! ARCH-31, SCL-03, SOL): the simulation runs without a window, orbital
//! positions after a known time, a generated system through the same
//! code, and Sol.

use super::*;
use crate::sim::body::{Belt, BeltComposition};
use crate::sim::system::{BeltBlueprint, GateBlueprint, SystemData};
use crate::sim::units::{EARTH_MASS_KG, SOLAR_MASS_KG};

/// ARCH-13: the simulation runs headless with `MinimalPlugins` and the sim
/// plugin alone - with no content it simply starts empty.
#[test]
fn simulation_runs_headless() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, SimPlugin));
    run(&mut app, 5);
    assert!(app.world().get_resource::<GameClock>().is_some());

    let mut app = App::new();
    app.add_plugins((MinimalPlugins, ContentPlugin, SimPlugin));
    run(&mut app, 5);
    let mut orbits = app.world_mut().query::<&Orbit>();
    assert!(orbits.iter(app.world()).count() > 20, "Corvane spawned");
}

/// ARCH-11 / ARCH-31 / SCL-04: after a known time, every body is where its
/// orbit says, in km, read from the sim component with no `Transform`.
#[test]
fn orbital_positions_after_a_known_time() {
    // Tenth-second frames: Bevy's virtual time caps a frame at 250 ms.
    let mut app = headless_app(Duration::from_millis(100));
    app.add_plugins((ContentPlugin, SimPlugin));
    run(&mut app, 101);

    let elapsed = app.world().resource::<GameClock>().elapsed_days();
    // One day per second at the default rate; the very first update has no
    // previous frame to measure from.
    assert!(
        (9.9..=10.1).contains(&elapsed),
        "{elapsed} days after 101 tenth-second frames"
    );

    let mut bodies = app.world_mut().query::<(&Orbit, &OrbitPositionKm)>();
    for (orbit, position) in bodies.iter(app.world()) {
        let expected = orbit_position_km(orbit, elapsed);
        assert!(
            (position.0 - expected).length() < 1e-3,
            "a body is {} km off its orbit",
            (position.0 - expected).length()
        );
    }

    // And in absolute terms: Halden IV is on its 2 AU orbit, and Arden
    // Highport, in the Low band (TEST-02), has gone round about 150 times
    // but is still 7,200 km from its planet; Orison sits on its named
    // orbit "High" (SCL-07).
    let halden = body(&mut app, "Halden IV");
    let halden_km = app.world().get::<OrbitPositionKm>(halden).unwrap().0;
    assert!((halden_km.length() / KM_PER_AU - 2.0).abs() < 1e-9);
    let highport = body(&mut app, "Arden Highport");
    let highport_km = app.world().get::<OrbitPositionKm>(highport).unwrap().0;
    assert!((highport_km.length() - 7_200.0).abs() < 1e-6);
    let orison = body(&mut app, "Orison");
    let orison_km = app.world().get::<OrbitPositionKm>(orison).unwrap().0;
    let high = app.world().get::<InNamedOrbit>(orison).unwrap().0;
    let high = app.world().get::<NamedOrbit>(high).unwrap();
    assert_eq!((high.parent, high.name.as_str()), (halden, "High"));
    assert!((orison_km.length() - high.radius_km).abs() < 1e-6);
}

/// A small system that isn't Corvane, built the way a generator would: a
/// planet with a moon, a ship in transit round the star, and a gate.
fn generated_system() -> SystemData {
    let mut planet = blueprint("Tessaly", BodyClass::Planet, 1.5 * KM_PER_AU);
    planet.mass_kg = EARTH_MASS_KG;
    planet.children = vec![blueprint("Tessaly a", BodyClass::Moon, 200_000.0)];
    SystemData {
        star_name: "Vey",
        star_class: "G5V",
        star_mass_kg: SOLAR_MASS_KG,
        star_radius_km: 696_000.0,
        star_knowledge: Knowledge::Unsurveyed,
        planets: vec![
            planet,
            blueprint("Courier", BodyClass::Ship, 0.9 * KM_PER_AU),
        ],
        belts: vec![BeltBlueprint {
            name: "Vey belt".to_string(),
            inner_radius_km: 0.5 * KM_PER_AU,
            outer_radius_km: 0.6 * KM_PER_AU,
            mass_kg: Some(2.0e21),
            composition: Some(BeltComposition {
                rocky: 0.7,
                metallic: 0.2,
                icy: 0.1,
            }),
            salvage_sites: vec![],
        }],
        gates: vec![GateBlueprint {
            name: "Gate Vey",
            destination: "Ostra",
            radius_km: 5.0,
            distance_km: 4.0 * KM_PER_AU,
            angle_deg: 30.0,
            entry_tolerance_kms: 0.5,
        }],
    }
}

/// SCL-03 / GRP-08: a system from other records runs through the same
/// simulation and view code - bodies placed, zoom stops from its own
/// orbits, its moon folded at the system fit - and its ship in transit,
/// which orbits the star, never gets fold state.
#[test]
fn another_system_runs_through_the_same_code() {
    let mut app = headless_app(FRAME);
    app.insert_resource(SystemDataResource(Some(generated_system())))
        .add_plugins(SimPlugin)
        .add_plugins((
            TunablesPlugin,
            ProjectionPlugin,
            StopsPlugin,
            GroupingPlugin,
        ));
    run(&mut app, 2);
    app.world_mut().resource_mut::<GameClock>().toggle_pause();
    let margin = app.world().resource::<Tunables>().zoom_fit_margin;
    let system_radius = app.world().resource::<ZoomStops>().system_radius;
    assert!((system_radius - km_to_render(1.5 * KM_PER_AU)).abs() < system_radius * 1e-5);
    let distance = fit_distance(system_radius, fov(), TILT, margin);
    app.world_mut().spawn(test_camera(Vec3::ZERO, distance));
    run(&mut app, SETTLE_FRAMES);

    let tessaly = body(&mut app, "Tessaly");
    assert!(
        app.world()
            .resource::<WorldPositions>()
            .0
            .contains_key(&tessaly)
    );
    let moon = body(&mut app, "Tessaly a");
    assert!(app.world().get::<SatelliteFoldState>(moon).unwrap().folded);
    let courier = body(&mut app, "Courier");
    assert!(
        app.world().get::<SatelliteFoldState>(courier).is_none(),
        "GRP-08: a ship in transit never folds"
    );
    body(&mut app, "Gate Vey");
}

/// The belt records a spawned system holds, by name, innermost first.
fn spawned_belts(app: &mut App) -> Vec<Belt> {
    let mut belts = app.world_mut().query::<&Belt>();
    let mut belts: Vec<Belt> = belts.iter(app.world()).cloned().collect();
    belts.sort_by(|a, b| a.inner_radius_km.total_cmp(&b.inner_radius_km));
    belts
}

/// SCL-08: a system holds any number of belts. Two spawn as two belt
/// records, each with its name, edges, mass and composition and the
/// salvage sites placed round the star; with none, none spawn.
#[test]
fn a_system_spawns_each_of_its_belts_or_none() {
    let mut system = generated_system();
    let site = blueprint("Drift", BodyClass::SalvageSite, 2.5 * KM_PER_AU);
    system.belts.push(BeltBlueprint {
        name: "Vey outer belt".to_string(),
        inner_radius_km: 2.2 * KM_PER_AU,
        outer_radius_km: 2.9 * KM_PER_AU,
        mass_kg: None,
        composition: None,
        salvage_sites: vec![site],
    });
    let mut app = headless_app(FRAME);
    app.insert_resource(SystemDataResource(Some(system)))
        .add_plugins(SimPlugin);
    run(&mut app, 1);
    let belts = spawned_belts(&mut app);
    let names: Vec<&str> = belts.iter().map(|b| b.name.as_str()).collect();
    assert_eq!(names, ["Vey belt", "Vey outer belt"]);
    assert_eq!(belts[0].mass_kg, Some(2.0e21));
    assert_eq!(belts[0].composition.map(|c| c.rocky), Some(0.7));
    assert_eq!(belts[1].inner_radius_km, 2.2 * KM_PER_AU);
    assert_eq!(belts[1].composition, None);
    let drift = body(&mut app, "Drift");
    let orbit = app.world().get::<Orbit>(drift).unwrap();
    assert_eq!(orbit.radius_km, 2.5 * KM_PER_AU);
    assert!(
        app.world().get::<ChildOf>(drift).is_none(),
        "round the star"
    );

    let mut system = generated_system();
    system.belts.clear();
    let mut app = headless_app(FRAME);
    app.insert_resource(SystemDataResource(Some(system)))
        .add_plugins(SimPlugin);
    run(&mut app, 1);
    assert!(spawned_belts(&mut app).is_empty());
}

/// TEST-01 / SOL-07: Corvane spawns one belt, the "Corvane belt", and Sol
/// one, the "Main belt".
#[test]
fn corvane_and_sol_spawn_their_named_belts() {
    let mut app = headless_app(FRAME);
    app.add_plugins((ContentPlugin, SimPlugin));
    run(&mut app, 1);
    let names: Vec<String> = spawned_belts(&mut app)
        .into_iter()
        .map(|b| b.name)
        .collect();
    assert_eq!(names, ["Corvane belt"]);

    let mut app = sol_app();
    run(&mut app, 1);
    let names: Vec<String> = spawned_belts(&mut app)
        .into_iter()
        .map(|b| b.name)
        .collect();
    assert_eq!(names, ["Main belt"]);
}

/// A headless app with Sol waiting to spawn, as `--system sol` gives.
fn sol_app() -> App {
    let mut app = headless_app(FRAME);
    app.insert_resource(SystemDataResource(Some(crate::content::sol::sol_system())))
        .add_plugins(SimPlugin);
    app
}

/// SOL-11: Sol spawns headless as one star, nine planets, twenty-one
/// moons, two stations, two ships and one belt, with no gates; every named
/// orbit (the bands) belongs to a spawned body, and every station and ship
/// is a member of one.
#[test]
fn sol_spawns_its_bodies_stations_ships_and_belt() {
    let mut app = sol_app();
    run(&mut app, 1);

    let mut classes = app.world_mut().query::<&BodyClass>();
    let classes: Vec<BodyClass> = classes.iter(app.world()).copied().collect();
    let count = |class: BodyClass| classes.iter().filter(|c| **c == class).count();
    assert_eq!(count(BodyClass::Star), 1);
    assert_eq!(count(BodyClass::Planet), 9);
    assert_eq!(count(BodyClass::Moon), 21);
    assert_eq!(count(BodyClass::Station), 2);
    assert_eq!(count(BodyClass::Ship), 2);
    assert_eq!(count(BodyClass::Gate), 0);
    assert_eq!(classes.len(), 35, "nothing else");
    for name in ["Perigee Station", "Marlin", "Vestige", "Apogee Station"] {
        let craft = body(&mut app, name);
        let member = app.world().get::<InNamedOrbit>(craft);
        assert!(member.is_some(), "{name} is in no named orbit");
        let orbit = app.world().get::<NamedOrbit>(member.unwrap().0);
        assert!(orbit.is_some(), "{name}'s named orbit does not resolve");
    }
    assert_eq!(spawned_belts(&mut app).len(), 1);

    let mut named = app.world_mut().query::<&NamedOrbit>();
    let parents: Vec<Entity> = named.iter(app.world()).map(|orbit| orbit.parent).collect();
    // Two bands each, less the six small moons with no High band (SOL-09).
    assert_eq!(parents.len(), 2 * 31 - 6);
    for parent in parents {
        assert!(app.world().get::<BodyClass>(parent).is_some());
    }
    let ceres = body(&mut app, "Ceres");
    let ceres_km = app.world().get::<Orbit>(ceres).unwrap().radius_km;
    assert!((ceres_km / KM_PER_AU - 2.768).abs() < 1e-9);
}

/// SOL-10: Sol's system fit is Neptune's orbit, 30.07 AU.
#[test]
fn sols_system_fit_is_neptunes_orbit() {
    let mut app = sol_app();
    app.add_plugins(StopsPlugin);
    run(&mut app, 1);
    let stops = app.world().resource::<ZoomStops>();
    let neptune = km_to_render(30.0699 * KM_PER_AU);
    assert!((stops.system_radius - neptune).abs() < neptune * 1e-5);
}
