//! Headless tests of orbit rings and their picking (HOV-02 to HOV-05,
//! MOT-14): rings as hit targets, shared orbits, and an exposure ring's
//! identity mark, label, period and chevron.

use super::*;
use crate::view::markers::BodyRim;

/// HOV-02: every drawn ring is traced for hit-testing, in dot mode too;
/// a folded satellite's isn't, and nor is the frame body's.
#[test]
fn dot_mode_rings_are_hit_targets() {
    let mut app = camera_app_with(|app| {
        // The backend's output, without the rest of picking.
        app.add_message::<bevy::picking::backend::PointerHits>()
            .insert_resource(placeholder_fonts())
            .add_plugins((
                GroupingPlugin,
                MotionPlugin,
                HoveredPlugin,
                SharedOrbitPlugin,
                crate::view::rings::picking::RingPickingPlugin,
            ));
    });
    run(&mut app, SETTLE_FRAMES);
    let halden = body(&mut app, "Halden IV");
    let ivc = body(&mut app, "Halden IVc");
    assert!(
        !app.world()
            .get::<MotionModeState>(halden)
            .unwrap()
            .ring_mode
    );
    let traced = |app: &mut App, body: Entity| {
        let mut targets = app.world_mut().query::<(Entity, &MarkerFor)>();
        let target = targets
            .iter(app.world())
            .filter(|(_, m)| m.0 == body)
            .map(|(e, _)| e)
            .collect::<Vec<_>>();
        let paths = app
            .world()
            .resource::<crate::view::rings::picking::RingPaths>();
        target.iter().any(|t| paths.0.contains_key(t))
    };
    assert!(traced(&mut app, halden), "a dot-mode planet's ring");
    assert!(!traced(&mut app, ivc), "IVc is folded at the system fit");
    set_frame(&mut app, halden, Vec3::ZERO);
    run(&mut app, 1);
    assert!(!traced(&mut app, halden), "the frame body's ring");
}

/// HOV-02 / HOV-05: framed on a ship in a shared orbit, the shared ring is
/// no target, nor is the frame body's own tail, but the other members'
/// tails still target those members.
#[test]
fn a_frame_body_on_a_shared_ring_leaves_the_others_tails_as_targets() {
    use crate::view::rings::picking::{MemberTailTarget, RingPaths, RingTarget};
    let mut app = camera_app_with(|app| {
        app.add_message::<bevy::picking::backend::PointerHits>()
            .insert_resource(placeholder_fonts())
            .add_plugins((
                GroupingPlugin,
                MotionPlugin,
                HoveredPlugin,
                SharedOrbitPlugin,
                crate::view::rings::picking::RingPickingPlugin,
            ));
    });
    let orison = body(&mut app, "Orison");
    let high = app.world().get::<InNamedOrbit>(orison).unwrap().0;
    set_frame(&mut app, orison, Vec3::ZERO);
    // Close enough that High's ships are unfolded round Halden IV.
    app.world_mut().resource_mut::<CameraRig>().distance = 1.0;
    run(&mut app, SETTLE_FRAMES);
    assert!(
        !app.world()
            .get::<MotionModeState>(orison)
            .unwrap()
            .ring_mode,
        "High is in dot mode at 1 d/s"
    );
    let mut rings = app.world_mut().query::<(Entity, &RingTarget)>();
    let ring = rings
        .iter(app.world())
        .find(|(_, r)| r.0 == high)
        .unwrap()
        .0;
    let mut tails = app.world_mut().query::<(Entity, &MemberTailTarget)>();
    let tails: Vec<(Entity, Entity)> = tails
        .iter(app.world())
        .filter(|(_, t)| t.orbit == high)
        .map(|(e, t)| (e, t.body))
        .collect();
    let paths = app.world().resource::<RingPaths>();
    assert!(!paths.0.contains_key(&ring), "the shared ring");
    let traced: Vec<Entity> = tails
        .iter()
        .filter(|(e, _)| paths.0.contains_key(e))
        .map(|&(_, b)| b)
        .collect();
    assert!(!traced.contains(&orison), "the frame body's own tail");
    assert!(traced.len() >= 2, "the other members' tails");
}

