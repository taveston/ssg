//! Headless tests of the sky behind the map (SKY-17, SKY-18, SKY-19,
//! SKY-22): the real sky plugin with its meshes and materials kept as
//! assets, no renderer. What the GPU draws follows the materials'
//! uniforms and the meshes checked here.

use bevy::mesh::VertexAttributeValues;

use super::double_click::{click_app_with, click_at, selected, side_of_ring, step};
use super::*;
use crate::view::screen::MapArea;
use crate::view::sky::SkyPlugin;
use crate::view::sky::draw::{
    GlowUniform, SkyGlow, SkyGlowMaterial, SkyStars, SkyStarsMaterial, StarsUniform, clip_rect,
};

/// The sky's plugin and the asset stores its meshes and materials go in.
fn with_sky(app: &mut App) {
    app.init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<SkyStarsMaterial>>()
        .init_resource::<Assets<SkyGlowMaterial>>()
        .add_plugins(SkyPlugin);
}

/// The stars' and the glow's uniforms as they stand.
fn uniforms(app: &mut App) -> (StarsUniform, GlowUniform) {
    let stars = {
        let mut query = app
            .world_mut()
            .query_filtered::<&MeshMaterial3d<SkyStarsMaterial>, With<SkyStars>>();
        let handle = query.single(app.world()).unwrap().0.clone();
        app.world()
            .resource::<Assets<SkyStarsMaterial>>()
            .get(&handle)
            .unwrap()
            .stars
    };
    let glow = {
        let mut query = app
            .world_mut()
            .query_filtered::<&MeshMaterial3d<SkyGlowMaterial>, With<SkyGlow>>();
        let handle = query.single(app.world()).unwrap().0.clone();
        app.world()
            .resource::<Assets<SkyGlowMaterial>>()
            .get(&handle)
            .unwrap()
            .glow
    };
    (stars, glow)
}

/// Every star's direction, as the mesh holds it.
fn star_directions(app: &mut App) -> Vec<[f32; 3]> {
    let mut query = app.world_mut().query_filtered::<&Mesh3d, With<SkyStars>>();
    let handle = query.single(app.world()).unwrap().0.clone();
    let mesh = app.world().resource::<Assets<Mesh>>().get(&handle).unwrap();
    match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
        Some(VertexAttributeValues::Float32x3(positions)) => positions.clone(),
        _ => panic!("the stars have no directions"),
    }
}

/// The sky's entities' transforms and visibilities.
fn placement(app: &mut App) -> Vec<(Transform, Visibility)> {
    let mut query = app
        .world_mut()
        .query_filtered::<(&Transform, &Visibility), Or<(With<SkyStars>, With<SkyGlow>)>>();
    query.iter(app.world()).map(|(t, v)| (*t, *v)).collect()
}

/// SKY-17: the sky is static: running the clock fast, then pausing,
/// changes no star, no uniform and no transform.
#[test]
fn the_sky_does_not_change_with_the_clock() {
    let mut app = camera_app_with(with_sky);
    let before = (
        uniforms(&mut app),
        star_directions(&mut app),
        placement(&mut app),
    );
    assert!(!before.1.is_empty());
    {
        let mut clock = app.world_mut().resource_mut::<GameClock>();
        clock.toggle_pause();
        clock.run_at(200.0);
    }
    run(&mut app, 90);
    assert!(day(&app) > 100.0, "the clock ran");
    app.world_mut().resource_mut::<GameClock>().toggle_pause();
    run(&mut app, 5);
    let after = (
        uniforms(&mut app),
        star_directions(&mut app),
        placement(&mut app),
    );
    assert!(after == before, "the sky changed with the clock");
}

/// SKY-18: the sky is clipped to the map area: below the top bar and
/// left of the sidebar, following the sidebar frame by frame as it
/// slides in, as the camera's viewport does.
#[test]
fn the_sky_stays_inside_the_map_area_while_the_inset_slides() {
    use crate::view::sidebar::SidebarState;
    let mut app = camera_app_with(with_sky);
    let halden = body(&mut app, "Halden IV");
    let check = |app: &mut App| {
        let viewport = app.world().resource::<ScreenPositions>().viewport;
        let (stars, glow) = uniforms(app);
        let expected = clip_rect(&MapArea::of(viewport), 1.0);
        assert_eq!(stars.clip, expected);
        assert_eq!(glow.clip, expected);
        assert_eq!(stars.clip.y, TOP_BAR_HEIGHT_PX, "not under the top bar");
        assert_eq!(stars.clip.z, viewport.x, "not under the sidebar");
    };
    check(&mut app);
    assert_eq!(uniforms(&mut app).0.clip.z, VIEWPORT.x as f32);
    app.world_mut().resource_mut::<Selection>().select(halden);
    let mut mid_slide = 0;
    for _ in 0..30 {
        app.update();
        compute_camera_headless(&mut app);
        let slide = app.world().resource::<SidebarState>().slide;
        if slide > 0.0 && slide < 1.0 {
            mid_slide += 1;
        }
        check(&mut app);
    }
    assert!(mid_slide > 2, "the slide was seen under way");
    let narrowed = VIEWPORT.x as f32 - Tunables::default().inset_width_px;
    assert_eq!(uniforms(&mut app).0.clip.z, narrowed);
}

/// SKY-19: the sky is never a target: a click on a planet's ring with
/// the sky beneath still selects the planet, and a click on empty sky
/// selects nothing.
#[test]
fn a_click_through_the_sky_hits_what_is_beneath() {
    let mut app = click_app_with(with_sky);
    let halden = body(&mut app, "Halden IV");
    let mut sky = app
        .world_mut()
        .query_filtered::<&Pickable, Or<(With<SkyStars>, With<SkyGlow>)>>();
    let pickables: Vec<Pickable> = sky.iter(app.world()).cloned().collect();
    assert_eq!(pickables.len(), 2);
    assert!(
        pickables
            .iter()
            .all(|p| !p.is_hoverable && !p.should_block_lower)
    );

    click_at(&mut app, Vec2::new(60.0, VIEWPORT.y as f32 - 60.0));
    assert_eq!(selected(&app), None, "empty sky selects nothing");
    let point = side_of_ring(&app, halden);
    click_at(&mut app, point);
    step(&mut app, 2);
    assert_eq!(selected(&app), Some(halden));
}

/// SKY-22: the sky is always on: drawn from the first frame, and still
/// drawn after the sidebar opens and the frame changes.
#[test]
fn the_sky_is_always_on() {
    let mut app = camera_app_with(with_sky);
    let visible = |app: &mut App| {
        let placed = placement(app);
        placed.len() == 2 && placed.iter().all(|(_, v)| *v == Visibility::Visible)
    };
    assert!(visible(&mut app));
    let halden = body(&mut app, "Halden IV");
    app.world_mut().resource_mut::<Selection>().select(halden);
    set_frame(&mut app, halden, Vec3::ZERO);
    run(&mut app, SETTLE_FRAMES);
    assert!(visible(&mut app));
}
