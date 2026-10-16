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
//! - `planner`: opens the transfer planner for the selected ship: `open`
//!   leaves it choosing; a body's name picks that body as the destination,
//!   as a click on it would (PLNR-02, PLNR-11).
//! - `fly`: with `planner` naming a destination, schedules the planner's
//!   order as Schedule would (PLNR-50), then moves the clock on to this
//!   many days after the ship leaves its orbit (PLNR-21), so the shot
//!   shows the ship in flight. Negative days stop short of the leave day.
//!
//! Names are the bodies' display names. The app starts, waits a moment for
//! the world to load, applies the selection, the rate and the frame, waits
//! for the frame move to end, sets distance, tilt and heading, waits
//! `wait` seconds, saves the screenshot and exits. A name that matches no
//! body is logged and the app exits with an error.

use std::path::PathBuf;

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk};

use super::camera::requests::FrameRequest;
use super::camera::rules::clamp_tilt;
use super::camera::state::{ActiveMove, CameraRig, MAX_DISTANCE, MIN_DISTANCE};
use super::countdown::leave_day;
use super::planner::input::order_to_schedule;
use super::planner_state::{PickTarget, PlannerState};
use super::scale::km_to_render;
use super::selection::Selection;
use super::targets::{Hovered, MarkerFor};
use super::tunables::Tunables;
use crate::Stage;
use crate::sim::body::DisplayName;
use crate::sim::clock::GameClock;
use crate::sim::transfer::plan::{Plan, ShipOrder};

/// The environment variable that asks for a screenshot.
pub const SHOT_ENV: &str = "OW_SHOT";

/// Seconds after start before the view is set up, so the world, fonts and
/// the first layout are in place.
const START_AFTER_SECS: f32 = 1.0;
/// Seconds the view settles before the capture, unless `wait` says.
const DEFAULT_WAIT_SECS: f32 = 2.0;

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
    /// `open`, or the name of a destination to pick.
    pub planner: Option<String>,
    /// Days after the leave day to move the clock to, once the planner's
    /// order is scheduled.
    pub fly_days: Option<f64>,
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
            planner: None,
            fly_days: None,
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
                "planner" => plan.planner = Some(value.to_string()),
                "fly" => plan.fly_days = Some(number(key, value)?),
                _ => return Err(format!("unknown key `{key}`")),
            }
        }
        if plan.out.as_os_str().is_empty() {
            return Err("`out` (the image path) is required".to_string());
        }
        Ok(plan)
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
}

pub struct DebugShotPlugin;

impl Plugin for DebugShotPlugin {
    fn build(&self, app: &mut App) {
        if let Ok(spec) = std::env::var(SHOT_ENV) {
            let plan = ShotPlan::parse(&spec).unwrap_or_else(|e| panic!("{SHOT_ENV}: {e}"));
            app.insert_resource(Shot::new(plan));
        }
        app.add_systems(
            Update,
            (fly_order, run_shot)
                .chain()
                .run_if(resource_exists::<Shot>)
                .after(Stage::Simulation)
                .before(Stage::Camera),
        );
    }
}

/// Steps the shot through its phases, one per frame at most.
#[allow(clippy::too_many_arguments)]
fn run_shot(
    mut commands: Commands,
    time: Res<Time>,
    mut shot: ResMut<Shot>,
    names: Query<(Entity, &DisplayName)>,
    markers: Query<(Entity, &MarkerFor)>,
    mut selection: ResMut<Selection>,
    mut planner: ResMut<PlannerState>,
    mut clock: ResMut<GameClock>,
    mut frame_request: ResMut<FrameRequest>,
    active: Res<ActiveMove>,
    mut rig: ResMut<CameraRig>,
    tunables: Res<Tunables>,
    mut exit: MessageWriter<AppExit>,
) {
    let now = time.elapsed_secs();
    let find = |name: &str| {
        names
            .iter()
            .find(|(_, display)| display.0 == name)
            .map(|(entity, _)| entity)
    };
    match shot.phase {
        ShotPhase::Starting if now >= START_AFTER_SECS => {
            let plan = &shot.plan;
            let destination = plan.planner.as_ref().filter(|p| *p != PLANNER_OPEN);
            let wanted = [&plan.frame, &plan.select, &plan.hover]
                .into_iter()
                .flatten()
                .chain(destination);
            if let Some(missing) = wanted.into_iter().find(|n| find(n).is_none()) {
                error!("{SHOT_ENV}: no body is named `{missing}`");
                exit.write(AppExit::error());
                return;
            }
            if let Some(body) = plan.select.as_deref().and_then(find) {
                selection.select(body);
                if plan.planner.is_some() {
                    planner.open(body);
                }
                if let Some(place) = destination.and_then(|name| find(name)) {
                    planner.pick(PickTarget::Body(place));
                }
            }
            if let Some(body) = plan.hover.as_deref().and_then(find) {
                for (marker, _) in markers.iter().filter(|(_, m)| m.0 == body) {
                    commands.entity(marker).insert(Hovered);
                }
            }
            match plan.rate_days_per_sec {
                Some(rate) if rate <= 0.0 => {
                    if !clock.is_paused() {
                        clock.toggle_pause();
                    }
                }
                Some(rate) => clock.run_at(rate),
                None => {}
            }
            frame_request.0 = plan.frame.as_deref().and_then(find);
            shot.phase = if plan.fly_days.is_some() && destination.is_some() {
                ShotPhase::Scheduling
            } else {
                ShotPhase::Framing
            };
        }
        ShotPhase::Framing if frame_request.0.is_none() && active.0.is_none() => {
            set_pose(&shot.plan, &mut rig, &tunables);
            shot.phase = ShotPhase::Settling { since: now };
        }
        ShotPhase::Settling { since } if now - since >= shot.plan.wait_secs => {
            commands
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
            if let Some((ship, order, _)) = order_to_schedule(&planner) {
                orders.write(ShipOrder::Transfer { ship, order });
                planner.end();
                shot.phase = ShotPhase::Flying;
            }
        }
        ShotPhase::Flying => {
            let Some(plan) = selection.selected.and_then(|ship| plans.get(ship).ok()) else {
                return;
            };
            let target = leave_day(plan.leg()) + shot.plan.fly_days.unwrap_or(0.0);
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

/// Distance, tilt and heading as the plan gives them, each left as it is
/// when not given. Distance and tilt keep the camera's limits.
fn set_pose(plan: &ShotPlan, rig: &mut CameraRig, tunables: &Tunables) {
    if let Some(km) = plan.distance_km {
        rig.distance = km_to_render(km).clamp(MIN_DISTANCE, MAX_DISTANCE);
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
             distance=2e6;tilt=40;heading=-15;rate=0;wait=3.5;planner=Iver;fly=12",
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
    }

    #[test]
    fn only_the_output_path_is_required() {
        let plan = ShotPlan::parse("out=a.png").unwrap();
        assert_eq!(plan.frame, None);
        assert_eq!(plan.wait_secs, DEFAULT_WAIT_SECS);
        assert!(ShotPlan::parse("frame=Morrow").is_err());
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
        set_pose(&plan, &mut rig, &tunables);
        assert_eq!(rig.tilt, tunables.max_pitch_deg.to_radians());
        assert_eq!(rig.distance, MAX_DISTANCE);
        assert!((rig.heading - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
    }
}
