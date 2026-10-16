//! A parent's **object marker** (GRP-12, GRP-13): what stands for the
//! ships, groups and stations folded into it. At rest it is a small pip,
//! a copy of the fold marker's front frame down and to the right of the
//! parent, with a back pip when it holds two or more things. It pops out
//! into the full fold marker, which then touches the parent's box, while
//! the pointer is near the parent, the fold marker or its tray is in use,
//! or the parent or one of its things is selected; then it shrinks back.
//!
//! The rules are plain functions (where the pip and the popped marker sit,
//! whether a marker wants to be out); `update_pops` keeps each parent's
//! pop state from frame to frame, and `gather.rs` turns it into the fold
//! marker's place and size.

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;

use crate::view::anim::{advance_towards, step_per_frame};
use crate::view::fold::FoldSummary;
use crate::view::multi::{MultiKey, MultiTarget, OpenTray, TrayFor, back_offset, popped_centre};
use crate::view::screen::ScreenPositions;
use crate::view::selection::Selection;
use crate::view::targets::Hovered;
use crate::view::tunables::Tunables;

/// One parent's object marker, kept from frame to frame.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Pop {
    /// Whether it wants to be out (GRP-13): the latch the release
    /// distance works on.
    pub out: bool,
    /// How far it has popped out: 0 a pip, 1 the full fold marker.
    pub t: f32,
    /// How far it has faded in since it appeared (GRP-12, GRP-03).
    pub shown: f32,
}

/// Every parent's object marker this frame, by parent. A parent is here
/// while it has a ship, group or station folded into it.
#[derive(Resource, Default, Debug)]
pub struct ObjectPops(pub HashMap<Entity, Pop>);

impl ObjectPops {
    /// How far `parent`'s object marker has popped out and faded in:
    /// a newly seen one is a pip, not yet shown.
    pub fn of(&self, parent: Entity) -> Pop {
        self.0.get(&parent).copied().unwrap_or_default()
    }
}

/// Where an object marker is drawn and how big: its front frame's centre
/// and side, its back frame's offset from the front, and whether its
/// count shows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObjectGeometry {
    pub centre: Vec2,
    pub frame_px: f32,
    pub back: Option<Vec2>,
    pub count_shown: bool,
}

/// GRP-12: the pip's centre: `h + object_pip_gap_px` right of and below
/// its parent's centre, `h` being half its parent's box (HOV-07, without
/// the moon ring).
pub fn pip_centre(parent: Vec2, h: f32, tunables: &Tunables) -> Vec2 {
    parent + Vec2::splat(h + tunables.object_pip_gap_px)
}

/// GRP-12 / GRP-13: an object marker holding `things`, popped out by `t`
/// (0 a pip, 1 the fold marker), beside a parent at `parent` whose box
/// reaches `h` from its centre. It grows from `object_pip_px` to
/// `ship_symbol_px` and slides from the pip's place to the popped one;
/// the back pip, there only for two or more things, moves out to the back
/// frame's offset, and for one thing the back frame comes out from behind
/// the front as it grows. The count shows once it has fully grown.
pub fn object_geometry(
    parent: Vec2,
    h: f32,
    (t, things): (f32, usize),
    tunables: &Tunables,
) -> ObjectGeometry {
    let t = t.clamp(0.0, 1.0);
    let centre = pip_centre(parent, h, tunables).lerp(popped_centre(parent, h, tunables), t);
    let frame_px = tunables.object_pip_px + (tunables.ship_symbol_px - tunables.object_pip_px) * t;
    let from = if things >= 2 {
        tunables.pip_back_offset_px
    } else {
        0.0
    };
    let offset = from + (tunables.back_frame_offset_px - from) * t;
    let back = (things >= 2 || t > 0.0).then(|| back_offset(offset));
    ObjectGeometry {
        centre,
        frame_px,
        back,
        count_shown: t >= 1.0,
    }
}

