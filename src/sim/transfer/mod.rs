//! The transfer rules (XFER-11): places and bands (`places.rs`), the
//! system snapshot they are applied to (`tree.rs`), legs with their
//! timing and delta-v (`legs.rs`) and orders with their status
//! (`plan.rs`). States, places, a drive and a budget in; a leg with its
//! trajectory and delta-v out, or the reason it can't be flown. Nothing
//! here reads the view (ARCH-02).

pub mod legs;
pub mod places;
pub mod plan;
pub mod tree;
