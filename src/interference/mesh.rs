//! Definition-local meshes. Triangle BVHs and topology are shared by occurrences.
use glam::DVec3;
use look::scene::Geometry;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug)]
pub(super) struct Bounds {
    pub min: DVec3,
    pub max: DVec3,
}

impl Bounds {
    pub fn empty() -> Self {
        Self {
            min: DVec3::splat(f64::INFINITY),
            max: DVec3::splat(f64::NEG_INFINITY),
        }
    }
    pub fn add(&mut self, p: DVec3) {
        self.min = self.min.min(p);
        self.max = self.max.max(p);
    }
    fn union(&mut self, b: Self) {
        self.add(b.min);
        self.add(b.max);
    }
    pub fn diagonal(self) -> f64 {
        (self.max - self.min).length()
    }
    pub fn overlaps(self, b: Self, tolerance: f64) -> bool {
        (self.max.min(b.max) - self.min.max(b.min)).min_element() > tolerance
    }
    pub fn contains(self, p: DVec3) -> bool {
        p.cmpge(self.min).all() && p.cmple(self.max).all()
    }
    fn distance_squared(self, p: DVec3) -> f64 {
        (p - p.clamp(self.min, self.max)).length_squared()
    }
    fn ray(self, origin: DVec3, direction: DVec3, limit: f64) -> bool {
        let mut near: f64 = 0.0;
        let mut far = limit;
        for axis in 0..3 {
            if direction[axis].abs() < 1e-30 {
                if origin[axis] < self.min[axis] || origin[axis] > self.max[axis] {
                    return false;
                }
            } else {
                let a = (self.min[axis] - origin[axis]) / direction[axis];
                let b = (self.max[axis] - origin[axis]) / direction[axis];
                near = near.max(a.min(b));
                far = far.min(a.max(b));
                if far < near {
                    return false;
                }
            }
        }
        true
    }
}

struct Node {
    bounds: Bounds,
    start: usize,
    end: usize,
    children: Option<[usize; 2]>,
}
pub(super) struct Mesh {
    pub points: Vec<DVec3>,
    pub triangles: Vec<[u32; 3]>,
    pub bounds: Bounds,
    pub closed: bool,
    pub epsilon: f64,
    order: Vec<usize>,
    nodes: Vec<Node>,
}

