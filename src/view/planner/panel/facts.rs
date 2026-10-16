//! What the planner's panel reads of the world: the selection, the
//! planner's state, the clock and the ships, places and orbits it names;
//! and the view it works out from them each frame.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::super::model::{
    DestKind, Line, TuningFacts, TuningModel, hover_line, order_model, tuning_model,
};
use super::super::state::{PlannerSession, PlannerState};
use super::super::words::{LegWords, at_place, band_place, coasting_place, place_and_drive};
use super::PanelView;
use crate::sim::body::{BodyClass, DisplayName};
use crate::sim::clock::GameClock;
use crate::sim::named_orbit::{DockingOrbitOf, InNamedOrbit, NamedOrbit};
use crate::sim::orbit::Orbit;
use crate::sim::ship::{DeltaV, Drive, InFlight};
use crate::sim::transfer::phase::{gap_rad, separation_km, together};
use crate::sim::transfer::places::{Band, BandRadii, Place};
use crate::sim::transfer::plan::{Plan, Status};
use crate::sim::transfer::search::Request;
use crate::sim::tunables::SimTunables;
use crate::view::selection::Selection;
use crate::view::tunables::Tunables;

/// What the panel reads of a ship: where it is parked, what it orbits,
/// its drive, its delta-v, its plan, and whether it is in flight.
type ShipView = (
    Option<&'static InNamedOrbit>,
    Option<&'static Orbit>,
    Option<&'static ChildOf>,
    Option<&'static Drive>,
    Option<&'static DeltaV>,
    Option<&'static Plan>,
    Has<InFlight>,
);

/// What the panel reads of the world besides the planner's state.
#[derive(SystemParam)]
pub struct PanelFacts<'w, 's> {
    selection: Res<'w, Selection>,
    state: Res<'w, PlannerState>,
    clock: Res<'w, GameClock>,
    sim_tunables: Res<'w, SimTunables>,
    names: Query<'w, 's, &'static DisplayName>,
    ships: Query<'w, 's, ShipView>,
    orbits: Query<
        'w,
        's,
        (
            &'static NamedOrbit,
            Option<&'static Band>,
            Option<&'static DockingOrbitOf>,
        ),
    >,
    stations: Query<'w, 's, (Entity, &'static BodyClass, &'static InNamedOrbit)>,
    radii: Query<'w, 's, &'static BandRadii>,
}

impl PanelFacts<'_, '_> {
    fn name(&self, entity: Entity) -> String {
        self.names
            .get(entity)
            .map(|n| n.0.clone())
            .unwrap_or_default()
    }

    /// The name a place goes by: its body's, or the station's, gate's,
    /// site's or ship's own.
    pub fn place_name(&self, place: Place) -> String {
        match place {
            Place::Body { body, .. } => self.name(body),
            Place::Station { station } => self.name(station),
            Place::Gate { gate } => self.name(gate),
            Place::Site { site } => self.name(site),
            Place::Ship { ship } => self.name(ship),
            Place::Hold => String::new(),
        }
    }

    fn dest_kind(&self, place: Place) -> DestKind {
        match place {
            Place::Body { body, .. } => DestKind::Body {
                both_bands: self.radii.get(body).is_ok_and(|r| r.high_km.is_some()),
            },
            Place::Station { .. } => DestKind::Station,
            Place::Gate { .. } => DestKind::Gate,
            Place::Site { .. } | Place::Hold => DestKind::Site,
            Place::Ship { .. } => DestKind::Ship,
        }
    }

    /// Where a parked ship is and what its orbit and phase are.
    fn parked(&self, ship: Entity) -> Option<(Entity, Orbit)> {
        let (in_orbit, orbit, ..) = self.ships.get(ship).ok()?;
        Some((in_orbit?.0, *orbit?))
    }

    /// PLNR-20: a parked ship's place: "At *name*" at a station it is
    /// together with or a gate's or site's orbit, else the band, as
    /// "High orbit, Halden IV".
    fn ship_place(&self, ship: Entity) -> String {
        let Some((orbit, at)) = self.parked(ship) else {
            return String::new();
        };
        let Ok((named, band, docking)) = self.orbits.get(orbit) else {
            return String::new();
        };
        if let Some(docking) = docking {
            return at_place(&self.name(docking.0));
        }
        let docked = self.stations.iter().find(|(station, class, in_orbit)| {
            **class == BodyClass::Station
                && in_orbit.0 == orbit
                && self.parked(*station).is_some_and(|(_, there)| {
                    together(
                        at.radius_km,
                        gap_rad(at.phase0_rad, there.phase0_rad),
                        self.sim_tunables.together_km,
                    )
                })
        });
        match (docked, band) {
            (Some((station, ..)), _) => at_place(&self.name(station)),
            (None, Some(band)) => band_place(*band, &self.name(named.parent)),
            (None, None) => format!("{} orbit, {}", named.name, self.name(named.parent)),
        }
    }

