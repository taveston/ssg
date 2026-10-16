//! Headless tests of the inspection sidebar (INS): the "In orbit"
//! outline, the key facts, and the map narrowing as the sidebar opens.

use super::*;

/// The outline's rows, as the body or orbit each stands for, in order.
fn outline_rows(app: &mut App) -> Vec<Entity> {
    use crate::view::inset::outline::OutlineRow;
    let mut rows = app.world_mut().query::<(Entity, &OutlineRow)>();
    let mut listed: Vec<(Entity, Entity)> = rows.iter(app.world()).map(|(e, r)| (e, r.0)).collect();
    listed.sort_by_key(|(e, _)| e.index_u32());
    listed.into_iter().map(|(_, body)| body).collect()
}

/// INS-05 / HOV-05 / ZOOM-16: the outline is the system from the star
/// down. Selecting Halden IV opens the path to it and its satellites,
/// inside out, with Arden Highport under a "Low orbit · 1" row and the
/// parked ships under one "High orbit · N" row that a ring click opens;
/// the selected row is highlighted. A click on an entry selects it at once, a double-click asks for a frame jump, and
/// the toggle folds an entry's satellites away while the rest stays.
#[test]
fn the_outline_shows_the_system_tree_and_opens_the_selections_path() {
    use crate::view::camera::requests::PlanetaryViewRequest;
    use crate::view::inset::outline::{Expanded, OutlinePlugin, OutlineRoot, OutlineRow};

    let mut app = headless_app(FRAME);
    app.add_plugins((ContentPlugin, SimPlugin, TunablesPlugin, OutlinePlugin))
        .init_resource::<Selection>()
        .init_resource::<PlannerState>()
        .init_resource::<Frame>()
        .init_resource::<FrameRequest>()
        .init_resource::<PlanetaryViewRequest>()
        .insert_resource(placeholder_fonts());
    run(&mut app, 1);
    let star = body(&mut app, "Corvane");
    let halden = body(&mut app, "Halden IV");
    let iver = body(&mut app, "Iver");
    let morrow = body(&mut app, "Morrow");
    let iva = body(&mut app, "Halden IVa");
    let ivd = body(&mut app, "Halden IVd");
    let highport = body(&mut app, "Arden Highport");
    let orison = body(&mut app, "Orison");
    let morrow_ix = body(&mut app, "Morrow IX");
    let high = app.world().get::<InNamedOrbit>(orison).unwrap().0;
    let low = app.world().get::<InNamedOrbit>(highport).unwrap().0;
    app.world_mut().spawn((OutlineRoot, Node::default()));

    run(&mut app, 2);
    assert_eq!(outline_rows(&mut app), [star], "collapsed to the star");

    app.world_mut().resource_mut::<Selection>().select(halden);
    run(&mut app, 2);
    let rows = outline_rows(&mut app);
    assert_eq!(rows[0], star);
    let at = |rows: &[Entity], e: Entity| rows.iter().position(|&r| r == e);
    assert!(
        at(&rows, iver) < at(&rows, halden) && at(&rows, halden) < at(&rows, morrow),
        "inside out"
    );
    assert!(
        at(&rows, low) < at(&rows, iva),
        "the Low orbit is the innermost"
    );
    assert!(!rows.contains(&highport), "listed only inside its row");
    assert!(
        at(&rows, iva) < at(&rows, high),
        "IVa orbits inside the High band"
    );
    assert!(at(&rows, high) < at(&rows, ivd));
    assert!(!rows.contains(&orison), "the shared orbit is collapsed");
    assert!(!rows.contains(&morrow_ix), "Morrow is not on the path");
    let row_of = |app: &mut App, body: Entity| {
        let mut rows = app.world_mut().query::<(Entity, &OutlineRow)>();
        rows.iter(app.world())
            .find(|(_, r)| r.0 == body)
            .map(|(e, _)| e)
            .unwrap()
    };
    let halden_row = row_of(&mut app, halden);
    assert_ne!(
        app.world().get::<BackgroundColor>(halden_row).unwrap().0,
        Color::NONE
    );
    let iver_row = row_of(&mut app, iver);
    assert_eq!(
        app.world().get::<BackgroundColor>(iver_row).unwrap().0,
        Color::NONE
    );
    let title_of = |app: &mut App, orbit: Entity| {
        let row = row_of(app, orbit);
        descendants(app, row)
            .into_iter()
            .filter_map(|e| app.world().get::<Text>(e).map(|t| t.0.clone()))
            .find(|t| t.contains(" orbit · "))
            .unwrap()
    };
    assert_eq!(title_of(&mut app, low), "Low orbit · 1");
    let members = {
        let mut q = app.world_mut().query::<&InNamedOrbit>();
        q.iter(app.world()).filter(|o| o.0 == high).count()
    };
    assert_eq!(title_of(&mut app, high), format!("High orbit · {members}"));

    app.world_mut()
        .resource_mut::<Selection>()
        .select_through_orbit(halden, high);
    run(&mut app, 2);
    assert!(outline_rows(&mut app).contains(&orison));

    // A single click on IVa's label selects it at once.
    let iva_row = row_of(&mut app, iva);
    let label = app.world().get::<Children>(iva_row).unwrap()[1];
    click_node(&mut app, label, 1);
    run(&mut app, 1);
    assert_eq!(app.world().resource::<Selection>().selected, Some(iva));
    assert!(app.world().resource::<FrameRequest>().0.is_none());
    let iva_row = row_of(&mut app, iva);
    let label = app.world().get::<Children>(iva_row).unwrap()[1];
    click_node(&mut app, label, 2);
    run(&mut app, 1);
    assert_eq!(app.world().resource::<FrameRequest>().0, Some(iva));

    // Halden IV's toggle folds its satellites away; the star stays open.
    let halden_row = row_of(&mut app, halden);
    let toggle = app.world().get::<Children>(halden_row).unwrap()[0];
    click_node(&mut app, toggle, 1);
    run(&mut app, 2);
    assert!(!app.world().resource::<Expanded>().entries.contains(&halden));
    let rows = outline_rows(&mut app);
    assert!(rows.contains(&halden) && !rows.contains(&iva));
}

