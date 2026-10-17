//! Headless tests of several objects on one marker (MUL, GRP-05, GRP-07,
//! GRP-09, SYM-10, TEST-11, TEST-12): the fold marker, station markers,
//! things together, symbols that overlap, and what a click on a
//! multi-marker selects.

use super::*;
use crate::view::markers::MarkerDrawn;
use crate::view::multi::BackFrame;
use crate::view::multi_markers::draw::{BackHolder, MultiCount};

/// The figures a multi-marker's count shows, under `root`.
fn count_under(app: &mut App, root: Entity) -> Vec<String> {
    texts_with::<MultiCount>(app, root)
}

/// Whether `body`'s own marker is drawn this frame.
fn drawn(app: &mut App, body: Entity) -> bool {
    let marker = marker_of(app, body);
    app.world().get::<MarkerDrawn>(marker).unwrap().0.is_some()
}

/// The key of the marker `thing` is joined into, if any.
fn joined_into(app: &App, thing: Entity) -> Option<MultiKey> {
    app.world()
        .resource::<MultiMarkers>()
        .joined
        .get(&thing)
        .copied()
}

/// Frame `body` and zoom in a notch at a time until `done` holds, or
/// fail after `notches`.
fn zoom_in_on_until(app: &mut App, body: Entity, notches: usize, done: impl Fn(&mut App) -> bool) {
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(body);
    run(app, 60 + move_frames());
    zoom_in_until(app, notches, done);
}

/// The count holder's display under a multi-marker's node.
fn count_display(app: &mut App, root: Entity) -> Display {
    use crate::view::multi_markers::draw::CountHolder;
    let holder = descendants(app, root)
        .into_iter()
        .find(|&e| app.world().get::<CountHolder>(e).is_some())
        .expect("a count holder");
    app.world().get::<Node>(holder).unwrap().display
}

/// Halden IV's satellites with a track record: its ships, groups and
/// stations, which fold into its object marker rather than its ring.
fn object_satellites(app: &App, parent: Entity) -> Vec<Entity> {
    let children = app.world().get::<Children>(parent).unwrap();
    children
        .iter()
        .filter(|&c| app.world().get::<ShipRecord>(c).is_some())
        .collect()
}

/// GRP-05 / GRP-09 / GRP-12 / MUL-04 / MUL-06 / SYM-16 / MRK-12: at the
/// system fit, with nothing selected and no pointer, Halden IV's ships,
/// groups and stations are held by its object marker, at rest a pip
/// `object_pip_px` across with no count, down and to the right of its
/// marker on a whole pixel, drawn with the symbols. It counts its ships,
/// groups and stations, the task force TF-1 once, and none of its moons.
/// Iver's counts only Iver Yard: the ships docked there are listed under
/// the yard, not counted, but count for its colours (MUL-02).
#[test]
fn a_parent_with_folded_ships_shows_an_object_pip() {
    use crate::sim::group::ShipGroup;
    use crate::view::layers;
    use crate::view::markers::parent_clear_px;

    let mut app = symbols_app_with(|_| {});
    run(&mut app, SETTLE_FRAMES);
    let halden = body(&mut app, "Halden IV");
    let tf1 = body(&mut app, "TF-1");
    let objects = object_satellites(&app, halden);
    let fold = multi_marker(&app, MultiKey::Fold(halden)).expect("Halden IV's object marker");
    assert_eq!(fold.count(), objects.len());
    assert!(fold.things.iter().all(|t| objects.contains(t)), "no moons");
    assert!(fold.things.contains(&tf1));
    let members = app.world().get::<ShipGroup>(tf1).unwrap().members.clone();
    assert!(members.iter().all(|m| !fold.all_things().any(|t| t == *m)));

    let tunables = Tunables::default();
    let pip = tunables.object_pip_px;
    let (node, _) = multi_node(&mut app, MultiKey::Fold(halden)).expect("drawn");
    let drawn = app.world().get::<Node>(node).unwrap().clone();
    assert_eq!(drawn.width, px(pip), "a pip");
    let centre = node_min(&drawn).unwrap() + Vec2::splat(pip / 2.0);
    assert_eq!(centre, centre.round(), "on a whole pixel");
    assert_eq!(centre, fold.centre);
    let marker = app
        .world()
        .resource::<ScreenPositions>()
        .get(halden)
        .unwrap();
    let h = parent_clear_px(BodyClass::Planet, false, 0.0, &tunables);
    let gap = h + tunables.object_pip_gap_px;
    assert!(
        close(centre, (marker + Vec2::splat(gap)).round()),
        "down and right"
    );
    assert_eq!(
        app.world().get::<GlobalZIndex>(node).unwrap().0,
        layers::SYMBOLS
    );
    assert_eq!(count_display(&mut app, node), Display::None, "no count");

    let iver = body(&mut app, "Iver");
    let yard = body(&mut app, "Iver Yard");
    let iver_fold = multi_marker(&app, MultiKey::Fold(iver)).unwrap();
    assert_eq!(iver_fold.things, [yard]);
    assert_eq!(iver_fold.count(), 1);
    assert_eq!(iver_fold.docked[0].0, yard);
    assert_eq!(iver_fold.docked[0].1.len(), 2);
    // MUL-02: the neutral trader docked at the own yard is the most
    // pressing, so the front is neutral and the back own.
    let frame = |affiliation| {
        crate::view::multi_markers::look::affiliation_frame(
            affiliation,
            crate::view::markers::symbol_settings(&tunables),
        )
    };
    assert_eq!(
        (iver_fold.look.front, iver_fold.look.back),
        (frame(Affiliation::Neutral), frame(Affiliation::Own))
    );
}

