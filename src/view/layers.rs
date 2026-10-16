//! Draw order (MRK-12): which UI layer each kind of overlay node is drawn
//! in, from the bottom. The frame cue's arms come first, with the orbit
//! rings under the whole overlay; then the paths' start dots and their
//! ghosts; then natural bodies, ordered by depth so nearer discs sit on
//! top of farther ones; then the cue's upper vertical half; then symbols;
//! then labels and pills; then edge arrows; then panels. Draw order is
//! only what's on top: picking is decided by `markers/picking.rs`
//! (MRK-08).
//!
//! Every layer is named here, so no two kinds of node share one by
//! accident: nodes in the same layer have no defined order.
//!
//! Shared by every module that spawns overlay nodes (ARCH-03).

use bevy::prelude::*;

/// The frame cue's arms, ticks and the lower half of its vertical axis,
/// with the orbit rings at the bottom; the rings are world lines drawn
/// under all of the overlay (REF-09).
pub const CUE: i32 = BODIES - 4;
/// The frame cue's "N", over its arms and under every body (REF-04).
pub const CUE_LABEL: i32 = BODIES - 3;
/// The dots at the paths' starts, just above the rings and the cue
/// (PLNR-69).
pub const PATH_DOTS: i32 = BODIES - 2;
/// The paths' ghosts, with the natural bodies but beneath every real
/// marker (PLNR-69).
pub const GHOSTS: i32 = BODIES - 1;
/// Natural bodies, two steps per body by depth; the step between is for a
/// satellite ring's near side drawn over its body.
pub const BODIES: i32 = 0;
/// The upper half of the frame cue's vertical axis: above every natural
/// body, so it runs over a true-size disc, and below the symbols (REF-06,
/// REF-09).
pub const CUE_UPPER: i32 = SYMBOLS - 1;
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

    /// MRK-12 / REF-09 / PLNR-69: the layers stack the frame cue's arms
    /// and its "N", the paths' dots and ghosts, bodies, the cue's upper
    /// vertical half, symbols, labels, edge arrows, panels, from the
    /// bottom.
    #[test]
    fn layers_stack_in_order() {
        const { assert!(CUE < CUE_LABEL && CUE_LABEL < PATH_DOTS) };
        const { assert!(PATH_DOTS < GHOSTS && GHOSTS < BODIES) };
        const { assert!(BODIES < CUE_UPPER && CUE_UPPER < SYMBOLS) };
        const { assert!(BODIES < SYMBOLS && SYMBOLS < LABELS) };
        const { assert!(LABELS < EDGE && EDGE < PANELS) };
    }
}
