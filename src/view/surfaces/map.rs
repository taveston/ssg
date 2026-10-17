//! The globe in the system view (SURF-50 to SURF-52, SURF-34, MRK-11):
//! close in, a planet's or moon's true-size disc is drawn as its picture.
//!
//! Each natural body's marker carries a picture node over its disc. Each
//! frame, once the markers are placed, the body's projected radius
//! decides whether its globe is due, with hysteresis: it becomes due
//! above `globe_min_px` and stays due until the radius falls below
//! `globe_release_px`. A due globe asks the shared cache for the body's
//! bake, the same bake as the inset's, and is drawn once that is ready,
//! covering the disc's fill and rim and painting the gap round it; until
//! then, and whenever the globe is not due, the picture is hidden and
//! MRK-11's disc shows. The marker's label and the selection and hover
//! brackets are drawn above all markers, so they stay on top.
//!
//! Further in still (SURF-34), the face of the cube turned toward the
//! camera is baked again finer, and drawn from that bake while it is the
//! face in view.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::bake::{BakeKey, SurfaceBakes, describe_surface};
use super::chooser::{is_telescope, needs_rebake, rebake_face_px, rebake_pieces};
use super::cube::face_point;
use super::inset::shows_globe;
use super::scene::{GlobeScene, SceneStar, WarnedNoFacts, fill_picture};
use super::shading::{CloseFace, GlobeMaterial, GlobeView, globe_uniform};
use crate::Stage;
use crate::sim::body::BodyClass;
use crate::view::colors;
use crate::view::fold::SatelliteFoldState;
use crate::view::markers::{MarkerDrawn, MarkerShape, MarkersPlaced, marker_fade};
use crate::view::motion_mode::MotionModeState;
use crate::view::multi::JoinSlide;
use crate::view::targets::MarkerFor;
use crate::view::true_size::DiscRadii;
use crate::view::tunables::Tunables;

/// A marker drawn at less than this strength is fading (folding, joining
/// a multi-marker or fading into its exposure ring); it draws MRK-11's
/// disc, which fades with it, rather than its globe, which cannot fade.
const FULL_STRENGTH: f32 = 0.999;

/// On a natural body's marker: whether its globe is due (SURF-50), and
/// the picture node that draws it.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapGlobe {
    pub due: bool,
    pub picture: Entity,
}

/// The node over a body's disc that its globe's material draws in;
/// hidden while the disc shows.
#[derive(Component, Debug)]
pub struct MapGlobePicture;

/// SURF-50: whether a body's globe is due this frame, from its projected
/// radius in pixels and whether it was due last frame: it becomes due
/// once the radius exceeds `globe_min_px`, and stays due until the radius
/// falls below `globe_release_px`.
pub fn globe_due(was_due: bool, projected_radius_px: f32, tunables: &Tunables) -> bool {
    if was_due {
        projected_radius_px >= tunables.globe_release_px
    } else {
        projected_radius_px > tunables.globe_min_px
    }
}

/// Where a globe's node sits on its marker, and how the disc sits in it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlobePlacement {
    /// The node's top left, from the marker's top left, in pixels.
    pub offset_px: f32,
    /// The node's side, in pixels.
    pub size_px: f32,
    /// The disc's radius as a share of half the node.
    pub disc_fraction: f32,
    /// Past this radius, in disc units, the node is transparent.
    pub opaque_radius: f32,
}

/// MRK-11 / SURF-50: the globe's node covers a disc of `disc_px` radius
/// and its gap of `gap_px` round it: the disc fills the node less the
/// gap, the gap is painted (the ground, and an atmosphere's limb glow
/// where it has one), and the node is transparent past the gap.
pub fn globe_placement(disc_px: f32, gap_px: f32) -> GlobePlacement {
    let outer = disc_px + gap_px;
    GlobePlacement {
        offset_px: -gap_px,
        size_px: 2.0 * outer,
        disc_fraction: disc_px / outer,
        opaque_radius: outer / disc_px.max(f32::EPSILON),
    }
}

/// SURF-34: the face of the cube turned toward the camera: the one under
/// the middle of the disc as seen.
pub fn face_in_view(view_from_body: Mat3) -> u32 {
    face_point(view_from_body.transpose() * Vec3::Z).face
}

pub struct MapGlobePlugin;

impl Plugin for MapGlobePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (attach_map_globes, draw_map_globes)
                .chain()
                .after(MarkersPlaced)
                .in_set(Stage::Overlay),
        );
    }
}

/// Markers without a picture node yet.
type NewMarkers<'w, 's> =
    Query<'w, 's, (Entity, &'static MarkerFor, &'static MarkerShape), Without<MapGlobe>>;

