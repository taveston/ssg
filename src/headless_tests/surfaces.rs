//! Headless tests of the inset's globe (SURF-31, SURF-45, SURF-46): the
//! image frame is empty until the body's bake lands, then shows the
//! picture lit for this frame; an unsurveyed body gets the telescope
//! frame with its corner brackets. And of the globe on the map (SURF-34,
//! SURF-50 to SURF-52, SURF-73): the projected radius decides globe or
//! disc, with hysteresis. No GPU: the render world's word that a piece
//! is baked is given by hand.

use super::*;
use crate::view::edge::EdgePlugin;
use crate::view::hovered::HoveredPlugin;
use crate::view::markers::{MarkerDrawn, MarkersPlugin, body_fill};
use crate::view::surfaces::SurfacesPlugin;
use crate::view::surfaces::bake::{BakeKey, BakedFaces, PendingBakeJobs, SurfaceBakes};
use crate::view::surfaces::inset::{GlobeImage, GlobePicture};
use crate::view::surfaces::map::MapGlobe;
use crate::view::surfaces::shading::GlobeMaterial;
use crate::view::true_size::DiscRadii;

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

/// Corvane with the camera, the markers, the inset and the globes on the
/// map, paused.
fn map_globe_app() -> App {
    use crate::view::inset::fidelity::FidelityImages;
    use crate::view::inset::inspect::InspectPlugin;
    use crate::view::inset::outline::OutlinePlugin;
    use crate::view::surfaces::map::MapGlobePlugin;

    camera_app_with(|app| {
        app.insert_resource(placeholder_fonts())
            .init_resource::<crate::view::fold::FoldSummary>()
            .init_resource::<Assets<Image>>()
            .init_resource::<Assets<GlobeMaterial>>()
            .insert_resource(FidelityImages {
                surveyed: Handle::default(),
                unsurveyed: Handle::default(),
            })
            .add_plugins((EdgePlugin, MarkersPlugin, HoveredPlugin))
            .add_plugins((OutlinePlugin, InspectPlugin, SurfacesPlugin, MapGlobePlugin));
    })
}

/// A body's map globe: its marker's state, whose `picture` is its node.
fn map_globe(app: &mut App, body: Entity) -> MapGlobe {
    let mut markers = app.world_mut().query::<(&MarkerFor, &MapGlobe)>();
    markers
        .iter(app.world())
        .find(|(marker, _)| marker.0 == body)
        .map(|(_, globe)| *globe)
        .expect("a natural body's marker has a globe")
}

/// A body's map globe's material, while the globe is drawn.
fn map_picture(app: &mut App, body: Entity) -> Option<GlobeMaterial> {
    let picture = map_globe(app, body).picture;
    let node = app.world().get::<Node>(picture).unwrap();
    if node.display == Display::None {
        return None;
    }
    let material = app
        .world()
        .get::<MaterialNode<GlobeMaterial>>(picture)
        .expect("a drawn globe has its material");
    let materials = app.world().resource::<Assets<GlobeMaterial>>();
    Some(materials.get(&material.0).unwrap().clone())
}

/// The half-size `body`'s marker is drawn at.
fn drawn_half_px(app: &mut App, body: Entity) -> f32 {
    let mut markers = app.world_mut().query::<(&MarkerFor, &MarkerDrawn)>();
    markers
        .iter(app.world())
        .find(|(marker, _)| marker.0 == body)
        .and_then(|(_, drawn)| drawn.0)
        .expect("the marker is drawn")
        .1
}

/// Moves the camera in or out until `body`'s projected radius is
/// `radius_px`: the radius goes as one over the distance.
fn zoom_to_radius(app: &mut App, body: Entity, radius_px: f32) {
    for _ in 0..3 {
        let now = app.world().resource::<DiscRadii>().get(body);
        app.world_mut().resource_mut::<CameraRig>().distance *= now / radius_px;
        run(app, 2);
    }
    let now = app.world().resource::<DiscRadii>().get(body);
    assert!(
        (now - radius_px).abs() < 0.05,
        "{now} px, wanted {radius_px}"
    );
}

