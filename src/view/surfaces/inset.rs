//! The globe in the inspection inset (SURF-45, SURF-46, INS-04): the
//! selected planet's or moon's picture, `inset_image_px` across, seen
//! along the system camera's line of sight at the current clock, so its
//! phase is the map's. The image frame is empty until the body's bake
//! lands (SURF-31); an unsurveyed body shows the telescope frame with
//! four corner brackets at the image's corners.
//!
//! The sidebar spawns the frame (`spawn_globe_image`) when it builds a
//! natural body's details; each frame `draw_inset_globes` gathers the
//! body's facts, asks the cache for its bake and, once it is ready, fills
//! the picture's material with this frame's view.

use std::collections::HashSet;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::bake::{BakeKey, SurfaceBakes, SurfaceInputs, describe_surface};
use super::seed::surface_seed;
use super::shading::{
    GlobeMaterial, GlobeView, body_rotation, globe_uniform, light_colour, light_in_view,
    view_from_body,
};
use super::star::StarLight;
use crate::content::start::LoadedSector;
use crate::sim::body::{BodyClass, DisplayName, Knowledge};
use crate::sim::planet_facts::{LookOverride, PlanetFacts, SurfaceSeed};
use crate::sim::star_facts::StarFacts;
use crate::view::camera::rules::camera_rotation;
use crate::view::camera::state::CameraRig;
use crate::view::colors;
use crate::view::markers::brackets::{BracketCorner, corner_node};
use crate::view::tunables::Tunables;
use crate::view::world::WorldPositions;

/// The disc's radius as a share of half the image, leaving room for the
/// limb halo and the plumes round it.
pub const INSET_DISC_FRACTION: f32 = 0.88;
/// Past this radius, in disc units, the image is transparent: never,
/// within the inset's square.
const INSET_OPAQUE_RADIUS: f32 = 10.0;

/// The inset's image frame for a natural body, for the body it shows
/// (SURF-45).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlobeImage {
    pub body: Entity,
}

/// The node inside the frame that the globe's material draws in; hidden
/// until the bake lands (SURF-31).
#[derive(Component, Debug)]
pub struct GlobePicture;

/// The bodies warned about for having no facts (SURF-05), each once.
#[derive(Resource, Default, Debug)]
pub struct WarnedNoFacts(pub HashSet<Entity>);

/// SURF-45 / SURF-46: whether a body's inset image is its globe: planets
/// and moons. Ships, stations, gates, salvage sites and the star keep the
/// placeholder.
pub fn shows_globe(class: BodyClass) -> bool {
    matches!(class, BodyClass::Planet | BodyClass::Moon)
}

/// SURF-45 / SURF-46: the image frame, `inset_image_px` square, with the
/// picture's node inside it, hidden until its bake is ready, and for a
/// telescope frame four corner brackets as HOV-06's at
/// `inset_bracket_alpha`.
pub fn spawn_globe_image(
    panel: &mut ChildSpawnerCommands,
    tunables: &Tunables,
    body: Entity,
    telescope: bool,
) {
    let size = tunables.inset_image_px;
    panel
        .spawn((
            GlobeImage { body },
            Node {
                width: px(size),
                height: px(size),
                // The sidebar's column scrolls: the frame keeps its square
                // rather than shrinking to fit.
                flex_shrink: 0.0,
                align_self: AlignSelf::Center,
                ..default()
            },
        ))
        .with_children(|frame| {
            frame.spawn((
                GlobePicture,
                Node {
                    position_type: PositionType::Absolute,
                    width: percent(100),
                    height: percent(100),
                    display: Display::None,
                    ..default()
                },
            ));
            if telescope {
                let arm = (tunables.bracket_arm_fraction * size).min(tunables.bracket_arm_max_px);
                let colour = colors::SELECTION.with_alpha(tunables.inset_bracket_alpha);
                for (top, right) in [(true, true), (true, false), (false, true), (false, false)] {
                    let corner = BracketCorner { top, right };
                    frame.spawn((
                        corner_node(corner, arm, tunables.bracket_stroke_px),
                        BorderColor::all(colour),
                    ));
                }
            }
        });
}

/// What a globe reads of its body: its name, knowledge, facts, override,
/// authored seed and what it orbits.
type BodyFacts = (
    &'static DisplayName,
    &'static Knowledge,
    Option<&'static PlanetFacts>,
    Option<&'static LookOverride>,
    Option<&'static SurfaceSeed>,
    Option<&'static ChildOf>,
);

/// The system's bodies as the inset's globe reads them, and the view
/// they are seen in.
#[derive(SystemParam)]
pub struct GlobeScene<'w, 's> {
    bodies: Query<'w, 's, BodyFacts>,
    stars: Query<'w, 's, (Entity, &'static BodyClass, Option<&'static StarFacts>)>,
    positions: Res<'w, WorldPositions>,
    rig: Res<'w, CameraRig>,
    sector: Option<Res<'w, LoadedSector>>,
}

