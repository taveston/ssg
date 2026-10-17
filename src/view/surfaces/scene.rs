//! What a globe reads of the system each frame (SURF-23, SURF-41,
//! SURF-45, SURF-51): a body's facts, override and seed, its star's
//! light, and how the camera sees it. The inset and the map both draw
//! their globes from here, so a body seen in both is the same surface,
//! turned the same way and lit by the same light, and the two agree on
//! phase.

use std::collections::HashSet;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::bake::{Surface, SurfaceInputs};
use super::seed::surface_seed;
use super::shading::{GlobeMaterial, body_rotation, light_colour, light_in_view, view_from_body};
use super::star::StarLight;
use crate::content::start::LoadedSector;
use crate::sim::body::{BodyClass, DisplayName, Knowledge};
use crate::sim::planet_facts::{LookOverride, PlanetFacts, SurfaceSeed};
use crate::sim::star_facts::StarFacts;
use crate::view::camera::rules::camera_rotation;
use crate::view::camera::state::CameraRig;
use crate::view::tunables::Tunables;
use crate::view::world::WorldPositions;

/// The bodies warned about for having no facts (SURF-05), each once.
#[derive(Resource, Default, Debug)]
pub struct WarnedNoFacts(pub HashSet<Entity>);

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

/// The system's bodies as a globe reads them, and the view they are
/// seen in.
#[derive(SystemParam)]
pub struct GlobeScene<'w, 's> {
    bodies: Query<'w, 's, BodyFacts>,
    stars: Query<'w, 's, (Entity, &'static BodyClass, Option<&'static StarFacts>)>,
    positions: Res<'w, WorldPositions>,
    rig: Res<'w, CameraRig>,
    sector: Option<Res<'w, LoadedSector>>,
}

/// The system's star and its light.
#[derive(Clone, Copy, Debug)]
pub struct SceneStar {
    pub entity: Entity,
    pub light: StarLight,
}

/// A body as its globe is drawn from: what its surface is made of, its
/// knowledge state, and what it orbits.
pub struct SceneBody<'a> {
    pub inputs: SurfaceInputs<'a>,
    pub knowledge: Knowledge,
    parent: Option<Entity>,
}

/// How a body is seen this frame: its turn into the view frame, and the
/// direction and colour of its star's light there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Seen {
    pub view_from_body: Mat3,
    pub light: Vec3,
    pub light_colour: Vec4,
}

impl GlobeScene<'_, '_> {
    /// The star and its light.
    pub fn star(&self) -> Option<SceneStar> {
        self.stars
            .iter()
            .find(|(_, class, _)| **class == BodyClass::Star)
            .map(|(entity, _, facts)| SceneStar {
                entity,
                light: StarLight::from_facts(facts),
            })
    }

    /// A body's position this frame, in render space.
    fn position(&self, body: Entity) -> Vec3 {
        self.positions.0.get(&body).copied().unwrap_or(Vec3::ZERO)
    }

    /// SURF-01 / SURF-02 / SURF-05: what `body`'s surface is drawn from:
    /// its facts, override, seed and knowledge, under `star`. A body
    /// without facts draws as an airless world and is warned about once.
    pub fn body(
        &self,
        body: Entity,
        star: SceneStar,
        warned: &mut WarnedNoFacts,
    ) -> Option<SceneBody<'_>> {
        let (name, knowledge, facts, look_override, authored, parent) =
            self.bodies.get(body).ok()?;
        if facts.is_none() && warned.0.insert(body) {
            warn!(
                "{} has no planet facts: drawn as an airless world with the default seed",
                name.0
            );
        }
        let seed = surface_seed(
            authored.map(|s| s.0),
            self.sector.as_ref().map(|s| s.sector.settings.seed),
            &name.0,
        );
        Some(SceneBody {
            inputs: SurfaceInputs {
                facts,
                look_override: look_override.copied(),
                star: star.light,
                seed,
                knowledge: *knowledge,
            },
            knowledge: *knowledge,
            parent: parent.map(ChildOf::parent),
        })
    }

    /// SURF-23 / SURF-41 / SURF-45: how a body is seen this frame along
    /// the system camera's line of sight: turned as SURF-41 says (facing
    /// its star or its planet if locked to it), and lit from its star.
    pub fn seen(
        &self,
        body: Entity,
        scene_body: &SceneBody,
        surface: &Surface,
        star: SceneStar,
        tunables: &Tunables,
    ) -> Seen {
        let camera = camera_rotation(self.rig.heading, self.rig.tilt);
        let here = self.position(body);
        let star_at = self.position(star.entity);
        let faces = if surface.locked_to_star {
            Some(star_at - here)
        } else {
            scene_body.parent.map(|p| self.position(p) - here)
        };
        Seen {
            view_from_body: view_from_body(camera, body_rotation(surface, faces)),
            light: light_in_view(camera, here, star_at),
            light_colour: light_colour(star.light.class, tunables),
        }
    }
}

/// Puts this frame's globe on a picture node: changes the material the
/// node holds where it differs, or gives the node one.
pub fn fill_picture(
    commands: &mut Commands,
    materials: &mut Assets<GlobeMaterial>,
    picture: Entity,
    held: Option<&MaterialNode<GlobeMaterial>>,
    wanted: GlobeMaterial,
) {
    let held = held.map(|m| m.0.id()).filter(|id| materials.contains(*id));
    if let Some(id) = held {
        let mut existing = materials.get_mut(id).unwrap();
        if existing.globe != wanted.globe
            || existing.strip != wanted.strip
            || existing.close != wanted.close
        {
            *existing = wanted;
        }
    } else {
        let handle = materials.add(wanted);
        commands.entity(picture).insert(MaterialNode(handle));
    }
}