/// INS-05 / INS-07: the sidebar keeps its outline across selections -
/// only the details are rebuilt - and a ring click on a shared orbit
/// opens its row in that same outline.
#[test]
fn the_sidebar_keeps_its_outline_across_selections() {
    use crate::view::inset::fidelity::FidelityImages;
    use crate::view::inset::inspect::{InsetPanel, InspectPlugin};
    use crate::view::inset::outline::{OutlinePlugin, OutlineRoot};
    use crate::view::sidebar::SidebarPlugin;

    let mut app = headless_app(FRAME);
    app.add_plugins((
        ContentPlugin,
        SimPlugin,
        TunablesPlugin,
        BindingsPlugin,
        SidebarPlugin,
        OutlinePlugin,
        InspectPlugin,
    ))
    .init_resource::<Selection>()
    .init_resource::<PlannerState>()
    .init_resource::<Frame>()
    .init_resource::<FrameRequest>()
    .init_resource::<crate::view::camera::requests::PlanetaryViewRequest>()
    .init_resource::<ButtonInput<KeyCode>>()
    .init_resource::<bevy::input::mouse::AccumulatedMouseScroll>()
    .insert_resource(FidelityImages {
        surveyed: Handle::default(),
        unsurveyed: Handle::default(),
    })
    .insert_resource(placeholder_fonts());
    run(&mut app, 1);
    let halden = body(&mut app, "Halden IV");
    let iver = body(&mut app, "Iver");
    let orison = body(&mut app, "Orison");
    let high = app.world().get::<InNamedOrbit>(orison).unwrap().0;
    let root = {
        let mut roots = app
            .world_mut()
            .query_filtered::<Entity, With<OutlineRoot>>();
        roots.single(app.world()).unwrap()
    };
    let panel = {
        let mut panels = app.world_mut().query_filtered::<Entity, With<InsetPanel>>();
        panels.single(app.world()).unwrap()
    };
    assert!(
        descendants(&mut app, panel).contains(&root),
        "the outline is in the sidebar"
    );

    app.world_mut().resource_mut::<Selection>().select(halden);
    run(&mut app, 3);
    assert!(outline_rows(&mut app).contains(&halden));
    assert!(!outline_rows(&mut app).contains(&orison));
    let halden_texts: Vec<String> = descendants(&mut app, panel)
        .into_iter()
        .filter_map(|e| app.world().get::<Text>(e).map(|t| t.0.clone()))
        .collect();
    assert!(halden_texts.contains(&"Halden IV".to_string()));

    app.world_mut().resource_mut::<Selection>().select(iver);
    run(&mut app, 3);
    let mut roots = app
        .world_mut()
        .query_filtered::<Entity, With<OutlineRoot>>();
    assert_eq!(roots.single(app.world()).unwrap(), root, "the same outline");
    assert!(
        outline_rows(&mut app).contains(&halden),
        "Halden IV stays open as left"
    );

    app.world_mut()
        .resource_mut::<Selection>()
        .select_through_orbit(halden, high);
    run(&mut app, 3);
    assert!(outline_rows(&mut app).contains(&orison));
}

