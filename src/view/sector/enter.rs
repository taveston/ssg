//! Entering a gated system from the sector view, and going back (SECT-30
//! to SECT-32).
//!
//! Entering generates the system from the sector (SYSG-01) and loads it in
//! place of any shown (`sim/load.rs`): the bodies are spawned and the
//! view derives everything from them as at start-up, so the system view
//! opens at the system fit, in the standard orientation, in the star's
//! frame with nothing selected. The sector view's camera and its
//! sidebar's slide are put aside, and the sidebar starts closed, as
//! nothing is selected in the system.
//!
//! Going back drops the system: its entities are despawned and the system
//! view's state that pointed at them is reset. The sector view comes back
//! with its camera and sidebar as they were, and its selection, which a
//! visit never touches. The clock runs on throughout. Entering the same
//! system again generates it again, the same (SYSG-02).
//!
//! Both happen at the start of a frame, before the simulation, so every
//! system of that frame sees the new layer and the new bodies.

use bevy::prelude::*;

use super::EnterRequest;
use crate::content::generator::system::generate_system;
use crate::content::start::LoadedSector;
use crate::sim::load::{drop_system, load_system};
use crate::view::camera::state::CameraRig;
use crate::view::map_layer::{MapLayer, WayBack};
use crate::view::sidebar::SidebarState;

/// The sector view as it was left when a system was entered, put back on
/// leaving (SECT-32).
#[derive(Resource, Debug, Clone, Copy)]
struct LeftSector {
    rig: CameraRig,
    sidebar: SidebarState,
}

/// What the requests ask for this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Switch {
    Enter(usize),
    Leave,
}

/// SECT-30 / SECT-32: which switch the requests make, if any: entering
/// only from the sector view, leaving only from a system entered from it.
fn wanted_switch(layer: MapLayer, enter: Option<usize>, back: Option<WayBack>) -> Option<Switch> {
    match (layer, enter, back) {
        (MapLayer::Sector, Some(id), _) => Some(Switch::Enter(id)),
        (MapLayer::System, _, Some(WayBack { asked: true })) => Some(Switch::Leave),
        _ => None,
    }
}

/// SECT-30 to SECT-32: takes this frame's requests and enters or leaves.
/// A request that doesn't apply to the layer shown is dropped.
pub fn switch_layer(world: &mut World) {
    let enter = world
        .get_resource_mut::<EnterRequest>()
        .and_then(|mut request| request.0.take());
    let back = world.get_resource::<WayBack>().copied();
    let layer = *world.resource::<MapLayer>();
    match wanted_switch(layer, enter, back) {
        Some(Switch::Enter(id)) => enter_system(world, id),
        Some(Switch::Leave) => leave_system(world),
        None => {}
    }
    if let Some(mut back) = world.get_resource_mut::<WayBack>() {
        back.asked = false;
    }
}

/// SECT-31: generates the gated system `id` and shows it in place of the
/// sector view. An entry that isn't a gated system's is refused.
fn enter_system(world: &mut World, id: usize) {
    let Some(loaded) = world.get_resource::<LoadedSector>() else {
        return;
    };
    let data = match generate_system(&loaded.sector, id, &loaded.tunables) {
        Ok(data) => data,
        Err(refused) => {
            warn!("can't enter: {refused}");
            return;
        }
    };
    let left = LeftSector {
        rig: *world.resource::<CameraRig>(),
        sidebar: *world.resource::<SidebarState>(),
    };
    drop_system(world);
    world.insert_resource(left);
    world.insert_resource(WayBack::default());
    world.insert_resource(CameraRig::default());
    world.insert_resource(SidebarState::default());
    world.insert_resource(MapLayer::System);
    load_system(world, data);
}

/// SECT-32: drops the shown system and shows the sector view again as it
/// was left.
fn leave_system(world: &mut World) {
    let Some(left) = world.remove_resource::<LeftSector>() else {
        return;
    };
    drop_system(world);
    world.remove_resource::<WayBack>();
    world.insert_resource(left.rig);
    world.insert_resource(left.sidebar);
    world.insert_resource(MapLayer::Sector);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SECT-30 / SECT-32 / SECT-33: an entry is entered only from the
    /// sector view, and the way back is taken only from a system entered
    /// from it, when asked.
    #[test]
    fn requests_switch_only_from_the_layer_they_belong_to() {
        let asked = Some(WayBack { asked: true });
        let idle = Some(WayBack { asked: false });
        assert_eq!(
            wanted_switch(MapLayer::Sector, Some(4), None),
            Some(Switch::Enter(4))
        );
        assert_eq!(wanted_switch(MapLayer::System, Some(4), None), None);
        assert_eq!(
            wanted_switch(MapLayer::System, None, asked),
            Some(Switch::Leave)
        );
        assert_eq!(wanted_switch(MapLayer::System, None, idle), None);
        assert_eq!(wanted_switch(MapLayer::Sector, None, asked), None);
        assert_eq!(wanted_switch(MapLayer::System, None, None), None);
    }
}
