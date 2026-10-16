//! Headless tests of the map's and the top bar's type sizes (MRK-19): on
//! Corvane in Halden IV's frame, with markers, exposure rings, a planned
//! path, the plot furniture, the frame cue and the breadcrumb, every text
//! takes its size from the tunable its role names.

use super::*;
use crate::view::edge::parent_tab::ParentTabPlugin;
use crate::view::frame_cue::FrameCuePlugin;
use crate::view::frame_cue::draw::CueNorthLabel;
use crate::view::markers::{MarkerLabel, SecondLineFor};
use crate::view::paths::PathsPlugin;
use crate::view::paths::marks::PathLabelText;
use crate::view::planner::PlannerPlugin;
use crate::view::plot_marks::NorthLabel;
use crate::view::rings::marks::{PeriodLine, RingLabelFor, RingMarksPlugin};
use crate::view::text_size::TextSize;

/// Corvane in Halden IV's frame with every kind of map text drawn: marker
/// names and second lines, exposure ring names and periods, a path's
/// labels, the north dial, the frame cue and the breadcrumb.
fn text_app() -> App {
    let mut app = symbols_app_with(|app| {
        app.add_plugins((
            PlannerPlugin,
            PathsPlugin,
            SharedOrbitPlugin,
            RingMarksPlugin,
            PlotMarksPlugin,
            FrameCuePlugin,
            ParentTabPlugin,
            BreadcrumbPlugin,
        ))
        .init_resource::<crate::view::camera::requests::PlanetaryViewRequest>();
    });
    app.world_mut().spawn((BreadcrumbRoot, Node::default()));
    let halden = body(&mut app, "Halden IV");
    let mut clock = app.world_mut().resource_mut::<GameClock>();
    clock.toggle_pause();
    clock.run_at(0.2);
    app.world_mut().resource_mut::<FrameRequest>().0 = Some(halden);
    run(&mut app, 60 + eased_frames());
    super::paths::plan_orison_to_iva(&mut app);
    run(&mut app, 3);
    app
}

/// Every text entity under an entity with `C`, itself included.
fn texts_under<C: Component>(app: &mut App) -> Vec<Entity> {
    let mut roots = app.world_mut().query_filtered::<Entity, With<C>>();
    let roots: Vec<Entity> = roots.iter(app.world()).collect();
    let mut found = vec![];
    for root in roots {
        found.push(root);
        found.extend(descendants(app, root));
    }
    found.sort();
    found.dedup();
    found
        .into_iter()
        .filter(|&e| app.world().get::<Text>(e).is_some())
        .collect()
}

/// A text's size in pixels.
fn size_of(app: &App, text: Entity) -> f32 {
    match app.world().get::<TextFont>(text).unwrap().font_size {
        FontSize::Px(px) => px,
        other => panic!("a size not in pixels: {other:?}"),
    }
}

/// The map's texts by role: names (a marker's, an exposure ring's, a
/// path's label) and second lines (a marker's, a ring's period, the "N"
/// of the north dial and the frame cue), and the breadcrumb's entries.
fn texts_by_role(app: &mut App) -> [(TextSize, Vec<Entity>); 3] {
    let second_lines = texts_under::<SecondLineFor>(app);
    let marker_names: Vec<Entity> = texts_under::<MarkerLabel>(app)
        .into_iter()
        .filter(|e| !second_lines.contains(e))
        .collect();
    let ring_names = texts_under::<RingLabelFor>(app);
    let periods = texts_under::<PeriodLine>(app);
    let path_labels = texts_under::<PathLabelText>(app);
    let north_labels = texts_under::<NorthLabel>(app);
    let cue_labels = texts_under::<CueNorthLabel>(app);
    let crumbs: Vec<Entity> = texts_under::<BreadcrumbRoot>(app);
    for (what, found) in [
        ("marker names", &marker_names),
        ("marker second lines", &second_lines),
        ("exposure ring names", &ring_names),
        ("exposure ring periods", &periods),
        ("path labels", &path_labels),
        ("the north dial's N", &north_labels),
        ("the frame cue's N", &cue_labels),
        ("breadcrumb entries", &crumbs),
    ] {
        assert!(!found.is_empty(), "no {what} drawn");
    }
    let names = [marker_names, ring_names, path_labels].concat();
    let small = [second_lines, periods, north_labels, cue_labels].concat();
    [
        (TextSize::MapLabel, names),
        (TextSize::SecondLine, small),
        (TextSize::Bar, crumbs),
    ]
}

/// MRK-19: an exposure ring's name is the same size as a marker's name,
/// `map_label_px`, and its period line the same as a marker's second
/// line, `map_second_line_px`.
#[test]
fn a_ring_name_and_a_marker_name_share_one_size() {
    let mut app = text_app();
    let t = Tunables::default();
    let second_lines = texts_under::<SecondLineFor>(&mut app);
    let marker_name = texts_under::<MarkerLabel>(&mut app)
        .into_iter()
        .find(|e| !second_lines.contains(e))
        .expect("a marker name");
    let ring_name = texts_under::<RingLabelFor>(&mut app)[0];
    let period = texts_under::<PeriodLine>(&mut app)[0];
    assert_eq!(size_of(&app, marker_name), t.map_label_px);
    assert_eq!(size_of(&app, ring_name), size_of(&app, marker_name));
    assert_eq!(size_of(&app, second_lines[0]), t.map_second_line_px);
    assert_eq!(size_of(&app, period), size_of(&app, second_lines[0]));
}

/// MRK-19: each of `map_label_px`, `map_second_line_px` and
/// `bar_text_px` sets the size of every text its role names, so changing
/// one resizes them all, even those already drawn, and nothing else.
#[test]
fn changing_a_text_size_tunable_changes_every_text_it_names() {
    let mut app = text_app();
    let defaults = Tunables::default();
    for (role, texts) in texts_by_role(&mut app) {
        for text in texts {
            assert_eq!(app.world().get::<TextSize>(text), Some(&role));
            assert_eq!(size_of(&app, text), role.px(&defaults));
        }
    }
    {
        let mut t = app.world_mut().resource_mut::<Tunables>();
        t.map_label_px = 21.0;
        t.map_second_line_px = 15.0;
        t.bar_text_px = 18.0;
    }
    run(&mut app, 2);
    let changed = app.world().resource::<Tunables>().clone();
    for (role, texts) in texts_by_role(&mut app) {
        for text in texts {
            assert_eq!(size_of(&app, text), role.px(&changed), "{role:?}");
        }
    }
    let mut all = app.world_mut().query::<(&TextSize, &TextFont)>();
    for (role, font) in all.iter(app.world()) {
        assert_eq!(font.font_size, FontSize::Px(role.px(&changed)));
    }
}