/// SCL-07 / TEST-10 / HOV-04 / HOV-02: every ship parked at Halden IV
/// sits in its named orbit "High" with no radius of its own, so they share
/// one ring: the orbit is a ring owner and the ships are not, and the ring
/// has one hit target standing for the orbit, traced at Halden IV's
/// satellite fit. "Low" holds Arden Highport alone, so its ring is the
/// station's (HOV-05); a body with no named orbit, such as K-3, is its own
/// ring.
#[test]
fn ships_parked_in_one_named_orbit_share_one_ring() {
    use crate::view::rings::picking::{RingPaths, RingPickingPlugin, RingTarget};
    use crate::view::shared_orbit::OrbitRings;
    use crate::view::targets::OrbitFor;
    use bevy::ecs::system::RunSystemOnce;

    let mut app = camera_app_with(|app| {
        app.add_message::<bevy::picking::backend::PointerHits>()
            .insert_resource(placeholder_fonts())
            .add_plugins((
                GroupingPlugin,
                MotionPlugin,
                HoveredPlugin,
                SharedOrbitPlugin,
                RingPickingPlugin,
            ));
    });
    let halden = body(&mut app, "Halden IV");
    let orison = body(&mut app, "Orison");
    let ardent = body(&mut app, "Ardent");
    let highport = body(&mut app, "Arden Highport");

    let high = app.world().get::<InNamedOrbit>(orison).unwrap().0;
    assert_eq!(app.world().get::<InNamedOrbit>(ardent).unwrap().0, high);
    let named = app.world().get::<NamedOrbit>(high).unwrap().clone();
    assert_eq!((named.parent, named.name.as_str()), (halden, "High"));
    for ship in [orison, ardent] {
        let orbit = app.world().get::<Orbit>(ship).unwrap();
        assert_eq!(orbit.radius_km, named.radius_km);
        assert_eq!(orbit.period_days, named.period_days);
    }

    let owners = app
        .world_mut()
        .run_system_once(|rings: OrbitRings| rings.owners())
        .unwrap();
    assert!(owners.contains(&high) && owners.contains(&halden));
    assert!(!owners.contains(&orison) && !owners.contains(&ardent));
    let ring = app
        .world_mut()
        .run_system_once(move |rings: OrbitRings| rings.ring(high).unwrap())
        .unwrap();
    assert!(ring.shared);
    let members: Vec<Entity> = ring.members.iter().map(|m| m.body).collect();
    assert_eq!(&members[..2], [orison, ardent]);
    assert!(members.len() >= 13 && members.contains(&body(&mut app, "Sennet")));
    let halden_orbits: Vec<(Entity, String)> = {
        let mut named = app.world_mut().query::<(Entity, &NamedOrbit)>();
        named
            .iter(app.world())
            .filter(|(_, o)| o.parent == halden)
            .map(|(e, o)| (e, o.name.clone()))
            .collect()
    };
    let mut names: Vec<&str> = halden_orbits.iter().map(|(_, n)| n.as_str()).collect();
    names.sort();
    assert_eq!(names, ["High", "Low"], "exactly two named orbits (TEST-10)");
    let low = halden_orbits
        .iter()
        .find(|(_, n)| n == "Low")
        .map(|(e, _)| *e)
        .unwrap();
    // SCL-07: the station sits in the Low band with no radius of its own.
    assert_eq!(app.world().get::<InNamedOrbit>(highport).unwrap().0, low);
    let low_km = app.world().get::<NamedOrbit>(low).unwrap().radius_km;
    assert_eq!(
        app.world().get::<Orbit>(highport).unwrap().radius_km,
        low_km
    );
    assert!(owners.contains(&low) && !owners.contains(&highport));
    let ring = app
        .world_mut()
        .run_system_once(move |rings: OrbitRings| rings.ring(low).unwrap())
        .unwrap();
    assert!(!ring.shared);
    assert_eq!(ring.members.len(), 1);
    assert_eq!(ring.members[0].body, highport);
    assert_eq!(
        ring.name, "Arden Highport",
        "a lone member's ring is its own"
    );

    let mut targets = app
        .world_mut()
        .query::<(Entity, &RingTarget, Option<&OrbitFor>, Option<&MarkerFor>)>();
    let targets: Vec<(Entity, Entity, Option<Entity>, Option<Entity>)> = targets
        .iter(app.world())
        .map(|(e, t, o, m)| (e, t.0, o.map(|o| o.0), m.map(|m| m.0)))
        .collect();
    let shared: Vec<_> = targets.iter().filter(|t| t.1 == high).collect();
    assert_eq!(shared.len(), 1, "one target for the shared orbit");
    assert_eq!((shared[0].2, shared[0].3), (Some(high), None));
    assert!(
        !targets
            .iter()
            .any(|t| t.3 == Some(orison) || t.3 == Some(ardent)),
        "a member of a shared orbit has no ring target of its own"
    );
    assert!(
        targets
            .iter()
            .any(|t| t.1 == low && t.3 == Some(highport) && t.2.is_none()),
        "a lone member's orbit is that member's ring"
    );

    app.world_mut().resource_mut::<FrameRequest>().0 = Some(halden);
    run(&mut app, 60 + SETTLE_FRAMES);
    assert_eq!(frame_body(&app), halden);
    let paths = app.world().resource::<RingPaths>();
    assert!(
        paths.0.contains_key(&shared[0].0),
        "the shared ring's tails are traced for hit-testing"
    );
}

