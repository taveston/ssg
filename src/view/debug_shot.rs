//! A debug option for seeing the game: set up a view, capture the game
//! window to a file and exit, with no code edits. Not part of the game.
//!
//! Set the `OW_SHOT` environment variable to a list of `key=value` pairs
//! separated by `;` (names may contain spaces), then `cargo run`:
//!
//! ```text
//! OW_SHOT="out=shot.png;frame=Halden IV;distance=2e6;tilt=40;rate=0" cargo run
//! ```
//!
//! - `out`: where to save the image (required). The extension picks the
//!   format (`.png`).
//! - `frame`: a body's name; the camera moves into its frame, as a
//!   double-click does.
//! - `select`: a body's name to select, opening its inset.
//! - `hover`: a body's name to show as hovered.
//! - `distance`: camera distance from the pivot, in km.
//! - `tilt`: tilt from straight down, in degrees (clamped as the camera's).
//! - `heading`: heading in degrees; 0 is north up.
//! - `rate`: clock rate in days per second; 0 pauses the clock.
//! - `wait`: seconds to let the view settle before the capture (default 2).
//! - `timeout`: seconds after start by which the shot must be saved
//!   (default 60). Past it the app logs the phase it was stuck in and exits
//!   with an error, so a shot that can't finish never leaves a window open.
//! - `planner`: opens the transfer planner for the selected ship: `open`
//!   leaves it choosing; a body's name picks that body as the destination,
//!   as a click on it would (PLNR-02, PLNR-11).
//! - `tray`: a body's name; once the view is set, opens the tray of the
//!   multi-marker that holds it (MUL-07): the body's fold marker if it has
//!   one, its station marker if it is a station with things docked, or
//!   else the marker it is joined, docked or folded into.
//! - `fly`: with `planner` naming a destination, schedules the planner's
//!   order as Schedule would (PLNR-50), then moves the clock on to this
//!   many days after the ship leaves its orbit (PLNR-21), so the shot
//!   shows the ship in flight. Negative days stop short of the leave day.
//! - `sector`: starts the shot in the sector view with a new sector of
//!   this size, as `--sector` takes it: a preset or a number of gated
//!   systems (SECT-44). `select` and `hover` then name a gated system or
//!   an entry's designation, and `distance` is from the sector's centre.
//! - `seed`: the sector's seed, as `--seed` takes it; only with `sector`.
//!
//! Names are the bodies' display names, or in the sector view the
//! systems' names and the entries' designations. The app starts, waits a moment for
//! the world to load, applies the selection, the rate and the frame, waits
//! for the frame move to end, sets distance, tilt and heading, waits
//! `wait` seconds, saves the screenshot and exits.
//!
//! A bad `sector` or `seed`, or `seed` without `sector`, stops the app as
//! an unknown key does. These exit at once with an error log instead of
//! waiting for the timeout:
//! - a name that matches no body (or, in the sector view, no entry);
//! - `planner` with no `select`, or selecting a body that can't open the
//!   planner: anything but a ship the faction commands, parked in a named
//!   orbit with no order (PLNR-01);
//! - with `fly`, a destination that can't be picked, or whose order can't
//!   be scheduled once the planner has searched and solved it, or that
//!   Schedule refuses when it solves a Soonest order again: the log
//!   gives the planner's own reason (PLNR-92), such as "Needs 13.2 km/s,
//!   has 9.9" for a trip over the ship's budget.
//!
//! A shot with `tray` whose tray never opened fails at the capture,
//! naming the body.

use std::path::PathBuf;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk};