/// MUL-03 / MUL-13 / SYM-13 / TEST-11: framing Iver unfolds Iver Yard,
/// which, with two ships docked, is a station marker counting 2: its own
/// frame and bar with a back frame and the count, labelled "2 docked".
/// The docked ships draw no symbols of their own. It stays one at every
/// zoom.
#[test]
fn a_station_with_ships_docked_is_a_station_marker_at_every_zoom() {
    let mut app = symbols_app_with(|_| {});
    let iver = body(&mut app, "Iver");
    let yard = body(&mut app, "Iver Yard");
    let corvid = body(&mut app, "Corvid");
    let pellam = body(&mut app, "Pellam trader");
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(iver);
    run(&mut app, 60 + move_frames());

    for notch in 0..4 {
        let station = multi_marker(&app, MultiKey::Station(yard))
            .unwrap_or_else(|| panic!("no station marker after {notch} notches"));
        // Zooming in moves the yard off screen, but it stays one marker.
        assert!(notch > 0 || station.drawn);
        assert_eq!(station.count(), 2);
        assert_eq!(station.things, [yard]);
        assert_eq!(station.docked.len(), 1);
        let (at, mut docked) = station.docked[0].clone();
        docked.sort();
        let mut want = vec![corvid, pellam];
        want.sort();
        assert_eq!((at, docked), (yard, want));
        for ship in [corvid, pellam] {
            assert_eq!(joined_into(&app, ship), Some(MultiKey::Station(yard)));
            assert!(!drawn(&mut app, ship), "a docked ship draws no symbol");
        }
        let marker = marker_of(&mut app, yard);
        assert!(app.world().get::<BackFrame>(marker).unwrap().0.is_some());
        let back = descendants(&mut app, marker)
            .into_iter()
            .find(|&e| app.world().get::<BackHolder>(e).is_some())
            .unwrap();
        assert_eq!(
            app.world().get::<Node>(back).unwrap().display,
            Display::Flex
        );
        assert_eq!(count_under(&mut app, marker), ["2"]);
        let second = texts_with::<crate::view::markers::SecondLineFor>(&mut app, marker);
        assert_eq!(second, ["2 docked"]);
        scroll(&mut app, 1.0);
        run(&mut app, SETTLE_FRAMES);
    }
}

