//! Whether two links cross when the sector is seen from above: their
//! segments projected onto the slab's plane (x and y; z is the thin axis)
//! (SGEN-61, SGEN-63).

use bevy::math::{DVec2, DVec3};

/// The top-down view of a position: its x and y.
pub fn top_down(position: DVec3) -> DVec2 {
    DVec2::new(position.x, position.y)
}

/// Which side of the line from `a` to `b` the point `c` lies on: positive
/// to the left, negative to the right, zero on it.
fn side(a: DVec2, b: DVec2, c: DVec2) -> f64 {
    (b - a).perp_dot(c - a)
}

/// Whether segments `a`–`b` and `c`–`d` cross: each one's ends lie on
/// opposite sides of the other. Segments that only meet at a shared end
/// do not cross.
pub fn segments_cross(a: DVec2, b: DVec2, c: DVec2, d: DVec2) -> bool {
    if a == c || a == d || b == c || b == d {
        return false;
    }
    let (s1, s2) = (side(a, b, c), side(a, b, d));
    let (s3, s4) = (side(c, d, a), side(c, d, b));
    s1 * s2 < 0.0 && s3 * s4 < 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SGEN-63: an X crosses; a shared end, parallel lines and a T that
    /// stops short do not.
    #[test]
    fn only_segments_that_pass_through_each_other_cross() {
        let p = |x: f64, y: f64| DVec2::new(x, y);
        assert!(segments_cross(
            p(0.0, 0.0),
            p(2.0, 2.0),
            p(0.0, 2.0),
            p(2.0, 0.0)
        ));
        assert!(!segments_cross(
            p(0.0, 0.0),
            p(2.0, 2.0),
            p(2.0, 2.0),
            p(4.0, 0.0)
        ));
        assert!(!segments_cross(
            p(0.0, 0.0),
            p(2.0, 0.0),
            p(0.0, 1.0),
            p(2.0, 1.0)
        ));
        assert!(!segments_cross(
            p(0.0, 0.0),
            p(2.0, 0.0),
            p(1.0, 1.0),
            p(1.0, 0.5)
        ));
        // Height above the plane does not matter.
        let a = top_down(DVec3::new(0.0, 0.0, 5.0));
        assert_eq!(a, p(0.0, 0.0));
    }
}