    /// PLNR-41: for a station or ship in the ship's own orbit and not
    /// together with it, the distance to it and whether it is ahead.
    fn gap_km(&self, ship: Entity, place: Place) -> Option<(f64, bool)> {
        let target = match place {
            Place::Station { station } => station,
            Place::Ship { ship } => ship,
            _ => return None,
        };
        let (orbit, at) = self.parked(ship)?;
        let (target_orbit, there) = self.parked(target)?;
        if orbit != target_orbit {
            return None;
        }
        let gap = gap_rad(at.phase0_rad, there.phase0_rad);
        let km = separation_km(at.radius_km, gap);
        (km >= self.sim_tunables.together_km).then_some((km, gap <= std::f64::consts::PI))
    }

    fn available_kms(&self, ship: Entity) -> f64 {
        self.ships
            .get(ship)
            .ok()
            .and_then(|(.., account, _, _)| account)
            .map_or(0.0, DeltaV::available_kms)
    }

    /// What the panel shows this frame.
    pub fn view(&self, tunables: &Tunables) -> PanelView {
        if let Some(session) = &self.state.session {
            return match session.request {
                None => PanelView::Choosing(self.hover_line(session.ship, tunables)),
                Some(request) => {
                    PanelView::Tuning(Box::new(self.tuning(session, request, tunables)))
                }
            };
        }
        let Some(ship) = self.selection.selected else {
            return PanelView::Empty;
        };
        self.order_view(ship)
    }

    /// PLNR-12: the hovered place's line.
    fn hover_line(&self, ship: Entity, tunables: &Tunables) -> Option<Line> {
        let preview = self.state.session.as_ref()?.hover.as_ref()?;
        let destination = self.place_name(preview.place);
        let words = LegWords {
            intercept: false,
            to_ship: matches!(preview.place, Place::Ship { .. }),
            destination: &destination,
        };
        let gap = self.gap_km(ship, preview.place).map(|(km, ahead)| {
            super::super::words::gap_line(km, ahead, tunables.distance_au_threshold_au)
        });
        Some(hover_line(
            preview,
            &words,
            &self.primary_name(ship, None),
            gap,
        ))
    }

    /// The crossing's primary's name, or the body the ship orbits.
    fn primary_name(&self, ship: Entity, primary: Option<Entity>) -> String {
        primary
            .or_else(|| {
                let (.., child_of, _, _, _, _) = self.ships.get(ship).ok()?;
                child_of.map(ChildOf::parent)
            })
            .map(|e| self.name(e))
            .unwrap_or_default()
    }

    /// PLNR-20 to PLNR-32, PLNR-40, PLNR-41: the tuning panel for the
    /// open `session`, planning `request`.
    fn tuning(
        &self,
        session: &PlannerSession,
        request: Request,
        tunables: &Tunables,
    ) -> TuningModel {
        let ship = session.ship;
        let destination = self.place_name(request.destination);
        let drive = self
            .ships
            .get(ship)
            .ok()
            .and_then(|(.., drive, _, _, _)| drive)
            .map_or("", |d| d.name);
        let place = place_and_drive(&self.ship_place(ship), drive);
        let found = session.found.as_ref();
        let available_kms = found
            .and_then(|f| f.context.as_ref())
            .map_or(self.available_kms(ship), |c| c.available_kms);
        let primary = self.primary_name(ship, found.and_then(|f| f.primary));
        let ship_name = self.name(ship);
        let facts = TuningFacts {
            ship: &ship_name,
            place: &place,
            destination: &destination,
            kind: self.dest_kind(request.destination),
            primary: &primary,
            request,
            setting: session.setting,
            found: found.map(|f| &f.result),
            leg: session.leg.as_ref(),
            travel_track: &session.travel_track,
            available_kms,
            today: self.clock.elapsed_days(),
            gap_km: self.gap_km(ship, request.destination),
            window_margin_kms: self.sim_tunables.window_margin_kms,
        };
        tuning_model(&facts, tunables)
    }

    /// PLNR-51 / PLNR-54: the selected ship's order, or its coasting
    /// place, while the planner is closed.
    fn order_view(&self, ship: Entity) -> PanelView {
        let Ok((.., Some(plan), in_flight)) = self.ships.get(ship) else {
            return PanelView::Empty;
        };
        let leg = plan.leg();
        let destination = self.place_name(leg.destination);
        let intercept = leg.arrival.is_intercept();
        match plan.status {
            Status::Scheduled | Status::InFlight => {
                let model = order_model(
                    &LegWords::of_leg(leg, &destination),
                    leg,
                    plan.status == Status::Scheduled,
                    self.clock.elapsed_days(),
                );
                PanelView::Order(model, ship)
            }
            Status::Done if intercept && in_flight => {
                let origin = leg.origin.map(|o| self.place_name(o)).unwrap_or_default();
                PanelView::Coasting(coasting_place(&destination, &origin))
            }
            _ => PanelView::Empty,
        }
    }
}
