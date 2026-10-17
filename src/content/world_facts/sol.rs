//! Sol's world facts (SURF-63) and its star's (SURF-60): real values,
//! within the facts' vocabulary, round a G2V Sun of 1.0 solar luminosity
//! and 4.6 Gyr.
//!
//! Mercury and the Moon count as dry: their polar ice is too little to
//! name. Mercury and Venus count as locked, since spin-orbit resonances
//! and slow retrograde spins have no fact. Earth's facts are its real
//! ones, but it is drawn with the lava look, as the world the collapse
//! left.

use super::{AuthoredWorld, world};
use crate::sim::planet_facts::Activity::{Active, Dead, Extreme};
use crate::sim::planet_facts::Atmosphere::{Airless, Dense, Envelope, Thin};
use crate::sim::planet_facts::LookOverride;
use crate::sim::planet_facts::Water::{Ice, None as Dry, Surface};
use crate::sim::planet_facts::WorldKind::{GasGiant, IceGiant, Moon, Rocky};

/// The Sun's spectral type.
pub const SPECTRAL_TYPE: &str = "G2V";
/// The Sun's luminosity, in solar luminosities.
pub const LUMINOSITY_LSUN: f64 = 1.0;
/// The Sun's age, in Gyr.
pub const AGE_GYR: f64 = 4.6;

/// Earth: its real facts, with the lava look drawn over them (SURF-63).
const EARTH: AuthoredWorld = AuthoredWorld {
    look_override: Some(LookOverride::Lava),
    ..world(
        "Earth",
        Rocky,
        (Dense, Surface),
        (false, Some(Active)),
        0x5011_0003,
    )
};

/// Table 2 of the surfaces spec: every planet, Ceres and moon of Sol,
/// with its kind, (atmosphere, water), (locked, activity) and surface
/// seed.
#[rustfmt::skip]
pub const WORLDS: [AuthoredWorld; 30] = [
    world("Mercury",   Rocky,    (Airless, Dry),  (true, Some(Dead)),     0x5011_0001),
    world("Venus",     Rocky,    (Dense, Dry),    (true, Some(Active)),   0x5011_0002),
    EARTH,
    world("Mars",      Rocky,    (Thin, Ice),     (false, Some(Dead)),    0x5011_0004),
    world("Ceres",     Rocky,    (Airless, Ice),  (false, Some(Dead)),    0x5011_0005),
    world("Jupiter",   GasGiant, (Envelope, Dry), (false, None),          0x5011_0006),
    world("Saturn",    GasGiant, (Envelope, Dry), (false, None),          0x5011_0007),
    world("Uranus",    IceGiant, (Envelope, Dry), (false, None),          0x5011_0008),
    world("Neptune",   IceGiant, (Envelope, Dry), (false, None),          0x5011_0009),
    world("Moon",      Moon,     (Airless, Dry),  (true, Some(Dead)),     0x5011_000a),
    world("Phobos",    Moon,     (Airless, Dry),  (true, Some(Dead)),     0x5011_000b),
    world("Deimos",    Moon,     (Airless, Dry),  (true, Some(Dead)),     0x5011_000c),
    world("Amalthea",  Moon,     (Airless, Dry),  (true, Some(Dead)),     0x5011_000d),
    world("Io",        Moon,     (Airless, Dry),  (true, Some(Extreme)),  0x5011_000e),
    world("Europa",    Moon,     (Airless, Ice),  (true, Some(Active)),   0x5011_000f),
    world("Ganymede",  Moon,     (Airless, Ice),  (true, Some(Dead)),     0x5011_0010),
    world("Callisto",  Moon,     (Airless, Ice),  (true, Some(Dead)),     0x5011_0011),
    world("Mimas",     Moon,     (Airless, Ice),  (true, Some(Dead)),     0x5011_0012),
    world("Enceladus", Moon,     (Airless, Ice),  (true, Some(Active)),   0x5011_0013),
    world("Tethys",    Moon,     (Airless, Ice),  (true, Some(Dead)),     0x5011_0014),
    world("Dione",     Moon,     (Airless, Ice),  (true, Some(Dead)),     0x5011_0015),
    world("Rhea",      Moon,     (Airless, Ice),  (true, Some(Dead)),     0x5011_0016),
    world("Titan",     Moon,     (Thin, Ice),     (true, Some(Dead)),     0x5011_0017),
    world("Iapetus",   Moon,     (Airless, Ice),  (true, Some(Dead)),     0x5011_0018),
    world("Ariel",     Moon,     (Airless, Ice),  (true, Some(Dead)),     0x5011_0019),
    world("Umbriel",   Moon,     (Airless, Ice),  (true, Some(Dead)),     0x5011_001a),
    world("Titania",   Moon,     (Airless, Ice),  (true, Some(Dead)),     0x5011_001b),
    world("Oberon",    Moon,     (Airless, Ice),  (true, Some(Dead)),     0x5011_001c),
    world("Triton",    Moon,     (Thin, Ice),     (true, Some(Active)),   0x5011_001d),
    world("Proteus",   Moon,     (Airless, Ice),  (true, Some(Dead)),     0x5011_001e),
];