use super::camera::requests::FrameRequest;
use super::camera::rules::clamp_tilt;
use super::camera::state::{ActiveMove, CameraRig, MAX_DISTANCE, MIN_DISTANCE};
use super::inset::content::is_commanded_ship;
use super::inset::inspect::{OrderableShip, may_take_order};
use super::map_layer::{MapLayer, SectorSelection};
use super::multi::{MultiKey, MultiMarkers, OpenTray, TrayFor, TrayRequest};
use super::planner::input::{order_to_schedule, schedule_order};
use super::planner::model::blocking_reason;
use super::planner::state::{PickTarget, PlannerState};
use super::planner::words::reason;
use super::scale::{km_to_render, sector_km_to_render};
use super::sector::ForEntry;
use super::sector::input::NodeTarget;
use super::selection::Selection;
use super::targets::{Hovered, MarkerFor};
use super::tunables::Tunables;
use crate::Stage;
use crate::content::generator::Sector;
use crate::content::generator::tunables::GeneratorTunables;
use crate::content::start::{LoadedSector, SectorRequest, Start, load_start, sector_size};
use crate::sim::body::{BodyClass, DisplayName};
use crate::sim::clock::GameClock;
use crate::sim::track::ShipRecord;
use crate::sim::transfer::order::Infeasible;
use crate::sim::transfer::plan::{Plan, ShipOrder};

/// The environment variable that asks for a screenshot.
pub const SHOT_ENV: &str = "OW_SHOT";

/// Seconds after start before the view is set up, so the world, fonts and
/// the first layout are in place.
const START_AFTER_SECS: f32 = 1.0;
/// Seconds the view settles before the capture, unless `wait` says.
const DEFAULT_WAIT_SECS: f32 = 2.0;
/// Seconds after start by which the shot must be saved, unless `timeout`
/// says.
const DEFAULT_TIMEOUT_SECS: f32 = 60.0;

/// What `OW_SHOT` asked for: the view to set up and where to save it.
#[derive(Debug, Clone, PartialEq)]
pub struct ShotPlan {
    pub out: PathBuf,
    pub frame: Option<String>,
    pub select: Option<String>,
    pub hover: Option<String>,
    pub distance_km: Option<f64>,
    pub tilt_deg: Option<f32>,
    pub heading_deg: Option<f32>,
    pub rate_days_per_sec: Option<f64>,
    pub wait_secs: f32,
    /// Seconds after start by which the shot must be saved.
    pub timeout_secs: f32,
    /// `open`, or the name of a destination to pick.
    pub planner: Option<String>,
    /// Days after the leave day to move the clock to, once the planner's
    /// order is scheduled.
    pub fly_days: Option<f64>,
    /// The body whose multi-marker's tray to open.
    pub tray: Option<String>,
    /// A new sector to start in, its size as `--sector` takes it.
    pub sector: Option<String>,
    /// The sector's seed, as `--seed` takes it.
    pub seed: Option<String>,
}

/// The `planner` value that opens the planner without picking.
const PLANNER_OPEN: &str = "open";

impl ShotPlan {
    /// Parse `key=value;key=value`. Unknown keys, bad numbers and a
    /// missing `out` are errors, so a typo never silently gives the
    /// default view.
    pub fn parse(spec: &str) -> Result<Self, String> {
        let mut plan = ShotPlan {
            out: PathBuf::new(),
            frame: None,
            select: None,
            hover: None,
            distance_km: None,
            tilt_deg: None,
            heading_deg: None,
            rate_days_per_sec: None,
            wait_secs: DEFAULT_WAIT_SECS,
            timeout_secs: DEFAULT_TIMEOUT_SECS,
            planner: None,
            fly_days: None,
            tray: None,
            sector: None,
            seed: None,
        };
        let pairs = spec.split(';').map(str::trim).filter(|p| !p.is_empty());
        for pair in pairs {
            let (key, value) = pair
                .split_once('=')
                .ok_or_else(|| format!("`{pair}` is not key=value"))?;
            let (key, value) = (key.trim(), value.trim());
            match key {
                "out" => plan.out = PathBuf::from(value),
                "frame" => plan.frame = Some(value.to_string()),
                "select" => plan.select = Some(value.to_string()),
                "hover" => plan.hover = Some(value.to_string()),
                "distance" => plan.distance_km = Some(number(key, value)?),
                "tilt" => plan.tilt_deg = Some(number(key, value)?),
                "heading" => plan.heading_deg = Some(number(key, value)?),
                "rate" => plan.rate_days_per_sec = Some(number(key, value)?),
                "wait" => plan.wait_secs = number(key, value)?,
                "timeout" => plan.timeout_secs = number(key, value)?,
                "planner" => plan.planner = Some(value.to_string()),
                "fly" => plan.fly_days = Some(number(key, value)?),
                "tray" => plan.tray = Some(value.to_string()),
                "sector" => plan.sector = Some(value.to_string()),
                "seed" => plan.seed = Some(value.to_string()),
                _ => return Err(format!("unknown key `{key}`")),
            }
        }
        if plan.out.as_os_str().is_empty() {
            return Err("`out` (the image path) is required".to_string());
        }
        plan.sector_request()?;
        Ok(plan)
    }

