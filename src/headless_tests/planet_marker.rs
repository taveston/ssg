//! Headless tests of what a planet's marker carries (GRP-05, GRP-10 to
//! GRP-13): a neutral moon ring and "+N" for its folded moons, and an
//! object pip for its folded ships, groups and stations that pops out
//! into the fold marker near the pointer or on selection.

use super::*;
use crate::view::markers::labels::Slot;
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

/// GRP-10 / HOV-07: as Morrow's ring fades in at the start, it is drawn
/// at the marker's back frame, which grows with the fade, so the ring and
/// the box round it agree on every frame.
#[test]
fn the_moon_ring_is_drawn_where_its_box_says_as_it_fades_in() {
    use crate::view::markers::moon_ring::{MoonRingParts, ring_offset};
    use crate::view::multi::BackFrame;
    let mut app = planet_app();
    let t = Tunables::default();
    let morrow = body(&mut app, "Morrow");
    let mut fading = 0;
    for _ in 0..SETTLE_FRAMES {
        run(&mut app, 1);
        let marker = marker_of(&mut app, morrow);
        let ring = *app.world().get::<MoonRing>(marker).unwrap();
        let back = app.world().get::<BackFrame>(marker).unwrap().0;
        assert_eq!(back, ring_offset(ring, t.moon_ring_offset_px));
        let parts = *app.world().get::<MoonRingParts>(marker).unwrap();
        let node = app.world().get::<Node>(parts.ring).unwrap();
        if ring.t > 0.0 {
            let offset = back.unwrap();
            assert_eq!((node.left, node.top), (px(offset.x), px(offset.y)));
        }
        if ring.t > 0.0 && ring.t < 1.0 {
            fading += 1;
        }
    }
    assert!(fading > 0, "never caught the ring fading in");
}

/// Halden IV's object marker this frame: how strongly it is drawn, and
/// how far it has faded in.
fn halden_pip(app: &App, halden: Entity) -> (f32, f32) {
    use crate::view::multi_markers::pop::ObjectPops;
    let alpha = multi_marker(app, MultiKey::Fold(halden)).map_or(0.0, |m| m.alpha);
    let shown = app.world().resource::<ObjectPops>().of(halden).shown;
    (alpha, shown)
}

/// GRP-12 / GRP-03: Halden IV's pip, new at the start, fades in a step of
/// `fold_unfold_anim_ms` a frame until it is fully drawn.
#[test]
fn a_new_pip_fades_in_over_the_fold_time() {
    let mut app = planet_app();
    let t = Tunables::default();
    let halden = body(&mut app, "Halden IV");
    let step = crate::view::anim::step_per_frame(FRAME.as_secs_f32(), t.fold_unfold_anim_ms);
    run(&mut app, 1);
    let (mut last, shown) = halden_pip(&app, halden);
    assert!(last > 0.0 && last < 1.0, "fading in: {last}");
    assert_eq!(last, shown);
    let frames = (1.0 / step).ceil() as usize;
    for _ in 0..frames {
        run(&mut app, 1);
        let (alpha, shown) = halden_pip(&app, halden);
        assert_eq!(alpha, shown, "drawn as far as it has faded in");
        assert!(
            alpha == 1.0 || (alpha - last - step).abs() < 1e-4,
            "{last} to {alpha}"
        );
        last = alpha;
    }
    assert_eq!(last, 1.0, "fully drawn within the fold time");
}

/// GRP-12 / MOT-11: at 200 d/s Halden IV's dot has faded into its
/// exposure ring and its pip with it. Paused, the dot fades back in as
/// its tail eases, and the pip is drawn at the dot's strength throughout.
#[test]
fn the_pip_fades_with_its_parents_dot() {
    use crate::view::motion_mode::MotionModeState;
    let mut app = planet_app();
    set_cursor(&mut app, None);
    let halden = body(&mut app, "Halden IV");
    {
        let mut clock = app.world_mut().resource_mut::<GameClock>();
        clock.toggle_pause();
        clock.run_at(200.0);
    }
    run(&mut app, eased_frames());
    let dot = |app: &App| {
        app.world()
            .get::<MotionModeState>(halden)
            .unwrap()
            .dot_alpha()
    };
    assert_eq!(dot(&app), 0.0, "Halden IV is a ring");
    assert_eq!(halden_pip(&app, halden).0, 0.0, "and so is its pip");
    app.world_mut().resource_mut::<GameClock>().toggle_pause();
    let mut between = 0;
    for _ in 0..eased_frames() {
        run(&mut app, 1);
        let (alpha, shown) = halden_pip(&app, halden);
        let dot = dot(&app);
        if dot > 0.05 {
            assert!((alpha - dot * shown).abs() < 1e-4, "{alpha} beside {dot}");
        }
        if alpha > 0.05 && alpha < 0.95 {
            between += 1;
        }
    }
    assert!(between > 0, "never caught the pip fading in");
    assert_eq!(halden_pip(&app, halden).0, 1.0);
}