/// MOT-02 / MOT-14 / SYM-16: every ring has a mark and label: a
/// multi-marker targeting the orbit for a shared orbit, counting its
/// things (MUL-04, the TEST-12 convoy three of them), a symbol for a lone
/// ship or station, a body marker for a lone moon. At 0.2 d/s in Halden
/// IV's frame IVa's ring (1.5 d, 7.5 s apparent) is an exposure ring, so
/// its mark shows, centred on a whole pixel; the parked ships' High band
/// (about 12 d, 60 s) is in dot mode, so its mark is hidden, and the
/// highport, folded at that fit, shows no mark either.
#[test]
fn an_exposure_ring_carries_an_identity_mark() {
    use crate::view::multi_markers::draw::{MultiCount, MultiPart};
    use crate::view::rings::marks::{RingMarkFor, RingMarksPlugin};
    use crate::view::symbols::shape::SymbolPart;
    use crate::view::targets::OrbitFor;

    let mut app = camera_app_with(|app| {
        app.insert_resource(placeholder_fonts()).add_plugins((
            GroupingPlugin,
            MotionPlugin,
            HoveredPlugin,
            SharedOrbitPlugin,
            RingMarksPlugin,
        ));
    });
    let halden = body(&mut app, "Halden IV");
    let orison = body(&mut app, "Orison");
    let highport = body(&mut app, "Arden Highport");
    let iva = body(&mut app, "Halden IVa");
    let high = app.world().get::<InNamedOrbit>(orison).unwrap().0;

    let mark_of = |app: &mut App, owner: Entity| {
        let mut marks =
            app.world_mut()
                .query::<(Entity, &RingMarkFor, Option<&OrbitFor>, Option<&MarkerFor>)>();
        marks
            .iter(app.world())
            .find(|(_, m, ..)| m.owner == owner)
            .map(|(e, m, o, k)| (e, m.half, o.map(|o| o.0), k.map(|k| k.0)))
            .unwrap_or_else(|| panic!("no mark for {owner}"))
    };

    let (high_mark, _, orbit_for, marker_for) = mark_of(&mut app, high);
    assert_eq!((orbit_for, marker_for), (Some(high), None));
    let parts = descendants(&mut app, high_mark);
    let frames = parts
        .iter()
        .filter(|&&e| app.world().get::<MultiPart>(e).is_some())
        .count();
    assert_eq!(frames, 4, "a shared orbit's mark is a multi-marker");
    let count = parts
        .iter()
        .find(|&&e| app.world().get::<MultiCount>(e).is_some())
        .unwrap();
    let in_high = app
        .world()
        .resource::<crate::view::shared_orbit::OrbitMembers>()
        .0[&high]
        .len();
    assert!(in_high >= 13);
    assert_eq!(
        app.world().get::<Text>(*count).unwrap().0,
        in_high.to_string()
    );
    assert!(
        parts
            .iter()
            .any(|&e| app.world().get::<Text>(e).is_some_and(|t| t.0 == "High")),
        "the mark is labelled with the orbit's name"
    );

    let low = app.world().get::<InNamedOrbit>(highport).unwrap().0;
    let (highport_mark, _, orbit_for, marker_for) = mark_of(&mut app, low);
    assert_eq!((orbit_for, marker_for), (None, Some(highport)));
    assert!(
        descendants(&mut app, highport_mark)
            .iter()
            .any(|&e| app.world().get::<SymbolPart>(e).is_some()),
        "a lone station's mark is its symbol"
    );
    assert!(
        descendants(&mut app, highport_mark).iter().any(|&e| app
            .world()
            .get::<Text>(e)
            .is_some_and(|t| t.0 == "Arden Highport")),
        "a lone ring is labelled with its body's name (MOT-02)"
    );
    let (iva_mark, iva_half, _, marker_for) = mark_of(&mut app, iva);
    assert_eq!(marker_for, Some(iva));
    assert!(
        descendants(&mut app, iva_mark)
            .iter()
            .any(|&e| app.world().get::<BodyRim>(e).is_some()),
        "a lone moon's mark is its body marker"
    );

    let mut clock = app.world_mut().resource_mut::<GameClock>();
    clock.toggle_pause();
    clock.run_at(0.2);
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(halden);
    run(&mut app, 60 + eased_frames());
    assert_eq!(frame_body(&app), halden);
    let shown = |app: &App, mark: Entity| app.world().get::<Node>(mark).unwrap().display;
    assert_eq!(
        shown(&app, high_mark),
        Display::None,
        "12 d at 0.2 d/s is dot mode"
    );
    assert_eq!(
        shown(&app, highport_mark),
        Display::None,
        "folded at this fit"
    );
    assert_eq!(shown(&app, iva_mark), Display::Flex);
    let node = app.world().get::<Node>(iva_mark).unwrap();
    let (Val::Px(left), Val::Px(top)) = (node.left, node.top) else {
        panic!("the mark is placed in pixels");
    };
    let centre = Vec2::new(left + iva_half, top + iva_half);
    assert_eq!(centre, centre.round(), "centred on a whole pixel");
}

