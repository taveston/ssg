//! The bake's cubemap as a strip of faces (SURF-30, SURF-35): six square
//! faces side by side in one image, one row of texels for the albedo and
//! height and, for a body that glows, a second row for the emissive. The
//! sums here map a direction to its face and texel and a texel back to
//! its direction; `bake.wesl` and `globe.wesl` run the same sums, so this
//! is the plain copy the tests check. A close-in face (SURF-34) is a
//! strip of one column holding a single face, laid out the same way.
//!
//! Each face keeps a one-texel border past its edge, baked as the true
//! continuation of the sphere, so a bilinear sample on a face's edge
//! reads the same value from either face: no seams, and no pole
//! pinching, since a cube has no pole.
//!
//! The body frame: +Y is the north pole, +Z the prime meridian, so the
//! substellar point of a locked body is +Z (SURF-21, SURF-41).

use bevy::math::Vec3;

/// The six faces, in strip order: +X, -X, +Y, -Y, +Z, -Z.
pub const FACES: u32 = 6;
/// The texels of border on each side of a face.
#[cfg(test)]
const BORDER: f32 = 1.0;

/// Where a direction lands on the cube: its face and its coordinates on
/// it, each in [-1, 1].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FacePoint {
    pub face: u32,
    pub s: f32,
    pub t: f32,
}

/// SURF-35: the face a direction lands on and where on it: the face of
/// its largest axis, with the other two axes divided by it.
pub fn face_point(d: Vec3) -> FacePoint {
    let a = d.abs();
    if a.x >= a.y && a.x >= a.z {
        let m = a.x.max(1e-9);
        if d.x > 0.0 {
            FacePoint {
                face: 0,
                s: -d.z / m,
                t: d.y / m,
            }
        } else {
            FacePoint {
                face: 1,
                s: d.z / m,
                t: d.y / m,
            }
        }
    } else if a.y >= a.z {
        let m = a.y.max(1e-9);
        if d.y > 0.0 {
            FacePoint {
                face: 2,
                s: d.x / m,
                t: -d.z / m,
            }
        } else {
            FacePoint {
                face: 3,
                s: d.x / m,
                t: d.z / m,
            }
        }
    } else {
        let m = a.z.max(1e-9);
        if d.z > 0.0 {
            FacePoint {
                face: 4,
                s: d.x / m,
                t: d.y / m,
            }
        } else {
            FacePoint {
                face: 5,
                s: -d.x / m,
                t: d.y / m,
            }
        }
    }
}

/// SURF-35: the direction of a point on a face, the inverse of
/// `face_point`; `s` and `t` may run past the face's edge, into the
/// border.
#[cfg(test)]
pub fn face_direction(face: u32, s: f32, t: f32) -> Vec3 {
    match face {
        0 => Vec3::new(1.0, t, -s),
        1 => Vec3::new(-1.0, t, s),
        2 => Vec3::new(s, 1.0, -t),
        3 => Vec3::new(s, -1.0, t),
        4 => Vec3::new(s, t, 1.0),
        _ => Vec3::new(-s, t, -1.0),
    }
    .normalize()
}

/// The face coordinate at a texel's centre: texel `i` of a face
/// `face_px` wide, the border included, so texel 0 and texel `face_px -
/// 1` lie just past the face's edges.
#[cfg(test)]
pub fn texel_coordinate(i: u32, face_px: u32) -> f32 {
    -1.0 + (i as f32 + 0.5 - BORDER) * 2.0 / (face_px as f32 - 2.0 * BORDER)
}

/// The direction a texel of the strip is baked for: texel (`x`, `y`) of
/// `face`, in a strip of faces `face_px` wide.
#[cfg(test)]
pub fn texel_direction(face: u32, x: u32, y: u32, face_px: u32) -> Vec3 {
    face_direction(
        face,
        texel_coordinate(x, face_px),
        texel_coordinate(y, face_px),
    )
}

/// SURF-35: where a direction is sampled in the strip, in texels from the
/// strip's top left, for its row of faces: the face's column, then its
/// coordinates mapped over the face less its border. On a face's edge the
/// sample lands on the boundary between the last inner texel and the
/// border texel, so both faces give the same blend.
#[cfg(test)]
pub fn strip_texel(d: Vec3, row: u32, face_px: u32) -> (f32, f32) {
    let FacePoint { face, s, t } = face_point(d);
    let inner = face_px as f32 - 2.0 * BORDER;
    let x = (face * face_px) as f32 + BORDER + (s + 1.0) * inner / 2.0;
    let y = (row * face_px) as f32 + BORDER + (t + 1.0) * inner / 2.0;
    (x, y)
}

/// The size of a strip in texels: `columns` faces wide (six for a whole
/// cube, one for a close-in face), `rows` faces tall.
pub fn strip_size(face_px: u32, columns: u32, rows: u32) -> (u32, u32) {
    (columns * face_px, rows * face_px)
}

