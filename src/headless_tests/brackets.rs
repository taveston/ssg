//! Headless tests of the selection and hover brackets (HOV-06, HOV-07):
//! what they enclose for a body, a true-size disc, a ship's symbol, a
//! multi-marker and a shared orbit's identity mark.

use super::*;

/// The overlay that draws selection and hover marks: markers,
/// multi-markers, exposure-ring marks and the brackets themselves.
fn brackets_app() -> App {
    use crate::view::markers::brackets::BracketsPlugin;
    use crate::view::rings::marks::RingMarksPlugin;
    camera_app_with(|app| {
        app.insert_resource(placeholder_fonts())
            .add_plugins((
                EdgePlugin,
                GroupingPlugin,
                MotionPlugin,
                MarkersPlugin,
                MultiMarkersPlugin,
                HoveredPlugin,
                SharedOrbitPlugin,
                RingMarksPlugin,
            ))
            .add_plugins(BracketsPlugin);
    })
}

/// A bracket set as drawn: its box's top-left and size, and its alpha.
#[derive(Debug, Clone, Copy)]
struct DrawnBracket {
    min: Vec2,
    size: Vec2,
    alpha: f32,
}

impl DrawnBracket {
    fn centre(&self) -> Vec2 {
        self.min + self.size / 2.0
    }
}

/// Every bracket set shown this frame. Each sits in the labels layer,
/// above all markers (MRK-12).
fn shown_brackets(app: &mut App) -> Vec<DrawnBracket> {
    use crate::view::layers;
    use crate::view::markers::brackets::Bracket;
    let mut sets = app
        .world_mut()
        .query_filtered::<(&Node, &Children, &GlobalZIndex), With<Bracket>>();
    let found: Vec<(Node, Entity, i32)> = sets
        .iter(app.world())
        .filter(|(node, ..)| node.display == Display::Flex)
        .map(|(node, children, z)| (node.clone(), children[0], z.0))
        .collect();
    let px = |v: Val| match v {
        Val::Px(x) => x,
        other => panic!("bracket placed as {other:?}"),
    };
    found
        .into_iter()
        .map(|(node, corner, z)| {
            assert_eq!(z, layers::LABELS, "brackets draw with labels");
            let border = app.world().get::<BorderColor>(corner).unwrap();
            DrawnBracket {
                min: Vec2::new(px(node.left), px(node.top)),
                size: Vec2::new(px(node.width), px(node.height)),
                alpha: border.top.alpha(),
            }
        })
        .collect()
}

/// Where a body's marker is drawn: centre and half-size.
fn marker_drawn(app: &mut App, body: Entity) -> Option<(Vec2, f32)> {
    use crate::view::markers::MarkerDrawn;
    let mut markers = app.world_mut().query::<(&MarkerFor, &MarkerDrawn)>();
    markers
        .iter(app.world())
        .find(|(marker, _)| marker.0 == body)
        .and_then(|(_, drawn)| drawn.0)
}

