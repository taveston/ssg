//! Headless tests of the sky behind the map (SKY-02, SKY-17, SKY-18,
//! SKY-19, SKY-22, SKY-23, SECT-05): the real sky plugin with its meshes
//! and materials kept as assets, no renderer, over a hand-made system and
//! over a generated sector. What the GPU draws follows the materials'
//! uniforms and the meshes checked here.

use bevy::render::storage::ShaderBuffer;

use super::double_click::{click_app_with, click_at, selected, side_of_ring, step};
use super::sector::sector_app_with;
use super::*;
use crate::content::generator::generate_sector;
use crate::content::generator::rng::derive_seed;
use crate::content::generator::settings::Settings;
use crate::content::generator::tunables::GeneratorTunables;
use crate::content::start::LoadedSector;
use crate::view::map_layer::MapLayer;
use crate::view::screen::MapArea;
use crate::view::sector::links::LinkPieces;
use crate::view::sky::SkyPlugin;
use crate::view::sky::draw::{
    GlowUniform, RECORD_FLOATS, SkyGlow, SkyGlowMaterial, SkySource, SkyStars, SkyStarsMaterial,
    StarsUniform, clip_rect,
};
use crate::view::sky::generate::make_sky;

/// The sky's plugin and the asset stores its meshes, star records and
/// materials go in.
fn with_sky(app: &mut App) {
    app.init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<ShaderBuffer>>()
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

/// Every star's record, as the material's storage buffer holds it.
fn star_records(app: &mut App) -> Vec<[f32; RECORD_FLOATS]> {
    let mut query = app
        .world_mut()
        .query_filtered::<&MeshMaterial3d<SkyStarsMaterial>, With<SkyStars>>();
    let handle = query.single(app.world()).unwrap().0.clone();
    let material = app
        .world()
        .resource::<Assets<SkyStarsMaterial>>()
        .get(&handle)
        .unwrap();
    let buffer = app
        .world()
        .resource::<Assets<ShaderBuffer>>()
        .get(&material.records)
        .unwrap();
    buffer
        .cast_slice::<[f32; RECORD_FLOATS]>()
        .expect("the stars have no records")
        .to_vec()
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
        star_records(&mut app),
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
        star_records(&mut app),
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

/// The seed the sky on screen was made from.
fn sky_seed_used(app: &App) -> u64 {
    app.world().resource::<SkySource>().seed
}

/// The map camera's turn: its heading and tilt as a rotation.
fn camera_rotation(app: &mut App) -> Quat {
    let mut query = app
        .world_mut()
        .query_filtered::<&Transform, With<OrbitCamera>>();
    query.single(app.world()).unwrap().rotation
}

/// The records the sky of `seed` has with these tunables.
fn records_of(seed: u64, t: &Tunables) -> Vec<[f32; RECORD_FLOATS]> {
    crate::view::sky::draw::star_records(&make_sky(seed, t), t)
}

/// SKY-02: a hand-made system loaded without a sector (Corvane here, Sol
/// the same way) gets the sky of `sky_default_seed`, drawn whole.
#[test]
fn a_system_without_a_sector_uses_the_default_sky_seed() {
    let mut app = camera_app_with(with_sky);
    let t = app.world().resource::<Tunables>().clone();
    assert!(app.world().get_resource::<LoadedSector>().is_none());
    assert_eq!(sky_seed_used(&app), t.sky_default_seed);
    let (stars, glow) = uniforms(&mut app);
    assert_eq!((stars.factor, glow.factor), (1.0, 1.0));
    assert_eq!(star_records(&mut app), records_of(t.sky_default_seed, &t));
}

/// SKY-02 / SKY-23: a game started with a sector draws the sky of the
/// seed derived from the sector's, and the sector view draws the very sky
/// the system view does, through the same camera turn, with every
/// opacity multiplied by `sector_sky_factor` and nothing else changed.
#[test]
fn the_sector_view_draws_the_system_views_sky_dimmed() {
    let mut app = sector_app_with(with_sky);
    let t = app.world().resource::<Tunables>().clone();
    let sector_seed = app.world().resource::<LoadedSector>().sector.settings.seed;
    let seed = derive_seed(sector_seed, "sky");
    assert_eq!(sky_seed_used(&app), seed);
    assert_ne!(seed, t.sky_default_seed);

    assert!(app.world().resource::<MapLayer>().is_sector());
    let in_sector = (
        uniforms(&mut app),
        star_records(&mut app),
        placement(&mut app),
        camera_rotation(&mut app),
    );
    assert_eq!(
        in_sector.1,
        records_of(seed, &t),
        "the same sky as the system view's"
    );
    assert_eq!(in_sector.0.0.factor, t.sector_sky_factor);
    assert_eq!(in_sector.0.1.factor, t.sector_sky_factor);

    *app.world_mut().resource_mut::<MapLayer>() = MapLayer::System;
    run(&mut app, 2);
    let (mut stars, mut glow) = uniforms(&mut app);
    assert_eq!(
        (stars.factor, glow.factor),
        (1.0, 1.0),
        "whole in the system view"
    );
    stars.factor = t.sector_sky_factor;
    glow.factor = t.sector_sky_factor;
    assert_eq!((stars, glow), in_sector.0, "only the factor differs");
    assert_eq!(star_records(&mut app), in_sector.1);
    assert_eq!(placement(&mut app), in_sector.2);
    assert_eq!(
        camera_rotation(&mut app),
        in_sector.3,
        "the same heading and tilt"
    );

    *app.world_mut().resource_mut::<MapLayer>() = MapLayer::Sector;
    run(&mut app, 2);
    assert_eq!(uniforms(&mut app), in_sector.0, "dimmed again on return");
}

/// SKY-02: the sky follows the loaded sector at run time, not only at
/// start-up: when another sector is loaded the sky is remade from its
/// seed, still two meshes, and nothing happens while the sector stays.
#[test]
fn the_sky_is_remade_when_another_sector_is_loaded() {
    let mut app = sector_app_with(with_sky);
    let t = app.world().resource::<Tunables>().clone();
    let before = (sky_seed_used(&app), star_records(&mut app));
    run(&mut app, 3);
    assert_eq!((sky_seed_used(&app), star_records(&mut app)), before);

    let tunables = GeneratorTunables::default();
    let settings = Settings {
        gated_count: tunables.preset_small,
        seed: 12,
    };
    let sector = generate_sector(settings, &tunables).unwrap();
    app.insert_resource(LoadedSector { sector, tunables });
    run(&mut app, 2);
    assert_eq!(sky_seed_used(&app), derive_seed(12, "sky"));
    assert_ne!(star_records(&mut app), before.1);
    assert_eq!(
        star_records(&mut app),
        records_of(derive_seed(12, "sky"), &t)
    );
    assert_eq!(placement(&mut app).len(), 2, "the old sky is gone");
    let (stars, glow) = uniforms(&mut app);
    assert_eq!(
        (stars.factor, glow.factor),
        (t.sector_sky_factor, t.sector_sky_factor)
    );
    assert_eq!(stars.clip, glow.clip);
    assert_ne!(stars.clip, Vec4::ZERO, "clipped to the map area again");
}

/// SECT-05 / SKY-20: in the sector view the sky lies below the links.
/// Both of its materials draw in the opaque pass, which runs before the
/// see-through pass the links' gizmo lines are drawn in, and write no
/// depth; the links are laid out over it, and the sky is no target.
#[test]
fn the_sky_lies_below_the_links_in_the_sector_view() {
    let mut app = sector_app_with(with_sky);
    assert!(
        !app.world().resource::<LinkPieces>().0.is_empty(),
        "links are drawn"
    );
    let mut materials = app
        .world_mut()
        .query_filtered::<(&MeshMaterial3d<SkyStarsMaterial>, &Pickable), With<SkyStars>>();
    let (handle, pickable) = materials.single(app.world()).unwrap();
    let stars = app
        .world()
        .resource::<Assets<SkyStarsMaterial>>()
        .get(&handle.0)
        .unwrap();
    assert_eq!(stars.alpha_mode(), AlphaMode::Opaque);
    assert!(!pickable.is_hoverable && !pickable.should_block_lower);
    let mut glows = app
        .world_mut()
        .query_filtered::<&MeshMaterial3d<SkyGlowMaterial>, With<SkyGlow>>();
    let handle = glows.single(app.world()).unwrap();
    let glow = app
        .world()
        .resource::<Assets<SkyGlowMaterial>>()
        .get(&handle.0)
        .unwrap();
    assert_eq!(glow.alpha_mode(), AlphaMode::Opaque);
}
