//! Headless tests of ship symbols and groups (SYM-09, SYM-10, SYM-12,
//! SYM-13, GRP-09, TEST-06): condition bars, task forces and their count
//! and bracket, groups that share a place or fold, and the amplifiers
//! round a group's symbol.

use super::*;

/// SYM-12: own ships with a known condition carry a bar filled to it under
/// the frame, in their colour above the warning fraction; other ships have
/// none.
#[test]
fn own_ships_carry_a_condition_bar_filled_to_their_condition() {
    use crate::view::symbols::draw::ConditionFill;
    let mut app = camera_app_with(|app| {
        app.insert_resource(placeholder_fonts()).add_plugins((
            EdgePlugin,
            MarkersPlugin,
            HoveredPlugin,
        ));
    });
    run(&mut app, 3);
    let orison = body(&mut app, "Orison");
    let tolland = body(&mut app, "RFA Tolland");
    let mut fills = app
        .world_mut()
        .query_filtered::<(Entity, &Node, &BackgroundColor), With<ConditionFill>>();
    let fills: Vec<(Entity, Val, Color)> = fills
        .iter(app.world())
        .map(|(e, node, bg)| (e, node.width, bg.0))
        .collect();
    let owner = |app: &App, mut e: Entity| loop {
        if let Some(marker) = app.world().get::<MarkerFor>(e) {
            return marker.0;
        }
        e = app
            .world()
            .get::<ChildOf>(e)
            .expect("under a marker")
            .parent();
    };
    let mut owners: Vec<Entity> = Vec::new();
    for &(fill, width, color) in &fills {
        let ship = owner(&app, fill);
        owners.push(ship);
        if ship == orison {
            assert_eq!(width, Val::Percent(90.0));
            assert_eq!(color, crate::view::colors::OWN);
        }
        if ship == tolland {
            assert_eq!(width, Val::Percent(55.0));
            assert_eq!(color, crate::view::colors::OWN);
        }
        let affiliation = *app.world().get::<Affiliation>(ship).unwrap();
        assert_eq!(affiliation, Affiliation::Own);
    }
    assert!(owners.contains(&orison) && owners.contains(&tolland));
}

/// SYM-09 / SYM-13 / TEST-06: TF-1 is drawn as one symbol, with its
/// member count "4" at the upper right and the task-force bracket; its
/// four warships have no markers of their own, no orbit and no class of
/// their own; its label's second line counts its members by class.
#[test]
fn a_task_force_is_one_symbol_with_its_count_and_bracket() {
    use crate::sim::group::ShipGroup;
    use crate::sim::track::ShipClass;
    use crate::view::markers::SecondLineFor;
    use crate::view::symbols::draw::{GroupCount, TaskForceBracket};

    let mut app = symbols_app_with(|_| {});
    let tf1 = body(&mut app, "TF-1");
    let group = app.world().get::<ShipGroup>(tf1).unwrap().clone();
    assert!(group.task_force);
    assert_eq!(group.members.len(), 4);
    let record = app.world().get::<ShipRecord>(tf1).unwrap();
    assert_eq!(record.class, ShipClass::Warship);

    let marker = marker_of(&mut app, tf1);
    assert_eq!(texts_with::<GroupCount>(&mut app, marker), ["4"]);
    let brackets = descendants(&mut app, marker)
        .into_iter()
        .filter(|&node| app.world().get::<TaskForceBracket>(node).is_some())
        .count();
    assert_eq!(brackets, 1);

    let mut targets = app.world_mut().query::<&MarkerFor>();
    let drawn: Vec<Entity> = targets.iter(app.world()).map(|m| m.0).collect();
    for member in &group.members {
        assert!(!drawn.contains(member), "a member is drawn on its own");
        let member = app.world().entity(*member);
        assert!(!member.contains::<Orbit>() && !member.contains::<BodyClass>());
    }

    let mut lines = app.world_mut().query::<(&SecondLineFor, &Text)>();
    let line = lines
        .iter(app.world())
        .find(|(line_for, _)| line_for.0 == tf1)
        .map(|(_, text)| text.0.clone());
    assert_eq!(line.as_deref(), Some("4 warships"));
}

