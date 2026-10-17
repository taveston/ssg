//! What each drawn orbit ring stands for (HOV-04, HOV-05, MOT-14): a
//! natural body's own orbit, or a named orbit (SCL-07) and the ships and
//! stations in it. A named orbit with several members is a **shared
//! orbit**: one ring, targeted as a set. Nothing is grouped by radius.
//!
//! Shared state, read by the ring strokes, the ring hit targets, the
//! exposure-ring marks and the inset's list (ARCH-03). `OrbitRings` gathers
//! a ring's members with their fold and motion state; the rules on a
//! gathered `Ring` are plain methods, tested without a camera.

use std::collections::HashMap;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::sim::load::SystemLoadApp;

use super::fold::SatelliteFoldState;
use super::hovered::HoveredBodies;
use super::motion_mode::MotionModeState;
use super::multi::JoinSlide;
use super::rings::has_orbit_line;
use super::scale::km_to_render;
use super::selection::Selection;
use super::world::ViewNow;
use crate::sim::body::{Affiliation, BodyClass};
use crate::sim::named_orbit::{InNamedOrbit, NamedOrbit};
use crate::sim::orbit::{Orbit, OrbitPositionKm};
use crate::{Boot, Stage};

/// Each named orbit's members this frame (SCL-07, RAIL-09): the ships and
/// stations parked in it, in spawn order. Kept up to date as ships leave
/// and arrive, and written only when the membership changes, so what is
/// built from it can be rebuilt on change. A named orbit with no members
/// has no entry and draws nothing.
#[derive(Resource, Default, Debug, PartialEq)]
pub struct OrbitMembers(pub HashMap<Entity, Vec<Entity>>);

pub struct SharedOrbitPlugin;

impl Plugin for SharedOrbitPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<OrbitMembers>()
            .add_load_systems(Boot::Derive, index_members)
            // Once the simulation has moved ships between orbits, before
            // anything on screen reads who is where.
            .add_systems(Update, index_members.in_set(Stage::Projection));
    }
}

/// Run condition for what is built per ring (strokes, hit targets): true
/// when the named orbits' membership changed, or a body gained or lost an
/// orbit of its own. A lone ship or station outside any named orbit owns
/// its ring through that orbit, and changes no membership when it leaves.
pub fn rings_changed(
    members: Res<OrbitMembers>,
    added: Query<(), Added<Orbit>>,
    mut removed: RemovedComponents<Orbit>,
) -> bool {
    // Read every removal, so none is seen again on a later frame.
    let removed = removed.read().count() > 0;
    members.is_changed() || !added.is_empty() || removed
}

/// SCL-07 / RAIL-09: which bodies sit in which named orbit, in spawn order
/// (an entity's index rises with its spawn). A ship that leaves an orbit
/// drops out of it and one that arrives joins it, whatever body the orbit
/// circles. The resource is written only when this changes.
fn index_members(bodies: Query<(Entity, &InNamedOrbit)>, mut members: ResMut<OrbitMembers>) {
    let mut sorted: Vec<(Entity, Entity)> = bodies
        .iter()
        .map(|(body, in_orbit)| (body, in_orbit.0))
        .collect();
    sorted.sort_by_key(|(body, _)| body.index_u32());
    let mut now = OrbitMembers::default();
    for (body, orbit) in sorted {
        now.0.entry(orbit).or_default().push(body);
    }
    members.set_if_neq(now);
}

/// One body on a ring, with what decides how it is drawn this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Member {
    pub body: Entity,
    pub class: BodyClass,
    pub affiliation: Affiliation,
    /// Where it is on the ring now, in radians.
    pub angle: f32,
    /// Its start phase in radians: the fixed spot its label sits at
    /// (MOT-02). Kept at full precision, so what is together on the ring
    /// is decided exactly as on the map (PHAS-02).
    pub phase0: f64,
    /// Its orbital period in days. Its angle grows with time while this is
    /// positive, and shrinks while it is negative.
    pub period_days: f64,
    /// GRP-01: folded into its parent, so not drawn.
    pub hidden: bool,
    /// MUL-05: joined into a multi-marker, so it draws no tail of its own.
    pub joined: bool,
    /// MUL-05: the first tray row of the together or overlap marker it is
    /// joined into, so in dot mode it lays that marker's one tail.
    pub lead: bool,
    /// Where its tail starts on the ring, in radians: at the body, or for
    /// a lead, at the point of the ring nearest where its symbol is drawn
    /// as it slides into its marker, so the marker's tail runs behind the
    /// marker's centre.
    pub tail_head: f32,
    pub mode: MotionModeState,
}