/// HOV-06 / HOV-07 / HOV-01 / INS-06 / GRP-07 / GRP-10 / GRP-13: at the
/// system fit, the selected Halden IV has white corner brackets on a
/// square that clears its marker and moon ring by `bracket_margin_px`;
/// hovered instead, the same brackets at `bracket_hover_alpha`. Orison,
/// folded into Halden IV, pops its fold marker out and puts its
/// selection mark on it, round both its frames; hovering
/// that marker merges into the one mark, and hovering it alone gives the
/// hover mark (MUL-11). With nothing hovered or selected, no brackets
/// show, and the frame body has none.
#[test]
fn selection_and_hover_bracket_what_stands_for_the_object() {
    let mut app = brackets_app();
    let tunables = Tunables::default();
    let margin = tunables.bracket_margin_px;
    let halden = body(&mut app, "Halden IV");
    let orison = body(&mut app, "Orison");
    run(&mut app, SETTLE_FRAMES);
    assert!(
        shown_brackets(&mut app).is_empty(),
        "no selection, no hover"
    );

    // GRP-10 / HOV-07: Halden IV's moons are folded into a moon ring,
    // which its box takes in, offset right and up.
    app.world_mut().resource_mut::<Selection>().select(halden);
    run(&mut app, 1);
    let (centre, half) = marker_drawn(&mut app, halden).expect("Halden IV is drawn");
    let ring = tunables.moon_ring_offset_px;
    let marks = shown_brackets(&mut app);
    assert_eq!(marks.len(), 1);
    assert!(close(
        marks[0].centre(),
        centre + Vec2::new(ring, -ring) / 2.0
    ));
    assert_eq!(
        marks[0].size,
        Vec2::splat(tunables.planet_marker_px + ring + 2.0 * margin)
    );
    assert_eq!(half, tunables.planet_marker_px / 2.0);
    assert_eq!(marks[0].alpha, 1.0, "the selection mark is full white");

    app.world_mut().resource_mut::<Selection>().clear();
    let pointer = app.world_mut().spawn((MarkerFor(halden), Hovered)).id();
    run(&mut app, 2);
    let marks = shown_brackets(&mut app);
    assert_eq!(marks.len(), 1);
    assert_eq!(marks[0].alpha, tunables.bracket_hover_alpha);
    app.world_mut().entity_mut(pointer).despawn();

    // INS-06 / HOV-07 / GRP-13: a folded ship's mark is on the fold
    // marker that holds it, popped out: the square that clears both its
    // frames by the margin.
    app.world_mut().resource_mut::<Selection>().select(orison);
    run(&mut app, SETTLE_FRAMES);
    let (node, target) = multi_node(&mut app, MultiKey::Fold(halden)).expect("a fold marker");
    let front_min = node_min(app.world().get::<Node>(node).unwrap()).unwrap();
    let offset = tunables.back_frame_offset_px;
    let marks = shown_brackets(&mut app);
    assert_eq!(marks.len(), 1);
    assert!(close(
        marks[0].min,
        front_min - Vec2::new(margin, offset + margin)
    ));
    let side = tunables.ship_symbol_px + offset + 2.0 * margin;
    assert!(close(marks[0].size, Vec2::splat(side)));
    assert_eq!(marks[0].alpha, 1.0);

    // GRP-07 / MUL-11: hovering the fold marker too leaves one full mark;
    // hovering it alone gives the hover mark.
    let fold_marker = target;
    app.world_mut().entity_mut(fold_marker).insert(Hovered);
    run(&mut app, 1);
    let marks = shown_brackets(&mut app);
    assert_eq!(marks.len(), 1);
    assert_eq!(marks[0].alpha, 1.0);
    app.world_mut().resource_mut::<Selection>().clear();
    run(&mut app, 1);
    let marks = shown_brackets(&mut app);
    assert_eq!(marks.len(), 1);
    assert_eq!(marks[0].alpha, tunables.bracket_hover_alpha);
    app.world_mut().entity_mut(fold_marker).remove::<Hovered>();

    // The frame body has no mark of its own.
    set_frame(&mut app, halden, Vec3::ZERO);
    run(&mut app, 2);
    assert!(shown_brackets(&mut app).is_empty());
}

/// HOV-07 / MRK-07: close in, Halden IV is a true-size disc and its box
/// grows with it, clearing the disc by the margin.
#[test]
fn the_bracket_box_grows_with_a_true_size_disc() {
    let mut app = brackets_app();
    let margin = Tunables::default().bracket_margin_px;
    let halden = body(&mut app, "Halden IV");
    set_frame(&mut app, halden, Vec3::ZERO);
    app.world_mut().resource_mut::<CameraRig>().distance = 0.01;
    app.world_mut().resource_mut::<Selection>().select(halden);
    run(&mut app, 3);
    let (centre, half) = marker_drawn(&mut app, halden).unwrap();
    assert!(
        half > Tunables::default().planet_marker_px,
        "a disc: {half}"
    );
    let marks = shown_brackets(&mut app);
    assert_eq!(marks.len(), 1);
    assert!(close(marks[0].centre(), centre));
    assert!(close(marks[0].size, Vec2::splat(2.0 * (half + margin))));
}

/// SYM-14 / HOV-07: an unfolded ship's box is its `ship_symbol_px` square
/// and the margin, not its amplifiers.
#[test]
fn a_ships_box_encloses_its_symbol_square() {
    let mut app = brackets_app();
    let tunables = Tunables::default();
    let halden = body(&mut app, "Halden IV");
    let orison = body(&mut app, "Orison");
    set_frame(&mut app, halden, Vec3::ZERO);
    app.world_mut().resource_mut::<Selection>().select(orison);
    let mut checked = false;
    for distance in [1.0, 0.5, 0.25] {
        app.world_mut().resource_mut::<CameraRig>().distance = distance;
        run(&mut app, 20);
        let folded = app
            .world()
            .get::<SatelliteFoldState>(orison)
            .unwrap()
            .anim_t
            < 1.0;
        let Some((centre, _)) = marker_drawn(&mut app, orison).filter(|_| !folded) else {
            continue;
        };
        let marks = shown_brackets(&mut app);
        assert_eq!(marks.len(), 1);
        assert!(close(marks[0].centre(), centre));
        let side = tunables.ship_symbol_px + 2.0 * tunables.bracket_margin_px;
        assert_eq!(marks[0].size, Vec2::splat(side));
        checked = true;
        break;
    }
    assert!(checked, "Orison was never drawn on its own");
}

