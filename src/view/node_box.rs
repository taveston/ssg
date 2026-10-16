//! Where a laid-out UI node sits on screen, in logical pixels: its
//! top-left as placed, its size from the last layout, and the box the
//! two make. Shared by everything that keeps clear of, or draws round,
//! another overlay node (labels, brackets, ring marks, edge tabs), so the
//! box is read one way everywhere (LBL-01, HOV-07).

use bevy::prelude::*;

/// A node's top-left as placed in pixels, or `None` while it isn't shown
/// or isn't placed by `left`/`top` in pixels.
pub fn node_min(node: &Node) -> Option<Vec2> {
    match (node.left, node.top) {
        (Val::Px(left), Val::Px(top)) if node.display != Display::None => {
            Some(Vec2::new(left, top))
        }
        _ => None,
    }
}

/// A laid-out node's size in logical pixels.
pub fn node_size(computed: &ComputedNode) -> Vec2 {
    computed.size() * computed.inverse_scale_factor()
}

/// The box a laid-out node takes on screen: its top-left as placed and
/// its computed size. `None` while it isn't shown.
pub fn node_rect(node: &Node, computed: &ComputedNode) -> Option<Rect> {
    let min = node_min(node)?;
    Some(Rect::from_corners(min, min + node_size(computed)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// LBL-01 / HOV-07: a shown node's box is its placed corner and its
    /// laid-out size, in logical pixels at any UI scale; a hidden node has
    /// none.
    #[test]
    fn a_shown_nodes_box_is_its_corner_and_logical_size() {
        let mut node = Node {
            left: px(10.0),
            top: px(20.0),
            ..default()
        };
        let computed = ComputedNode {
            size: Vec2::new(60.0, 30.0),
            inverse_scale_factor: 0.5,
            ..default()
        };
        let rect = node_rect(&node, &computed).unwrap();
        assert_eq!(rect.min, Vec2::new(10.0, 20.0));
        assert_eq!(rect.size(), Vec2::new(30.0, 15.0));
        node.display = Display::None;
        assert_eq!(node_rect(&node, &computed), None);
    }
}