/// A ring and what it carries: a lone natural body, or a named orbit's
/// members.
#[derive(Debug, Clone, PartialEq)]
pub struct Ring {
    /// The body or named orbit the ring stands for.
    pub owner: Entity,
    /// The body it circles; `None` for the star.
    pub centre: Option<Entity>,
    pub radius_km: f64,
    /// The orbit's name: the body's own, or the named orbit's.
    pub name: String,
    /// HOV-05: a named orbit with several members, targeted as a set.
    pub shared: bool,
    pub members: Vec<Member>,
}

impl Ring {
    /// The class the ring is drawn for: its members' (a named orbit holds
    /// ships and stations, a natural body itself).
    pub fn class(&self) -> BodyClass {
        self.members.first().map_or(BodyClass::Ship, |m| m.class)
    }

    /// The motion state the ring is drawn with: its members share a period,
    /// so any member's.
    pub fn mode(&self) -> MotionModeState {
        self.members.first().map(|m| m.mode).unwrap_or_default()
    }

    /// HOV-04: the members drawn this frame, leaving out folded ones.
    pub fn drawn(&self) -> impl Iterator<Item = &Member> {
        self.members.iter().filter(|m| !m.hidden)
    }

    /// MOT-01 / MUL-05: the members whose tails are drawn: drawn, and
    /// either not joined into a multi-marker or laying the tail of the
    /// together or overlap marker they lead.
    pub fn tails(&self) -> impl Iterator<Item = &Member> {
        self.drawn().filter(|m| !m.joined || m.lead)
    }

    /// MOT-01 / MOT-02 / MUL-05: the angles the ring's brightness is laid
    /// from: each tail's head in dot mode; in ring mode the exposure ring
    /// is the ring itself, so every drawn member's, joined or not.
    pub fn tail_angles(&self) -> Vec<f32> {
        if self.mode().ring_mode {
            self.drawn().map(|m| m.angle).collect()
        } else {
            self.tails().map(|m| m.tail_head).collect()
        }
    }

    /// Whether the frame body sits on this ring, which makes it no target
    /// (HOV-02).
    pub fn carries(&self, body: Option<Entity>) -> bool {
        body.is_some_and(|body| self.members.iter().any(|m| m.body == body))
    }

    /// HOV-03 / HOV-05: whether the ring is highlighted: any member is
    /// selected or hovered, or (shared) the ring or its identity mark is
    /// hovered. Returns (selected, hovered).
    pub fn highlight(&self, selection: &Selection, hovered: &HoveredBodies) -> (bool, bool) {
        let selected = self.members.iter().any(|m| selection.is_selected(m.body));
        let hovered =
            hovered.contains(self.owner) || self.members.iter().any(|m| hovered.contains(m.body));
        (selected, hovered)
    }

    /// MOT-02 / MOT-14: the fixed spot on the ring the label and identity
    /// mark sit at: the first member's start phase.
    pub fn label_angle(&self) -> f32 {
        self.members.first().map_or(0.0, |m| m.phase0 as f32)
    }

    /// MOT-02: the ring's period in days: its members share one, so any
    /// member's.
    pub fn period_days(&self) -> f64 {
        self.members.first().map_or(0.0, |m| m.period_days)
    }

    /// MOT-14: the way the ring's bodies circle it: +1 while their angle
    /// grows with time, -1 while it shrinks.
    pub fn sense(&self) -> f32 {
        if self.period_days() < 0.0 { -1.0 } else { 1.0 }
    }
}

/// The bodies a ring is gathered from.
type RingBodies<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Orbit,
        &'static OrbitPositionKm,
        &'static BodyClass,
        &'static Affiliation,
        Option<&'static ChildOf>,
        Option<&'static InNamedOrbit>,
        Option<&'static SatelliteFoldState>,
        Option<&'static MotionModeState>,
        Option<&'static JoinSlide>,
    ),
>;