/// MOT-02 / MOT-14 / LBL-01 / LBL-03: an exposure ring's label is placed
/// like any other label: `label_gap_px` from its mark's box, clear of
/// the mark's chevron. Walled in so that only a one-line label fits, it
/// drops its period line, which is its second line.
#[test]
fn an_exposure_ring_label_is_placed_like_any_label() {
    use crate::view::markers::labels::{LabelState, LabelsPlugin, Slot, label_rect, overlaps};
    use crate::view::rings::marks::{PeriodLine, RingLabelBox, RingMarkFor, RingMarksPlugin};

    const FULL: Vec2 = Vec2::new(60.0, 26.0);
    const PERIOD: Vec2 = Vec2::new(60.0, 12.0);
    fn size_ring_labels(app: &mut App) {
        let mut boxes = app
            .world_mut()
            .query_filtered::<&mut ComputedNode, With<RingLabelBox>>();
        for mut computed in boxes.iter_mut(app.world_mut()) {
            computed.size = FULL;
        }
        let mut periods = app
            .world_mut()
            .query_filtered::<&mut ComputedNode, With<PeriodLine>>();
        for mut computed in periods.iter_mut(app.world_mut()) {
            computed.size = PERIOD;
        }
    }

    let mut app = camera_app_with(|app| {
        app.insert_resource(placeholder_fonts()).add_plugins((
            GroupingPlugin,
            MotionPlugin,
            HoveredPlugin,
            SharedOrbitPlugin,
            RingMarksPlugin,
            LabelsPlugin,
        ));
    });
    let halden = body(&mut app, "Halden IV");
    let iva = body(&mut app, "Halden IVa");
    let mut clock = app.world_mut().resource_mut::<GameClock>();
    clock.toggle_pause();
    clock.run_at(0.2);
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(halden);
    run(&mut app, 60 + eased_frames());
    size_ring_labels(&mut app);
    run(&mut app, 30);

    let tunables = Tunables::default();
    let mark = {
        let mut marks = app.world_mut().query::<(Entity, &RingMarkFor)>();
        marks
            .iter(app.world())
            .find(|(_, m)| m.owner == iva)
            .map(|(e, _)| e)
            .unwrap()
    };
    let label = descendants(&mut app, mark)
        .into_iter()
        .find(|&e| app.world().get::<RingLabelBox>(e).is_some())
        .unwrap();
    let period = descendants(&mut app, mark)
        .into_iter()
        .find(|&e| app.world().get::<PeriodLine>(e).is_some())
        .unwrap();
    let px_of = |app: &App, e: Entity| {
        let node = app.world().get::<Node>(e).unwrap();
        let (Val::Px(left), Val::Px(top)) = (node.left, node.top) else {
            panic!("not placed in pixels");
        };
        Vec2::new(left, top)
    };
    let ring_mark = app.world().get::<RingMarkFor>(mark).unwrap();
    let bbox = ring_mark.mark.unwrap().inflate(tunables.bracket_margin_px);
    let chevron = ring_mark.chevron;
    let state = *app.world().get::<LabelState>(label).unwrap();
    let placement = state.shown.expect("the ring's label is shown");
    assert!(placement.full, "with room, the period line shows");
    let min = px_of(&app, mark) + px_of(&app, label);
    let rect = Rect::from_corners(min, min + FULL);
    let want = label_rect(bbox, FULL, placement.slot, tunables.label_gap_px);
    assert!(
        (rect.min - want.min).length() < 1e-3,
        "label at {}, want {}",
        rect.min,
        want.min
    );
    assert!(!overlaps(rect, bbox));
    if let Some(chevron) = chevron {
        assert!(!overlaps(rect, chevron), "the label runs over the chevron");
    }

    // Walls left, right, above and below the mark's box, with room above
    // and below for the name alone.
    let inner = tunables.label_gap_px + (FULL.y - PERIOD.y) + 3.0;
    let side = 30.0;
    let far = 4000.0;
    let walls = [
        Rect::new(-far, -far, far, bbox.min.y - inner),
        Rect::new(-far, bbox.max.y + inner, far, far),
        Rect::new(-far, -far, bbox.min.x - side, far),
        Rect::new(bbox.max.x + side, -far, far, far),
    ];
    // Each wall stands as a marker, whose box is its node grown by the
    // margin.
    for wall in walls {
        let wall = wall.inflate(-tunables.bracket_margin_px);
        app.world_mut().spawn((
            MarkerFor(halden),
            MarkerHalfSize(Vec2::ZERO),
            Node {
                left: px(wall.min.x),
                top: px(wall.min.y),
                ..default()
            },
            ComputedNode {
                size: wall.size(),
                ..default()
            },
        ));
    }
    run(&mut app, 30);
    let state = *app.world().get::<LabelState>(label).unwrap();
    let placement = state.shown.expect("still shown, on one line");
    assert!(!placement.full, "the period line goes first");
    assert!(matches!(placement.slot, Slot::Above | Slot::Below));
    assert_eq!(
        app.world().get::<Node>(period).unwrap().display,
        Display::None
    );
}