/// MUL-04 / MUL-13: a station's "N docked" line follows what is docked in
/// the same frame: when Corvid moves off Iver Yard's phase, the line
/// reads "1 docked" at the end of that frame, not a frame later.
#[test]
fn a_stations_docked_line_follows_its_docked_count_in_the_same_frame() {
    use crate::sim::orbit::Orbit;
    let mut app = symbols_app_with(|_| {});
    let iver = body(&mut app, "Iver");
    let yard = body(&mut app, "Iver Yard");
    let corvid = body(&mut app, "Corvid");
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(iver);
    run(&mut app, 60 + move_frames());
    let marker = marker_of(&mut app, yard);
    let second = |app: &mut App| texts_with::<crate::view::markers::SecondLineFor>(app, marker);
    assert_eq!(second(&mut app), ["2 docked"]);
    // To the far side of the orbit, so it overlaps nothing at the yard.
    app.world_mut().get_mut::<Orbit>(corvid).unwrap().phase0_rad += std::f64::consts::PI;
    app.update();
    let station = multi_marker(&app, MultiKey::Station(yard)).expect("still a station marker");
    assert_eq!(station.count(), 1);
    assert_eq!(second(&mut app), ["1 docked"]);
}

/// Adds a second station, Iver Dock, to Iver's "High" band 0.002 rad on
/// from Iver Yard: tens of km apart, so not together (PHAS-02) but one
/// spot on screen. A tender is docked at the dock, at its phase.
fn add_a_station_beside_iver_yard(mut waiting: ResMut<SystemDataResource>) {
    use crate::sim::track::ShipClass;
    let data = waiting.0.as_mut().unwrap();
    let iver = data.planets.iter_mut().find(|p| p.name == "Iver").unwrap();
    let yard = iver
        .children
        .iter()
        .find(|c| c.name == "Iver Yard")
        .unwrap();
    let phase0_rad = yard.phase0_rad + 0.002;
    let in_high = |name, class, record| BodyBlueprint {
        affiliation: Affiliation::Own,
        record: Some(record),
        orbit: OrbitPlacement::Named("High"),
        phase0_rad,
        ..blueprint(name, class, 0.0)
    };
    let dock = in_high(
        "Iver Dock",
        BodyClass::Station,
        ShipRecord::new(ShipClass::Station, true),
    );
    let tender = in_high(
        "Dock tender",
        BodyClass::Ship,
        ShipRecord::new(ShipClass::Cargo, true),
    );
    iver.children.extend([dock, tender]);
}

/// MUL-03 / MUL-04 / MUL-05: two stations whose markers overlap join as
/// two things: one marker counting 2, holding both stations, neither
/// docked at the other, each with its own docked things listed under it.
#[test]
fn two_overlapping_stations_are_a_marker_of_two_with_neither_docked() {
    let mut app = symbols_app_with(|app| {
        app.add_systems(
            Startup,
            add_a_station_beside_iver_yard
                .in_set(Boot::World)
                .before(crate::sim::spawn::spawn_system),
        );
    });
    let iver = body(&mut app, "Iver");
    let [yard, dock, tender, corvid, pellam] = [
        "Iver Yard",
        "Iver Dock",
        "Dock tender",
        "Corvid",
        "Pellam trader",
    ]
    .map(|n| body(&mut app, n));
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(iver);
    run(&mut app, 60 + move_frames());

    let key = joined_into(&app, yard).expect("the yard is joined");
    assert!(matches!(key, MultiKey::Overlap(_)), "{key:?}");
    assert_eq!(joined_into(&app, dock), Some(key));
    let marker = multi_marker(&app, key).unwrap();
    assert!(marker.drawn);
    assert_eq!(marker.count(), 2);
    let mut things = marker.things.clone();
    things.sort();
    let mut want = vec![yard, dock];
    want.sort();
    assert_eq!(things, want);
    let docked_at = |station| {
        let mut at = marker
            .docked
            .iter()
            .find(|(s, _)| *s == station)
            .map(|(_, d)| d.clone())
            .unwrap_or_default();
        at.sort();
        at
    };
    let mut at_yard = vec![corvid, pellam];
    at_yard.sort();
    assert_eq!(docked_at(yard), at_yard);
    assert_eq!(docked_at(dock), [tender]);
    assert!(
        marker
            .docked
            .iter()
            .all(|(_, d)| !d.contains(&yard) && !d.contains(&dock)),
        "neither station is docked"
    );
    let stations = app.world().resource::<MultiMarkers>();
    assert!(stations.get(MultiKey::Station(yard)).is_none());
    assert!(stations.get(MultiKey::Station(dock)).is_none());
}

/// Swaps Corvane for Sol before the system is spawned.
fn load_sol(mut waiting: ResMut<SystemDataResource>) {
    waiting.0 = Some(crate::content::sol::sol_system());
}