/// Every ring in the system, by what it stands for.
#[derive(SystemParam)]
pub struct OrbitRings<'w, 's> {
    members: Res<'w, OrbitMembers>,
    named: Query<'w, 's, (Entity, &'static NamedOrbit)>,
    bodies: RingBodies<'w, 's>,
    names: Query<'w, 's, &'static crate::sim::body::DisplayName>,
    view: ViewNow<'w, 's>,
}

impl OrbitRings<'_, '_> {
    /// HOV-04: everything that gets a ring: each named orbit with members,
    /// and each natural body with an orbit line of its own.
    pub fn owners(&self) -> Vec<Entity> {
        let mut owners: Vec<Entity> = self
            .named
            .iter()
            .filter(|(orbit, _)| self.members.0.contains_key(orbit))
            .map(|(orbit, _)| orbit)
            .chain(
                self.bodies
                    .iter()
                    .filter(|(_, _, _, class, _, _, in_orbit, ..)| {
                        in_orbit.is_none() && has_orbit_line(**class)
                    })
                    .map(|(body, ..)| body),
            )
            .collect();
        owners.sort();
        owners
    }

    /// Whether `owner` can stand for a ring at all, with members or not: a
    /// named orbit, or a natural body with an orbit line of its own. What
    /// is kept for a ring (its stroke, its identity mark) is kept while
    /// this holds, so an orbit that empties and fills again picks up where
    /// it was.
    pub fn stands(&self, owner: Entity) -> bool {
        self.named.contains(owner)
            || self
                .bodies
                .get(owner)
                .is_ok_and(|(_, _, _, class, _, _, in_orbit, ..)| {
                    in_orbit.is_none() && has_orbit_line(*class)
                })
    }

    fn member(&self, body: Entity) -> Option<Member> {
        let (_, orbit, local, class, affiliation, _, _, fold, mode, slide) =
            self.bodies.get(body).ok()?;
        let angle = local.0.z.atan2(local.0.x) as f32;
        Some(Member {
            body,
            class: *class,
            affiliation: *affiliation,
            angle,
            phase0: orbit.phase0_rad,
            period_days: orbit.period_days,
            hidden: fold.is_some_and(|f| f.effective_hidden),
            joined: slide.is_some_and(|s| s.held),
            lead: slide.is_some_and(|s| s.held && s.lead),
            tail_head: angle,
            mode: mode.copied().unwrap_or_default(),
        })
    }

    /// MUL-05: move each lead's tail head to the point of its ring nearest
    /// where its symbol is drawn this frame, part way into its marker as
    /// it slides there, so the marker's tail runs behind the marker's
    /// centre. Things together share a point, so theirs stays put.
    fn place_lead_heads(&self, mut ring: Ring) -> Ring {
        for i in 0..ring.members.len() {
            let member = ring.members[i];
            if !member.lead {
                continue;
            }
            let Some((.., Some(slide))) = self.bodies.get(member.body).ok() else {
                continue;
            };
            // Under half a pixel off its own place: the head stays put.
            let shift = slide.offset * slide.t;
            if shift.length_squared() < 0.25 {
                continue;
            }
            let point = |angle| ring_screen_point(&ring, angle, &self.view);
            let Some(own) = point(member.angle) else {
                continue;
            };
            let head = nearest_ring_angle(member.angle, own + shift, point);
            ring.members[i].tail_head = head;
        }
        ring
    }

    /// The ring `owner` stands for, gathered this frame; `None` if it has
    /// none (no members, or no orbit line).
    pub fn ring(&self, owner: Entity) -> Option<Ring> {
        if let Ok((_, named)) = self.named.get(owner) {
            let members: Vec<Member> = self
                .members
                .0
                .get(&owner)?
                .iter()
                .filter_map(|&body| self.member(body))
                .collect();
            let shared = match members.len() {
                0 => return None,
                n => n > 1,
            };
            // MOT-02 / HOV-05: a named orbit with one member is that
            // member's ring, so it carries the member's name.
            let name = if shared {
                named.name.clone()
            } else {
                self.name_of(members[0].body)
            };
            return Some(self.place_lead_heads(Ring {
                owner,
                centre: Some(named.parent),
                radius_km: named.radius_km,
                name,
                shared,
                members,
            }));
        }
        let (_, orbit, _, class, _, child_of, in_orbit, ..) = self.bodies.get(owner).ok()?;
        if in_orbit.is_some() || !has_orbit_line(*class) {
            return None;
        }
        Some(self.place_lead_heads(Ring {
            owner,
            centre: child_of.map(ChildOf::parent),
            radius_km: orbit.radius_km,
            name: self.name_of(owner),
            shared: false,
            members: vec![self.member(owner)?],
        }))
    }