/// Halden IV as the frame, the camera close.
fn close_on_halden(app: &mut App) -> Entity {
    let halden = body(app, "Halden IV");
    set_frame(app, halden, Vec3::ZERO);
    app.world_mut().resource_mut::<CameraRig>().distance = 0.01;
    run(app, 3);
    halden
}

/// SURF-50 / SURF-52 / SURF-73 / MRK-11: close in on Halden IV, its
/// disc becomes its globe once the projected radius passes
/// `globe_min_px`, but only once the bake has landed: until then the
/// disc is MRK-11's. The globe covers the disc and its gap, holds while
/// the radius stays above `globe_release_px`, and gives way to the disc
/// below it, coming back only above `globe_min_px` again.
#[test]
fn the_disc_becomes_the_globe_close_in_with_hysteresis() {
    let mut app = map_globe_app();
    let t = Tunables::default();
    let halden = close_on_halden(&mut app);

    zoom_to_radius(&mut app, halden, t.globe_release_px - 1.0);
    assert!(!map_globe(&mut app, halden).due, "a disc below the release");
    zoom_to_radius(&mut app, halden, t.globe_min_px - 1.0);
    assert!(!map_globe(&mut app, halden).due, "a disc below the minimum");
    zoom_to_radius(&mut app, halden, t.globe_min_px + 2.0);
    assert!(map_globe(&mut app, halden).due);
    // SURF-52: not baked yet, so the disc shows, filled as MRK-11's.
    assert!(map_picture(&mut app, halden).is_none());
    let mut markers = app.world_mut().query::<(&MarkerFor, &Tint)>();
    let (_, tint) = markers
        .iter(app.world())
        .find(|(marker, _)| marker.0 == halden)
        .unwrap();
    assert_eq!(tint.color, body_fill(true, &t));

    land_bake(&mut app, halden, Knowledge::Surveyed);
    run(&mut app, 2);
    let globe = map_picture(&mut app, halden).expect("the globe once baked");
    let key = BakeKey {
        body: halden,
        knowledge: Knowledge::Surveyed,
    };
    let bakes = app.world().resource::<SurfaceBakes>();
    assert_eq!(globe.strip, bakes.entry(key).unwrap().strip);
    assert!(globe.close.is_none());
    // MRK-11: the picture covers the disc and paints its gap.
    let half = drawn_half_px(&mut app, halden);
    let picture = map_globe(&mut app, halden).picture;
    let node = app.world().get::<Node>(picture).unwrap();
    let gap = t.body_gap_px;
    assert_eq!((node.left, node.top), (px(-gap), px(-gap)));
    assert_eq!(node.width, px(2.0 * (half + gap)));
    let disc_px = globe.globe.disc_fraction * globe.globe.node_px / 2.0;
    assert!((disc_px - half).abs() < 1e-3);
    assert!((globe.globe.opaque_radius * disc_px - (half + gap)).abs() < 1e-3);

    // Between the release and the minimum it holds; below the release it
    // goes, and between them again it stays a disc.
    let between = (t.globe_min_px + t.globe_release_px) / 2.0;
    zoom_to_radius(&mut app, halden, between);
    assert!(map_picture(&mut app, halden).is_some(), "held in the band");
    zoom_to_radius(&mut app, halden, t.globe_release_px - 1.0);
    assert!(map_picture(&mut app, halden).is_none(), "gone below it");
    assert!(!map_globe(&mut app, halden).due);
    zoom_to_radius(&mut app, halden, between);
    assert!(
        map_picture(&mut app, halden).is_none(),
        "a disc in the band"
    );
    zoom_to_radius(&mut app, halden, t.globe_min_px + 1.0);
    assert!(map_picture(&mut app, halden).is_some(), "back above it");
}