/// MUL-03 / TEST-11 / SOL-14: on day 0 in Sol, Marlin is docked at
/// Perigee Station, which shows a station marker counting 1 once Earth's
/// Low band unfolds; folded, Marlin is listed under the station in
/// Earth's fold marker, which counts the station alone.
#[test]
fn marlin_docked_at_perigee_station_gives_a_count_of_one() {
    let mut app = symbols_app_with(|app| {
        app.add_systems(
            Startup,
            load_sol
                .in_set(Boot::World)
                .before(crate::sim::spawn::spawn_system),
        );
    });
    let earth = body(&mut app, "Earth");
    let perigee = body(&mut app, "Perigee Station");
    let marlin = body(&mut app, "Marlin");
    let vestige = body(&mut app, "Vestige");
    run(&mut app, SETTLE_FRAMES);
    let fold = multi_marker(&app, MultiKey::Fold(earth)).expect("Earth's fold marker");
    assert!(fold.things.contains(&perigee) && !fold.things.contains(&marlin));
    assert!(fold.docked.contains(&(perigee, vec![marlin])));
    zoom_in_on_until(&mut app, earth, 20, |app| {
        multi_marker(app, MultiKey::Station(perigee)).is_some_and(|m| m.drawn)
    });
    let station = multi_marker(&app, MultiKey::Station(perigee)).expect("a station marker");
    assert_eq!(station.count(), 1);
    assert_eq!(station.docked, [(perigee, vec![marlin])]);
    assert_eq!(joined_into(&app, marlin), Some(MultiKey::Station(perigee)));
    assert_ne!(joined_into(&app, vestige), Some(MultiKey::Station(perigee)));
    let marker = marker_of(&mut app, perigee);
    assert_eq!(count_under(&mut app, marker), ["1"]);
}

/// MUL-05 / MUL-06 / TEST-12: TF-2 and the two haulers parked at its
/// phase are together, so once the High band is large enough that no
/// other ship overlaps them they are one multi-marker counting 3, centred
/// on their shared point, and none of them draws its own symbol.
#[test]
fn things_together_are_one_multi_marker() {
    let mut app = symbols_app_with(|_| {});
    let halden = body(&mut app, "Halden IV");
    let trio = ["TF-2", "Varga Courser", "Varga Drifter"].map(|n| body(&mut app, n));
    let key = MultiKey::Together(trio[0].min(trio[1]).min(trio[2]));
    zoom_in_on_until(&mut app, halden, 12, |app| {
        multi_marker(app, key).is_some_and(|m| m.drawn)
    });
    let together = multi_marker(&app, key).unwrap();
    let mut things = together.things.clone();
    things.sort();
    let mut want = trio.to_vec();
    want.sort();
    assert_eq!(things, want);
    assert_eq!(together.count(), 3);
    // MUL-06 / SYM-16: centred on their shared point, on a whole pixel.
    let shared = app
        .world()
        .resource::<ScreenPositions>()
        .get(trio[0])
        .unwrap();
    assert_eq!(together.centre, together.centre.round());
    assert!((together.centre - shared).abs().max_element() <= 0.5);
    for thing in trio {
        assert_eq!(joined_into(&app, thing), Some(key));
        assert!(!drawn(&mut app, thing));
    }
    let (node, _) = multi_node(&mut app, key).expect("drawn by a node");
    assert_eq!(count_under(&mut app, node), ["3"]);
}

/// Every drawn tail laid by one of `things` this frame, as (thing, where
/// its head lands on screen, where the thing itself is on screen).
fn tails_of(app: &mut App, things: Vec<Entity>) -> Vec<(Entity, Vec2, Vec2)> {
    use crate::view::shared_orbit::{OrbitRings, ring_screen_point};
    use crate::view::world::ViewNow;
    use bevy::ecs::system::RunSystemOnce;
    app.world_mut()
        .run_system_once(move |rings: OrbitRings, view: ViewNow| {
            let mut tails = Vec::new();
            for ring in rings.owners().into_iter().filter_map(|o| rings.ring(o)) {
                assert!(!ring.mode().ring_mode, "a paused clock is in dot mode");
                let angles = ring.tail_angles();
                for (member, head) in ring.tails().zip(angles) {
                    if things.contains(&member.body) {
                        let at = |angle| ring_screen_point(&ring, angle, &view).unwrap();
                        tails.push((member.body, at(head), at(member.angle)));
                    }
                }
            }
            tails
        })
        .unwrap()
}