/// Where the pointer is on the map this frame, if it is, and whether a
/// mouse button is held down.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pointer {
    pub at: Vec2,
    pub button_held: bool,
}

/// GRP-13: the parent whose marker centre is nearest the pointer, of
/// `parents`: no other parent's object marker pops while the pointer is
/// nearer to another parent.
pub fn nearest_parent(at: Vec2, parents: &[(Entity, Vec2)]) -> Option<Entity> {
    parents
        .iter()
        .min_by(|a, b| a.1.distance(at).total_cmp(&b.1.distance(at)))
        .map(|&(parent, _)| parent)
}

/// GRP-13: whether the pointer keeps an object marker out: within
/// `pop_reach_px` of its parent's centre (`centre`), nearer to it than to
/// any other parent and with no button held; or, once out, still within
/// `pop_release_px`, so it doesn't flicker at the edge of its reach.
pub fn pointer_pops(
    was_out: bool,
    pointer: Option<Pointer>,
    (centre, nearest): (Vec2, bool),
    tunables: &Tunables,
) -> bool {
    let Some(pointer) = pointer else {
        return false;
    };
    let distance = pointer.at.distance(centre);
    let reached = distance <= tunables.pop_reach_px && nearest && !pointer.button_held;
    reached || (was_out && distance <= tunables.pop_release_px)
}

/// The other reasons an object marker pops out, besides the pointer's
/// distance (GRP-13).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Held {
    /// The pointer is over the fold marker (or its pip).
    pub hovered: bool,
    /// Its tray is open; the pointer over the tray needs it open too.
    pub tray_open: bool,
    /// The parent, or a thing it holds, is selected.
    pub selected: bool,
}

impl Held {
    fn any(self) -> bool {
        self.hovered || self.tray_open || self.selected
    }
}

/// GRP-13: whether an object marker is out this frame.
pub fn wants_out(pointer: bool, held: Held) -> bool {
    pointer || held.any()
}

/// The window's pointer, while it is over the map, and whether a mouse
/// button is held.
fn map_pointer(
    windows: &Query<&Window>,
    buttons: &ButtonInput<MouseButton>,
    screen: &ScreenPositions,
) -> Option<Pointer> {
    let at = windows.single().ok()?.cursor_position()?;
    screen.in_viewport(at).then(|| Pointer {
        at,
        button_held: buttons.get_pressed().next().is_some(),
    })
}

/// What decides which object markers pop out this frame.
#[derive(bevy::ecs::system::SystemParam)]
pub struct PopInputs<'w, 's> {
    time: Res<'w, Time>,
    tunables: Res<'w, Tunables>,
    folds: Res<'w, FoldSummary>,
    screen: Res<'w, ScreenPositions>,
    windows: Query<'w, 's, &'static Window>,
    buttons: Res<'w, ButtonInput<MouseButton>>,
    selection: Res<'w, Selection>,
    open: Res<'w, OpenTray>,
    hovered: Query<'w, 's, &'static MultiTarget, With<Hovered>>,
}

impl PopInputs<'_, '_> {
    /// GRP-13: the parent's own hold on its object marker: hovered, its
    /// tray open, or it or a thing folded into it selected.
    fn held(&self, parent: Entity, hovered: &HashSet<Entity>) -> Held {
        let key = MultiKey::Fold(parent);
        let selected = self.selection.selected.is_some_and(|s| {
            s == parent
                || self
                    .folds
                    .0
                    .get(&parent)
                    .is_some_and(|g| g.objects.contains(&s))
        });
        Held {
            hovered: hovered.contains(&parent),
            tray_open: self.open.0 == Some(TrayFor::Marker(key)),
            selected,
        }
    }
}

