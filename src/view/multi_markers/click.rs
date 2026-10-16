//! A click on a multi-marker (MUL-07, GRP-07): it opens the marker's tray
//! (`view/tray/`). A fold marker also does what a click on its parent
//! does; any other selects the first thing in its order (MUL-09), and a
//! double-click also focuses it (ZOOM-16). While the transfer planner is
//! open, a fold marker picks as its parent does and any other picks
//! nothing (MUL-12).
//!
//! A station marker is the station's own marker, so a click on it goes
//! through the marker's handler (`hover.rs`), which does the same.

use bevy::prelude::*;

use super::order::TrayOrder;
use crate::view::hover::{BodyClick, ClickCheck};
use crate::view::multi::{MultiKey, MultiMarkers, MultiTarget, TrayFor};

/// MUL-07 / GRP-07: what a click on a multi-marker's hit area does.
pub fn on_multi_click(
    click: On<PointerClick>,
    targets: Query<&MultiTarget>,
    multis: Res<MultiMarkers>,
    order: TrayOrder,
    check: ClickCheck,
    mut body_click: BodyClick,
) {
    if !check.selects(&click) || body_click.completes_double(click.count) {
        return;
    }
    let Some(marker) = targets
        .get(click.entity)
        .ok()
        .and_then(|target| target.0)
        .and_then(|key| multis.get(key))
    else {
        return;
    };
    body_click.open_tray(TrayFor::Marker(marker.key));
    match marker.key {
        MultiKey::Fold(parent) => body_click.click(parent, click.count),
        _ => {
            if let Some(first) = order.first(marker) {
                body_click.select_only(first, click.count);
            }
        }
    }
}