/// The thing a marker's first tray row lists (MUL-09).
fn first_row(app: &mut App, key: MultiKey) -> Entity {
    use crate::view::multi_markers::order::TrayOrder;
    use bevy::ecs::system::RunSystemOnce;
    app.world_mut()
        .run_system_once(move |order: TrayOrder, multis: Res<MultiMarkers>| {
            order.first(multis.get(key).unwrap()).unwrap()
        })
        .unwrap()
}

/// MUL-05 / MOT-01 / TEST-12: in dot mode the convoy's marker draws one
/// tail, its first tray row's, from the marker's centre; the other two
/// draw none of their own.
#[test]
fn a_together_marker_draws_one_tail_in_dot_mode() {
    let mut app = symbols_app_with(|app| {
        app.add_plugins(SharedOrbitPlugin);
    });
    let halden = body(&mut app, "Halden IV");
    let trio = ["TF-2", "Varga Courser", "Varga Drifter"].map(|n| body(&mut app, n));
    let key = MultiKey::Together(trio[0].min(trio[1]).min(trio[2]));
    zoom_in_on_until(&mut app, halden, 12, |app| {
        multi_marker(app, key).is_some_and(|m| m.drawn)
    });
    run(&mut app, SETTLE_FRAMES);
    let centre = multi_marker(&app, key).unwrap().centre;
    let lead = first_row(&mut app, key);
    let tails = tails_of(&mut app, trio.to_vec());
    assert_eq!(tails.len(), 1, "one tail for the convoy: {tails:?}");
    let (thing, head, _) = tails[0];
    assert_eq!(thing, lead, "the first tray row's tail");
    assert!(head.distance(centre) <= 1.0, "from the marker's centre");
}

/// MUL-05: an overlap cluster's marker draws its lead's tail only, laid
/// along the lead's orbit from the point of it nearest the marker's
/// centre.
#[test]
fn an_overlap_marker_draws_its_leads_tail() {
    let mut app = symbols_app_with(|app| {
        app.add_plugins(SharedOrbitPlugin);
    });
    let halden = body(&mut app, "Halden IV");
    let overlap = |app: &mut App| {
        app.world()
            .resource::<MultiMarkers>()
            .markers
            .iter()
            .find(|m| matches!(m.key, MultiKey::Overlap(_)) && m.drawn)
            .cloned()
    };
    zoom_in_on_until(&mut app, halden, 6, |app| overlap(app).is_some());
    run(&mut app, SETTLE_FRAMES);
    let marker = overlap(&mut app).expect("an overlap marker");
    let lead = first_row(&mut app, marker.key);
    let tails = tails_of(&mut app, marker.things.clone());
    assert_eq!(tails.len(), 1, "one tail for the cluster: {tails:?}");
    let (thing, head, own) = tails[0];
    assert_eq!(thing, lead, "the lead's tail");
    assert!(
        head.distance(marker.centre) <= own.distance(marker.centre) + 0.5,
        "the head is at least as near the marker's centre as the lead is"
    );
}

/// Adds a second task force beside TF-1, at its phase in Halden IV's
/// "High" band, before the system is spawned.
fn add_a_second_group_beside_tf1(mut waiting: ResMut<SystemDataResource>) {
    use crate::sim::system::{GroupBlueprint, MemberBlueprint};
    use crate::sim::track::ShipClass;
    let data = waiting.0.as_mut().unwrap();
    let halden = data
        .planets
        .iter_mut()
        .find(|p| p.name == "Halden IV")
        .unwrap();
    let phase0_rad = halden
        .children
        .iter()
        .find(|c| c.name == "TF-1")
        .unwrap()
        .phase0_rad;
    let member = |name| MemberBlueprint {
        name,
        record: ShipRecord::new(ShipClass::Warship, true),
    };
    halden.children.push(BodyBlueprint {
        class: BodyClass::Ship,
        affiliation: Affiliation::Own,
        group: Some(GroupBlueprint {
            task_force: true,
            members: vec![member("Ward"), member("Sentinel")],
        }),
        orbit: OrbitPlacement::Named("High"),
        phase0_rad,
        ..blueprint("TF-3", BodyClass::Ship, 0.0)
    });
}

