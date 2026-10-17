//! Stage 5, links: joins the gated systems into the old network, all of
//! it working, as a dense mesh (SGEN-60 to SGEN-65). Three steps: a
//! spanning tree, then extra links between near pairs that cross nothing,
//! then more links for any system still short of them (`mesh`). Then the
//! systems that lead to valuable ones are labelled strategic
//! (`strategic`). The failure stage fails most links afterwards.
//!
//! Nothing here is random: the network follows from the gated systems'
//! positions and scores.

pub mod crossing;
mod mesh;
pub mod strategic;

use bevy::math::DVec3;

use super::catalogue::Entry;
use super::gates::GateRecord;
use super::network::Link;
use super::tunables::GeneratorTunables;

/// Stage 5's output.
#[derive(Debug, Clone, PartialEq)]
pub struct Network {
    /// Every link, all working, in the order they were made.
    pub links: Vec<Link>,
    /// The final link range, in km (SGEN-60).
    pub link_range_km: f64,
    /// For each gated system, in the gate choice's order, whether it is
    /// strategic (SGEN-64).
    pub strategic: Vec<bool>,
}

/// Runs stage 5 (SGEN-60 to SGEN-65): the links between the gated
/// systems, starting from the gate stage's link range.
pub fn run(
    gated: &[GateRecord],
    entries: &[Entry],
    start_range_km: f64,
    t: &GeneratorTunables,
) -> Network {
    let ids: Vec<usize> = gated.iter().map(|g| g.id).collect();
    let positions: Vec<DVec3> = ids.iter().map(|&id| entries[id].position_km).collect();
    let (links, link_range_km) = mesh::build(&ids, &positions, start_range_km, t);
    let overall: Vec<f64> = gated.iter().map(|g| g.scores.overall()).collect();
    let strategic = strategic::label(&ids, &overall, &links, t);
    Network {
        links,
        link_range_km,
        strategic,
    }
}