/// SURF-51 / SURF-33: with Halden IV selected and drawn as a globe on the
/// map, the map and the inset draw from the one bake, turned the same way
/// and lit by the same light.
#[test]
fn the_map_and_the_inset_share_the_bake_and_the_light() {
    let mut app = map_globe_app();
    let t = Tunables::default();
    let halden = close_on_halden(&mut app);
    app.world_mut().resource_mut::<Selection>().select(halden);
    zoom_to_radius(&mut app, halden, t.globe_min_px * 2.0);
    land_bake(&mut app, halden, Knowledge::Surveyed);
    run(&mut app, 2);
    let map = map_picture(&mut app, halden).expect("the map's globe");
    let (_, picture) = globe_nodes(&mut app).expect("the inset's globe");
    let material = app
        .world()
        .get::<MaterialNode<GlobeMaterial>>(picture)
        .unwrap();
    let inset = app
        .world()
        .resource::<Assets<GlobeMaterial>>()
        .get(&material.0)
        .unwrap()
        .clone();
    assert_eq!(map.strip, inset.strip, "one bake");
    assert_eq!(map.globe.light, inset.globe.light);
    assert_eq!(map.globe.light_colour, inset.globe.light_colour);
    assert_eq!(map.globe.view_from_body_x, inset.globe.view_from_body_x);
    assert_eq!(map.globe.view_from_body_y, inset.globe.view_from_body_y);
    assert_eq!(map.globe.view_from_body_z, inset.globe.view_from_body_z);
}

/// SURF-34: further in, once the projected radius passes `bake_face_px`
/// times `rebake_zoom_factor`, the face in view is baked again at
/// `rebake_face_px` with the finest craters on, and drawn from that bake
/// once it lands; the cube's bake is kept. Back out, the close face goes.
#[test]
fn close_in_the_face_in_view_is_baked_again_finer() {
    use crate::view::surfaces::chooser::{needs_rebake, rebake_face_px};
    let mut app = map_globe_app();
    let t = Tunables::default();
    let halden = close_on_halden(&mut app);
    zoom_to_radius(&mut app, halden, t.globe_min_px * 2.0);
    land_bake(&mut app, halden, Knowledge::Surveyed);
    run(&mut app, 2);
    let key = BakeKey {
        body: halden,
        knowledge: Knowledge::Surveyed,
    };
    let near = t.bake_face_px as f32 * t.rebake_zoom_factor + 20.0;
    assert!(needs_rebake(near, &t));
    zoom_to_radius(&mut app, halden, near);
    let (face, strip, params) = {
        let bakes = app.world().resource::<SurfaceBakes>();
        let (face, entry) = bakes.close_entry(key).expect("a close face");
        assert!(!entry.ready);
        (face, entry.strip.id(), entry.params)
    };
    assert_eq!(params.face_px, rebake_face_px(&t));
    assert_eq!(params.crater_scales.w, 1.0, "the finest craters on");
    let globe = map_picture(&mut app, halden).expect("the cube's globe meanwhile");
    assert!(globe.close.is_none());
    let word = app.world().resource::<BakedFaces>().clone();
    for piece in 0..64 {
        word.report(strip, piece);
    }
    run(&mut app, 2);
    let globe = map_picture(&mut app, halden).unwrap();
    assert_eq!(globe.close.as_ref().map(|h| h.id()), Some(strip));
    assert_eq!(globe.globe.close_face, face);
    assert_eq!(globe.globe.close_px, rebake_face_px(&t));
    let bakes = app.world().resource::<SurfaceBakes>();
    assert!(bakes.entry(key).unwrap().ready, "the cube is kept");

    zoom_to_radius(&mut app, halden, t.globe_min_px * 2.0);
    let bakes = app.world().resource::<SurfaceBakes>();
    assert!(bakes.close_entry(key).is_none(), "the close face goes");
    let globe = map_picture(&mut app, halden).unwrap();
    assert!(globe.close.is_none());
}