/// SURF-26: `count` directions spread evenly over the sphere, on a
/// Fibonacci spiral.
#[cfg(test)]
pub fn fibonacci_sphere(count: usize) -> Vec<Vec3> {
    let golden = std::f32::consts::PI * (3.0 - 5.0_f32.sqrt());
    (0..count)
        .map(|i| {
            let y = 1.0 - 2.0 * (i as f32 + 0.5) / count as f32;
            let ring = (1.0 - y * y).sqrt();
            let angle = golden * i as f32;
            Vec3::new(ring * angle.cos(), y, ring * angle.sin())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::generator::rng::Rng;

    /// SURF-35: a direction maps to a face and back to itself, from every
    /// face, and a texel's direction maps back to that texel.
    #[test]
    fn directions_round_trip_through_their_face_and_texel() {
        let mut rng = Rng::new(3);
        let mut faces_seen = [false; FACES as usize];
        for _ in 0..5_000 {
            let d = rng.direction().as_vec3();
            let p = face_point(d);
            faces_seen[p.face as usize] = true;
            assert!(p.s.abs() <= 1.0 && p.t.abs() <= 1.0);
            let back = face_direction(p.face, p.s, p.t);
            assert!((back - d).length() < 1e-5, "{d} -> {p:?} -> {back}");
        }
        assert!(faces_seen.iter().all(|&seen| seen));
        let face_px = 64;
        for face in 0..FACES {
            for (x, y) in [(1, 1), (30, 17), (62, 62), (0, 40), (63, 5)] {
                let d = texel_direction(face, x, y, face_px);
                let (sx, sy) = strip_texel(d, 0, face_px);
                // A border texel's direction lies on the neighbouring
                // face, so only inner texels map back to themselves.
                if (1..face_px - 1).contains(&x) && (1..face_px - 1).contains(&y) {
                    assert!((sx - (face * face_px + x) as f32 - 0.5).abs() < 1e-3);
                    assert!((sy - y as f32 - 0.5).abs() < 1e-3);
                }
            }
        }
        assert_eq!(strip_size(64, FACES, 2), (384, 128));
        assert_eq!(strip_size(64, 1, 2), (64, 128));
    }

    /// SURF-35: on a face's edge the sample sits on the boundary between
    /// the last inner texel and the border texel, from either face, and
    /// the two texel pairs are baked for the same directions to first
    /// order, so the blend is the same.
    #[test]
    fn a_face_edge_samples_the_same_texels_from_either_face() {
        let face_px = 64;
        // The edge shared by +X (face 0) and +Z (face 4).
        let edge = Vec3::new(1.0, 0.3, 1.0).normalize();
        let from_x = strip_texel(edge, 0, face_px);
        let from_z = strip_texel(edge * Vec3::new(1.0, 1.0, 1.0 + 1e-6), 0, face_px);
        // Face 0's s runs to -1 at this edge (x = 0 * 64 + 1); face 4's s
        // runs to +1 (x = 4 * 64 + 63).
        assert!((from_x.0 - 1.0).abs() < 1e-3 || (from_z.0 - 1.0).abs() < 1e-3);
        assert!(
            (from_x.0 - (4 * face_px + 63) as f32).abs() < 1e-3
                || (from_z.0 - (4 * face_px + 63) as f32).abs() < 1e-3
        );
        // The border texel of one face and the last inner texel of the
        // other are baked for nearly the same direction.
        let y = ((from_x.1 - 0.5).round()) as u32;
        let x_border = texel_direction(0, 0, y, face_px);
        let z_inner = texel_direction(4, face_px - 2, y, face_px);
        assert!((x_border - z_inner).length() < 0.02, "{x_border} {z_inner}");
        let x_inner = texel_direction(0, 1, y, face_px);
        let z_border = texel_direction(4, face_px - 1, y, face_px);
        assert!((x_inner - z_border).length() < 0.02);
    }

    /// SURF-26: the Fibonacci sphere's directions are unit vectors spread
    /// over the whole sphere, not bunched.
    #[test]
    fn the_fibonacci_sphere_spreads_its_points_evenly() {
        let points = fibonacci_sphere(32);
        assert_eq!(points.len(), 32);
        for p in &points {
            assert!((p.length() - 1.0).abs() < 1e-5);
        }
        let octants: std::collections::HashSet<(bool, bool, bool)> = points
            .iter()
            .map(|p| (p.x > 0.0, p.y > 0.0, p.z > 0.0))
            .collect();
        assert_eq!(octants.len(), 8);
        let mean = points.iter().copied().sum::<Vec3>() / 32.0;
        assert!(mean.length() < 0.1, "{mean}");
    }
}
