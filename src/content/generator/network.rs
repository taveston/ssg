//! The types of the gate network: why a system was gated, its scores, its
//! gate ring and names, and the links between gated systems with their
//! state after the old network failed (SGEN-50 to SGEN-90). The gates,
//! links, failure and names stages fill them in; `generate_sector`
//! gathers them into the sector.

/// A reason the old network's builders had to reach a system (SGEN-50 to
/// SGEN-53), in the order that breaks ties between equal scores
/// (SGEN-54).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Reason {
    Settlement,
    Extraction,
    Industry,
    Science,
}

impl Reason {
    /// Every reason, in tie-breaking order.
    pub const ALL: [Reason; 4] = [
        Reason::Settlement,
        Reason::Extraction,
        Reason::Industry,
        Reason::Science,
    ];
}

/// A system's score for each reason, each from 0 to 1 (SGEN-50 to
/// SGEN-53).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scores {
    pub settlement: f64,
    pub extraction: f64,
    pub industry: f64,
    pub science: f64,
}

impl Scores {
    /// The score for `reason`.
    pub fn get(&self, reason: Reason) -> f64 {
        match reason {
            Reason::Settlement => self.settlement,
            Reason::Extraction => self.extraction,
            Reason::Industry => self.industry,
            Reason::Science => self.science,
        }
    }

    /// The overall score: the best of the four (SGEN-54).
    pub fn overall(&self) -> f64 {
        Reason::ALL
            .iter()
            .map(|&reason| self.get(reason))
            .fold(0.0, f64::max)
    }

    /// The highest-scoring reason; equal scores go to the reason first in
    /// `Reason::ALL` (SGEN-54).
    pub fn best_reason(&self) -> Reason {
        Reason::ALL
            .into_iter()
            .fold(Reason::Settlement, |best, reason| {
                if self.get(reason) > self.get(best) {
                    reason
                } else {
                    best
                }
            })
    }
}

/// How a system came to be gated (SGEN-55 to SGEN-57).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pick {
    /// Picked to fill a reason's quota (SGEN-55).
    Quota(Reason),
    /// Picked for its overall score: the rest of the core, or a system
    /// added to reach the gated count (SGEN-55, SGEN-57).
    Score,
    /// An unchosen system that joined two or more groups of chosen ones
    /// (SGEN-56).
    Junction,
    /// One of a chain of systems that joined two groups no junction could
    /// (SGEN-56).
    Relay,
}

/// The names of one gated system (SGEN-82).
#[derive(Debug, Clone, PartialEq)]
pub struct SystemNames {
    /// The system's own name, which its star also takes.
    pub system: String,
    /// One name per planet of its summary, innermost first.
    pub planets: Vec<String>,
    /// For each belt of its summary, innermost first, `belt_body_max`
    /// spare names for its large bodies.
    pub belt_bodies: Vec<Vec<String>>,
}

/// One gated system (SGEN-90).
#[derive(Debug, Clone, PartialEq)]
pub struct GatedSystem {
    /// Its entry's id.
    pub id: usize,
    pub scores: Scores,
    pub pick: Pick,
    /// The reason it carries a gate (SGEN-54).
    pub main_reason: Reason,
    /// The radius of the circle its gates sit on, in AU (SGEN-58).
    pub ring_radius_au: f64,
    /// The planets, by index in its summary, that moved to clear the ring
    /// (SGEN-59).
    pub moved_planets: Vec<usize>,
    /// Whether it is an approach to valuable systems (SGEN-64).
    pub strategic: bool,
    pub names: SystemNames,
}

/// Which step of building the old network made a link (SGEN-61, SGEN-65).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkOrigin {
    /// The spanning tree that joins every gated system.
    Spanning,
    /// Added between near pairs, shortest first, without crossings.
    Extra,
    /// Added in the second pass, for a system short of links.
    Second,
}

/// A link's state after the old network failed (SGEN-70, SGEN-76).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkState {
    /// Ships can pass.
    Working,
    /// Intact but closed: too narrow, faulted or locked. The flags say
    /// which ends are faulted, in the order of the link's ends; why is
    /// left to the historical layer.
    Dormant { faulted: [bool; 2] },
    /// Both mouths are gone.
    Collapsed,
}

/// One link of the old network between two gated systems (SGEN-65). Each
/// end becomes a gate or a wreck in its system.
#[derive(Debug, Clone, PartialEq)]
pub struct Link {
    /// The entry ids of its two systems, the lower first.
    pub ends: [usize; 2],
    /// The 3D distance between them, in km.
    pub length_km: f64,
    pub origin: LinkOrigin,
    pub state: LinkState,
}

impl Link {
    /// Whether `id` is one of its ends.
    pub fn touches(&self, id: usize) -> bool {
        self.ends.contains(&id)
    }

    /// The end that is not `id`.
    pub fn other_end(&self, id: usize) -> usize {
        if self.ends[0] == id {
            self.ends[1]
        } else {
            self.ends[0]
        }
    }
}
