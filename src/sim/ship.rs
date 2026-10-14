//! What a ship carries for flying: its drive, its delta-v account and,
//! while it is away from a named orbit, its rail (PLAN-20, PLAN-21,
//! RAIL-09). Plain components in real units; the transfer rules read and
//! update them.

use bevy::prelude::*;

use super::rail::ShipTrajectory;
use super::units::accel_km_per_day2_from_m_per_s2;

/// A ship's drive (PLAN-20): whether it burns in short impulses or pushes
/// gently for the whole trip, its exhaust velocity, and its loaded
/// acceleration in km/day², the unit the rails work in.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Drive {
    /// `true` for a drive planned as two burns (chemical, nuclear thermal,
    /// pulsed); `false` for a low-thrust drive planned as a spiral.
    pub impulsive: bool,
    pub exhaust_velocity_kms: f64,
    /// Loaded acceleration in km/day².
    pub accel: f64,
}

impl Drive {
    /// A drive from the catalogue's figures: the acceleration comes in as
    /// m/s² and is converted once, here (PLAN-20).
    pub fn new(impulsive: bool, exhaust_velocity_kms: f64, accel_m_per_s2: f64) -> Self {
        Self {
            impulsive,
            exhaust_velocity_kms,
            accel: accel_km_per_day2_from_m_per_s2(accel_m_per_s2),
        }
    }
}

/// A ship's delta-v account in km/s (PLAN-20, PLAN-21): what it holds,
/// what scheduled plans have reserved, and what burns have spent.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct DeltaV {
    pub delta_v_kms: f64,
    pub reserved_kms: f64,
    pub spent_kms: f64,
}

impl DeltaV {
    /// A full account with nothing reserved or spent.
    pub fn full(delta_v_kms: f64) -> Self {
        Self {
            delta_v_kms,
            reserved_kms: 0.0,
            spent_kms: 0.0,
        }
    }

    /// PLAN-21: what is left to plan with: the total less what is reserved
    /// and what is spent.
    pub fn available_kms(&self) -> f64 {
        self.delta_v_kms - self.reserved_kms - self.spent_kms
    }

    /// PLAN-21: scheduling a plan sets its delta-v aside.
    pub fn reserve(&mut self, kms: f64) {
        self.reserved_kms += kms;
    }

    /// PLAN-21: cancelling or failing a plan gives back what it reserved.
    pub fn release(&mut self, kms: f64) {
        self.reserved_kms = (self.reserved_kms - kms).max(0.0);
    }

    /// PLAN-21: a burn moves its amount from reserved to spent.
    pub fn burn(&mut self, kms: f64) {
        self.reserved_kms = (self.reserved_kms - kms).max(0.0);
        self.spent_kms += kms;
    }
}

/// What a ship was given by content: its drive and its delta-v.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Propulsion {
    pub drive: Drive,
    pub delta_v_kms: f64,
}

/// RAIL-09: on a ship while it is on a Conic or Spiral segment. A ship
/// without it is in its named orbit and nothing else.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct InFlight;

/// A ship's trajectory while a plan has one running, and afterwards while
/// it coasts (BAND-07). Positions are read from it by `drive_rails`.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct ShipRail(pub ShipTrajectory);

#[cfg(test)]
mod tests {
    use super::*;

    /// PLAN-21: available is total less reserved less spent; reserving,
    /// burning and releasing move the figures as the spec says.
    #[test]
    fn delta_v_account_moves_between_reserved_and_spent() {
        let mut account = DeltaV::full(10.0);
        assert_eq!(account.available_kms(), 10.0);
        account.reserve(4.0);
        assert_eq!(account.available_kms(), 6.0);
        account.burn(1.5);
        assert_eq!((account.reserved_kms, account.spent_kms), (2.5, 1.5));
        assert_eq!(account.available_kms(), 6.0);
        account.release(2.5);
        assert_eq!(account.reserved_kms, 0.0);
        assert_eq!(account.available_kms(), 8.5);
    }

    /// PLAN-20: the catalogue's m/s² becomes km/day² once, in the
    /// constructor: 1 m/s² is 7,464,960 km/day².
    #[test]
    fn drive_acceleration_is_converted_from_m_per_s2() {
        let drive = Drive::new(true, 9.0, 1.0);
        assert!((drive.accel - 7_464_960.0).abs() < 1e-6);
        assert!(drive.impulsive);
        assert_eq!(drive.exhaust_velocity_kms, 9.0);
    }
}
