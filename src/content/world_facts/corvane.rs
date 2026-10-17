//! Corvane's world facts (SURF-62) and its star's (SURF-60): a K2V star
//! of 5.1 Gyr whose luminosity follows from its mass. Pell is a dark,
//! baked, locked rock; Iver is locked; Halden IV, the capital, is a cold,
//! thin-aired, icy world that is still active; Morrow is a Jupiter-class
//! giant whose innermost moons are an Io (I) and a Europa (II), heated by
//! its tides.

use super::{AuthoredWorld, world};
use crate::sim::planet_facts::Activity::{Active, Dead, Extreme};
use crate::sim::planet_facts::Atmosphere::{Airless, Envelope, Thin};
use crate::sim::planet_facts::Water::{Ice, None as Dry};
use crate::sim::planet_facts::WorldKind::{GasGiant, Moon, Rocky};

/// Corvane's spectral type.
pub const SPECTRAL_TYPE: &str = "K2V";
/// Corvane's age, in Gyr.
pub const AGE_GYR: f64 = 5.1;

/// Table 1 of the surfaces spec: every planet and moon of Corvane, with
/// its kind, (atmosphere, water), (locked, activity) and surface seed.
#[rustfmt::skip]
pub const WORLDS: [AuthoredWorld; 19] = [
    world("Pell",        Rocky,    (Airless, Dry),  (true, Some(Dead)),     0x7e11_0001),
    world("Iver",        Rocky,    (Thin, Ice),     (true, Some(Dead)),     0x7e11_0002),
    world("Halden IV",   Rocky,    (Thin, Ice),     (false, Some(Active)),  0x7e11_0003),
    world("Halden IVa",  Moon,     (Airless, Dry),  (true, Some(Dead)),     0x7e11_0004),
    world("Halden IVb",  Moon,     (Airless, Dry),  (true, Some(Dead)),     0x7e11_0005),
    world("Halden IVc",  Moon,     (Airless, Ice),  (true, Some(Dead)),     0x7e11_0006),
    world("Halden IVd",  Moon,     (Airless, Dry),  (true, Some(Dead)),     0x7e11_0007),
    world("Morrow",      GasGiant, (Envelope, Dry), (false, None),          0x7e11_0008),
    world("Morrow I",    Moon,     (Airless, Dry),  (true, Some(Extreme)),  0x7e11_0009),
    world("Morrow II",   Moon,     (Airless, Dry),  (true, Some(Active)),   0x7e11_000a),
    world("Morrow III",  Moon,     (Airless, Dry),  (true, Some(Dead)),     0x7e11_000b),
    world("Morrow IV",   Moon,     (Airless, Dry),  (true, Some(Dead)),     0x7e11_000c),
    world("Morrow V",    Moon,     (Airless, Ice),  (true, Some(Dead)),     0x7e11_000d),
    world("Morrow VI",   Moon,     (Airless, Ice),  (true, Some(Dead)),     0x7e11_000e),
    world("Morrow VII",  Moon,     (Airless, Dry),  (true, Some(Dead)),     0x7e11_000f),
    world("Morrow VIII", Moon,     (Airless, Dry),  (true, Some(Dead)),     0x7e11_0010),
    world("Morrow IX",   Moon,     (Airless, Ice),  (true, Some(Dead)),     0x7e11_0011),
    world("Morrow X",    Moon,     (Airless, Dry),  (true, Some(Dead)),     0x7e11_0012),
    world("Morrow XI",   Moon,     (Airless, Dry),  (true, Some(Dead)),     0x7e11_0013),
];
