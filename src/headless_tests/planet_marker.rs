//! Headless tests of what a planet's marker carries (GRP-05, GRP-10 to
//! GRP-13): a neutral moon ring and "+N" for its folded moons, and an
//! object pip for its folded ships, groups and stations that pops out
//! into the fold marker near the pointer or on selection.

use super::*;
use crate::view::markers::moon_ring::{MoonCountLine, MoonRing, MoonRingNode};

/// Corvane's markers, multi-markers and labels, with marker picking.
fn planet_app() -> App {
    use crate::view::markers::picking::MarkerPickingPlugin;
    use bevy::picking::PickingPlugin;
    symbols_app_with(|app| {
        app.add_plugins((PickingPlugin, MarkerPickingPlugin));
    })
}

/// The marker's moon ring state, the ring node's display, the colours its
/// rim is drawn in, and the label's moon count.
fn ring_of(app: &mut App, body: Entity) -> (MoonRing, Display, Vec<Color>, String) {
    use crate::view::symbols::draw::TintSlot;
    let marker = marker_of(app, body);
    let ring = *app.world().get::<MoonRing>(marker).expect("a moon ring");
    let nodes = descendants(app, marker);
    let node = nodes
        .iter()
        .copied()
        .find(|&e| app.world().get::<MoonRingNode>(e).is_some())
        .expect("a ring node");
    let display = app.world().get::<Node>(node).unwrap().display;
    let rims: Vec<Color> = descendants(app, node)
        .into_iter()
        .filter_map(|e| app.world().get::<Tint>(e))
        .filter(|tint| tint.slot == TintSlot::Border)
        .map(|tint| tint.color)
        .collect();
    let count = nodes
        .iter()
        .find_map(|&e| {
            let line = app.world().get::<MoonCountLine>(e)?;
            (line.0 == body).then(|| app.world().get::<Text>(e).unwrap().0.clone())
        })
        .expect("a moon count line");
    (ring, display, rims, count)
}

/// The satellites of `parent` that are natural bodies: no track record.
fn natural_satellites(app: &App, parent: Entity) -> usize {
    app.world()
        .get::<Children>(parent)
        .unwrap()
        .iter()
        .filter(|&c| app.world().get::<ShipRecord>(c).is_none())
        .count()
}

fn set_cursor(app: &mut App, to: Option<Vec2>) {
    let window = the_window(app);
    app.world_mut()
        .get_mut::<Window>(window)
        .unwrap()
        .set_cursor_position(to);
}

fn on_screen(app: &App, body: Entity) -> Vec2 {
    app.world().resource::<ScreenPositions>().get(body).unwrap()
}

/// GRP-10 / GRP-11: at the system fit Morrow holds only moons: its marker
/// shows the moon ring, its rim in the neutral colour, and its label
/// "+N" counting them; it has no object marker.
#[test]
fn moons_alone_show_a_neutral_ring_and_a_count_and_no_pip() {
    let mut app = planet_app();
    run(&mut app, SETTLE_FRAMES);
    let morrow = body(&mut app, "Morrow");
    let moons = natural_satellites(&app, morrow);
    let children = app.world().get::<Children>(morrow).unwrap().len();
    assert!(moons > 0 && moons == children, "moons alone");
    let (ring, display, rims, count) = ring_of(&mut app, morrow);
    assert_eq!(
        ring,
        MoonRing {
            t: 1.0,
            count: moons
        }
    );
    assert_eq!(display, Display::Flex);
    assert_eq!(rims, [crate::view::colors::NEUTRAL]);
    assert_eq!(count, format!("+{moons}"));
    assert!(
        multi_marker(&app, MultiKey::Fold(morrow)).is_none(),
        "no pip"
    );
}

/// GRP-05 / GRP-10 / GRP-11 / GRP-12: Halden IV holds moons and ships:
/// the ships add the object pip, while the ring stays neutral and "+N"
/// counts the moons alone.
#[test]
fn folded_ships_add_a_pip_and_leave_the_ring_and_count_alone() {
    let mut app = planet_app();
    run(&mut app, SETTLE_FRAMES);
    let halden = body(&mut app, "Halden IV");
    let moons = natural_satellites(&app, halden);
    let children = app.world().get::<Children>(halden).unwrap().len();
    assert!(moons > 0 && moons < children, "moons and ships");
    let (ring, _, rims, count) = ring_of(&mut app, halden);
    assert_eq!(ring.count, moons);
    assert_eq!(rims, [crate::view::colors::NEUTRAL]);
    assert_eq!(count, format!("+{moons}"));
    let fold = multi_marker(&app, MultiKey::Fold(halden)).expect("an object pip");
    assert_eq!(fold.count(), children - moons);
}

/// The front frame's side Halden IV's object marker is drawn at, and
/// whether its count shows.
fn halden_pop(app: &App, halden: Entity) -> (f32, bool) {
    let fold = multi_marker(app, MultiKey::Fold(halden)).expect("an object marker");
    (fold.frame_px, fold.count_shown)
}

