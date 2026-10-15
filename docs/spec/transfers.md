# Spec: Transfer simulation

**In short:** ships fly on rails: a trajectory is a list of segments (parked, conic, spiral), each evaluated analytically from the clock. A transfer between two places is one crossing round a primary, with the planet-centred legs collapsed to a delta-v cost and a few days at each end. Impulsive drives get a two-burn Lambert arc; low-thrust drives get an Edelbaum spiral of fixed length. Bodies never bend a coasting ship. A destination is a place (a band at a body, a station, a gate, a site, a ship), never a bare orbit; ships in one band are co-located for every rule. Orders are the save-game truth and trajectories are recomputed from them.

*Scope:* the simulation of transfers: the trajectory model, the solver, places and bands, orders and their data. No UI: the planner, sliders, previews, alerts, the return reserve, multi-leg plans, groups, gates' entry rule, re-tasking in flight, gravity assists and tactical rules are not specced here (see *Not yet specced*). *Source ideas:* `docs/ideas/mechanics/transfers.md` sections 2, 4 and 8, with figures from `docs/ideas/mechanics/drives-and-transit.md` sections 2 and 3. **Built** is ✓ once a requirement is in the code.

*Retired IDs* (never reuse): none.

---

## 1. Rails (RAIL)

A ship's position and velocity at any day come from its trajectory, never from integration.