/// SURF-50: gives each planet's and moon's marker its picture node, a
/// child drawn after the marker's fill and rim and so over them, hidden
/// until its globe is drawn.
fn attach_map_globes(mut commands: Commands, markers: NewMarkers, classes: Query<&BodyClass>) {
    for (marker, marker_for, shape) in &markers {
        let natural = classes
            .get(marker_for.0)
            .is_ok_and(|class| shows_globe(*class));
        if *shape != MarkerShape::Disc || !natural {
            continue;
        }
        let picture = commands
            .spawn((
                MapGlobePicture,
                Node {
                    position_type: PositionType::Absolute,
                    display: Display::None,
                    ..default()
                },
                Pickable::IGNORE,
                ChildOf(marker),
            ))
            .id();
        commands.entity(marker).insert(MapGlobe {
            due: false,
            picture,
        });
    }
}

/// What the map's globes draw from: the cache, the images and materials,
/// and the tunables.
#[derive(SystemParam)]
struct GlobeAssets<'w> {
    bakes: ResMut<'w, SurfaceBakes>,
    images: ResMut<'w, Assets<Image>>,
    materials: ResMut<'w, Assets<GlobeMaterial>>,
    warned: ResMut<'w, WarnedNoFacts>,
    tunables: Res<'w, Tunables>,
}

/// The markers with a globe, where they are drawn this frame.
type GlobeMarkers<'w, 's> = Query<
    'w,
    's,
    (
        &'static MarkerFor,
        &'static MarkerDrawn,
        &'static mut MapGlobe,
    ),
>;

/// What fades a body's marker (GRP-03, MUL-05, MOT-11).
type MarkerFades<'w, 's> = Query<
    'w,
    's,
    (
        Option<&'static SatelliteFoldState>,
        Option<&'static MotionModeState>,
        Option<&'static JoinSlide>,
    ),
>;

/// The picture nodes, with their materials.
type MapPictures<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Node,
        Option<&'static MaterialNode<GlobeMaterial>>,
    ),
    With<MapGlobePicture>,
>;

/// SURF-50 to SURF-52 / SURF-34: decides each frame which bodies' globes
/// are due from their projected radii, asks for their bakes, and draws
/// each whose bake is ready over its disc at the disc's size, seen and
/// lit as the inset sees it. A globe not due, not baked yet, or on a
/// fading marker is hidden, leaving MRK-11's disc.
fn draw_map_globes(
    mut commands: Commands,
    scene: GlobeScene,
    mut assets: GlobeAssets,
    discs: Res<DiscRadii>,
    mut markers: GlobeMarkers,
    fades: MarkerFades,
    mut pictures: MapPictures,
) {
    let star = scene.star();
    for (marker_for, drawn, mut globe) in &mut markers {
        let body = marker_for.0;
        let radius = discs.get(body);
        let due = drawn.0.is_some() && globe_due(globe.due, radius, &assets.tunables);
        if globe.due != due {
            globe.due = due;
        }
        let Ok((mut node, held)) = pictures.get_mut(globe.picture) else {
            continue;
        };
        let wanted = match (due, drawn.0, star) {
            (true, Some((_, half)), Some(star)) => {
                map_globe(body, (radius, half), &scene, star, &mut assets)
            }
            _ => None,
        };
        let strength = fades.get(body).map_or(1.0, |(fold, motion, slide)| {
            marker_fade(fold, motion, slide).0
        });
        let Some((wanted, place)) = wanted.filter(|_| strength >= FULL_STRENGTH) else {
            if node.display != Display::None {
                node.display = Display::None;
            }
            // A hidden picture lets go of its material, so the strips it
            // drew from (a close face's especially) can be freed once the
            // cache drops them.
            if held.is_some() {
                commands
                    .entity(globe.picture)
                    .remove::<MaterialNode<GlobeMaterial>>();
            }
            continue;
        };
        fill_picture(
            &mut commands,
            &mut assets.materials,
            globe.picture,
            held,
            wanted,
        );
        let (offset, size) = (px(place.offset_px), px(place.size_px));
        if node.display != Display::Flex
            || node.left != offset
            || node.top != offset
            || node.width != size
            || node.height != size
        {
            node.display = Display::Flex;
            node.left = offset;
            node.top = offset;
            node.width = size;
            node.height = size;
        }
    }
}