/// MOT-02 / MOT-14: at 0.2 d/s in Halden IV's frame IVa's ring is an
/// exposure ring. Its label's second line is the orbit's period, and its
/// direction chevron shows on the ring, `chevron_gap_px` beyond the mark's
/// edge on the side IVa is heading, pointing the way it moves on screen,
/// and its box is kept for the labels to keep clear of.
#[test]
fn an_exposure_ring_shows_its_period_and_a_chevron_the_way_it_runs() {
    use crate::sim::orbit::Orbit;
    use crate::view::readout::format_duration;
    use crate::view::rings::marks::{PeriodLine, RingChevron, RingMarkFor, RingMarksPlugin};
    use crate::view::shared_orbit::{OrbitRings, ring_screen_point};
    use crate::view::world::ViewNow;
    use bevy::ecs::system::RunSystemOnce;

    let mut app = camera_app_with(|app| {
        app.insert_resource(placeholder_fonts()).add_plugins((
            GroupingPlugin,
            MotionPlugin,
            HoveredPlugin,
            SharedOrbitPlugin,
            RingMarksPlugin,
        ));
    });
    let halden = body(&mut app, "Halden IV");
    let iva = body(&mut app, "Halden IVa");
    let mut clock = app.world_mut().resource_mut::<GameClock>();
    clock.toggle_pause();
    clock.run_at(0.2);
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(halden);
    run(&mut app, 60 + eased_frames());

    let (mark, half, chevron_box) = {
        let mut marks = app.world_mut().query::<(Entity, &RingMarkFor)>();
        marks
            .iter(app.world())
            .find(|(_, m)| m.owner == iva)
            .map(|(e, m)| (e, m.half, m.chevron))
            .unwrap()
    };
    let parts = descendants(&mut app, mark);
    let find = |app: &App, has: fn(&App, Entity) -> bool| {
        *parts
            .iter()
            .find(|&&e| has(app, e))
            .expect("a part of the mark")
    };
    let period = find(&app, |app, e| app.world().get::<PeriodLine>(e).is_some());
    let days = app.world().get::<Orbit>(iva).unwrap().period_days;
    assert_eq!(
        app.world().get::<Text>(period).unwrap().0,
        format_duration(days)
    );

    // The ring on screen, a full turn from the mark's point the way IVa
    // runs.
    const STEPS: usize = 3600;
    let ring_points = app
        .world_mut()
        .run_system_once(move |rings: OrbitRings, view: ViewNow| {
            let ring = rings.ring(iva).unwrap();
            let angle = ring.label_angle();
            (0..STEPS)
                .map(|k| {
                    let turn = std::f32::consts::TAU * k as f32 / STEPS as f32;
                    ring_screen_point(&ring, angle + turn, &view).unwrap()
                })
                .collect::<Vec<Vec2>>()
        })
        .unwrap();
    let at = ring_points[0];
    let ahead = (ring_points[1] - at).normalize();
    let node = app.world().get::<Node>(mark).unwrap();
    let (Val::Px(left), Val::Px(top)) = (node.left, node.top) else {
        panic!("the mark is placed");
    };
    let origin = Vec2::new(left, top);

    let chevron = find(&app, |app, e| app.world().get::<RingChevron>(e).is_some());
    let chevron_node = app.world().get::<Node>(chevron).unwrap();
    assert_eq!(chevron_node.display, Display::Flex);
    let (Val::Px(cl), Val::Px(ct)) = (chevron_node.left, chevron_node.top) else {
        panic!("the chevron is placed");
    };
    let tunables = Tunables::default();
    let centre = origin + Vec2::new(cl, ct) + Vec2::splat(tunables.chevron_px / 2.0);
    let (nearest, gap) = ring_points
        .iter()
        .enumerate()
        .map(|(k, p)| (k, p.distance(centre)))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .unwrap();
    assert!(gap < 1.0, "on the ring, {gap} px off");
    assert!(nearest < STEPS / 2, "ahead of the mark the way IVa runs");
    let reach = half / ahead.x.abs().max(ahead.y.abs())
        + tunables.chevron_gap_px
        + crate::view::rings::chevron::chevron_half_length(tunables.chevron_px);
    let away = centre.distance(at);
    assert!(
        (away - reach).abs() < 1.5,
        "{away} px from the mark's point, expected {reach}"
    );
    let along = (ring_points[nearest + 1] - ring_points[nearest - 1]).normalize();
    let rotation = app.world().get::<UiTransform>(chevron).unwrap().rotation;
    let pointing = rotation * Vec2::NEG_Y;
    assert!(
        pointing.dot(along) > 0.95,
        "pointing along the ring the way IVa runs"
    );
    assert_eq!(
        chevron_box.map(|b| b.center().round()),
        Some(centre.round()),
        "the labels keep clear of the chevron's box"
    );
}

