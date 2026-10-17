//! Headless tests of the inset's globe (SURF-31, SURF-45, SURF-46): the
//! image frame is empty until the body's bake lands, then shows the
//! picture lit for this frame; an unsurveyed body gets the telescope
//! frame with its corner brackets. No GPU: the render world's word that
//! a face is baked is given by hand.

use super::*;
use crate::view::surfaces::SurfacesPlugin;
use crate::view::surfaces::bake::{BakeKey, BakedFaces, PendingBakeJobs, SurfaceBakes};
use crate::view::surfaces::inset::{GlobeImage, GlobePicture};
use crate::view::surfaces::shading::GlobeMaterial;

/// Corvane in a headless app with the sidebar and the surfaces.
fn surfaces_app() -> App {
    use crate::view::inset::fidelity::FidelityImages;
    use crate::view::inset::inspect::InspectPlugin;
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
        SurfacesPlugin,
    ))
    .init_resource::<Selection>()
    .init_resource::<PlannerState>()
    .init_resource::<Frame>()
    .init_resource::<FrameRequest>()
    .init_resource::<CameraRig>()
    .init_resource::<WorldPositions>()
    .init_resource::<Assets<Image>>()
    .init_resource::<Assets<GlobeMaterial>>()
    .init_resource::<crate::view::camera::requests::PlanetaryViewRequest>()
    .init_resource::<ButtonInput<KeyCode>>()
    .init_resource::<bevy::input::mouse::AccumulatedMouseScroll>()
    .insert_resource(FidelityImages {
        surveyed: Handle::default(),
        unsurveyed: Handle::default(),
    })
    .insert_resource(placeholder_fonts());
    run(&mut app, 1);
    app
}

/// The inset's globe frame and its picture node, if the inset shows one.
fn globe_nodes(app: &mut App) -> Option<(GlobeImage, Entity)> {
    let mut frames = app.world_mut().query::<(Entity, &GlobeImage)>();
    let (frame, image) = frames.iter(app.world()).next().map(|(e, g)| (e, *g))?;
    let mut pictures = app
        .world_mut()
        .query_filtered::<(Entity, &ChildOf), With<GlobePicture>>();
    let picture = pictures
        .iter(app.world())
        .find(|(_, child_of)| child_of.parent() == frame)
        .map(|(e, _)| e)?;
    Some((image, picture))
}

/// Gives the render world's word that every face of `body`'s bake is in.
fn land_bake(app: &mut App, body: Entity, knowledge: Knowledge) {
    let strip = app
        .world()
        .resource::<SurfaceBakes>()
        .entry(BakeKey { body, knowledge })
        .expect("a bake was asked for")
        .strip
        .id();
    let word = app.world().resource::<BakedFaces>().clone();
    for face in 0..6 {
        word.report(strip, face);
    }
}

/// SURF-31 / SURF-45: selecting a planet puts its globe frame in the
/// inset with the picture hidden and starts its bake, one face a frame;
/// once every face is in, the picture shows with a material on the
/// bake's strip, lit for this frame; selecting a ship shows no globe.
#[test]
fn the_inset_frame_is_empty_until_the_bake_lands() {
    let mut app = surfaces_app();
    let halden = body(&mut app, "Halden IV");
    let orison = body(&mut app, "Orison");
    app.world_mut().resource_mut::<Selection>().select(halden);
    run(&mut app, 2);
    let (image, picture) = globe_nodes(&mut app).expect("a globe frame");
    assert_eq!(image.body, halden);
    assert_eq!(
        app.world().get::<Node>(picture).unwrap().display,
        Display::None
    );
    assert!(
        app.world()
            .get::<MaterialNode<GlobeMaterial>>(picture)
            .is_none()
    );
    let jobs = app.world().resource::<PendingBakeJobs>();
    assert_eq!(jobs.0.len(), 1, "one face a frame");
    let key = BakeKey {
        body: halden,
        knowledge: Knowledge::Surveyed,
    };
    let strip = app
        .world()
        .resource::<SurfaceBakes>()
        .entry(key)
        .unwrap()
        .strip
        .clone();
    assert_eq!(jobs.0[0].strip, strip);

    land_bake(&mut app, halden, Knowledge::Surveyed);
    run(&mut app, 2);
    assert_eq!(
        app.world().get::<Node>(picture).unwrap().display,
        Display::Flex
    );
    let material = app
        .world()
        .get::<MaterialNode<GlobeMaterial>>(picture)
        .expect("the picture's material");
    let materials = app.world().resource::<Assets<GlobeMaterial>>();
    let globe = materials.get(&material.0).unwrap();
    assert_eq!(globe.strip, strip);
    assert_eq!(globe.globe.telescope, 0);
    assert!(globe.globe.light.xyz().is_normalized());
    assert!(app.world().resource::<PendingBakeJobs>().0.is_empty());

    app.world_mut().resource_mut::<Selection>().select(orison);
    run(&mut app, 2);
    assert!(
        globe_nodes(&mut app).is_none(),
        "a ship keeps the placeholder"
    );
}

/// SURF-46: an unsurveyed moon's frame carries four corner brackets at
/// `inset_bracket_alpha`, and its picture, once baked at the unsurveyed
/// face size, is the telescope frame.
#[test]
fn an_unsurveyed_body_gets_the_telescope_frame_and_brackets() {
    let mut app = surfaces_app();
    let t = Tunables::default();
    let ivc = body(&mut app, "Halden IVc");
    assert_eq!(
        *app.world().get::<Knowledge>(ivc).unwrap(),
        Knowledge::Unsurveyed
    );
    app.world_mut().resource_mut::<Selection>().select(ivc);
    run(&mut app, 2);
    let (image, picture) = globe_nodes(&mut app).expect("a globe frame");
    let frame = app.world().get::<ChildOf>(picture).unwrap().parent();
    assert_eq!(image.body, ivc);
    let brackets: Vec<Entity> = descendants(&mut app, frame)
        .into_iter()
        .filter(|&e| e != picture && app.world().get::<BorderColor>(e).is_some())
        .collect();
    assert_eq!(brackets.len(), 4);
    for bracket in brackets {
        let node = app.world().get::<Node>(bracket).unwrap();
        assert_eq!(node.position_type, PositionType::Absolute);
        let arm = (t.bracket_arm_fraction * t.inset_image_px).min(t.bracket_arm_max_px);
        assert_eq!(node.width, px(arm));
        let colour = app.world().get::<BorderColor>(bracket).unwrap().top;
        assert!((colour.alpha() - t.inset_bracket_alpha).abs() < 1e-6);
    }
    let key = BakeKey {
        body: ivc,
        knowledge: Knowledge::Unsurveyed,
    };
    let params = app
        .world()
        .resource::<SurfaceBakes>()
        .entry(key)
        .unwrap()
        .params;
    assert_eq!(params.face_px, t.bake_face_unsurveyed_px);
    land_bake(&mut app, ivc, Knowledge::Unsurveyed);
    run(&mut app, 2);
    let material = app
        .world()
        .get::<MaterialNode<GlobeMaterial>>(picture)
        .unwrap();
    let materials = app.world().resource::<Assets<GlobeMaterial>>();
    assert_eq!(materials.get(&material.0).unwrap().globe.telescope, 1);
}