/// SYM-10: two groups at one place are one multi-marker counting 2, each
/// group one thing: neither is merged into the other or split up.
#[test]
fn two_groups_in_one_place_are_one_multi_marker_of_two() {
    use crate::sim::group::ShipGroup;
    let mut app = symbols_app_with(|app| {
        app.add_systems(
            Startup,
            add_a_second_group_beside_tf1
                .in_set(Boot::World)
                .before(crate::sim::spawn::spawn_system),
        );
    });
    let halden = body(&mut app, "Halden IV");
    let tf1 = body(&mut app, "TF-1");
    let tf3 = body(&mut app, "TF-3");
    let key = MultiKey::Together(tf1.min(tf3));
    zoom_in_on_until(&mut app, halden, 12, |app| {
        multi_marker(app, key).is_some_and(|m| m.drawn)
    });
    let together = multi_marker(&app, key).unwrap();
    assert_eq!(together.count(), 2);
    for group in [tf1, tf3] {
        assert!(together.things.contains(&group));
        let members = app.world().get::<ShipGroup>(group).unwrap().members.clone();
        assert!(members.iter().all(|m| !together.things.contains(m)));
    }
}

/// MUL-05: in Halden IV's frame, zoomed in step by step, no two symbols
/// drawn on their own are closer than `cluster_join_px`: closer ones are
/// joined into one marker. Overlap markers form where the High band is
/// small on screen.
#[test]
fn no_two_symbols_drawn_on_their_own_overlap() {
    use crate::view::markers::MarkerShape;
    let mut app = symbols_app_with(|_| {});
    let halden = body(&mut app, "Halden IV");
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(halden);
    run(&mut app, 60 + move_frames());
    let join = Tunables::default().cluster_join_px;
    let mut overlaps_seen = false;
    for _ in 0..6 {
        overlaps_seen |= app
            .world()
            .resource::<MultiMarkers>()
            .markers
            .iter()
            .any(|m| matches!(m.key, MultiKey::Overlap(_)) && m.drawn);
        let mut symbols = app
            .world_mut()
            .query::<(&MarkerShape, &MarkerDrawn, &MarkerFor)>();
        let centres: Vec<(Entity, Vec2)> = symbols
            .iter(app.world())
            .filter(|(shape, ..)| **shape == MarkerShape::Symbol)
            .filter_map(|(_, drawn, body)| Some((body.0, drawn.0?.0)))
            .collect();
        for (i, (a, pa)) in centres.iter().enumerate() {
            for (b, pb) in &centres[i + 1..] {
                assert!(
                    pa.distance(*pb) >= join - 0.5,
                    "{a} and {b} overlap, {} px apart",
                    pa.distance(*pb)
                );
            }
        }
        scroll(&mut app, 1.0);
        run(&mut app, SETTLE_FRAMES);
    }
    assert!(overlaps_seen, "the High band's ships never overlapped");
}

/// GRP-07 / MUL-07: a click on Halden IV's fold marker selects Halden IV,
/// as a click on its marker does; a double-click asks for its frame.
#[test]
fn clicking_a_fold_marker_is_clicking_its_parent() {
    let mut app = symbols_app_with(|_| {});
    run(&mut app, SETTLE_FRAMES);
    let halden = body(&mut app, "Halden IV");
    let (_, target) = multi_node(&mut app, MultiKey::Fold(halden)).unwrap();
    click_node(&mut app, target, 1);
    assert_eq!(app.world().resource::<Selection>().selected, Some(halden));
    app.world_mut().resource_mut::<Selection>().clear();
    click_node(&mut app, target, 2);
    assert_eq!(app.world().resource::<Selection>().selected, Some(halden));
    assert_eq!(app.world().resource::<FrameRequest>().0, Some(halden));
}