/// GRP-13: with no pointer and nothing selected, hovering Halden IV's pip
/// pops it out, and so does its tray being open; each lets it back when
/// it ends.
#[test]
fn hovering_the_object_marker_or_opening_its_tray_holds_it_out() {
    use crate::view::multi::{OpenTray, TrayFor};
    let mut app = planet_app();
    let t = Tunables::default();
    set_cursor(&mut app, None);
    run(&mut app, SETTLE_FRAMES);
    let halden = body(&mut app, "Halden IV");
    assert_eq!(halden_pop(&app, halden).0, t.object_pip_px, "at rest");

    let (_, target) = multi_node(&mut app, MultiKey::Fold(halden)).expect("a pip");
    app.world_mut().entity_mut(target).insert(Hovered);
    run(&mut app, SETTLE_FRAMES);
    assert_eq!(
        halden_pop(&app, halden),
        (t.ship_symbol_px, true),
        "hovered"
    );
    // Popped out, it is still the same target under the pointer.
    let (_, popped) = multi_node(&mut app, MultiKey::Fold(halden)).unwrap();
    assert_eq!(popped, target);
    app.world_mut().entity_mut(target).remove::<Hovered>();
    run(&mut app, SETTLE_FRAMES);
    assert_eq!(halden_pop(&app, halden).0, t.object_pip_px, "unhovered");

    let open = Some(TrayFor::Marker(MultiKey::Fold(halden)));
    app.world_mut().resource_mut::<OpenTray>().0 = open;
    run(&mut app, SETTLE_FRAMES);
    assert_eq!(
        halden_pop(&app, halden),
        (t.ship_symbol_px, true),
        "tray open"
    );
    app.world_mut().resource_mut::<OpenTray>().0 = None;
    run(&mut app, SETTLE_FRAMES);
    assert_eq!(halden_pop(&app, halden).0, t.object_pip_px, "tray closed");
}

/// No layout runs headless: give every marker the size its node asks
/// for, and every label line the size of its text (a glyph about 0.6 of
/// the font size wide, a line 1.2 high), the label the lines stacked; and
/// let the labels settle, long enough for one that moves to fade out and
/// back in (LBL-04).
fn settle_labels_to_their_text(app: &mut App) {
    use crate::view::markers::MarkerLabel;
    let fade_ms = Tunables::default().label_fade_ms;
    let frames = (2.0 * fade_ms / 1000.0 / FRAME.as_secs_f32()).ceil() as usize + 4;
    for _ in 0..frames.div_ceil(2) {
        let mut markers = app
            .world_mut()
            .query_filtered::<(&Node, &mut ComputedNode), With<MarkerHalfSize>>();
        for (node, mut computed) in markers.iter_mut(app.world_mut()) {
            if let (Val::Px(w), Val::Px(h)) = (node.width, node.height) {
                computed.size = Vec2::new(w, h);
            }
        }
        let mut labels = app
            .world_mut()
            .query_filtered::<(Entity, &Children), With<MarkerLabel>>();
        let labels: Vec<(Entity, Vec<Entity>)> = labels
            .iter(app.world())
            .map(|(e, c)| (e, c.iter().collect()))
            .collect();
        for (label, lines) in labels {
            let mut size = Vec2::ZERO;
            for line in lines {
                let world = app.world();
                let (Some(text), Some(font)) =
                    (world.get::<Text>(line), world.get::<TextFont>(line))
                else {
                    continue;
                };
                let FontSize::Px(font_px) = font.font_size else {
                    panic!("a size not in pixels");
                };
                let line_size = if text.0.is_empty() {
                    Vec2::ZERO
                } else {
                    Vec2::new(text.0.chars().count() as f32 * 0.6 * font_px, 1.2 * font_px)
                };
                app.world_mut().get_mut::<ComputedNode>(line).unwrap().size = line_size;
                size = Vec2::new(size.x.max(line_size.x), size.y + line_size.y);
            }
            app.world_mut().get_mut::<ComputedNode>(label).unwrap().size = size;
        }
        run(app, 2);
    }
}

/// Halden IV framed and pulled back three notches: its ships are folded
/// into its pip, while IVc, IVd and K-3 are drawn close beside it.
fn halden_with_neighbours() -> (App, Entity) {
    let mut app = planet_app();
    set_cursor(&mut app, None);
    run(&mut app, SETTLE_FRAMES);
    let halden = body(&mut app, "Halden IV");
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(halden);
    run(&mut app, 60 + move_frames());
    for _ in 0..3 {
        scroll(&mut app, -1.0);
        run(&mut app, SETTLE_FRAMES);
    }
    assert!(multi_marker(&app, MultiKey::Fold(halden)).is_some());
    let at = on_screen(&app, halden);
    for name in ["Halden IVc", "Halden IVd", "K-3"] {
        let neighbour = body(&mut app, name);
        assert!(drawn_alone(&mut app, neighbour), "{name} is drawn");
        assert!(
            on_screen(&app, neighbour).distance(at) < 60.0,
            "{name} is near"
        );
    }
    (app, halden)
}