/// MOT-14 / MUL-04: a shared orbit's identity mark follows its members as
/// they move, dock and leave. Iver Yard's band starts as a station marker
/// counting Corvid and the Pellam trader docked there. Corvid moving off
/// the yard's phase makes it a multi-marker of the yard and Corvid, the
/// trader still docked; the trader moving off too makes it count 3. With
/// both ships gone from the band, the yard alone is a lone station, and
/// its mark is its own symbol, fading on from where the old mark was.
#[test]
fn a_shared_orbits_identity_mark_follows_ships_that_move_dock_and_leave() {
    use crate::view::multi_markers::draw::{MultiCount, MultiPart};
    use crate::view::rings::marks::{IdentityFront, MarkShows, RingMarkFor, RingMarksPlugin};

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
    let ships = ["Corvid", "Pellam trader"].map(|n| body(&mut app, n));
    let band = app.world().get::<InNamedOrbit>(yard).unwrap().0;
    let mark = |app: &mut App| {
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
    };
    let drawn = |app: &mut App| {
        let (root, shows) = mark(app);
        let parts = descendants(app, root);
        let frames = parts
            .iter()
            .filter(|&&e| app.world().get::<MultiPart>(e).is_some())
            .count();
        let count: Vec<String> = parts
            .iter()
            .filter(|&&e| app.world().get::<MultiCount>(e).is_some())
            .filter_map(|&e| app.world().get::<Text>(e).map(|t| t.0.clone()))
            .collect();
        (shows, frames, count)
    };
    let move_off = |app: &mut App, ship: Entity| {
        app.world_mut().get_mut::<Orbit>(ship).unwrap().phase0_rad += std::f64::consts::PI;
        app.update();
    };

    let (shows, frames, count) = drawn(&mut app);
    assert!(matches!(
        shows,
        MarkShows::Multi(IdentityFront::Station(_), _)
    ));
    assert_eq!(
        (frames, count),
        (2, vec!["2".to_string()]),
        "back frame and count"
    );

    move_off(&mut app, ships[0]);
    let (shows, frames, count) = drawn(&mut app);
    assert!(matches!(shows, MarkShows::Multi(IdentityFront::Frame, _)));
    assert_eq!(
        (frames, count),
        (4, vec!["2".to_string()]),
        "the trader is still docked"
    );

    move_off(&mut app, ships[1]);
    let (_, frames, count) = drawn(&mut app);
    assert_eq!((frames, count), (4, vec!["3".to_string()]));

    let (old_root, _) = mark(&mut app);
    app.world_mut()
        .get_mut::<RingMarkFor>(old_root)
        .unwrap()
        .shown = 0.5;
    for ship in ships {
        app.world_mut()
            .entity_mut(ship)
            .remove::<(Orbit, InNamedOrbit)>()
            .insert(InFlight);
    }
    app.update();
    let (root, shows) = mark(&mut app);
    assert!(matches!(shows, MarkShows::Body(b, _) if b == yard));
    assert_eq!(app.world().get::<MarkerFor>(root).map(|m| m.0), Some(yard));
    let shown = app.world().get::<RingMarkFor>(root).unwrap().shown;
    // One frame's fade step is 16.7 ms of `identity_mark_fade_ms`.
    let step = FRAME.as_secs_f32() * 1000.0 / Tunables::default().identity_mark_fade_ms;
    assert!(
        (shown - 0.5).abs() <= step + 1e-4,
        "fading from where it was, not from 0"
    );
}

