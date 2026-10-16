//! Several objects on one marker (MUL-01 to MUL-06, MUL-11, MUL-13): the
//! **multi-marker**, a ship symbol's frame with a count where the icon
//! would be and a second frame behind it, standing for everything at one
//! place on screen: satellites folded into a parent (the fold marker,
//! GRP-05), ships docked at a station (the station marker), ships
//! together in one phase, and symbols that merely overlap.
//!
//! `gather.rs` decides which markers there are and what each holds,
//! `look.rs` how each looks, `draw.rs` draws them, `order.rs` lists a
//! marker's things in order and `click.rs` handles a click on one. What
//! they produce is shared state in `view/multi.rs`.

use bevy::prelude::*;

use crate::view::markers::MarkersPlaced;
use crate::view::multi::MultiMarkers;
use crate::{Boot, Stage};

pub mod click;
pub mod draw;
pub mod gather;
pub mod look;
pub mod order;

/// This frame's multi-markers are gathered and the station markers set,
/// before the markers are placed and faded.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct MultiGathered;

pub struct MultiMarkersPlugin;

impl Plugin for MultiMarkersPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MultiMarkers>()
            .init_resource::<gather::ClusterLatch>()
            .configure_sets(
                Update,
                MultiGathered.in_set(Stage::Overlay).before(MarkersPlaced),
            )
            .add_systems(Startup, draw::spawn_pool.in_set(Boot::Overlay))
            .add_systems(
                Update,
                (
                    (gather::gather_multi_markers, draw::show_station_markers)
                        .chain()
                        .in_set(MultiGathered),
                    draw::place_multi_markers
                        .in_set(MarkersPlaced)
                        .in_set(Stage::Overlay),
                ),
            );
    }
}