/// Where `body`'s label is on screen, as last laid out.
fn label_rect_on_screen(app: &mut App, body: Entity) -> Rect {
    use crate::view::markers::MarkerLabel;
    let marker = marker_of(app, body);
    let marker_node = app.world().get::<Node>(marker).unwrap().clone();
    let mut labels = app
        .world_mut()
        .query_filtered::<(&ChildOf, &Node, &ComputedNode), With<MarkerLabel>>();
    let (node, size) = labels
        .iter(app.world())
        .find(|(child_of, ..)| child_of.parent() == marker)
        .map(|(_, node, computed)| (node.clone(), computed.size))
        .unwrap();
    let px = |v: Val| match v {
        Val::Px(x) => x,
        other => panic!("placed as {other:?}"),
    };
    let min = Vec2::new(
        px(marker_node.left) + px(node.left),
        px(marker_node.top) + px(node.top),
    );
    Rect::from_corners(min, min + size)
}

/// GRP-12 / LBL-01: with labels the size of their text and Halden IV's
/// moons and K-3 close by, Halden IV's label keeps clear of their
/// markers, and stays put when its object marker pops out on selection.
#[test]
fn the_parents_label_keeps_clear_of_its_neighbours_and_stays_put_on_pop() {
    use super::markers::label_place;
    use crate::view::markers::MarkerDrawn;
    let (mut app, halden) = halden_with_neighbours();
    let margin = Tunables::default().bracket_margin_px;
    settle_labels_to_their_text(&mut app);
    let before = label_place(&mut app, halden);
    assert!(before.1.full, "shown with its moon count");
    let rect = label_rect_on_screen(&mut app, halden);
    for name in ["Halden IVc", "Halden IVd", "K-3"] {
        let neighbour = body(&mut app, name);
        let marker = marker_of(&mut app, neighbour);
        let (centre, half) = app.world().get::<MarkerDrawn>(marker).unwrap().0.unwrap();
        let reach = Vec2::splat(half + margin);
        let around = Rect::from_corners(centre - reach, centre + reach);
        assert!(
            rect.intersect(around).is_empty(),
            "the label {rect:?} covers {name} at {around:?}"
        );
    }
    app.world_mut().resource_mut::<Selection>().select(halden);
    run(&mut app, SETTLE_FRAMES);
    settle_labels_to_their_text(&mut app);
    assert!(
        multi_marker(&app, MultiKey::Fold(halden))
            .unwrap()
            .count_shown,
        "popped out"
    );
    assert_eq!(label_place(&mut app, halden), before, "moved on pop");
}

/// Marks beside `label`, a label shown in full in `slot`, that cover the
/// room its "+N" line takes and none of the room its name takes, its
/// name being `name_px` high: the label then fits there only without its
/// count. Each is a ring mark, which labels keep `margin` clear of.
fn crowd_out_the_count(app: &mut App, label: Rect, slot: Slot, name_px: f32, margin: f32) {
    use crate::view::rings::marks::RingMarkFor;
    const REACH: f32 = 50.0;
    let count_px = label.height() - name_px;
    let (top, bottom) = match slot {
        Slot::Left | Slot::Right => (
            Some(label.min.y + count_px / 2.0),
            Some(label.max.y - count_px / 2.0),
        ),
        Slot::Above => (Some(label.max.y - name_px), None),
        Slot::Below => (None, Some(label.min.y + name_px)),
    };
    let (x0, x1) = (label.min.x + margin, label.max.x - margin);
    let marks = [
        top.map(|y| Rect::new(x0, y - REACH + margin, x1, y - margin)),
        bottom.map(|y| Rect::new(x0, y + margin, x1, y + REACH - margin)),
    ];
    for mark in marks.into_iter().flatten() {
        app.world_mut().spawn((
            Node::default(),
            RingMarkFor {
                owner: Entity::PLACEHOLDER,
                half: 0.0,
                shown: 1.0,
                mark: Some(mark),
                chevron: None,
            },
        ));
    }
}

/// GRP-11 / LBL-03: boxed in so that its label with the "+N" line fits
/// in none of its positions, while its name alone would, Halden IV drops
/// the count and still shows its name.
#[test]
fn the_moon_count_is_dropped_first_when_the_label_is_crowded() {
    use super::markers::label_place;
    let (mut app, halden) = halden_with_neighbours();
    let t = Tunables::default();
    let name_px = 1.2 * t.map_label_px;
    let count_line = {
        let marker = marker_of(&mut app, halden);
        descendants(&mut app, marker)
            .into_iter()
            .find(|&e| app.world().get::<MoonCountLine>(e).is_some())
            .unwrap()
    };
    settle_labels_to_their_text(&mut app);
    let display = |app: &App| app.world().get::<Node>(count_line).unwrap().display;
    assert_eq!(display(&app), Display::Flex, "the count shows");

    // Each place the full label settles in is crowded in turn.
    for _ in 0..4 {
        let (_, placement) = label_place(&mut app, halden);
        if !placement.full {
            break;
        }
        let rect = label_rect_on_screen(&mut app, halden);
        crowd_out_the_count(&mut app, rect, placement.slot, name_px, t.bracket_margin_px);
        settle_labels_to_their_text(&mut app);
    }
    let (_, placement) = label_place(&mut app, halden);
    assert!(!placement.full, "the count is dropped, the name shown");
    assert_eq!(display(&app), Display::None);
}
