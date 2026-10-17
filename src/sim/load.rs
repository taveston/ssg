//! Loading and dropping the shown system at run time (SECT-31, SECT-32).
//!
//! Everything that is spawned or worked out once per system - the
//! simulation's entities (`spawn.rs`) and whatever the view derives from
//! them (transforms, fold state, zoom stops, markers, edge tabs) - runs in
//! one schedule, `LoadSystem`, in the `Boot` sets. The schedule runs once
//! at the end of start-up, and again each time a system is loaded. Every
//! entity it spawns is tagged `OfLoadedSystem`, so dropping the system
//! despawns exactly what the load made, and nothing else. What the view
//! spawns for a system later, frame by frame, keeps itself in step with
//! the bodies and goes when they go.
//!
//! A resource that holds the system's entities or what was worked out
//! from them is registered with `reset_with_system`; dropping the system
//! puts each back to its default, so nothing points at a body that is
//! gone.
//!
//! The simulation's own entities are made only by `spawn.rs`, so this
//! module knows nothing of the view: the view adds its systems and
//! resources through `SystemLoadApp`.

use std::any::TypeId;
use std::collections::HashSet;

use bevy::ecs::observer::Observer;
use bevy::ecs::resource::IsResource;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::ecs::system::ScheduleSystem;
use bevy::prelude::*;

use super::system::{SystemData, SystemDataResource};
use crate::Boot;

/// The schedule that spawns a waiting system and derives what the view
/// needs from it, in the `Boot` sets, in order.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct LoadSystem;

/// On every entity the load spawned: it belongs to the loaded system and
/// is despawned with it.
#[derive(Component, Debug)]
pub struct OfLoadedSystem;

/// Puts one resource back to its default.
type Reset = fn(&mut World);

/// The resets of the resources that hold the loaded system's state, as
/// plugins registered them, each with its resource's type.
#[derive(Resource, Default)]
struct SystemResets(Vec<(TypeId, Reset)>);

/// How plugins take part in loading a system.
pub trait SystemLoadApp {
    /// Adds `systems` to the load, in `set`. The first call sets up the
    /// schedule and runs it once at the end of start-up.
    fn add_load_systems<M>(
        &mut self,
        set: Boot,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self;

    /// Initialises `R` and puts it back to its default whenever the loaded
    /// system is dropped, because it holds that system's entities or what
    /// was worked out from them.
    fn reset_with_system<R: Resource + Default>(&mut self) -> &mut Self;
}

impl SystemLoadApp for App {
    fn add_load_systems<M>(
        &mut self,
        set: Boot,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self {
        set_up_the_load(self);
        self.add_systems(LoadSystem, systems.in_set(set))
    }

    fn reset_with_system<R: Resource + Default>(&mut self) -> &mut Self {
        self.init_resource::<R>().init_resource::<SystemResets>();
        let mut resets = self.world_mut().resource_mut::<SystemResets>();
        let id = TypeId::of::<R>();
        if !resets.0.iter().any(|(registered, _)| *registered == id) {
            resets.0.push((id, reset_resource::<R>));
        }
        self
    }
}

/// Marks that the load schedule has been set up in this app.
#[derive(Resource)]
struct LoadReady;

/// The load's sets run in start-up's order, and the load runs once after
/// everything else at start-up, so the camera, the pools and the sector's
/// fit are in place before it.
fn set_up_the_load(app: &mut App) {
    if app.world().contains_resource::<LoadReady>() {
        return;
    }
    app.insert_resource(LoadReady)
        .init_resource::<SystemResets>()
        .configure_sets(
            LoadSystem,
            (Boot::World, Boot::Derive, Boot::Overlay).chain(),
        )
        .add_systems(Startup, run_the_load.after(Boot::Overlay));
}

fn reset_resource<R: Resource + Default>(world: &mut World) {
    world.insert_resource(R::default());
}

/// Every entity of the world's own, leaving out Bevy's bookkeeping: the
/// entities that hold resources, and observers.
fn own_entities(world: &mut World) -> Vec<Entity> {
    let mut query = world.query_filtered::<Entity, (Without<IsResource>, Without<Observer>)>();
    query.iter(world).collect()
}

/// Runs the load and tags everything it spawned.
fn run_the_load(world: &mut World) {
    let before: HashSet<Entity> = own_entities(world).into_iter().collect();
    world.run_schedule(LoadSystem);
    for entity in own_entities(world) {
        if !before.contains(&entity) {
            world.entity_mut(entity).insert(OfLoadedSystem);
        }
    }
}

/// SECT-31: loads `data` as the shown system: its entities are spawned
/// and everything the view derives from them is worked out, as at
/// start-up.
pub fn load_system(world: &mut World, data: SystemData) {
    world.insert_resource(SystemDataResource(Some(data)));
    run_the_load(world);
}

/// SECT-32: drops the loaded system: every entity the load spawned is
/// despawned, with its children, and every registered resource goes back
/// to its default.
pub fn drop_system(world: &mut World) {
    let mut loaded = world.query_filtered::<Entity, With<OfLoadedSystem>>();
    let entities: Vec<Entity> = loaded.iter(world).collect();
    for entity in entities {
        // A child goes with its parent, so it may be gone already.
        if let Ok(entity) = world.get_entity_mut(entity) {
            entity.despawn();
        }
    }
    let resets = world
        .get_resource::<SystemResets>()
        .map(|resets| resets.0.clone())
        .unwrap_or_default();
    for (_, reset) in resets {
        reset(world);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Component)]
    struct Body;

    #[derive(Resource, Default)]
    struct Seen(Vec<Entity>);

    fn spawn_two(mut commands: Commands) {
        commands.spawn(Body).with_child(Body);
        commands.spawn(Body);
    }

    fn see(bodies: Query<Entity, With<Body>>, mut seen: ResMut<Seen>) {
        seen.0 = bodies.iter().collect();
    }

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .configure_sets(Startup, (Boot::World, Boot::Derive, Boot::Overlay).chain())
            .add_load_systems(Boot::World, spawn_two)
            .add_load_systems(Boot::Derive, see)
            .reset_with_system::<Seen>();
        app.update();
        app
    }

    fn bodies(app: &mut App) -> usize {
        let mut query = app.world_mut().query_filtered::<(), With<Body>>();
        query.iter(app.world()).count()
    }

    /// SECT-31 / SECT-32: the load runs at start-up, and what it spawned
    /// is tagged; dropping the system despawns exactly that and resets
    /// the registered resources, and loading again spawns it afresh.
    #[test]
    fn dropping_the_system_despawns_what_the_load_spawned_and_nothing_else() {
        let mut app = app();
        let unrelated = app.world_mut().spawn_empty().id();
        assert_eq!(bodies(&mut app), 3);
        assert_eq!(app.world().resource::<Seen>().0.len(), 3);
        drop_system(app.world_mut());
        assert_eq!(bodies(&mut app), 0);
        assert!(app.world().resource::<Seen>().0.is_empty());
        assert!(app.world().get_entity(unrelated).is_ok());
        run_the_load(app.world_mut());
        assert_eq!(bodies(&mut app), 3);
        assert_eq!(app.world().resource::<Seen>().0.len(), 3);
        drop_system(app.world_mut());
        assert_eq!(bodies(&mut app), 0);
        assert!(app.world().get_entity(unrelated).is_ok());
    }
}