| ID | Requirement | Built |
|---|---|---|
| RAIL-01 | A `Trajectory` is an ordered list of segments. Each `Segment` has `start_day`, `end_day` (which may be unbounded) and a `kind`. Segments are contiguous: each starts on the day the previous one ends, and the last segment is unbounded. | ✓ |
| RAIL-02 | Segment kinds: **Parked** `{ orbit }` (a reference to a named orbit, SCL-07: position and velocity are the orbit's at the ship's phase); **Conic** `{ primary, elements }` (an ellipse or hyperbola round the primary); **Spiral** `{ primary, r0, r1, θ0, θ1, v0, accel }` (a low-thrust crossing, RAIL-07). | ✓ |
| RAIL-03 | Position and velocity are stored and returned in km and km/day, `f64`, relative to the segment's primary (SCL-01, ARCH-10). Delta-v budgets and costs are in km/s; one named function converts between km/day and km/s. | ✓ |
| RAIL-04 | A ship's state on day *d* is read from the one segment whose span contains *d* (a segment owns its start day, not its end day). Evaluating a trajectory for any day in its span is a pure function of the trajectory and the day (ARCH-20). | ✓ |
| RAIL-05 | **Conic** elements are the orbital plane's (XZ, the plane every body orbits in, SCL-04): semi-latus rectum *p* (km), eccentricity *e*, argument of periapsis ω (rad, measured in the sense of motion), time of periapsis *t_p* (days), and the sense of motion (prograde or retrograde). Position and velocity on any day come from Kepler's equation in its elliptic (e < 1) or hyperbolic (e > 1) form, or a universal-variable form that covers both. A conic built from a state has *e* at least `parabola_epsilon` away from 1 (the state's *e* is nudged to the nearer side), so there is no parabolic branch. | ✓ |
| RAIL-06 | A conic is built from a state (position, velocity, day) and the primary's gravitational parameter, and a state read back from it on that day reproduces the state to within 1 km and 1 km/day. Holds for ellipses and hyperbolae in both senses of motion. | ✓ |
| RAIL-07 | **Spiral** evaluation: with *s* = days since `start_day`, the speed is *v(s)* = *v0* ∓ `accel`·*s* (minus when climbing to a higher orbit, plus when descending) and the radius is *r(s)* = μ / *v(s)*²; the angle is θ(s) = θ0 + ∫₀ˢ √(μ/r³) ds + *k*·*s*, where *k* is the one constant that makes θ(end) = θ1 (so the spiral winds at the local circular rate and lands where the target is). The velocity is the curve's time derivative. The integral is evaluated in closed form or by a fixed quadrature: no per-frame integration. | ✓ |
| RAIL-08 | A coasting ship keeps its conic: no body other than the segment's primary changes a Conic or Spiral segment, however close the path comes to it. Passing through a body's Hill sphere changes nothing in the simulation. | ✓ |
| RAIL-09 | A ship on a Conic or Spiral segment is **in flight**; a ship on a Parked segment is a member of its named orbit (SCL-07) and nothing else. | ✓ |

## 2. Places and bands (BAND)

A destination is a place. A place resolves to one named orbit (SCL-07) and a body that charges for leaving and arriving.

| ID | Requirement | Built |
|---|---|---|
| BAND-01 | Every star, planet and moon has two bands, which are named orbits (SCL-07): **Low** at radius = mean radius (SCL-05) + `low_band_altitude_km`, and **High** at radius = min(`high_band_radii` × mean radius, `high_band_hill_fraction` × Hill radius), where the Hill radius is *a* (*m* / 3*M*)^⅓ for the body's orbit radius *a*, mass *m* and parent mass *M*. The star has no Hill radius, so its High band is `high_band_radii` × its radius. A body whose High radius would not exceed its Low radius has no High band. | ✓ |
| BAND-02 | A place is one of: **Body** `{ body, band }`, the band being Low or High (a band of a star, planet or moon); **Station** `{ station }` (the band the station occupies, SCL-07); **Gate** `{ gate }` (the gate's own orbit); **Site** `{ site }` (a belt or salvage site's own orbit); **Ship** `{ ship }` (wherever the ship is on the arrival day); **Hold** (no destination: stop burning, PLAN-13). | |
| BAND-03 | The **departure cost** of a place in a named orbit of radius *r* round a body of gravitational parameter μ is *v_circ*(√2 − 1) = √(μ/r)(√2 − 1), in km/s: the cost of escaping a circular orbit to rest at infinity. The **capture cost** of arriving at a place is the same figure. A place whose orbit has no massive body (a gate, a site, a ship in flight) costs 0. | ✓ |
| BAND-04 | The **approach days** of a place are (Hill radius − *r*) ÷ √(2μ/*r*), converted to days: the time to coast from the orbit to the body's Hill radius at the orbit's escape speed. Zero for a place with no massive body. Both the departure and the arrival end of a crossing add their place's approach days (XFER-02). | ✓ |
| BAND-05 | Ships in one named orbit are co-located for every rule: no rule reads a ship's phase along the orbit. Phase exists only so ships draw as separate dots. | ✓ |
| BAND-06 | A ship that captures into a named orbit takes the phase of its arrival direction: the angle, in the orbital plane, of its velocity relative to the orbit's body on the arrival day. A ship placed by content or a generator takes the phase it is given. | ✓ |
| BAND-07 | A ship that is not in a named orbit is **coasting**: its state is its rail's, and its place is described by what it last did (the plan that ended there, PLAN-12). Nothing can be ordered from a coasting state except a new plan starting from it. | ✓ |
| BAND-08 | A coasting ship on a conic round the star with *e* ≥ 1, or a bound conic whose apoapsis exceeds `lost_beyond_au`, is **escaping**. An escaping ship with no plan is removed from the simulation when its distance from the star first exceeds `lost_beyond_au`; what the game says about it first is for the alerts spec. | ✓ |

## 3. Transfers (XFER)

A transfer is a crossing from a start state to a target place, solved once when ordered and never adjusted.

| ID | Requirement | Built |
|---|---|---|
| XFER-01 | The **primary** of a crossing is the lowest common ancestor of the departure place's body and the destination's body in the system tree (SCL-03): the star for planet-to-planet, the planet for band-to-band, moon-to-moon and station-to-moon trips round it. The crossing's start state is that of the departure place's highest ancestor below the primary on the crossing's start day (a ship leaving a moon for another planet starts from its planet's state), and its end state is the destination's highest ancestor below the primary on the crossing's end day. A trip between two bands of the same body has the body as primary, no departure or capture cost, and no approach days. | ✓ |
| XFER-02 | A leg's timing, in days: **crossing start** = `depart_day` + departure approach days (BAND-04) + burn days (XFER-05); **crossing end** = crossing start + `travel_days`; **arrival** = crossing end + arrival approach days. The ship's rail is Parked in its departure orbit until the crossing start, then the crossing segment, then (for a Stop) Parked in the destination orbit from the crossing end. The plan is InFlight from `depart_day` to arrival (PLAN-10). | ✓ |
| XFER-03 | **Impulsive crossing.** For a drive with `impulsive = true`, the crossing is the prograde, zero-revolution Lambert arc from the start state's position at the crossing start to the end state's position at the crossing end, with transfer angle in (0°, 360°). Burn 1 = the magnitude of (arc start velocity − start state velocity); burn 2 = the magnitude of (end state velocity − arc end velocity), in km/s. | ✓ |
| XFER-04 | **Leg delta-v.** Stop: departure cost + burn 1 + burn 2 + capture cost. Pass: departure cost + burn 1. A leg whose delta-v exceeds the ship's available delta-v (PLAN-21) is infeasible. | ✓ |
| XFER-05 | **Burn days** for an impulsive drive = (departure cost + burn 1) ÷ `accel`, in days, computed from a solve with zero burn days; the crossing is then solved once more from the later start, and that second solve's burns are the leg's. A low-thrust drive has zero burn days (its costs are in the spiral, XFER-06). | ✓ |
| XFER-06 | **Low-thrust crossing.** For a drive with `impulsive = false`, the crossing is a Spiral (RAIL-07) between the start and end states' circular orbits round the primary. Its delta-v = departure cost + *Δv_c* + capture cost (Stop) or departure cost + *Δv_c* (Pass), where *Δv_c* is the magnitude of (*v_circ*(r0) − *v_circ*(r1)), and `travel_days` = that delta-v ÷ `accel`, in days: fixed by the drive, not chosen. The spiral's *v0* is the start orbit's circular speed and it ends at the end state's angle on the crossing end day. A low-thrust crossing is only solved between circular start and end states; from a coasting state it is infeasible. | ✓ |
| XFER-07 | A crossing whose conic passes within `min_periapsis_radii` × the primary's mean radius of the primary (periapsis inside that radius, reached during the crossing) is infeasible. | ✓ |
| XFER-08 | **Arrival modes.** **Stop** captures: burn 2 is made, the capture cost paid, and the ship joins the destination's named orbit (BAND-06). **Pass** `{ band }` skips both: the crossing segment is extended without end and the ship coasts (BAND-07). The pass band is High: a Pass through Low needs the flyby turn and is not specced. A Pass has no arrival approach days. | ✓ |
| XFER-09 | **Relative speed at a Pass** = √(*v∞*² + 2μ/*r*), where *v∞* = the magnitude of (arc end velocity − body velocity) and *r* is the pass band's radius; reported in km/s. | ✓ |
| XFER-10 | **Ship as destination.** The end state is the target ship's rail state on the crossing end day (its primary must be the crossing's primary, else infeasible). Stop matches the target's velocity (burn 2) and the ship then shares the target's segment: its named orbit if the target is parked, a copy of its conic if the target is coasting. Pass closes at the relative speed, the magnitude of (arc end velocity − target velocity). | ✓ |
| XFER-11 | The maths (conics, Lambert, the spiral, trajectory evaluation) is the `orbits` crate (ARCH-40). The game's rules are pure functions in `src/sim/transfer/` (`places.rs` for bands and costs, `legs.rs` for timing and delta-v, `plan.rs` for orders): states, places, a drive and a budget in, a leg with its trajectory and delta-v out, or an infeasibility reason. Nothing in it reads Bevy resources or knows about the view (ARCH-02, ARCH-20). *Checked in review.* | ✓ |
| XFER-12 | **Worked checks (must hold within 3%), in plain figures, as `orbits` crate tests:** a star of 0.75 solar masses; a planet of radius 5,753 km and surface gravity 0.82 g at 0.8 AU; an inner planet at 0.5 AU; a belt at 0.64 AU (the reference layout of `drives-and-transit.md` section 2, which is not the content's Corvane). The planet's Low band (500 km up) costs about 2.7 km/s to leave and its High band (10 radii) about 0.9 km/s; the Hohmann crossing 0.5 → 0.8 AU has burn 1 + burn 2 of about 7.5 km/s and takes about 111 days; 0.8 → 0.64 AU about 3.4 km/s and 129 days. | ✓ |

## 4. Orders and status (PLAN)

Orders are the truth; a trajectory is derived from its order and can be rebuilt from it.

| ID | Requirement | Built |
|---|---|---|
| PLAN-01 | A `Plan { ship, legs, status }` holds one or more `Leg { destination, arrival, depart_day, travel_days, trajectory }`, where `arrival` is Stop or Pass `{ band }`. A plain transfer is a one-leg plan. Only one-leg plans are specced; a plan with more legs is rejected until multi-leg plans are. | ✓ |
| PLAN-02 | `status` is one of **Scheduled**, **InFlight**, **Holding**, **Done**, **Failed**. A ship has at most one plan that is Scheduled, InFlight or Holding. | ✓ |
| PLAN-03 | A leg's `trajectory` is computed when the plan is made (XFER-02) and stored on the leg. Rebuilding every trajectory from its order (ship, destination, arrival mode, `depart_day`, `travel_days`) with the same content gives the same trajectory, so a save holds orders and ships' states, not positions. | ✓ |
| PLAN-10 | **Transitions.** Scheduled → InFlight on `depart_day`; InFlight → Done on the arrival day (Stop) or the crossing end day (Pass, after which the ship is coasting). A Hold leg (PLAN-13) is Done at once. | ✓ |
| PLAN-11 | On `depart_day` the leg is re-validated: the ship must be in the departure orbit the plan assumed and have at least the leg's delta-v available (PLAN-21). If not, the plan is Failed and the ship is unchanged. | ✓ |
| PLAN-12 | A plan's last leg records, for display, the ship's **origin place**, **destination** and **arrival mode**, so a coasting ship's place can be described from its plan ("passed Halden IV, from Iver"). Text is the view's. | ✓ |
| PLAN-13 | **Hold** as a destination: the ship's current segment is extended without end and all later burns are dropped. For a parked ship it does nothing. | ✓ |
| PLAN-20 | A ship holds `delta_v_kms`, `reserved_kms` and `spent_kms`. A drive holds `impulsive`, `exhaust_velocity_kms` and `accel` (loaded acceleration, stored in km/day², set from the catalogue's m/s² by one conversion). | ✓ |
| PLAN-21 | **Available** delta-v = `delta_v_kms` − `reserved_kms` − `spent_kms`. Scheduling a plan reserves the leg's delta-v. A burn moves its amount from reserved to spent on the day it is made: departure cost + burn 1 at the crossing start, burn 2 + capture cost at the crossing end (Stop only); a spiral moves its whole delta-v at the crossing start. Cancelling or failing a plan releases what it reserved. | ✓ |
| PLAN-22 | Cancelling a Scheduled plan leaves the ship as it was. Cancelling an InFlight plan is not specced (re-tasking). | ✓ |
| PLAN-30 | Per body record: the derived Low and High band radii (BAND-01). Per gate: `entry_tolerance_kms`, stored but not yet read by any rule. | ✓ |

## 5. Test content (TEST)

| ID | Requirement | Built |
|---|---|---|
| TEST-20 | Corvane's "High" orbit of Halden IV (TEST-10) is the High band of BAND-01, and every star, planet and moon in Corvane carries its two bands. The parked warship of TEST-03 holds a hydrogen NTR drive (9.0 km/s exhaust, 0.2 g, impulsive) with 9.9 km/s of delta-v; the own fleet tanker of TEST-05 holds an NEP ion/Hall drive (35 km/s, 0.048 mg, low-thrust) with 24 km/s. | ✓ |

## Tunables

Simulation tunables live in a `SimTunables` resource (`src/sim/tunables.rs`), separate from the view's `Tunables` (ARCH-22). Defaults are starting values.

| Tunable | Default | Used by |
|---|---|---|
| `low_band_altitude_km` | 500 km | BAND-01 |
| `high_band_radii` | 10 | BAND-01 |
| `high_band_hill_fraction` | 0.5 | BAND-01 |
| `lost_beyond_au` | 6 AU | BAND-08 |
| `min_periapsis_radii` | 2 | XFER-07 |
| `parabola_epsilon` | 1e-6 | RAIL-05 |

## Not yet specced

From `transfers.md`, not to be built until specced: the planner and its sliders, presets, previews, ghosts and map drawing (section 3); the return reserve and one-way orders (3.6); multi-leg plans, round trips, refuelling stops and the fields `refuel`, `reserve_kms`, `refuel_days`, `return_window_days` (3.7); other people's ships (3.4); alerts on departure, arrival, crossings and escape (3.5, 4.1); gravity assists and Pass through Low (2.6, 7); groups and the gate entry rule (5); re-tasking in flight, aborts and cancelling an InFlight plan (6); tactical manoeuvring beyond the arrival modes above (7); eccentric flavour bodies (2.5).