/// INS-09: the key facts are label-over-value pairs, the label as small
/// spaced capitals in the secondary colour and the value in the data face
/// in the neutral colour; the "In orbit" section below them has a heading
/// in spaced capitals over a hairline.
#[test]
fn key_facts_are_label_over_value_pairs_under_headed_sections() {
    use crate::view::colors;
    use crate::view::inset::fidelity::FidelityImages;
    use crate::view::inset::inspect::{
        FactLabel, FactValue, Hairline, InsetPanel, InspectPlugin, SectionHeading,
    };
    use crate::view::inset::outline::OutlinePlugin;
    use crate::view::sidebar::SidebarPlugin;

    let mut app = headless_app(FRAME);
    app.add_plugins((
        ContentPlugin,
        SimPlugin,
        TunablesPlugin,
        BindingsPlugin,
        SidebarPlugin,
        OutlinePlugin,
        InspectPlugin,
    ))
    .init_resource::<Selection>()
    .init_resource::<PlannerState>()
    .init_resource::<Frame>()
    .init_resource::<FrameRequest>()
    .init_resource::<crate::view::camera::requests::PlanetaryViewRequest>()
    .init_resource::<ButtonInput<KeyCode>>()
    .init_resource::<bevy::input::mouse::AccumulatedMouseScroll>()
    .insert_resource(FidelityImages {
        surveyed: Handle::default(),
        unsurveyed: Handle::default(),
    })
    .insert_resource(placeholder_fonts());
    run(&mut app, 1);
    let fonts = placeholder_fonts();
    let halden = body(&mut app, "Halden IV");
    app.world_mut().resource_mut::<Selection>().select(halden);
    run(&mut app, 3);

    let panel = {
        let mut panels = app.world_mut().query_filtered::<Entity, With<InsetPanel>>();
        panels.single(app.world()).unwrap()
    };
    let parts = descendants(&mut app, panel);
    let labels: Vec<(Entity, String)> = parts
        .iter()
        .filter_map(|&e| app.world().get::<FactLabel>(e).map(|l| (e, l.0.clone())))
        .collect();
    let names: Vec<&str> = labels.iter().map(|(_, l)| l.as_str()).collect();
    assert!(
        names.contains(&"Population") && names.contains(&"Mean radius"),
        "{names:?}"
    );
    let (label, _) = labels.iter().find(|(_, l)| l == "Mean radius").unwrap();
    let letters: Vec<String> = descendants(&mut app, *label)
        .into_iter()
        .filter_map(|e| app.world().get::<Text>(e).map(|t| t.0.clone()))
        .collect();
    assert_eq!(
        letters.concat(),
        "MEANRADIUS",
        "one letter per text, upper case"
    );
    let first = descendants(&mut app, *label)
        .into_iter()
        .find(|&e| app.world().get::<Text>(e).is_some())
        .unwrap();
    assert_eq!(
        app.world().get::<TextColor>(first).unwrap().0,
        colors::SECONDARY
    );
    assert_eq!(
        app.world().get::<TextFont>(first).unwrap().font,
        fonts.label(1.0).font
    );
    // The value follows its label in the same fact, in the data face.
    let fact = app.world().get::<ChildOf>(*label).unwrap().parent();
    let value = app
        .world()
        .get::<Children>(fact)
        .unwrap()
        .iter()
        .find(|&e| app.world().get::<FactValue>(e).is_some())
        .unwrap();
    assert_eq!(app.world().get::<FactValue>(value).unwrap().0, "6,700 km");
    assert_eq!(
        app.world().get::<TextColor>(value).unwrap().0,
        colors::NEUTRAL
    );
    assert_eq!(
        app.world().get::<TextFont>(value).unwrap().font,
        fonts.data(1.0).font
    );
    let row = app.world().get::<ChildOf>(fact).unwrap().parent();
    assert_eq!(
        app.world().get::<Node>(row).unwrap().flex_wrap,
        FlexWrap::Wrap
    );

    let heading = parts
        .iter()
        .find(|&&e| {
            app.world()
                .get::<SectionHeading>(e)
                .is_some_and(|h| h.0 == "In orbit")
        })
        .expect("the outline's heading");
    let hairline = parts
        .iter()
        .find(|&&e| app.world().get::<Hairline>(e).is_some())
        .expect("a hairline above the section");
    assert!(
        hairline.index_u32() < heading.index_u32(),
        "the hairline comes first"
    );
    assert_eq!(app.world().get::<Node>(*hairline).unwrap().height, px(1.0));
}