    /// SECT-44: the sector `sector` and `seed` ask for, if any; a bad
    /// size or seed, or a seed with no sector, is an error.
    pub fn sector_request(&self) -> Result<Option<SectorRequest>, String> {
        let Some(size) = self.sector.as_deref() else {
            return match self.seed {
                Some(_) => Err("`seed` seeds a sector: give `sector` too".to_string()),
                None => Ok(None),
            };
        };
        let gated_count = sector_size(size, &GeneratorTunables::default())?;
        let seed = self
            .seed
            .as_deref()
            .map(|text| number::<u64>("seed", text))
            .transpose()?;
        Ok(Some(SectorRequest { gated_count, seed }))
    }
}

fn number<T: std::str::FromStr>(key: &str, value: &str) -> Result<T, String> {
    value
        .parse()
        .map_err(|_| format!("`{key}` needs a number, got `{value}`"))
}

/// Where the shot is in its run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShotPhase {
    /// Waiting for the world to load.
    Starting,
    /// Waiting for the planner's order to be ready, to schedule it.
    Scheduling,
    /// Schedule refused the order, solved again from the clock; the shot
    /// fails in this frame with the planner's reason.
    Refused,
    /// Waiting for the order to reach the ship, to move the clock on.
    Flying,
    /// The frame move is in flight.
    Framing,
    /// The view is set; waiting for it to settle.
    Settling { since: f32 },
    /// The screenshot is asked for; waiting for it to be saved.
    Capturing,
    /// Saved; the app exits.
    Saved,
    /// The shot can't be taken as asked; the app exits with an error.
    Failed,
}

/// The shot in progress. Present only when `OW_SHOT` is set (or a test
/// inserts it).
#[derive(Resource, Debug, Clone)]
pub struct Shot {
    pub plan: ShotPlan,
    pub phase: ShotPhase,
}

impl Shot {
    pub fn new(plan: ShotPlan) -> Self {
        Self {
            plan,
            phase: ShotPhase::Starting,
        }
    }

    /// True once the shot has run past its timeout without being saved
    /// or failing.
    pub fn timed_out(&self, now: f32) -> bool {
        !matches!(self.phase, ShotPhase::Saved | ShotPhase::Failed) && now >= self.plan.timeout_secs
    }

    /// Logs why the shot can't be taken and asks the app to exit with an
    /// error.
    fn fail(&mut self, why: &str, exit: &mut MessageWriter<AppExit>) {
        error!("{SHOT_ENV}: {why}");
        exit.write(AppExit::error());
        self.phase = ShotPhase::Failed;
    }
}

pub struct DebugShotPlugin;

impl Plugin for DebugShotPlugin {
    fn build(&self, app: &mut App) {
        if let Ok(spec) = std::env::var(SHOT_ENV) {
            let plan = ShotPlan::parse(&spec).unwrap_or_else(|e| panic!("{SHOT_ENV}: {e}"));
            // SECT-44: a shot of a sector starts the game in it, whatever
            // the arguments asked for.
            if let Ok(Some(request)) = plan.sector_request() {
                load_start(app, Start::Sector(request));
            }
            app.insert_resource(Shot::new(plan));
        }
        app.add_systems(
            Update,
            (fly_order, refuse_unplannable, open_tray, run_shot)
                .chain()
                .run_if(resource_exists::<Shot>)
                .after(Stage::Simulation)
                .before(Stage::Camera),
        );
    }
}