/// GRP-12 / GRP-13: each parent with an object marker pops it out or lets
/// it back, over `pop_anim_ms`, and fades a new one in over
/// `fold_unfold_anim_ms`. A parent that no longer has one is forgotten,
/// so its marker hides at once (INS-06).
pub fn update_pops(inputs: PopInputs, mut pops: ResMut<ObjectPops>) {
    let tunables = &inputs.tunables;
    let delta = inputs.time.delta_secs();
    let pop_step = step_per_frame(delta, tunables.pop_anim_ms);
    let show_step = step_per_frame(delta, tunables.fold_unfold_anim_ms);
    let pointer = map_pointer(&inputs.windows, &inputs.buttons, &inputs.screen);
    let parents: Vec<(Entity, Vec2)> = inputs
        .folds
        .0
        .keys()
        .filter_map(|&parent| Some((parent, inputs.screen.get(parent)?)))
        .collect();
    let nearest = pointer.and_then(|p| nearest_parent(p.at, &parents));
    let hovered: HashSet<Entity> = inputs
        .hovered
        .iter()
        .filter_map(|target| match target.0 {
            Some(MultiKey::Fold(parent)) => Some(parent),
            _ => None,
        })
        .collect();
    let mut next = HashMap::new();
    for &(parent, centre) in &parents {
        if inputs.folds.0[&parent].objects.is_empty() {
            continue;
        }
        let pop = pops.of(parent);
        let near = pointer_pops(
            pop.out,
            pointer,
            (centre, nearest == Some(parent)),
            tunables,
        );
        let out = wants_out(near, inputs.held(parent, &hovered));
        let target = if out { 1.0 } else { 0.0 };
        next.insert(
            parent,
            Pop {
                out,
                t: advance_towards(pop.t, target, pop_step),
                shown: advance_towards(pop.shown, 1.0, show_step),
            },
        );
    }
    if pops.0 != next {
        pops.0 = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entities(n: usize) -> Vec<Entity> {
        let mut world = World::new();
        (0..n).map(|_| world.spawn_empty().id()).collect()
    }

    /// GRP-12: at rest the pip is `object_pip_px` across, centred
    /// `h + object_pip_gap_px` right of and below its parent, with no
    /// count; a back pip `pip_back_offset_px` right and up only when it
    /// holds two or more things.
    #[test]
    fn the_resting_pip_sits_down_and_right_with_a_back_pip_for_two() {
        let t = Tunables::default();
        let parent = Vec2::new(100.0, 50.0);
        let h = 10.0;
        let one = object_geometry(parent, h, (0.0, 1), &t);
        let gap = h + t.object_pip_gap_px;
        assert_eq!(one.centre, parent + Vec2::new(gap, gap), "right and below");
        assert_eq!(one.frame_px, t.object_pip_px);
        assert_eq!(one.back, None, "one thing has no back pip");
        assert!(!one.count_shown, "the pip has no count");
        let two = object_geometry(parent, h, (0.0, 2), &t);
        assert_eq!(two.back, Some(back_offset(t.pip_back_offset_px)));
    }

    /// GRP-13 / MUL-06: popped out, it is the full fold marker, centred
    /// `h + ship_symbol_px / 2` right of and below the parent, so its
    /// front frame's square starts at the corner of the parent's box; the
    /// count shows only once it has fully grown.
    #[test]
    fn popped_out_it_is_the_fold_marker_touching_the_parents_box() {
        let t = Tunables::default();
        let parent = Vec2::new(100.0, 50.0);
        let h = 10.0;
        let out = object_geometry(parent, h, (1.0, 3), &t);
        assert_eq!(out.frame_px, t.ship_symbol_px);
        assert_eq!(out.back, Some(back_offset(t.back_frame_offset_px)));
        assert!(out.count_shown);
        let front = Rect::from_center_size(out.centre, Vec2::splat(out.frame_px));
        assert_eq!(front.min, parent + Vec2::splat(h), "touches, no overlap");
        let half = object_geometry(parent, h, (0.5, 3), &t);
        assert!(half.frame_px > t.object_pip_px && half.frame_px < t.ship_symbol_px);
        assert!(!half.count_shown, "the count waits for the growth to end");
        let single = object_geometry(parent, h, (0.5, 1), &t);
        assert!(single.back.is_some(), "one thing's back frame comes out");
    }

    /// GRP-12 / MUL-02: an allied and an own ship folded in make an
    /// allied front pip with an own back pip behind it.
    #[test]
    fn an_allied_and_an_own_ship_make_an_allied_pip_on_an_own_back_pip() {
        use crate::sim::body::{Affiliation, BodyClass};
        use crate::view::markers::symbol_settings;
        use crate::view::multi_markers::look::{MultiLook, affiliation_frame};
        let t = Tunables::default();
        let settings = symbol_settings(&t);
        let things = [
            (BodyClass::Ship, Affiliation::Allied),
            (BodyClass::Ship, Affiliation::Own),
        ];
        let look = MultiLook::of_things(&things, things.len(), settings);
        assert_eq!(look.front, affiliation_frame(Affiliation::Allied, settings));
        assert_eq!(look.back, affiliation_frame(Affiliation::Own, settings));
        let pip = object_geometry(Vec2::ZERO, 10.0, (0.0, things.len()), &t);
        assert!(pip.back.is_some(), "two things show the back pip");
    }

    fn pointer(x: f32) -> Option<Pointer> {
        Some(Pointer {
            at: Vec2::new(x, 0.0),
            button_held: false,
        })
    }

    /// GRP-13: the pointer pops it within `pop_reach_px` of the parent,
    /// only when nearest that parent and with no button held; once out it
    /// stays out to `pop_release_px`, and goes back beyond it.
    #[test]
    fn the_pointer_pops_within_reach_and_releases_beyond_the_band() {
        let t = Tunables::default();
        let at = (Vec2::ZERO, true);
        let reach = t.pop_reach_px;
        let band = (t.pop_reach_px + t.pop_release_px) / 2.0;
        assert!(pointer_pops(false, pointer(reach - 1.0), at, &t));
        assert!(!pointer_pops(false, pointer(band), at, &t), "not yet");
        assert!(
            pointer_pops(true, pointer(band), at, &t),
            "held in the band"
        );
        assert!(!pointer_pops(true, pointer(t.pop_release_px + 1.0), at, &t));
        assert!(
            !pointer_pops(false, pointer(1.0), (Vec2::ZERO, false), &t),
            "another parent is nearer"
        );
        let held = Some(Pointer {
            at: Vec2::ONE,
            button_held: true,
        });
        assert!(!pointer_pops(false, held, at, &t), "a button is held");
        assert!(!pointer_pops(true, None, at, &t), "no pointer on the map");
    }

    /// GRP-13: in a crowd only the parent nearest the pointer pops.
    #[test]
    fn only_the_nearest_parent_pops() {
        let e = entities(2);
        let parents = [(e[0], Vec2::ZERO), (e[1], Vec2::new(20.0, 0.0))];
        assert_eq!(nearest_parent(Vec2::new(8.0, 0.0), &parents), Some(e[0]));
        assert_eq!(nearest_parent(Vec2::new(12.0, 0.0), &parents), Some(e[1]));
        assert_eq!(nearest_parent(Vec2::ZERO, &[]), None);
    }

    /// GRP-13: it is out while hovered, while its tray is open or while
    /// its parent or one of its things is selected, whatever the pointer.
    #[test]
    fn selection_hover_and_an_open_tray_hold_it_out() {
        assert!(!wants_out(false, Held::default()));
        assert!(wants_out(true, Held::default()));
        for held in [
            Held {
                hovered: true,
                ..default()
            },
            Held {
                tray_open: true,
                ..default()
            },
            Held {
                selected: true,
                ..default()
            },
        ] {
            assert!(wants_out(false, held), "{held:?}");
        }
    }
}
