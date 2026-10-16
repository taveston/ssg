//! Opening, closing and working a tray (MUL-07, MUL-10, MUL-12, CAM-10).
//! A click asks for a tray through `TrayRequest`; with none asked, a
//! click anywhere but on the tray closes it and does what it does (the
//! map's own handlers). Esc closes it in `inset/inspect.rs`, before the
//! planner or the inset (INS-03). Up and Down step the selection through
//! the rows, Right and Left open and close a row's members; the keys come
//! from the bindings (CAM-18), and the camera leaves the arrows alone
//! while a tray is open (CAM-14). A click on a row selects its thing, or
//! while the planner is open picks it as a click on its symbol would.

use bevy::ecs::system::SystemParam;
use bevy::input::mouse::AccumulatedMouseScroll;
use bevy::prelude::*;

use super::TrayState;
use super::contents::TrayContents;
use super::draw::{TrayList, TrayRoot};
use super::rows::{row_to_close, step_row};
use crate::sim::group::MemberOf;
use crate::view::bindings::{Bindings, pressed};
use crate::view::hover::ClickCheck;
use crate::view::inset::inspect::wheel_scroll;
use crate::view::inset::outline::scroll_to_reveal;
use crate::view::multi::{OpenTray, TrayRequest};
use crate::view::node_box::node_size;
use crate::view::planner::state::{PickTarget, PlannerState};
use crate::view::selection::Selection;
use crate::view::targets::PointerTargets;
use crate::view::tunables::Tunables;

/// MUL-07 / MUL-10: a click that asked for a tray opens it in place of
/// any other, scrolled to its top, with the selection as its selected
/// row; asked again for the open tray, it keeps its opened rows and takes
/// the selection the click made. A click that asked for none and isn't on
/// the tray closes the open tray. A drag's release is not a click and
/// leaves it open.
pub fn open_or_close(
    (mut request, mut open, mut state): (ResMut<TrayRequest>, ResMut<OpenTray>, ResMut<TrayState>),
    selection: Res<Selection>,
    (buttons, bindings): (Res<ButtonInput<MouseButton>>, Res<Bindings>),
    (check, targets): (ClickCheck, PointerTargets),
    trays: Query<(), With<TrayRoot>>,
    mut lists: Query<&mut ScrollPosition, With<TrayList>>,
) {
    if let Some(tray) = request.0.take() {
        if open.0 != Some(tray) {
            open.0 = Some(tray);
            state.open_rows.clear();
            for mut position in &mut lists {
                *position = ScrollPosition::default();
            }
        }
        state.cursor = selection.selected;
        return;
    }
    let on_tray = targets.panel().is_some_and(|panel| trays.contains(panel));
    if open.0.is_some()
        && buttons.just_released(bindings.select_button)
        && check.released_as_click()
        && !on_tray
    {
        open.0 = None;
    }
}

/// What selecting a row does: select its thing (a member's group, as a
/// member has no inset of its own), unless the planner is open, and mark
/// the row as selected.
#[derive(SystemParam)]
pub struct RowSelect<'w, 's> {
    state: ResMut<'w, TrayState>,
    selection: ResMut<'w, Selection>,
    planner: ResMut<'w, PlannerState>,
    members: Query<'w, 's, &'static MemberOf>,
}

impl RowSelect<'_, '_> {
    /// MUL-10: the keys move the selection to `entity`'s row. While the
    /// planner is open they only move the row's highlight: a pick takes
    /// a click.
    fn step_to(&mut self, entity: Entity) {
        self.state.cursor = Some(entity);
        if !self.planner.is_open() {
            let thing = self.members.get(entity).map_or(entity, |m| m.0);
            self.selection.select(thing);
        }
    }

    /// MUL-10 / MUL-12: a click on `entity`'s row selects its thing; with
    /// the planner open it picks the thing as a click on its symbol
    /// would, and a row that can't be picked does nothing.
    pub fn clicked(&mut self, entity: Entity, dimmed: bool) {
        if self.planner.is_open() {
            if !dimmed {
                self.planner.pick(PickTarget::Body(entity));
            }
            return;
        }
        self.step_to(entity);
    }

    fn open_row(&mut self, entity: Entity) {
        self.state.open_rows.insert(entity);
    }

    fn close_row(&mut self, entity: Entity) {
        self.state.open_rows.remove(&entity);
    }

    fn cursor(&self) -> Option<Entity> {
        self.state.cursor
    }
}

/// MUL-10: Up and Down step the selection through the rows, stopping at
/// the ends; Right opens the selected row's members and Left closes
/// them, or from a member closes the row it is under and selects that.
pub fn tray_keys(
    keys: Res<ButtonInput<KeyCode>>,
    bindings: Res<Bindings>,
    open: Res<OpenTray>,
    contents: Res<TrayContents>,
    mut select: RowSelect,
) {
    if open.0.is_none() || contents.tray != open.0 {
        return;
    }
    let rows = contents.rows();
    let cursor = select.cursor();
    for (keys_bound, forward) in [
        (&bindings.tray_next, true),
        (&bindings.tray_previous, false),
    ] {
        if pressed(&keys, keys_bound)
            && let Some(row) = step_row(&rows, cursor, forward)
        {
            select.step_to(row);
            return;
        }
    }
    if pressed(&keys, &bindings.tray_open_row)
        && let Some(row) = rows.iter().find(|r| Some(r.entity) == cursor)
        && row.open == Some(false)
    {
        select.open_row(row.entity);
    }
    if pressed(&keys, &bindings.tray_close_row)
        && let Some(row) = row_to_close(&rows, cursor)
    {
        select.close_row(row);
        if cursor != Some(row) {
            select.step_to(row);
        }
    }
}

/// MUL-09: a click on a row's toggle opens or closes its members.
pub fn toggle_row(state: &mut TrayState, entity: Entity) {
    if !state.open_rows.remove(&entity) {
        state.open_rows.insert(entity);
    }
}

/// MUL-08: the wheel over a tray scrolls its rows, within them, in place
/// of zooming the map.
pub fn scroll_tray(
    scroll: Res<AccumulatedMouseScroll>,
    targets: PointerTargets,
    trays: Query<(), With<TrayRoot>>,
    mut lists: Query<(&mut ScrollPosition, &ComputedNode), With<TrayList>>,
) {
    if scroll.delta.y == 0.0 || !targets.panel().is_some_and(|p| trays.contains(p)) {
        return;
    }
    if let Ok((mut position, computed)) = lists.single_mut() {
        wheel_scroll(&scroll, &mut position, computed);
    }
}

/// MUL-08 / MUL-10: when the selected row changes in a tray that scrolls,
/// its rows scroll just enough to show it, as the outline does (INS-05).
pub fn reveal_selected_row(
    state: Res<TrayState>,
    contents: Res<TrayContents>,
    tunables: Res<Tunables>,
    mut lists: Query<(&mut ScrollPosition, &ComputedNode), With<TrayList>>,
) {
    if !state.is_changed() {
        return;
    }
    let Some(index) = contents
        .lines
        .iter()
        .position(|line| Some(line.row.entity) == state.cursor)
    else {
        return;
    };
    let Ok((mut position, computed)) = lists.single_mut() else {
        return;
    };
    let height = node_size(computed).y;
    if height <= 0.0 {
        return;
    }
    let top = index as f32 * tunables.tray_row_px - position.y;
    position.y += scroll_to_reveal(top, top + tunables.tray_row_px, height);
}