/// HOV-05 / HOV-06 / HOV-07 / MOT-14: at 4 d/s in Halden IV's frame the
/// parked ships' High band (about 12 d, 3 s apparent) is an exposure ring
/// whose identity mark is a multi-marker; hovering the orbit brackets
/// both its frames at `bracket_hover_alpha`.
#[test]
fn hovering_a_shared_orbit_brackets_its_identity_multi_marker() {
    use crate::view::rings::marks::RingMarkFor;
    use crate::view::targets::OrbitFor;
    let mut app = brackets_app();
    let tunables = Tunables::default();
    let halden = body(&mut app, "Halden IV");
    let orison = body(&mut app, "Orison");
    let high = app.world().get::<InNamedOrbit>(orison).unwrap().0;
    let mut clock = app.world_mut().resource_mut::<GameClock>();
    clock.toggle_pause();
    clock.run_at(4.0);
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(halden);
    run(&mut app, 60 + eased_frames());

    let (mark, half, node) = {
        let mut marks = app.world_mut().query::<(Entity, &RingMarkFor, &Node)>();
        marks
            .iter(app.world())
            .find(|(_, mark, _)| mark.owner == high)
            .map(|(e, mark, node)| (e, mark.half, node.clone()))
            .unwrap()
    };
    assert_eq!(node.display, Display::Flex, "the High band is a ring");
    assert_eq!(half, tunables.ship_symbol_px / 2.0);
    app.world_mut().spawn((OrbitFor(high), Hovered));
    run(&mut app, 2);

    let node = app.world().get::<Node>(mark).unwrap().clone();
    let ring_point = node_min(&node).expect("mark placed") + Vec2::splat(half);
    let offset = tunables.back_frame_offset_px;
    let marks = shown_brackets(&mut app);
    let around = marks
        .iter()
        .find(|m| close(m.centre(), ring_point + Vec2::new(offset, -offset) / 2.0))
        .expect("brackets round the identity multi-marker");
    let margin = tunables.bracket_margin_px;
    let side = tunables.ship_symbol_px + offset + 2.0 * margin;
    assert!(close(around.size, Vec2::splat(side)));
    assert!((around.alpha - tunables.bracket_hover_alpha).abs() < 1e-3);
}

/// INS-06 / HOV-06: Iver Yard, alone in Iver's fold marker and selected,
/// carries its mark on that marker. Framing Iver unfolds it: the fold
/// marker hides at once, as it held nothing else, and the mark goes with
/// the yard's own marker, which starts out from its parent and slides to
/// its place, so the mark never jumps.
#[test]
fn a_last_things_mark_leaves_its_multi_marker_with_it() {
    use crate::view::fold::SatelliteFoldState;
    let mut app = brackets_app();
    run(&mut app, SETTLE_FRAMES);
    let iver = body(&mut app, "Iver");
    let yard = body(&mut app, "Iver Yard");
    app.world_mut().resource_mut::<Selection>().select(yard);
    run(&mut app, SETTLE_FRAMES);
    let fold = multi_marker(&app, MultiKey::Fold(iver)).expect("Iver's fold marker");
    assert_eq!(fold.things, [yard]);
    let marks = shown_brackets(&mut app);
    assert_eq!(marks.len(), 1);
    let offset = Tunables::default().back_frame_offset_px;
    assert!(close(
        marks[0].centre(),
        fold.centre + Vec2::new(offset, -offset) / 2.0
    ));

    app.world_mut().resource_mut::<FrameRequest>().0 = Some(iver);
    let mut first_out: Option<f32> = None;
    for _ in 0..(60 + move_frames()) {
        run(&mut app, 1);
        let t = app.world().get::<SatelliteFoldState>(yard).unwrap().anim_t;
        let Some((at, _)) = marker_drawn(&mut app, yard) else {
            continue;
        };
        if t <= 0.0 || t >= 1.0 {
            continue;
        }
        assert!(multi_marker(&app, MultiKey::Fold(iver)).is_none());
        let marks = shown_brackets(&mut app);
        assert_eq!(marks.len(), 1);
        // The yard is a station marker: its box takes in its back frame.
        let marker = marker_of(&mut app, yard);
        let back = app
            .world()
            .get::<crate::view::multi::BackFrame>(marker)
            .and_then(|b| b.0)
            .unwrap_or(Vec2::ZERO);
        let centre = marks[0].centre();
        assert!(close(centre, at + back / 2.0), "{centre} is not on {at}");
        let positions = app.world().resource::<ScreenPositions>();
        let (parent, own) = (positions.get(iver).unwrap(), positions.get(yard).unwrap());
        // How far out from its parent towards its own place the marker is.
        first_out.get_or_insert(at.distance(parent) / own.distance(parent));
    }
    let first_out = first_out.expect("never caught the yard unfolding");
    assert!(first_out < 0.5, "it starts from its parent: {first_out}");
}
