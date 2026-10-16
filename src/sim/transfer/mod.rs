//! The transfer rules (XFER-11): places and bands (`places.rs`), the
//! system snapshot they are applied to (`tree.rs`), phase round an orbit
//! (`phase.rs`), what goes in and out of the solver (`order.rs`), the two
//! ends of a leg (`ends.rs`), crossings with their timing and delta-v
//! (`legs.rs`), destinations in the ship's own orbit (`phasing.rs`) and
//! orders with their status (`plan.rs`). States, places, a drive and a
//! budget in; a leg with its trajectory and delta-v out, or the reason it
//! can't be flown. Nothing here reads the view (ARCH-02).

mod ends;
pub mod legs;
pub mod order;
pub mod phase;
pub mod phasing;
pub mod places;
pub mod plan;
#[cfg(test)]
mod test_layout;
pub mod tree;