impl Mesh {
    pub fn prepare(geometry: &Geometry) -> Result<Self, String> {
        if geometry.indices.is_empty() || geometry.vertices.is_empty() {
            return Err("contains no triangles".into());
        }
        if !geometry.indices.len().is_multiple_of(3) {
            return Err("has a non-triangular index buffer".into());
        }
        if let Some(i) = geometry
            .indices
            .iter()
            .find(|&&i| i as usize >= geometry.vertices.len())
        {
            return Err(format!("references missing vertex index {i}"));
        }
        let mut bounds = Bounds::empty();
        for v in &geometry.vertices {
            let p = DVec3::from(glam::Vec3::from_array(v.position));
            if !p.is_finite() {
                return Err("contains non-finite vertices".into());
            }
            bounds.add(p);
        }
        // Hash welding replaces Truck's quadratic attribute scan. Work in local
        // coordinates so placement rounding cannot change topology per occurrence.
        // Look stores f32 positions: shared endpoints far from the origin can
        // differ by an ULP even when their part's own diagonal is very small.
        let coordinate_magnitude = bounds.min.abs().max(bounds.max.abs()).max_element();
        let epsilon = (bounds.diagonal() * 1e-7)
            .max(coordinate_magnitude * f64::from(f32::EPSILON) * 2.0)
            .max(1e-9);
        let mut cells: HashMap<[i64; 3], Vec<u32>> = HashMap::new();
        let mut points: Vec<DVec3> = Vec::new();
        let mut remap = Vec::with_capacity(geometry.vertices.len());
        for v in &geometry.vertices {
            let p = DVec3::from(glam::Vec3::from_array(v.position));
            let q = ((p - bounds.min) / epsilon).floor();
            let cell = [q.x as i64, q.y as i64, q.z as i64];
            let mut found = cells.get(&cell).and_then(|ids| {
                ids.iter()
                    .copied()
                    .find(|&id| points[id as usize].distance_squared(p) <= epsilon * epsilon)
            });
            'neighbors: for x in -1..=1 {
                if found.is_some() {
                    break 'neighbors;
                }
                for y in -1..=1 {
                    for z in -1..=1 {
                        if let Some(ids) = cells.get(&[cell[0] + x, cell[1] + y, cell[2] + z]) {
                            for &id in ids {
                                if points[id as usize].distance_squared(p) <= epsilon * epsilon {
                                    found = Some(id);
                                    break 'neighbors;
                                }
                            }
                        }
                    }
                }
            }
            let id = found.unwrap_or_else(|| {
                let id = points.len() as u32;
                points.push(p);
                cells.entry(cell).or_default().push(id);
                id
            });
            remap.push(id);
        }
        let mut triangles = Vec::with_capacity(geometry.indices.len() / 3);
        let mut edges: HashMap<[u32; 2], usize> = HashMap::new();
        for t in geometry.indices.as_chunks::<3>().0 {
            let t = [
                remap[t[0] as usize],
                remap[t[1] as usize],
                remap[t[2] as usize],
            ];
            if t[0] == t[1] || t[1] == t[2] || t[2] == t[0] {
                continue;
            }
            if (points[t[1] as usize] - points[t[0] as usize])
                .cross(points[t[2] as usize] - points[t[0] as usize])
                .length_squared()
                < epsilon.powi(4)
            {
                continue;
            }
            triangles.push(t);
            for [a, b] in [[t[0], t[1]], [t[1], t[2]], [t[2], t[0]]] {
                *edges.entry([a.min(b), a.max(b)]).or_default() += 1;
            }
        }
        // A closed edge graph can be a doubled sheet with no solid interior.
        // Center the tetrahedra to avoid cancellation from an origin offset.
        let origin = (bounds.min + bounds.max) * 0.5;
        let six_volume: f64 = triangles
            .iter()
            .map(|t| {
                (points[t[0] as usize] - origin)
                    .dot((points[t[1] as usize] - origin).cross(points[t[2] as usize] - origin))
            })
            .sum();
        let closed = !triangles.is_empty()
            && edges.values().all(|&count| count == 2)
            && six_volume.abs() > bounds.diagonal().powi(3) * 6e-12;
        drop(edges);
        drop(cells);
        drop(remap);
        if triangles.is_empty() {
            return Err("contains no non-degenerate triangles".into());
        }
        let order = (0..triangles.len()).collect();
        let mut mesh = Self {
            points,
            triangles,
            bounds,
            closed,
            epsilon,
            order,
            nodes: Vec::new(),
        };
        mesh.build(0, mesh.triangles.len());
        Ok(mesh)
    }
    pub fn triangle(&self, index: usize) -> [DVec3; 3] {
        self.triangles[index].map(|i| self.points[i as usize])
    }
    fn triangle_bounds(&self, i: usize) -> Bounds {
        let mut b = Bounds::empty();
        for p in self.triangle(i) {
            b.add(p);
        }
        b
    }
    fn build(&mut self, start: usize, end: usize) -> usize {
        let mut bounds = Bounds::empty();
        for &i in &self.order[start..end] {
            bounds.union(self.triangle_bounds(i));
        }
        let index = self.nodes.len();
        self.nodes.push(Node {
            bounds,
            start,
            end,
            children: None,
        });
        if end - start > 8 {
            let extent = bounds.max - bounds.min;
            let axis = if extent.x >= extent.y && extent.x >= extent.z {
                0
            } else if extent.y >= extent.z {
                1
            } else {
                2
            };
            let mid = (start + end) / 2;
            let points = &self.points;
            let triangles = &self.triangles;
            self.order[start..end].select_nth_unstable_by(mid - start, |&a, &b| {
                let center = |i: usize| {
                    triangles[i]
                        .iter()
                        .map(|&v| points[v as usize][axis])
                        .sum::<f64>()
                };
                center(a).total_cmp(&center(b))
            });
            let left = self.build(start, mid);
            let right = self.build(mid, end);
            self.nodes[index].children = Some([left, right]);
        }
        index
    }
    pub fn ray_hits(&self, origin: DVec3, direction: DVec3, limit: f64) -> Vec<f64> {
        self.cast_ray(origin, direction, limit).0
    }
    fn cast_ray(&self, origin: DVec3, direction: DVec3, limit: f64) -> (Vec<f64>, bool) {
        let mut ambiguous = false;
        let mut hits = Vec::new();
        let mut stack = vec![0];
        while let Some(i) = stack.pop() {
            let node = &self.nodes[i];
            if !node.bounds.ray(origin, direction, limit) {
                continue;
            }
            if let Some(children) = node.children {
                stack.extend(children);
            } else {
                for &t in &self.order[node.start..node.end] {
                    if let Some((hit, boundary)) = ray_triangle(origin, direction, self.triangle(t))
                    {
                        if hit >= 0.0 && hit <= limit {
                            ambiguous |= boundary;
                            hits.push(hit);
                        }
                    }
                }
            }
        }
        hits.sort_unstable_by(f64::total_cmp);
        let epsilon = self.epsilon / direction.length();
        hits.dedup_by(|a, b| (*a - *b).abs() <= epsilon);
        (hits, ambiguous)
    }
    pub fn near_surface(&self, p: DVec3, tolerance: f64) -> bool {
        let squared = tolerance * tolerance;
        let mut stack = vec![0];
        while let Some(i) = stack.pop() {
            let node = &self.nodes[i];
            if node.bounds.distance_squared(p) > squared {
                continue;
            }
            if let Some(children) = node.children {
                stack.extend(children);
            } else {
                for &t in &self.order[node.start..node.end] {
                    if point_triangle_distance_squared(p, self.triangle(t)) <= squared {
                        return true;
                    }
                }
            }
        }
        false
    }
    pub fn inside(&self, p: DVec3, tolerance: f64) -> bool {
        if !self.bounds.contains(p) || self.near_surface(p, tolerance) {
            return false;
        }
        // An edge/vertex hit can be a tangency rather than a crossing. Never
        // turn deduplicating such a hit into odd parity: recast in a different
        // direction and use only a ray whose crossings are face-interior hits.
        for direction in [
            DVec3::new(1.0, 0.3713906763541037, 0.6947465906068658),
            DVec3::new(0.219513694312, 1.0, 0.51789347219),
            DVec3::new(0.754877666247, 0.324717957245, 1.0),
        ] {
            let (hits, ambiguous) = self.cast_ray(p, direction, f64::INFINITY);
            if !ambiguous {
                return hits.len() % 2 == 1;
            }
        }
        false
    }

    pub fn candidate_triangles(&self, bounds: Bounds) -> Vec<usize> {
        let mut result = Vec::new();
        let mut stack = vec![0];
        while let Some(i) = stack.pop() {
            let node = &self.nodes[i];
            if !node.bounds.overlaps(bounds, -self.epsilon) {
                continue;
            }
            if let Some(children) = node.children {
                stack.extend(children);
            } else {
                result.extend_from_slice(&self.order[node.start..node.end]);
            }
        }
        result
    }
}

