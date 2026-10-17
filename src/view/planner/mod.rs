//! The transfer planner's panel (`docs/spec/planner.md` sections 1 to 5):
//! the inset's Transfer button opens it for a ship (`inset/inspect.rs`); the
//! player picks a destination on the map or in the outline, tunes Depart
//! and Travel time, and schedules the order. The rules are the
//! simulation's (`sim/transfer/`); this module decides when to ask them
//! and shows the answers.
//!
//! - `pick.rs`: what a click picks (PLNR-11).
//! - `compute.rs`: the search, the leg, the tracks and the hovered
//!   place's preview, and when each is worked out again (PLNR-12,
//!   PLNR-13, PLNR-24, PLNR-87).
//! - `rest.rs`: where a thumb may come to rest, off the settings the
//!   solver rejects, and the search moved on with the clock (PLNR-24,
//!   PLNR-33).
//! - `model.rs`: what the panel says, as plain values.
//! - `slider.rs`: the sliders' rules, the tracks' colours and their
//!   rejected stretches (PLNR-23, PLNR-28, PLNR-35).
//! - `panel/`: the panel's nodes, rebuilt by shape and written each
//!   frame; the order shown on a ship's inset (PLNR-51, PLNR-54).
//! - `input.rs`: the panel's buttons, sliders and keys, and scheduling
//!   and changing orders (PLNR-50 to PLNR-53).
//! - `tooltip.rs`: the labels' tooltips and the tracks' (PLNR-35,
//!   PLNR-91, PLNR-94).
//! - `words.rs`: every line of text the planner shows (PLNR-90 to
//!   PLNR-94).
//!
//! The state they share with the rest of the view, the map's paths
//! included, is `state.rs`; the rest of the view reads it and the words
//! without reaching into the panel (ARCH-03).

use bevy::prelude::*;

use crate::sim::load::SystemLoadApp;

use crate::{Boot, Stage};

pub mod compute;
pub mod input;
pub mod model;
pub mod panel;
pub mod pick;
pub mod rest;
pub mod slider;
pub mod state;
pub mod tooltip;
pub mod words;

/// The planner's systems have brought its state up to date for this
/// frame; the map's paths are drawn after (`view/paths/`).
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlannerUpdated;

pub struct PlannerPlugin;

impl Plugin for PlannerPlugin {
    fn build(&self, app: &mut App) {
        app.reset_with_system::<tooltip::TooltipHover>()
            .reset_with_system::<compute::HoveredPlace>()
            .add_systems(Startup, tooltip::spawn_tooltip_box.in_set(Boot::Overlay))
            .add_systems(
                Update,
                (
                    panel::spawn_root,
                    input::end_with_selection,
                    input::watch_changed_order,
                    input::slider_keys,
                    compute::mark_unpickable,
                    compute::resolve_clicks,
                    compute::refresh_search,
                    compute::solve_setting,
                    compute::find_hovered_place,
                    compute::preview_hover,
                    panel::update_panel,
                    panel::update_notice,
                    panel::paint_segments,
                    tooltip::show_tooltips,
                )
                    .chain()
                    .in_set(PlannerUpdated)
                    .in_set(Stage::Overlay),
            );
    }
}