    fn name_of(&self, body: Entity) -> String {
        self.names
            .get(body)
            .map(|n| n.0.clone())
            .unwrap_or_default()
    }
}

/// Newton steps taken towards the nearest point, and the most one step
/// may turn, in radians, so a far target can't send the search round the
/// ring.
const NEAREST_STEPS: usize = 4;
const NEAREST_MAX_STEP: f32 = 0.25;
/// The angle the slope on screen is measured over, in radians.
const NEAREST_PROBE: f32 = 1e-3;

/// MUL-05: the angle on a ring whose point on screen (`point`) is nearest
/// `target`, searched from `angle` along the ring's on-screen slope. Where
/// the ring has no slope on screen, or doesn't project, it stays put.
pub fn nearest_ring_angle(angle: f32, target: Vec2, point: impl Fn(f32) -> Option<Vec2>) -> f32 {
    let mut at = angle;
    for _ in 0..NEAREST_STEPS {
        let (Some(here), Some(ahead), Some(behind)) = (
            point(at),
            point(at + NEAREST_PROBE),
            point(at - NEAREST_PROBE),
        ) else {
            break;
        };
        let slope = (ahead - behind) / (2.0 * NEAREST_PROBE);
        let slope_sq = slope.length_squared();
        if slope_sq < 1e-6 {
            break;
        }
        let step =
            ((target - here).dot(slope) / slope_sq).clamp(-NEAREST_MAX_STEP, NEAREST_MAX_STEP);
        at += step;
    }
    at
}

/// Where the point at `angle` on a ring lands on screen this frame.
pub fn ring_screen_point(ring: &Ring, angle: f32, view: &ViewNow) -> Option<Vec2> {
    let centre = match ring.centre {
        Some(parent) => view.position(parent)?,
        None => view.star(),
    };
    let radius = km_to_render(ring.radius_km);
    let world = centre + Vec3::new(radius * angle.cos(), 0.0, radius * angle.sin());
    view.project(world)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member(world: &mut World, class: BodyClass, affiliation: Affiliation) -> Member {
        Member {
            body: world.spawn_empty().id(),
            class,
            affiliation,
            angle: 0.0,
            phase0: 1.0,
            period_days: 1.0,
            hidden: false,
            joined: false,
            lead: false,
            tail_head: 0.0,
            mode: MotionModeState::default(),
        }
    }

    fn ring(world: &mut World, members: Vec<Member>) -> Ring {
        Ring {
            owner: world.spawn_empty().id(),
            centre: None,
            radius_km: 1.0,
            name: "High".to_string(),
            shared: members.len() > 1,
            members,
        }
    }

    /// HOV-03 / HOV-05: a shared ring lights up when the ring itself, its
    /// mark or any member is hovered, and when any member is selected.
    #[test]
    fn a_shared_ring_is_highlighted_through_any_of_its_members() {
        let mut world = World::new();
        let a = member(&mut world, BodyClass::Ship, Affiliation::Own);
        let b = member(&mut world, BodyClass::Ship, Affiliation::Own);
        let ring = ring(&mut world, vec![a, b]);
        let mut selection = Selection::default();
        let mut hovered = HoveredBodies::default();
        assert_eq!(ring.highlight(&selection, &hovered), (false, false));
        hovered.0.insert(ring.owner);
        assert_eq!(ring.highlight(&selection, &hovered), (false, true));
        hovered.0.clear();
        hovered.0.insert(b.body);
        assert_eq!(ring.highlight(&selection, &hovered), (false, true));
        selection.select(a.body);
        assert_eq!(ring.highlight(&selection, &hovered), (true, true));
    }

    /// HOV-04 / HOV-02: folded members aren't drawn on the ring; the ring
    /// carrying the frame body is no target.
    #[test]
    fn folded_members_are_left_off_the_ring() {
        let mut world = World::new();
        let a = member(&mut world, BodyClass::Ship, Affiliation::Own);
        let folded = Member {
            hidden: true,
            ..member(&mut world, BodyClass::Ship, Affiliation::Own)
        };
        let ring = ring(&mut world, vec![a, folded]);
        let drawn: Vec<Entity> = ring.drawn().map(|m| m.body).collect();
        assert_eq!(drawn, [a.body]);
        assert!(ring.carries(Some(a.body)));
        assert!(!ring.carries(None));
    }

    /// MUL-05: a member joined into a multi-marker is still on its ring
    /// but draws no tail of its own.
    #[test]
    fn joined_members_draw_no_tail() {
        let mut world = World::new();
        let a = member(&mut world, BodyClass::Ship, Affiliation::Own);
        let joined = Member {
            joined: true,
            ..member(&mut world, BodyClass::Ship, Affiliation::Own)
        };
        let ring = ring(&mut world, vec![a, joined]);
        assert_eq!(ring.drawn().count(), 2);
        let tails: Vec<Entity> = ring.tails().map(|m| m.body).collect();
        assert_eq!(tails, [a.body]);
    }

    /// MUL-05: in dot mode the things together on a ring draw one tail,
    /// the lead's, from where its symbol is drawn; in ring mode the whole
    /// ring is the exposure ring and no extra tail is laid.
    #[test]
    fn a_joined_lead_lays_the_markers_one_tail_in_dot_mode() {
        let mut world = World::new();
        let joined = |world: &mut World, lead| Member {
            joined: true,
            lead,
            angle: 1.0,
            tail_head: if lead { 1.1 } else { 1.0 },
            ..member(world, BodyClass::Ship, Affiliation::Own)
        };
        let convoy = [false, true, false].map(|lead| joined(&mut world, lead));
        let dots = ring(&mut world, convoy.to_vec());
        let tails: Vec<Entity> = dots.tails().map(|m| m.body).collect();
        assert_eq!(tails, [convoy[1].body], "one tail, the lead's");
        assert_eq!(dots.tail_angles(), [1.1], "from the lead's head");
        let ring_mode = MotionModeState {
            ring_mode: true,
            ..default()
        };
        let running = convoy.map(|m| Member {
            mode: ring_mode,
            ..m
        });
        let exposure = ring(&mut world, running.to_vec());
        assert_eq!(exposure.tail_angles(), [1.0, 1.0, 1.0]);
    }

    /// MUL-05: the search for the ring point nearest a marker's centre
    /// lands on the ring where the centre lies off it, both ahead and
    /// behind; a ring with no slope on screen keeps its angle.
    #[test]
    fn the_lead_head_is_the_ring_point_nearest_the_marker() {
        let circle = |angle: f32| Some(Vec2::new(angle.cos(), angle.sin()) * 100.0);
        for want in [0.3_f32, -0.2] {
            let target = Vec2::new(want.cos(), want.sin()) * 110.0;
            let got = nearest_ring_angle(0.0, target, circle);
            assert!((got - want).abs() < 1e-3, "{got} for {want}");
        }
        // A tilted ring is an ellipse on screen: the search finds the
        // point nearest by brute force, not the one at the same angle.
        let tilted = |angle: f32| Some(Vec2::new(angle.cos() * 100.0, angle.sin() * 25.0));
        let target = Vec2::new(70.0, 40.0);
        let gap = |angle| tilted(angle).unwrap().distance(target);
        let best = (0..=2000)
            .map(|i| i as f32 * 1e-3 - 0.5)
            .map(gap)
            .fold(f32::INFINITY, f32::min);
        let got = nearest_ring_angle(0.5, target, tilted);
        assert!(gap(got) <= best + 0.1, "{} for {best}", gap(got));
        let flat = |_| Some(Vec2::ZERO);
        assert_eq!(nearest_ring_angle(0.5, Vec2::ONE, flat), 0.5);
        // Where the ring doesn't project (behind the camera), it stays put.
        assert_eq!(nearest_ring_angle(0.5, Vec2::ONE, |_| None), 0.5);
    }

    /// MOT-02 / MOT-14: a ring's period is its members', and its bodies
    /// circle with growing angle for a positive period, shrinking for a
    /// negative one, which turns the direction chevron round.
    #[test]
    fn a_rings_period_gives_the_way_its_bodies_circle() {
        let mut world = World::new();
        let forward = Member {
            period_days: 1.5,
            ..member(&mut world, BodyClass::Moon, Affiliation::Neutral)
        };
        let backward = Member {
            period_days: -1.5,
            ..forward
        };
        let ahead = ring(&mut world, vec![forward]);
        assert_eq!((ahead.period_days(), ahead.sense()), (1.5, 1.0));
        assert_eq!(ring(&mut world, vec![backward]).sense(), -1.0);
    }

    /// MOT-02 / MOT-14: the label spot is the first member's start phase,
    /// so a lone ship's label stays where it was.
    #[test]
    fn the_label_spot_is_the_first_members_start_phase() {
        let mut world = World::new();
        let a = Member {
            phase0: 2.5,
            ..member(&mut world, BodyClass::Station, Affiliation::Own)
        };
        let b = Member {
            phase0: 0.3,
            ..member(&mut world, BodyClass::Ship, Affiliation::Own)
        };
        assert_eq!(ring(&mut world, vec![a, b]).label_angle(), 2.5);
        assert_eq!(ring(&mut world, vec![]).label_angle(), 0.0);
    }
}