/// MUL-07 / MUL-09: a click on the TEST-12 convoy's marker selects the
/// first of its things in the order of the "In orbit" outline; a
/// double-click also asks for its frame. While the planner is open, the
/// click picks nothing and leaves the selection alone.
#[test]
fn clicking_a_together_marker_selects_its_first_thing() {
    use crate::view::inset::outline::{OutlineBodies, outline_positions};
    use bevy::ecs::system::RunSystemOnce;
    let mut app = symbols_app_with(|_| {});
    let halden = body(&mut app, "Halden IV");
    let trio = ["TF-2", "Varga Courser", "Varga Drifter"].map(|n| body(&mut app, n));
    let key = MultiKey::Together(trio[0].min(trio[1]).min(trio[2]));
    zoom_in_on_until(&mut app, halden, 12, |app| {
        multi_marker(app, key).is_some_and(|m| m.drawn)
    });
    let positions = app
        .world_mut()
        .run_system_once(|bodies: OutlineBodies| outline_positions(&bodies.nodes()))
        .unwrap();
    let first = *trio.iter().min_by_key(|t| positions[t]).unwrap();

    let (_, target) = multi_node(&mut app, key).unwrap();
    app.world_mut().resource_mut::<FrameRequest>().0 = None;
    click_node(&mut app, target, 1);
    assert_eq!(app.world().resource::<Selection>().selected, Some(first));
    assert_eq!(app.world().resource::<FrameRequest>().0, None);
    click_node(&mut app, target, 2);
    assert_eq!(app.world().resource::<FrameRequest>().0, Some(first));

    let orison = body(&mut app, "Orison");
    app.world_mut().resource_mut::<Selection>().select(orison);
    app.world_mut().resource_mut::<PlannerState>().open(orison);
    click_node(&mut app, target, 1);
    assert_eq!(app.world().resource::<Selection>().selected, Some(orison));
    assert_eq!(app.world().resource::<PlannerState>().clicked, None);
}

/// HOV-06 / INS-06: a selected thing joined into a multi-marker has its
/// mark on that marker's box, round both frames.
#[test]
fn a_joined_things_mark_is_on_its_multi_marker() {
    use crate::view::markers::brackets::{Bracket, BracketsPlugin};
    let mut app = symbols_app_with(|app| {
        app.add_plugins((SharedOrbitPlugin, BracketsPlugin));
    });
    let halden = body(&mut app, "Halden IV");
    let courser = body(&mut app, "Varga Courser");
    zoom_in_on_until(&mut app, halden, 12, |app| {
        joined_into(app, courser).is_some_and(|k| multi_marker(app, k).is_some_and(|m| m.drawn))
    });
    app.world_mut().resource_mut::<Selection>().select(courser);
    run(&mut app, 2);
    let key = joined_into(&app, courser).unwrap();
    let centre = multi_marker(&app, key).unwrap().centre;
    let tunables = Tunables::default();
    let (offset, margin) = (tunables.back_frame_offset_px, tunables.bracket_margin_px);
    let side = tunables.ship_symbol_px + offset + 2.0 * margin;
    let mut brackets = app.world_mut().query_filtered::<&Node, With<Bracket>>();
    let shown: Vec<Rect> = brackets
        .iter(app.world())
        .filter(|n| n.display == Display::Flex)
        .filter_map(|n| match (n.left, n.top, n.width, n.height) {
            (Val::Px(l), Val::Px(t), Val::Px(w), Val::Px(h)) => Some(Rect::new(l, t, l + w, t + h)),
            _ => None,
        })
        .collect();
    assert_eq!(shown.len(), 1);
    assert!(close(
        shown[0].center(),
        centre + Vec2::new(offset, -offset) / 2.0
    ));
    assert!(close(shown[0].size(), Vec2::splat(side)));
}

/// MUL-01 / MUL-06: with the clock paused and the camera still, the
/// multi-markers' pool nodes aren't written from frame to frame, so a
/// still marker costs no layout.
#[test]
fn still_multi_markers_leave_their_nodes_unwritten() {
    let mut app = symbols_app_with(|_| {});
    run(&mut app, SETTLE_FRAMES);
    let halden = body(&mut app, "Halden IV");
    assert!(multi_node(&mut app, MultiKey::Fold(halden)).is_some());
    let written = |app: &mut App| {
        let mut nodes = app
            .world_mut()
            .query_filtered::<(Entity, Ref<Node>), With<MultiNode>>();
        let mut ticks: Vec<_> = nodes
            .iter(app.world())
            .map(|(e, node)| (e, node.last_changed()))
            .collect();
        ticks.sort_by_key(|(e, _)| *e);
        ticks
    };
    let before = written(&mut app);
    run(&mut app, 3);
    assert_eq!(written(&mut app), before);
}