/// INS-07 / CAM-05 / ZOOM-15: opening the sidebar narrows the camera's
/// viewport, the map area and the fits to the window less its width, and
/// the frame body stays centred in the visible map at the same scale
/// throughout the slide; closing it gives the whole window back.
#[test]
fn the_sidebar_narrows_the_map_and_keeps_the_frame_body_centred() {
    use crate::view::camera::fits::Fits;
    use crate::view::sidebar::SidebarState;
    use bevy::ecs::system::RunSystemOnce;

    let mut app = camera_app();
    let halden = body(&mut app, "Halden IV");
    set_frame(&mut app, halden, Vec3::ZERO);
    run(&mut app, 2);
    let distance = app.world().resource::<CameraRig>().distance;
    fn screen(app: &App) -> &ScreenPositions {
        app.world().resource::<ScreenPositions>()
    }
    assert_eq!(screen(&app).viewport, VIEWPORT.as_vec2());
    let centred = |app: &App| {
        let s = screen(app);
        let x = s.get(halden).unwrap().x;
        assert!(
            (x - s.viewport.x / 2.0).abs() <= 1.0,
            "{x} in {}",
            s.viewport.x
        );
    };
    centred(&app);

    app.world_mut().resource_mut::<Selection>().select(halden);
    let mut seen_mid_slide = false;
    for _ in 0..30 {
        app.update();
        compute_camera_headless(&mut app);
        app.update();
        compute_camera_headless(&mut app);
        let slide = app.world().resource::<SidebarState>().slide;
        if slide > 0.0 && slide < 1.0 {
            seen_mid_slide = true;
            let width = screen(&app).viewport.x;
            assert!(width < VIEWPORT.x as f32 && width > VIEWPORT.x as f32 - 320.0);
            centred(&app);
        }
    }
    assert!(seen_mid_slide, "the slide takes several frames");
    assert_eq!(app.world().resource::<SidebarState>().slide, 1.0);
    assert_eq!(
        screen(&app).viewport,
        Vec2::new(VIEWPORT.x as f32 - 320.0, VIEWPORT.y as f32)
    );
    centred(&app);
    assert_eq!(
        app.world().resource::<CameraRig>().distance,
        distance,
        "same scale"
    );
    let fits_viewport = app
        .world_mut()
        .run_system_once(|fits: Fits| fits.viewport())
        .unwrap();
    assert_eq!(fits_viewport.x, VIEWPORT.x as f32 - 320.0);
    let mut cameras = app
        .world_mut()
        .query_filtered::<&Camera, With<OrbitCamera>>();
    let viewport = cameras
        .single(app.world())
        .unwrap()
        .viewport
        .as_ref()
        .unwrap();
    assert_eq!(
        viewport.physical_size,
        UVec2::new(VIEWPORT.x - 320, VIEWPORT.y)
    );

    app.world_mut().resource_mut::<Selection>().clear();
    for _ in 0..30 {
        app.update();
        compute_camera_headless(&mut app);
    }
    assert_eq!(screen(&app).viewport, VIEWPORT.as_vec2());
    assert!(cameras.single(app.world()).unwrap().viewport.is_none());
    centred(&app);
}
