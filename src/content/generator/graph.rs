//! Small graph helpers the gate stages share: grouping items joined by
//! pairs (union-find), and distances between catalogue positions. Used to
//! find groups of chosen systems within link range (SGEN-56), to build
//! the spanning tree (SGEN-61) and to find the regions joined by working
//! links (SGEN-72 to SGEN-74).

use bevy::math::DVec3;

/// Items 0 to n − 1 split into groups, merged pair by pair (union-find
/// with path halving). Two items are in the same group once a chain of
/// merged pairs joins them.
#[derive(Debug, Clone)]
pub struct Groups {
    parent: Vec<usize>,
}

impl Groups {
    /// `count` items, each in a group of its own.
    pub fn new(count: usize) -> Self {
        Self {
            parent: (0..count).collect(),
        }
    }

    /// The item that stands for `item`'s group.
    pub fn root(&mut self, mut item: usize) -> usize {
        while self.parent[item] != item {
            self.parent[item] = self.parent[self.parent[item]];
            item = self.parent[item];
        }
        item
    }

    /// Puts `a` and `b` in one group. Returns false if they already were.
    pub fn join(&mut self, a: usize, b: usize) -> bool {
        let (ra, rb) = (self.root(a), self.root(b));
        if ra == rb {
            return false;
        }
        // The lower root stands for the merged group, so results never
        // depend on the order pairs were joined in.
        let (keep, merge) = (ra.min(rb), ra.max(rb));
        self.parent[merge] = keep;
        true
    }

    /// Each item's group, numbered by the group's lowest item, so equal
    /// numbers mean the same group.
    pub fn labels(&mut self) -> Vec<usize> {
        (0..self.parent.len()).map(|i| self.root(i)).collect()
    }

    /// How many groups there are.
    pub fn count(&mut self) -> usize {
        let mut labels = self.labels();
        labels.sort_unstable();
        labels.dedup();
        labels.len()
    }
}

/// The groups of `positions` in which every member is joined to another
/// by a chain of pairs no farther apart than `range_km`.
pub fn groups_within(positions: &[DVec3], range_km: f64) -> Groups {
    let mut groups = Groups::new(positions.len());
    for a in 0..positions.len() {
        for b in a + 1..positions.len() {
            if positions[a].distance(positions[b]) <= range_km {
                groups.join(a, b);
            }
        }
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Joined pairs make one group; unjoined items stay apart.
    #[test]
    fn joined_items_share_a_group() {
        let mut groups = Groups::new(5);
        assert!(groups.join(3, 1));
        assert!(groups.join(1, 4));
        assert!(!groups.join(4, 3));
        assert_eq!(groups.labels(), vec![0, 1, 2, 1, 1]);
        assert_eq!(groups.count(), 3);
    }

    /// Points within range of a neighbour group together, chains
    /// included.
    #[test]
    fn points_within_range_group_by_chains() {
        let points = [
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(1.0, 0.0, 0.0),
            DVec3::new(2.0, 0.0, 0.0),
            DVec3::new(10.0, 0.0, 0.0),
        ];
        let mut groups = groups_within(&points, 1.0);
        assert_eq!(groups.labels(), vec![0, 0, 0, 3]);
    }
}