/// SURF-50 / SURF-51 / SURF-52 / SURF-34: the material for `body`'s
/// globe on the map this frame, at its projected radius and drawn
/// half-size in pixels, and where its node sits; `None` while its bake is
/// not ready. Its view and light are the inset's, so the two agree. Close
/// in, the face in view is drawn from its finer bake once that is ready.
fn map_globe(
    body: Entity,
    (radius_px, half_px): (f32, f32),
    scene: &GlobeScene,
    star: SceneStar,
    assets: &mut GlobeAssets,
) -> Option<(GlobeMaterial, GlobePlacement)> {
    let tunables = &*assets.tunables;
    let scene_body = scene.body(body, star, &mut assets.warned)?;
    let surface = describe_surface(&scene_body.inputs, None, false, tunables);
    let key = BakeKey {
        body,
        knowledge: scene_body.knowledge,
    };
    let entry = assets
        .bakes
        .request(key, &surface.params, &mut assets.images);
    let strip = entry.ready.then(|| entry.strip.clone())?;
    let seen = scene.seen(body, &scene_body, &surface, star, tunables);
    let mut close = None;
    if !is_telescope(surface.knowledge, tunables) && needs_rebake(radius_px, tunables) {
        let face = face_in_view(seen.view_from_body);
        let fine = describe_surface(
            &scene_body.inputs,
            Some(rebake_face_px(tunables)),
            true,
            tunables,
        );
        let entry = assets.bakes.request_close(
            key,
            face,
            &fine.params,
            rebake_pieces(tunables),
            &mut assets.images,
        );
        if entry.ready {
            let face_px = fine.face_px();
            close = Some((entry.strip.clone(), CloseFace { face, face_px }));
        }
    }
    let place = globe_placement(half_px, tunables.body_gap_px);
    let ground = colors::GROUND.to_srgba();
    let view = GlobeView {
        view_from_body: seen.view_from_body,
        light: seen.light,
        light_colour: seen.light_colour,
        ground: [ground.red, ground.green, ground.blue],
        disc_fraction: place.disc_fraction,
        node_px: place.size_px,
        opaque_radius: place.opaque_radius,
        grain_seed: scene_body.inputs.seed as u32,
        close: close.as_ref().map(|(_, face)| *face),
    };
    let material = GlobeMaterial {
        globe: globe_uniform(&surface, &view, tunables),
        strip,
        close: close.map(|(strip, _)| strip),
    };
    Some((material, place))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SURF-50 / SURF-73: the globe comes in once the projected radius
    /// passes `globe_min_px`, holds down to `globe_release_px`, and goes
    /// only below that, so a radius wavering between the two never
    /// flickers.
    #[test]
    fn the_globe_comes_in_above_the_minimum_and_goes_below_the_release() {
        let t = Tunables::default();
        assert!(t.globe_release_px < t.globe_min_px);
        assert!(!globe_due(false, t.globe_min_px, &t));
        assert!(globe_due(false, t.globe_min_px + 0.1, &t));
        let between = (t.globe_min_px + t.globe_release_px) / 2.0;
        assert!(!globe_due(false, between, &t), "not yet due stays a disc");
        assert!(globe_due(true, between, &t), "due stays a globe");
        assert!(globe_due(true, t.globe_release_px, &t));
        assert!(!globe_due(true, t.globe_release_px - 0.1, &t));
        // A radius swinging across the band's middle changes nothing.
        let mut due = globe_due(false, t.globe_min_px + 1.0, &t);
        for frame in 0..20 {
            let wobble = if frame % 2 == 0 { 1.0 } else { -1.0 };
            due = globe_due(due, between + wobble, &t);
            assert!(due);
        }
    }

    /// MRK-11 / SURF-50: the node covers the disc and its gap: the disc
    /// fills it less the gap, and it is opaque out to the gap's edge.
    #[test]
    fn the_globe_covers_the_disc_and_its_gap() {
        let place = globe_placement(40.0, 2.0);
        assert_eq!(place.offset_px, -2.0);
        assert_eq!(place.size_px, 84.0);
        let disc_px = place.disc_fraction * place.size_px / 2.0;
        assert!((disc_px - 40.0).abs() < 1e-4);
        assert!((place.opaque_radius * disc_px - 42.0).abs() < 1e-4);
    }

    /// SURF-34: the face in view is the one facing the camera: straight
    /// down on an unturned body, its north pole (+Y, face 2); turned to
    /// face the camera with its prime meridian, +Z (face 4).
    #[test]
    fn the_face_in_view_is_the_one_facing_the_camera() {
        use super::super::shading::view_from_body;
        use crate::view::camera::rules::camera_rotation;
        let down = camera_rotation(0.0, 0.0);
        assert_eq!(face_in_view(view_from_body(down, Quat::IDENTITY)), 2);
        let side = camera_rotation(0.0, std::f32::consts::FRAC_PI_2);
        let facing = (0..6)
            .map(|k| Quat::from_rotation_y(k as f32 * std::f32::consts::FRAC_PI_2))
            .map(|turn| face_in_view(view_from_body(side, turn)))
            .collect::<Vec<_>>();
        assert!(facing.iter().all(|&face| face != 2 && face != 3));
        assert!(facing.contains(&4), "{facing:?}");
    }
}