/// GRP-13: the pointer at Halden IV pops its pip out into the full fold
/// marker with its count; moved into the band between `pop_reach_px` and
/// `pop_release_px` it stays out, and beyond the band it goes back to a
/// pip.
#[test]
fn the_pointer_near_pops_the_pip_out_and_away_lets_it_back() {
    let mut app = planet_app();
    let t = Tunables::default();
    run(&mut app, SETTLE_FRAMES);
    let halden = body(&mut app, "Halden IV");
    assert_eq!(
        halden_pop(&app, halden),
        (t.object_pip_px, false),
        "at rest"
    );
    let at = on_screen(&app, halden);
    // Up and to the left, away from the other parents.
    let away = |d: f32| at + Vec2::new(-d, -d) / std::f32::consts::SQRT_2;
    set_cursor(&mut app, Some(away(t.pop_reach_px - 2.0)));
    run(&mut app, SETTLE_FRAMES);
    assert_eq!(halden_pop(&app, halden), (t.ship_symbol_px, true), "popped");
    set_cursor(
        &mut app,
        Some(away((t.pop_reach_px + t.pop_release_px) / 2.0)),
    );
    run(&mut app, SETTLE_FRAMES);
    assert_eq!(
        halden_pop(&app, halden).0,
        t.ship_symbol_px,
        "held in the band"
    );
    set_cursor(&mut app, Some(away(t.pop_release_px + 4.0)));
    run(&mut app, SETTLE_FRAMES);
    assert_eq!(halden_pop(&app, halden), (t.object_pip_px, false), "back");
}

/// GRP-13: with no pointer at all, selecting Halden IV pops its marker
/// out, and clearing the selection lets it back.
#[test]
fn selecting_the_parent_pops_it_without_a_pointer() {
    let mut app = planet_app();
    let t = Tunables::default();
    set_cursor(&mut app, None);
    run(&mut app, SETTLE_FRAMES);
    let halden = body(&mut app, "Halden IV");
    app.world_mut().resource_mut::<Selection>().select(halden);
    run(&mut app, SETTLE_FRAMES);
    let fold = multi_marker(&app, MultiKey::Fold(halden)).unwrap();
    assert_eq!((fold.frame_px, fold.count_shown), (t.ship_symbol_px, true));
    // GRP-13 / MUL-06: popped out, its front frame starts at the corner of
    // Halden IV's box.
    let h = crate::view::markers::parent_clear_px(BodyClass::Planet, false, 0.0, &t);
    let front_min = fold.centre - Vec2::splat(t.ship_symbol_px / 2.0);
    assert!(close(front_min, on_screen(&app, halden) + Vec2::splat(h)));
    app.world_mut().resource_mut::<Selection>().clear();
    run(&mut app, SETTLE_FRAMES);
    assert_eq!(halden_pop(&app, halden).0, t.object_pip_px);
}

/// GRP-12 / LBL-01: Halden IV's label tries left first, beside the box
/// that takes in its popped fold marker, so it stays put when the marker
/// pops out on selection.
#[test]
fn the_parents_label_stays_put_when_its_marker_pops() {
    use super::markers::label_place;
    use crate::view::markers::labels::Slot;
    const LABEL_SIZE: Vec2 = Vec2::new(70.0, 26.0);
    let mut app = planet_app();
    set_cursor(&mut app, None);
    run(&mut app, SETTLE_FRAMES);
    settle_labels(&mut app, LABEL_SIZE);
    let halden = body(&mut app, "Halden IV");
    let before = label_place(&mut app, halden);
    assert_eq!(before.1.slot, Slot::Left, "left first");
    app.world_mut().resource_mut::<Selection>().select(halden);
    run(&mut app, SETTLE_FRAMES);
    settle_labels(&mut app, LABEL_SIZE);
    assert!(
        multi_marker(&app, MultiKey::Fold(halden))
            .unwrap()
            .count_shown
    );
    assert_eq!(label_place(&mut app, halden), before, "moved on pop");
}

/// GRP-10 / MRK-08: the moon ring adds to its parent's hit area: a point
/// on the ring past the marker's own reach hits Halden IV.
#[test]
fn the_moon_ring_counts_in_the_parents_hit_area() {
    use crate::view::markers::picking::{MarkerSpots, MarkerTarget, marker_under};
    let mut app = planet_app();
    set_cursor(&mut app, None);
    run(&mut app, SETTLE_FRAMES);
    let t = Tunables::default();
    let halden = body(&mut app, "Halden IV");
    let target = {
        let mut targets = app
            .world_mut()
            .query_filtered::<(Entity, &MarkerFor), With<MarkerTarget>>();
        targets
            .iter(app.world())
            .find(|(_, m)| m.0 == halden)
            .map(|(e, _)| e)
            .unwrap()
    };
    let half = t.planet_marker_px / 2.0;
    let ring = t.moon_ring_offset_px;
    // The ring's upper right corner, beyond the minimum reach.
    let point = on_screen(&app, halden) + Vec2::new(half + ring - 0.5, -(half + ring - 0.5));
    assert!(point.distance(on_screen(&app, halden)) > t.min_hit_radius_px);
    let spots = &app.world().resource::<MarkerSpots>().0;
    assert_eq!(marker_under(point, spots).map(|h| h.0), Some(target));
}