/// The pictures to fill: each with its frame's body, its node and its
/// material if it has one.
type Pictures<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static ChildOf,
        &'static mut Node,
        Option<&'static MaterialNode<GlobeMaterial>>,
    ),
    With<GlobePicture>,
>;

impl GlobeScene<'_, '_> {
    /// The star and its light.
    fn star(&self) -> Option<(Entity, StarLight)> {
        self.stars
            .iter()
            .find(|(_, class, _)| **class == BodyClass::Star)
            .map(|(entity, _, facts)| (entity, StarLight::from_facts(facts)))
    }

    /// A body's position this frame, in render space.
    fn position(&self, body: Entity) -> Vec3 {
        self.positions.0.get(&body).copied().unwrap_or(Vec3::ZERO)
    }
}

/// SURF-45: fills each inset globe with this frame's view of its body:
/// the bake once it is ready, the body turned as SURF-41 says, lit from
/// its star along the camera's line of sight. Until the bake is ready
/// the picture stays hidden, leaving the frame empty (SURF-31). A body
/// without facts draws as an airless world and is warned about once
/// (SURF-05).
pub fn draw_inset_globes(
    mut commands: Commands,
    scene: GlobeScene,
    (mut bakes, mut images, mut materials): (
        ResMut<SurfaceBakes>,
        ResMut<Assets<Image>>,
        ResMut<Assets<GlobeMaterial>>,
    ),
    tunables: Res<Tunables>,
    frames: Query<&GlobeImage>,
    mut pictures: Pictures,
    mut warned: ResMut<WarnedNoFacts>,
) {
    let Some((star, light)) = scene.star() else {
        return;
    };
    for (picture, child_of, mut node, material) in &mut pictures {
        let Ok(frame) = frames.get(child_of.parent()) else {
            continue;
        };
        let body = frame.body;
        let Ok((name, knowledge, facts, look_override, authored, parent)) = scene.bodies.get(body)
        else {
            continue;
        };
        if facts.is_none() && warned.0.insert(body) {
            warn!(
                "{} has no planet facts: drawn as an airless world with the default seed",
                name.0
            );
        }
        let seed = surface_seed(
            authored.map(|s| s.0),
            scene.sector.as_ref().map(|s| s.sector.settings.seed),
            &name.0,
        );
        let inputs = SurfaceInputs {
            facts,
            look_override: look_override.copied(),
            star: light,
            seed,
            knowledge: *knowledge,
        };
        let surface = describe_surface(&inputs, None, false, &tunables);
        let key = BakeKey {
            body,
            knowledge: *knowledge,
        };
        let entry = bakes.request(key, &surface.params, &mut images);
        if !entry.ready {
            if node.display != Display::None {
                node.display = Display::None;
            }
            continue;
        }
        let strip = entry.strip.clone();
        let camera = camera_rotation(scene.rig.heading, scene.rig.tilt);
        let here = scene.position(body);
        let star_at = scene.position(star);
        let faces = if surface.locked_to_star {
            Some(star_at - here)
        } else {
            parent.map(|p| scene.position(p.parent()) - here)
        };
        let panel = colors::PANEL.to_srgba();
        let view = GlobeView {
            view_from_body: view_from_body(camera, body_rotation(&surface, faces)),
            light: light_in_view(camera, here, star_at),
            light_colour: light_colour(light.class, &tunables),
            ground: [panel.red, panel.green, panel.blue],
            disc_fraction: INSET_DISC_FRACTION,
            node_px: tunables.inset_image_px,
            opaque_radius: INSET_OPAQUE_RADIUS,
            grain_seed: seed as u32,
        };
        let globe = globe_uniform(&surface, &view, &tunables);
        let held = material
            .map(|m| m.0.id())
            .filter(|id| materials.contains(*id));
        if let Some(id) = held {
            let mut existing = materials.get_mut(id).unwrap();
            if existing.globe != globe || existing.strip != strip {
                existing.globe = globe;
                existing.strip = strip;
            }
        } else {
            let handle = materials.add(GlobeMaterial { globe, strip });
            commands.entity(picture).insert(MaterialNode(handle));
        }
        if node.display != Display::Flex {
            node.display = Display::Flex;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SURF-45 / INS-04: planets and moons show their globe; everything
    /// else keeps the placeholder.
    #[test]
    fn only_planets_and_moons_show_a_globe() {
        assert!(shows_globe(BodyClass::Planet));
        assert!(shows_globe(BodyClass::Moon));
        for class in [
            BodyClass::Star,
            BodyClass::Ship,
            BodyClass::Station,
            BodyClass::Gate,
            BodyClass::SalvageSite,
        ] {
            assert!(!shows_globe(class), "{class:?}");
        }
    }
}