fn ray_triangle(o: DVec3, d: DVec3, [a, b, c]: [DVec3; 3]) -> Option<(f64, bool)> {
    let e1 = b - a;
    let e2 = c - a;
    let h = d.cross(e2);
    let det = e1.dot(h);
    if det.abs() <= 1e-14 * e1.length() * e2.length() * d.length() {
        return None;
    }
    let s = o - a;
    let u = s.dot(h) / det;
    if !(-1e-10..=1.0 + 1e-10).contains(&u) {
        return None;
    }
    let q = s.cross(e1);
    let v = d.dot(q) / det;
    if v < -1e-10 || u + v > 1.0 + 1e-10 {
        return None;
    }
    Some((
        e2.dot(q) / det,
        u <= 1e-9 || v <= 1e-9 || 1.0 - u - v <= 1e-9,
    ))
}

// Closest point regions from Real-Time Collision Detection, section 5.1.5.
fn point_triangle_distance_squared(p: DVec3, [a, b, c]: [DVec3; 3]) -> f64 {
    let ab = b - a;
    let ac = c - a;
    let ap = p - a;
    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return ap.length_squared();
    }
    let bp = p - b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0.0 && d4 <= d3 {
        return bp.length_squared();
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return (p - (a + ab * (d1 / (d1 - d3)))).length_squared();
    }
    let cp = p - c;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0.0 && d5 <= d6 {
        return cp.length_squared();
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return (p - (a + ac * (d2 / (d2 - d6)))).length_squared();
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && d4 - d3 >= 0.0 && d5 - d6 >= 0.0 {
        return (p - (b + (c - b) * ((d4 - d3) / ((d4 - d3) + (d5 - d6))))).length_squared();
    }
    let inverse = 1.0 / (va + vb + vc);
    (p - (a + ab * (vb * inverse) + ac * (vc * inverse))).length_squared()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ray_tangent_to_a_shared_edge_does_not_make_an_outside_point_inside() {
        let rotation = glam::DMat4::from_rotation_z(std::f64::consts::FRAC_PI_4);
        let points = [
            [-1., -1., -1.],
            [1., -1., -1.],
            [1., 1., -1.],
            [-1., 1., -1.],
            [-1., -1., 1.],
            [1., -1., 1.],
            [1., 1., 1.],
            [-1., 1., 1.],
        ]
        .map(|p| rotation.transform_point3(DVec3::from_array(p)))
        .to_vec();
        let triangles = vec![
            [0, 2, 1],
            [0, 3, 2],
            [4, 5, 6],
            [4, 6, 7],
            [0, 1, 5],
            [0, 5, 4],
            [1, 2, 6],
            [1, 6, 5],
            [2, 3, 7],
            [2, 7, 6],
            [3, 0, 4],
            [3, 4, 7],
        ];
        let mut bounds = Bounds::empty();
        for &p in &points {
            bounds.add(p);
        }
        let mut mesh = Mesh {
            points,
            triangles,
            bounds,
            closed: true,
            epsilon: 1e-7,
            order: (0..12).collect(),
            nodes: Vec::new(),
        };
        mesh.build(0, 12);
        let direction = DVec3::new(1.0, 0.3713906763541037, 0.6947465906068658);
        let point = (mesh.points[2] + mesh.points[6]) * 0.5 - direction * 0.1;
        assert!(mesh.bounds.contains(point));
        assert!(!mesh.inside(point, mesh.epsilon));
    }
}