/// MOT-14: a lone body's identity mark is drawn again when its look
/// changes: Iver Yard, alone on its band once the ships have left, turns
/// hostile and its mark is rebuilt in the hostile colour. RAIL-09: once
/// the yard leaves too, the empty band keeps its mark (to fade out, and in
/// again from there if a ship comes back); a mark whose orbit is gone is
/// removed.
#[test]
fn a_lone_bodys_mark_follows_its_look_and_goes_with_its_ring() {
    use crate::sim::body::Affiliation;
    use crate::view::rings::marks::{MarkShows, RingMarkFor, RingMarksPlugin};
    use crate::view::shared_orbit::OrbitMembers;

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
    for ship in ["Corvid", "Pellam trader"].map(|n| body(&mut app, n)) {
        app.world_mut()
            .entity_mut(ship)
            .remove::<(Orbit, InNamedOrbit)>()
            .insert(InFlight);
    }
    app.update();
    let marks = |app: &mut App| {
        let mut marks = app
            .world_mut()
            .query::<(Entity, &RingMarkFor, &MarkShows)>();
        marks
            .iter(app.world())
            .filter(|(_, m, _)| m.owner == band)
            .map(|(e, _, shows)| (e, shows.clone()))
            .collect::<Vec<_>>()
    };
    let tints = |app: &mut App, root: Entity| {
        descendants(app, root)
            .into_iter()
            .filter_map(|e| app.world().get::<Tint>(e).map(|t| t.color))
            .collect::<Vec<Color>>()
    };
    let lone = |app: &mut App| match marks(app).as_slice() {
        [(root, MarkShows::Body(_, look))] => (*root, look.affiliation),
        other => panic!("one lone body's mark, not {other:?}"),
    };
    let (before, affiliation) = lone(&mut app);
    assert_eq!(affiliation, Affiliation::Own);
    assert!(!tints(&mut app, before).contains(&crate::view::colors::HOSTILE));

    *app.world_mut().get_mut::<Affiliation>(yard).unwrap() = Affiliation::Hostile;
    app.update();
    let (after, affiliation) = lone(&mut app);
    assert_eq!(affiliation, Affiliation::Hostile);
    assert_ne!(after, before, "built again");
    assert!(tints(&mut app, after).contains(&crate::view::colors::HOSTILE));

    app.world_mut()
        .entity_mut(yard)
        .remove::<(Orbit, InNamedOrbit)>()
        .insert(InFlight);
    app.update();
    assert!(
        !app.world().resource::<OrbitMembers>().0.contains_key(&band),
        "the band is empty"
    );
    assert_eq!(marks(&mut app).len(), 1, "an empty orbit keeps its mark");

    app.world_mut().entity_mut(band).despawn();
    app.update();
    assert!(marks(&mut app).is_empty(), "no orbit, no mark");
}
