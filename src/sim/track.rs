//! What the faction knows of a ship or station (SYM-01): its affiliation,
//! whether the faction commands it, its class, whether it is armed, a troop
//! transport's load, how firm its track is, and an own ship's condition.
//! Plain data with no presentation in it; the view draws the symbol from
//! these values only.

use bevy::prelude::*;

/// How the faction sees an object (SYM-01). Every body has one; for planets
/// and moons it is simply whose they are.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Affiliation {
    Own,
    Allied,
    Neutral,
    Hostile,
    Unknown,
}

/// What kind of ship an object is known to be (SYM-01).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShipClass {
    Warship,
    FleetSupply,
    Cargo,
    TroopTransport,
    Station,
    Unknown,
}

/// The size of an embarked formation, where known (SYM-06).
#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub enum Echelon {
    Battalion,
    Regiment,
    Brigade,
}

/// A formation embarked on a troop transport.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Formation {
    pub name: &'static str,
    pub echelon: Option<Echelon>,
}

/// What is known of a troop transport's load (SYM-06).
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TroopLoad {
    Unknown,
    Empty,
    Embarked(Vec<Formation>),
}

/// How firm the faction's track on an object is (SYM-08). Days are game
/// days since the campaign epoch, negative for before it.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Track {
    Firm,
    Weak,
    Stale { last_observed_day: f64 },
}

/// SYM-01: what the faction knows of a ship or station.
#[derive(Component, Clone, PartialEq, Debug)]
pub struct ShipRecord {
    /// Whether the faction gives it orders. An own ship that isn't commanded
    /// is an own-flag civilian.
    pub commanded: bool,
    /// An own-flag civilian the faction has taken command of (requisition or
    /// charter). Implies `commanded`.
    pub requisitioned: bool,
    pub class: ShipClass,
    /// Known to be armed: drawn as a warship whatever its class (SYM-05).
    pub armed: bool,
    /// For a troop transport, what is known of its load.
    pub load: TroopLoad,
    pub track: Track,
    /// For own ships, a condition from 0 to 1.
    pub condition: Option<f64>,
}

impl ShipRecord {
    /// A firm-tracked, unarmed record of `class`, commanded or not, with
    /// nothing else known: the starting point content adjusts.
    pub fn new(class: ShipClass, commanded: bool) -> Self {
        Self {
            commanded,
            requisitioned: false,
            class,
            armed: class == ShipClass::Warship,
            load: TroopLoad::Unknown,
            track: Track::Firm,
            condition: None,
        }
    }
}