/// SYM-12 / TEST-06: TF-1's condition bar is filled to its members'
/// mean, 0.45, in the warning orange; every other own ship's bar is in
/// its own colour, so both bar colours show.
#[test]
fn the_task_force_bar_warns_and_the_other_own_bars_do_not() {
    use crate::view::colors;
    use crate::view::symbols::draw::ConditionFill;

    let mut app = symbols_app_with(|_| {});
    let tf1 = body(&mut app, "TF-1");
    let mut own = app
        .world_mut()
        .query::<(Entity, &Affiliation, &ShipRecord, &DisplayName)>();
    let own_ships: Vec<(Entity, String)> = own
        .iter(app.world())
        .filter(|(_, a, r, _)| **a == Affiliation::Own && r.condition.is_some())
        .map(|(e, .., name)| (e, name.0.clone()))
        .collect();
    assert!(
        own_ships.len() >= 9,
        "{} own ships with a bar",
        own_ships.len()
    );
    for (ship, name) in own_ships {
        let marker = marker_of(&mut app, ship);
        let fill = descendants(&mut app, marker)
            .into_iter()
            .find(|&node| app.world().get::<ConditionFill>(node).is_some())
            .unwrap_or_else(|| panic!("{name} has no condition bar"));
        let tint = app.world().get::<Tint>(fill).unwrap().color;
        if ship == tf1 {
            assert_eq!(tint, colors::WARNING);
            let width = app.world().get::<Node>(fill).unwrap().width;
            let Val::Percent(percent) = width else {
                panic!("the fill is not a share of the bar");
            };
            assert!((percent - 45.0).abs() < 1e-3, "{percent}");
        } else {
            assert_ne!(tint, colors::WARNING, "{name} warns");
        }
    }
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

/// SYM-10: two groups at the same place are two symbols, each with its
/// own count, never merged into one.
#[test]
fn two_groups_in_one_place_stay_two_symbols() {
    use crate::view::symbols::draw::GroupCount;

    let mut app = symbols_app_with(|app| {
        app.add_systems(
            Startup,
            add_a_second_group_beside_tf1
                .in_set(Boot::World)
                .before(crate::sim::spawn::spawn_system),
        );
    });
    let halden = body(&mut app, "Halden IV");
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(halden);
    run(&mut app, 60 + move_frames());

    let tf1 = body(&mut app, "TF-1");
    let tf2 = body(&mut app, "TF-3");
    let positions = app.world().resource::<WorldPositions>();
    assert!(positions.0[&tf1].distance(positions.0[&tf2]) < 1e-3);
    let (one, two) = (marker_of(&mut app, tf1), marker_of(&mut app, tf2));
    assert_ne!(one, two);
    assert_eq!(texts_with::<GroupCount>(&mut app, one), ["4"]);
    assert_eq!(texts_with::<GroupCount>(&mut app, two), ["2"]);
}

/// GRP-09: at the system fit TF-1 is folded into Halden IV as one
/// satellite: Halden IV's "+N" pill counts its satellites with the group
/// once, and the group's ships are not satellites at all.
#[test]
fn a_folded_group_counts_once_in_the_pill() {
    use crate::sim::group::ShipGroup;

    let mut app = symbols_app_with(|_| {});
    run(&mut app, SETTLE_FRAMES);
    let halden = body(&mut app, "Halden IV");
    let tf1 = body(&mut app, "TF-1");
    assert!(app.world().get::<SatelliteFoldState>(tf1).unwrap().folded);
    let mut satellites = app.world_mut().query::<(Entity, &ChildOf, &BodyClass)>();
    let of_halden: Vec<Entity> = satellites
        .iter(app.world())
        .filter(|(_, child_of, _)| child_of.parent() == halden)
        .map(|(entity, ..)| entity)
        .collect();
    assert!(of_halden.contains(&tf1));
    let members = app.world().get::<ShipGroup>(tf1).unwrap().members.clone();
    assert!(members.iter().all(|m| !of_halden.contains(m)));

    let mut pills = app.world_mut().query::<(&GroupPillFor, &Children)>();
    let count = pills
        .iter(app.world())
        .find(|(pill_for, _)| pill_for.0 == halden)
        .and_then(|(_, children)| {
            children
                .iter()
                .find_map(|child| app.world().get::<Text>(child))
        })
        .unwrap();
    assert_eq!(count.0, format!("+{}", of_halden.len()));
}

/// SYM-13: TF-1's label sits `label_gap_px` from the outer edge of its
/// frame's box (HOV-07) and its amplifiers - the count to the right, the
/// bracket above, the condition bar below - not from the frame alone.
#[test]
fn a_group_label_keeps_clear_of_its_amplifiers() {
    use crate::view::markers::MarkerLabel;
    use crate::view::markers::labels::{LabelState, label_rect};
    use crate::view::symbols::draw::AmplifierReach;

    let mut app = symbols_app_with(|_| {});
    let halden = body(&mut app, "Halden IV");
    let tf1 = body(&mut app, "TF-1");
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(halden);
    run(&mut app, 60 + move_frames());
    app.world_mut().resource_mut::<Selection>().select(tf1);

    const LABEL_SIZE: Vec2 = Vec2::new(80.0, 28.0);
    settle_labels(&mut app, LABEL_SIZE);

    let marker = marker_of(&mut app, tf1);
    let reach = *app.world().get::<AmplifierReach>(marker).unwrap();
    assert!(reach.right > 0.0 && reach.top > 0.0 && reach.bottom > 0.0);
    let size = app.world().get::<MarkerHalfSize>(marker).unwrap().0 * 2.0;
    let mut labels = app
        .world_mut()
        .query_filtered::<(&ChildOf, &Node, &LabelState), With<MarkerLabel>>();
    let (node, state) = labels
        .iter(app.world())
        .find(|(child_of, ..)| child_of.parent() == marker)
        .map(|(_, node, state)| (node.clone(), *state))
        .unwrap();
    let placement = state.shown.expect("the selection's label is shown");
    let gap = app.world().resource::<Tunables>().label_gap_px;
    let margin = app.world().resource::<Tunables>().bracket_margin_px;
    let grown = reach.label_box(Rect::from_corners(Vec2::ZERO, size), margin);
    let want = label_rect(grown, LABEL_SIZE, placement.slot, gap).min;
    let (Val::Px(left), Val::Px(top)) = (node.left, node.top) else {
        panic!("label not placed in pixels");
    };
    assert!(
        (Vec2::new(left, top) - want).length() < 1e-3,
        "label at ({left}, {top}), want {want} ({:?})",
        placement.slot
    );
}

/// SYM-09 / SYM-12: TF-1's count sits `amplifier_gap_px` right of its
/// symbol's box (HOV-07), its task-force bracket's feet that far above
/// it, and its condition bar starts that far below it, with the unfilled
/// part in the bar's colour mixed into the ground at
/// `condition_track_mix`. An own ship whose condition isn't known has no
/// bar.
#[test]
fn group_amplifiers_sit_just_outside_the_symbols_box() {
    use crate::view::colors;
    use crate::view::symbols::draw::{
        ConditionFill, GroupCount, TaskForceBracket, condition_track_color,
    };

    let mut app = symbols_app_with(|_| {});
    let tunables = app.world().resource::<Tunables>().clone();
    let tf1 = body(&mut app, "TF-1");
    let marker = marker_of(&mut app, tf1);
    let size = tunables.ship_symbol_px;
    let outside = tunables.bracket_margin_px + tunables.amplifier_gap_px;
    let parts = descendants(&mut app, marker);
    let part_with = |app: &App, has: fn(EntityRef) -> bool| {
        *parts
            .iter()
            .find(|&&node| has(app.world().entity(node)))
            .expect("part drawn")
    };
    let count = part_with(&app, |e| e.contains::<GroupCount>());
    let holder = app.world().get::<ChildOf>(count).unwrap().parent();
    assert_eq!(
        app.world().get::<Node>(holder).unwrap().left,
        px(size + outside)
    );
    let bracket = part_with(&app, |e| e.contains::<TaskForceBracket>());
    assert_eq!(
        app.world().get::<Node>(bracket).unwrap().bottom,
        px(size + outside)
    );
    let fill = part_with(&app, |e| e.contains::<ConditionFill>());
    let track = app.world().get::<ChildOf>(fill).unwrap().parent();
    assert_eq!(
        app.world().get::<Node>(track).unwrap().top,
        px(size + outside)
    );
    // TF-1's bar warns (SYM-12, TEST-06).
    assert_eq!(
        app.world().get::<Tint>(track).unwrap().color,
        condition_track_color(colors::WARNING, tunables.condition_track_mix)
    );

    // An own ship with no known condition has no bar.
    let mut own = app
        .world_mut()
        .query::<(Entity, &Affiliation, &ShipRecord)>();
    let unknown: Vec<Entity> = own
        .iter(app.world())
        .filter(|(_, a, r)| **a == Affiliation::Own && r.condition.is_none())
        .map(|(e, ..)| e)
        .collect();
    assert!(!unknown.is_empty());
    for ship in unknown {
        let marker = marker_of(&mut app, ship);
        let bars = descendants(&mut app, marker)
            .into_iter()
            .filter(|&node| app.world().get::<ConditionFill>(node).is_some())
            .count();
        assert_eq!(bars, 0);
    }
}
