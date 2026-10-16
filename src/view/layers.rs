//! Draw order (MRK-12): which UI layer each kind of overlay node is drawn
//! in, from the bottom. Natural bodies share the lowest band, ordered by
//! depth so nearer discs sit on top of farther ones; then symbols; then
//! labels and pills; then edge arrows; then panels. Draw order is only
//! what's on top: picking is decided by `marker_picking.rs` (MRK-08).
//!
//! Shared by every module that spawns overlay nodes (ARCH-03).

use bevy::prelude::*;

/// The lowest layer: natural bodies, two steps per body by depth; the step
/// between is for a satellite ring's near side drawn over its body.
pub const BODIES: i32 = 0;
/// Ship, group and station symbols, and the other non-body markers.
pub const SYMBOLS: i32 = 10_000;
/// Labels, pills and the selection and hover brackets, above all markers
/// (HOV-06).
pub const LABELS: i32 = 20_000;
/// Edge arrows and their labels.
pub const EDGE: i32 = 30_000;
/// Panels: the top bar, the inset and anything floating over the map.
pub const PANELS: i32 = 40_000;

/// MRK-12: each body's layer from its depth from the camera: the farthest
/// is drawn first (lowest), the nearest last. `depths` pairs a body with
/// its depth; the result pairs it with its layer, in the same order.
pub fn depth_layers<T: Copy>(depths: &[(T, f32)]) -> Vec<(T, i32)> {
    let mut order: Vec<usize> = (0..depths.len()).collect();
    order.sort_by(|&a, &b| depths[b].1.total_cmp(&depths[a].1));
    let mut layers = vec![0; depths.len()];
    for (rank, index) in order.into_iter().enumerate() {
        layers[index] = BODIES + 2 * rank as i32;
    }
    depths
        .iter()
        .zip(layers)
        .map(|(&(body, _), layer)| (body, layer))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// MRK-12: the nearest body gets the highest layer, the farthest the
    /// lowest, and every body stays below the symbols.
    #[test]
    fn nearer_bodies_draw_on_top() {
        let layers = depth_layers(&[('a', 5.0), ('b', 1.0), ('c', 9.0)]);
        let layer = |name| layers.iter().find(|(n, _)| *n == name).unwrap().1;
        assert!(layer('b') > layer('a') && layer('a') > layer('c'));
        assert!(layers.iter().all(|&(_, l)| (BODIES..SYMBOLS).contains(&l)));
    }

    /// MRK-12: the layers stack bodies, symbols, labels, edge arrows,
    /// panels, from the bottom.
    #[test]
    fn layers_stack_in_order() {
        const { assert!(BODIES < SYMBOLS && SYMBOLS < LABELS) };
        const { assert!(LABELS < EDGE && EDGE < PANELS) };
    }
}