/// What the shot reads of a body to decide whether it can open the
/// planner (PLNR-01).
type PlannerShip = (
    &'static BodyClass,
    Option<&'static ShipRecord>,
    OrderableShip,
);

/// What the shot sets up at its start: the bodies it names, their
/// markers, the selection, the planner, the clock and the frame request.
#[derive(SystemParam)]
struct ShotScene<'w, 's> {
    commands: Commands<'w, 's>,
    names: Query<'w, 's, (Entity, &'static DisplayName)>,
    markers: Query<'w, 's, (Entity, &'static MarkerFor)>,
    ships: Query<'w, 's, PlannerShip>,
    selection: ResMut<'w, Selection>,
    planner: ResMut<'w, PlannerState>,
    clock: ResMut<'w, GameClock>,
    frame_request: ResMut<'w, FrameRequest>,
    sector: ShotSector<'w, 's>,
}

/// What a shot of the sector view reads and sets: the layer, the sector,
/// its selection and its nodes' hit targets (SECT-44).
#[derive(SystemParam)]
struct ShotSector<'w, 's> {
    layer: Res<'w, MapLayer>,
    loaded: Option<Res<'w, LoadedSector>>,
    selection: ResMut<'w, SectorSelection>,
    targets: Query<'w, 's, (Entity, &'static ForEntry), With<NodeTarget>>,
}

impl ShotSector<'_, '_> {
    /// SECT-44: the entry a name stands for in the loaded sector.
    fn find(&self, name: &str) -> Option<usize> {
        sector_entry_named(&self.loaded.as_ref()?.sector, name)
    }

    /// SECT-44: select and hover the entries the plan names; a name that
    /// is no entry's is an error.
    fn start(&mut self, plan: &ShotPlan, commands: &mut Commands) -> Result<(), String> {
        let wanted = [&plan.frame, &plan.select, &plan.hover, &plan.tray];
        if let Some(missing) = wanted
            .into_iter()
            .flatten()
            .find(|n| self.find(n).is_none())
        {
            return Err(format!(
                "no system or entry in the sector is named `{missing}`"
            ));
        }
        if plan.frame.is_some() || plan.tray.is_some() || plan.planner.is_some() {
            return Err("in the sector view only `select` and `hover` name things".to_string());
        }
        self.selection.0 = plan.select.as_deref().and_then(|n| self.find(n));
        if let Some(id) = plan.hover.as_deref().and_then(|n| self.find(n)) {
            for (target, _) in self.targets.iter().filter(|(_, e)| e.0 == id) {
                commands.entity(target).insert(Hovered);
            }
        }
        Ok(())
    }
}

impl ShotScene<'_, '_> {
    /// The body with this display name.
    fn find(&self, name: &str) -> Option<Entity> {
        self.names
            .iter()
            .find(|(_, display)| display.0 == name)
            .map(|(entity, _)| entity)
    }

    /// Whether the body named `select` is a ship the planner can open
    /// for, as the inset's Transfer button shows (PLNR-01); or why not.
    fn check_planner_ship(&self, select: Option<&str>) -> Result<(), String> {
        let Some(name) = select else {
            return Err("`planner` needs `select` to name the ship to plan for".to_string());
        };
        let ship = self.find(name);
        let ok = ship.and_then(|ship| self.ships.get(ship).ok()).is_some_and(
            |(class, record, orderable)| {
                is_commanded_ship(*class, record) && may_take_order(orderable)
            },
        );
        if ok {
            Ok(())
        } else {
            Err(format!(
                "`{name}` can't open the planner: only a ship the faction \
                 commands, parked in a named orbit with no order, can"
            ))
        }
    }

    /// Applies the plan's selection, planner, hover, rate and frame, and
    /// says which phase comes next; or says why the shot can't be taken:
    /// a body that doesn't exist, or a planner that can't open.
    fn start(&mut self, plan: &ShotPlan) -> Result<ShotPhase, String> {
        if self.sector.layer.is_sector() {
            self.sector.start(plan, &mut self.commands)?;
            self.set_rate(plan.rate_days_per_sec);
            return Ok(ShotPhase::Framing);
        }
        let destination = plan.planner.as_ref().filter(|p| *p != PLANNER_OPEN);
        let wanted = [&plan.frame, &plan.select, &plan.hover, &plan.tray]
            .into_iter()
            .flatten()
            .chain(destination);
        if let Some(missing) = wanted.into_iter().find(|n| self.find(n).is_none()) {
            return Err(format!("no body is named `{missing}`"));
        }
        if plan.planner.is_some() {
            self.check_planner_ship(plan.select.as_deref())?;
        }
        if let Some(body) = plan.select.as_deref().and_then(|n| self.find(n)) {
            self.selection.select(body);
            if plan.planner.is_some() {
                self.planner.open(body);
            }
            if let Some(place) = destination.and_then(|name| self.find(name)) {
                self.planner.pick(PickTarget::Body(place));
            }
        }
        if let Some(body) = plan.hover.as_deref().and_then(|n| self.find(n)) {
            let hovered: Vec<Entity> = self
                .markers
                .iter()
                .filter(|(_, m)| m.0 == body)
                .map(|(marker, _)| marker)
                .collect();
            for marker in hovered {
                self.commands.entity(marker).insert(Hovered);
            }
        }
        self.set_rate(plan.rate_days_per_sec);
        self.frame_request.0 = plan.frame.as_deref().and_then(|n| self.find(n));
        Ok(if plan.fly_days.is_some() && destination.is_some() {
            ShotPhase::Scheduling
        } else {
            ShotPhase::Framing
        })
    }

    /// `rate`: 0 or less pauses the clock; more runs it at that rate.
    fn set_rate(&mut self, rate: Option<f64>) {
        match rate {
            Some(rate) if rate <= 0.0 => {
                if !self.clock.is_paused() {
                    self.clock.toggle_pause();
                }
            }
            Some(rate) => self.clock.run_at(rate),
            None => {}
        }
    }
}

/// SECT-44: the entry a name stands for: a gated system's name, or an
/// entry's designation.
pub fn sector_entry_named(sector: &Sector, name: &str) -> Option<usize> {
    let gated = sector.gated.iter().find(|g| g.names.system == name);
    gated.map(|g| g.id).or_else(|| {
        let mut entries = sector.entries.iter();
        entries.find(|e| e.designation == name).map(|e| e.id)
    })
}

/// Steps the shot through its phases, one per frame at most.
fn run_shot(
    time: Res<Time>,
    mut shot: ResMut<Shot>,
    mut scene: ShotScene,
    active: Res<ActiveMove>,
    mut rig: ResMut<CameraRig>,
    (tunables, tray): (Res<Tunables>, Res<OpenTray>),
    mut exit: MessageWriter<AppExit>,
) {
    let now = time.elapsed_secs();
    if shot.timed_out(now) {
        let why = format!(
            "gave up after {}s, stuck in {:?}",
            shot.plan.timeout_secs, shot.phase
        );
        shot.fail(&why, &mut exit);
        return;
    }
    match shot.phase {
        ShotPhase::Starting if now >= START_AFTER_SECS => match scene.start(&shot.plan) {
            Ok(next) => shot.phase = next,
            Err(why) => shot.fail(&why, &mut exit),
        },
        ShotPhase::Framing if scene.frame_request.0.is_none() && active.0.is_none() => {
            set_pose(&shot.plan, &mut rig, (&tunables, *scene.sector.layer));
            shot.phase = ShotPhase::Settling { since: now };
        }
        ShotPhase::Settling { since } if now - since >= shot.plan.wait_secs => {
            if let Some(name) = shot.plan.tray.clone().filter(|_| tray.0.is_none()) {
                shot.fail(&format!("no multi-marker holds `{name}`"), &mut exit);
                return;
            }
            scene
                .commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(shot.plan.out.clone()))
                .observe(|_: On<ScreenshotCaptured>, mut shot: ResMut<Shot>| {
                    shot.phase = ShotPhase::Saved;
                });
            shot.phase = ShotPhase::Capturing;
        }
        ShotPhase::Saved => {
            info!("{SHOT_ENV}: saved {}", shot.plan.out.display());
            exit.write(AppExit::Success);
        }
        _ => {}
    }
}

/// `fly`: schedules the planner's order once it can be flown, then, once
/// the ship has its plan, moves the clock on to `fly` days after the ship
/// leaves its orbit, and lets the shot go on to framing.
fn fly_order(
    mut shot: ResMut<Shot>,
    mut planner: ResMut<PlannerState>,
    mut orders: MessageWriter<ShipOrder>,
    mut clock: ResMut<GameClock>,
    selection: Res<Selection>,
    plans: Query<&Plan>,
) {
    match shot.phase {
        ShotPhase::Scheduling => {
            // Once the planner has an order, Schedule is pressed: at
            // Soonest it is solved again from the clock (PLNR-50).
            let now = clock.elapsed_days();
            if order_to_schedule(&planner).is_none() {
                return;
            }
            match schedule_order(&mut planner, now) {
                Some((ship, order, _)) => {
                    orders.write(ShipOrder::Transfer { ship, order });
                    planner.end();
                    shot.phase = ShotPhase::Flying;
                }
                // Solved again, the leg is rejected or over budget: the
                // planner now holds why, and `refuse_unplannable`, next
                // in the chain, ends the shot with it.
                None => shot.phase = ShotPhase::Refused,
            }
        }
        ShotPhase::Flying => {
            let Some(plan) = selection.selected.and_then(|ship| plans.get(ship).ok()) else {
                return;
            };
            let target = plan.leg().leave_day() + shot.plan.fly_days.unwrap_or(0.0);
            let paused = clock.is_paused();
            if paused {
                clock.toggle_pause();
            }
            let days = (target - clock.elapsed_days()).max(0.0);
            let rate = clock.rate_days_per_sec();
            clock.advance(days / rate);
            if paused {
                clock.toggle_pause();
            }
            shot.phase = ShotPhase::Framing;
        }
        _ => {}
    }
}

/// `tray`: once the view is set, ask for the tray of the multi-marker
/// holding the named body, and again if it closes, until the capture.
fn open_tray(
    shot: Res<Shot>,
    names: Query<(Entity, &DisplayName)>,
    multis: Res<MultiMarkers>,
    open: Res<OpenTray>,
    mut request: ResMut<TrayRequest>,
) {
    let (ShotPhase::Settling { .. }, Some(name)) = (shot.phase, shot.plan.tray.as_deref()) else {
        return;
    };
    if open.0.is_some() {
        return;
    }
    let Some(body) = names.iter().find(|(_, n)| n.0 == name).map(|(e, _)| e) else {
        return;
    };
    let own = [MultiKey::Fold(body), MultiKey::Station(body)]
        .into_iter()
        .find(|&key| multis.get(key).is_some());
    let holder = || {
        multis
            .markers
            .iter()
            .find(|m| m.all_things().any(|t| t == body))
            .map(|m| m.key)
    };
    if let Some(key) = own.or_else(holder) {
        request.0 = Some(TrayFor::Marker(key));
    }
}

/// Why the planner can't give the shot its order.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Refusal {
    /// The planner closed before the order was ready.
    Closed,
    /// The destination can't be picked, so nothing is planned.
    NotPickable,
    /// The search or the leg can't be flown, or the leg is over budget;
    /// with the crossing's primary, for the reasons that name it.
    Unplannable {
        why: Infeasible,
        primary: Option<Entity>,
    },
}

/// Why the planner's order can't be scheduled, once the planner has
/// answered: `None` while the pick, the search or the leg is still to
/// come, and when the order can be scheduled.
fn refusal(planner: &PlannerState) -> Option<Refusal> {
    let Some(session) = planner.session.as_ref() else {
        return Some(Refusal::Closed);
    };
    if session.request.is_none() {
        // A pick still waiting in `clicked` is yet to be resolved.
        return planner.clicked.is_none().then_some(Refusal::NotPickable);
    }
    let found = session.found.as_ref()?;
    let available_kms = match (&found.result, &found.context) {
        (Err(_), _) => 0.0,
        // The leg must be solved for the setting the search left.
        (Ok(_), Some(context)) if session.setting.is_some() => {
            if session.solved_for != session.setting {
                return None;
            }
            context.available_kms
        }
        (Ok(_), _) => return None,
    };
    let why = blocking_reason(Some(&found.result), session.leg.as_ref(), available_kms)?;
    Some(Refusal::Unplannable {
        why,
        primary: found.primary,
    })
}

/// `fly`: once the planner has answered, a destination it can't plan for
/// ends the shot at once with the planner's reason, rather than leaving
/// it waiting for the timeout. So does an order Schedule refused when it
/// solved the leg again from the clock (PLNR-50).
fn refuse_unplannable(
    mut shot: ResMut<Shot>,
    planner: Res<PlannerState>,
    names: Query<&DisplayName>,
    parents: Query<&ChildOf>,
    mut exit: MessageWriter<AppExit>,
) {
    let refusal = match (shot.phase, refusal(&planner)) {
        (ShotPhase::Scheduling | ShotPhase::Refused, Some(refusal)) => refusal,
        // Schedule refuses an order only once it has solved the leg again
        // and found it rejected or over budget, which the planner then
        // gives as its reason.
        (ShotPhase::Refused, None) => {
            unreachable!("Schedule refused the order, but the planner holds no reason")
        }
        _ => return,
    };
    let ship = shot.plan.select.clone().unwrap_or_default();
    let target = shot.plan.planner.clone().unwrap_or_default();
    let because = match refusal {
        Refusal::Closed => "the planner closed before its order was ready".to_string(),
        Refusal::NotPickable => format!("`{target}` can't be picked as a destination"),
        Refusal::Unplannable { why, primary } => {
            // The crossing's primary, or else the body the ship orbits,
            // as the planner's panel names it.
            let ship_entity = planner.session.as_ref().map(|s| s.ship);
            let primary = primary
                .or_else(|| {
                    ship_entity
                        .and_then(|s| parents.get(s).ok())
                        .map(ChildOf::parent)
                })
                .and_then(|e| names.get(e).ok())
                .map_or(String::new(), |n| n.0.clone());
            reason(why, &primary, &target)
        }
    };
    let why = format!(
        "can't plan `{ship}` to `{target}` (fly={}): {because}",
        shot.plan.fly_days.unwrap_or(0.0)
    );
    shot.fail(&why, &mut exit);
}

/// Distance, tilt and heading as the plan gives them, each left as it is
/// when not given. Distance and tilt keep the camera's limits; in the
/// sector view the distance is at the sector's scale.
fn set_pose(plan: &ShotPlan, rig: &mut CameraRig, (tunables, layer): (&Tunables, MapLayer)) {
    if let Some(km) = plan.distance_km {
        let render = match layer {
            MapLayer::System => km_to_render(km),
            MapLayer::Sector => sector_km_to_render(km),
        };
        rig.distance = render.clamp(MIN_DISTANCE, MAX_DISTANCE);
    }
    if let Some(deg) = plan.tilt_deg {
        rig.tilt = clamp_tilt(deg.to_radians(), tunables.max_pitch_deg);
    }
    if let Some(deg) = plan.heading_deg {
        rig.heading = deg.to_radians();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_full_spec_parses_every_key() {
        let plan = ShotPlan::parse(
            "out=shots/a.png; frame=Halden IV;select=Halden IV I;hover=Morrow;\
             distance=2e6;tilt=40;heading=-15;rate=0;wait=3.5;planner=Iver;fly=12;\
             tray=Halden IV",
        )
        .unwrap();
        assert_eq!(plan.out, PathBuf::from("shots/a.png"));
        assert_eq!(plan.frame.as_deref(), Some("Halden IV"));
        assert_eq!(plan.select.as_deref(), Some("Halden IV I"));
        assert_eq!(plan.hover.as_deref(), Some("Morrow"));
        assert_eq!(plan.distance_km, Some(2e6));
        assert_eq!(plan.tilt_deg, Some(40.0));
        assert_eq!(plan.heading_deg, Some(-15.0));
        assert_eq!(plan.rate_days_per_sec, Some(0.0));
        assert_eq!(plan.wait_secs, 3.5);
        assert_eq!(plan.planner.as_deref(), Some("Iver"));
        assert_eq!(plan.fly_days, Some(12.0));
        assert_eq!(plan.tray.as_deref(), Some("Halden IV"));
    }

    #[test]
    fn a_shot_that_is_not_saved_by_its_timeout_gives_up() {
        let mut shot = Shot::new(ShotPlan::parse("out=a.png;timeout=5").unwrap());
        assert!(!shot.timed_out(4.9));
        assert!(shot.timed_out(5.0));
        shot.phase = ShotPhase::Saved;
        assert!(!shot.timed_out(10.0));
        shot.phase = ShotPhase::Failed;
        assert!(!shot.timed_out(10.0), "a failed shot has already exited");
        let default = ShotPlan::parse("out=a.png").unwrap();
        assert_eq!(default.timeout_secs, DEFAULT_TIMEOUT_SECS);
    }

    #[test]
    fn only_the_output_path_is_required() {
        let plan = ShotPlan::parse("out=a.png").unwrap();
        assert_eq!(plan.frame, None);
        assert_eq!(plan.wait_secs, DEFAULT_WAIT_SECS);
        assert!(ShotPlan::parse("frame=Morrow").is_err());
    }

    /// SECT-44: `sector` takes a preset or a count, and `seed` a decimal
    /// seed; a bad size or seed, or a seed alone, is an error.
    #[test]
    fn the_sector_and_seed_keys_start_a_sector() {
        let plan = ShotPlan::parse("out=a.png;sector=small;seed=42;select=OW-0003").unwrap();
        let request = plan.sector_request().unwrap().unwrap();
        let small = GeneratorTunables::default().preset_small;
        assert_eq!((request.gated_count, request.seed), (small, Some(42)));
        let counted = ShotPlan::parse("out=a.png;sector=40").unwrap();
        assert_eq!(counted.sector_request().unwrap().unwrap().gated_count, 40);
        assert_eq!(counted.sector_request().unwrap().unwrap().seed, None);
        assert_eq!(
            ShotPlan::parse("out=a.png").unwrap().sector_request(),
            Ok(None)
        );
        for bad in ["sector=huge", "sector=small;seed=x", "seed=4", "sector=1"] {
            assert!(
                ShotPlan::parse(&format!("out=a.png;{bad}")).is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn typos_and_bad_numbers_are_errors() {
        assert!(ShotPlan::parse("out=a.png;tlit=40").is_err());
        assert!(ShotPlan::parse("out=a.png;tilt=steep").is_err());
        assert!(ShotPlan::parse("out=a.png;frame").is_err());
    }

    #[test]
    fn the_pose_keeps_the_camera_limits() {
        let tunables = Tunables::default();
        let mut rig = CameraRig::default();
        let plan = ShotPlan {
            tilt_deg: Some(120.0),
            distance_km: Some(1e30),
            heading_deg: Some(90.0),
            ..ShotPlan::parse("out=a.png").unwrap()
        };
        set_pose(&plan, &mut rig, (&tunables, MapLayer::System));
        assert_eq!(rig.tilt, tunables.max_pitch_deg.to_radians());
        assert_eq!(rig.distance, MAX_DISTANCE);
        assert!((rig.heading - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
    }
}
