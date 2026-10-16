//! What the view reads of a group of ships (SYM-09, SYM-13, GRP-09): its
//! member count, whether it is a task force, and its members' classes for
//! the label. A group is one body on the map; its members are never drawn
//! (`sim/group.rs`).
//!
//! Shared by every module that draws a symbol or lists a body (ARCH-03).

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::symbols::GroupFace;
use crate::sim::group::{MemberOf, ShipGroup};
use crate::sim::track::{ShipClass, ShipRecord};

/// Reads groups and their members.
#[derive(SystemParam)]
pub struct GroupFaces<'w, 's> {
    groups: Query<'w, 's, &'static ShipGroup>,
    members: Query<'w, 's, &'static ShipRecord, With<MemberOf>>,
}

impl GroupFaces<'_, '_> {
    /// SYM-09: a group's count and task-force mark; `None` for anything
    /// that isn't a group.
    pub fn face(&self, body: Entity) -> Option<GroupFace> {
        self.groups.get(body).ok().map(|group| GroupFace {
            count: group.members.len(),
            task_force: group.task_force,
        })
    }

    /// SYM-13: a group's members' classes, for its member count by class;
    /// empty for anything that isn't a group.
    pub fn member_classes(&self, body: Entity) -> Vec<ShipClass> {
        self.groups.get(body).map_or_else(
            |_| Vec::new(),
            |group| {
                group
                    .members
                    .iter()
                    .filter_map(|&member| self.members.get(member).ok())
                    .map(|record| record.class)
                    .collect()
            },
        )
    }
}
