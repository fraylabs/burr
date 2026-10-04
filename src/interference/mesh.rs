//! Definition-local meshes. Triangle BVHs and topology are shared by occurrences.
use glam::DVec3;
use look::scene::Geometry;
use std::collections::{HashMap, HashSet};

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
    pub oriented: bool,
    pub epsilon: f64,
    sampled_curve: Vec<bool>,
    legacy_curve: Vec<bool>,
    surface_curvature: Vec<Option<bool>>,
    sampled_deviation: Vec<f64>,
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
        let mut source_samples = Vec::new();
        let mut edges: HashMap<[u32; 2], usize> = HashMap::new();
        for t in geometry.indices.as_chunks::<3>().0 {
            let original = *t;
            let mut t = [
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
            let mut normals = geometry
                .surface_normals
                .as_ref()
                .filter(|normals| normals.len() == geometry.vertices.len())
                .and_then(|normals| {
                    let a = DVec3::from_array(normals.get(original[0] as usize)?.map(f64::from));
                    let b = DVec3::from_array(normals.get(original[1] as usize)?.map(f64::from));
                    let c = DVec3::from_array(normals.get(original[2] as usize)?.map(f64::from));
                    [a, b, c]
                        .iter()
                        .all(|n| n.is_finite() && n.length_squared() > 1e-12)
                        .then(|| [a.normalize(), b.normalize(), c.normalize()])
                });
            if let Some(ref mut normals) = normals {
                let [a, b, c] = t.map(|i| points[i as usize]);
                if (b - a).cross(c - a).dot(normals.iter().sum::<DVec3>()) < 0.0 {
                    t.swap(1, 2);
                    normals.swap(1, 2);
                }
            }
            let mut curved = false;
            let mut deviation = 0.0_f64;
            if let Some(normals) = normals {
                for (a, b) in [(0, 1), (1, 2), (2, 0)] {
                    let angle = normals[a].dot(normals[b]).clamp(-1.0, 1.0).acos();
                    curved |= angle > 64.0 * f64::from(f32::EPSILON);
                    let chord = (points[t[b] as usize] - points[t[a] as usize]).length();
                    deviation = deviation.max(chord * 0.5 * (angle * 0.25).tan());
                }
            }
            source_samples.push((normals.is_some(), curved, deviation));
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
        // A gentle nonplanar transition can be a sampled curved boundary.
        // Distinct planar faces meeting at a sharp corner retain the ordinary
        // coordinate-accuracy path. This is a conservative ambiguity marker,
        // not a reconstruction or certification of the source surface.
        let normals = triangles
            .iter()
            .map(|&[a, b, c]| {
                (points[b as usize] - points[a as usize])
                    .cross(points[c as usize] - points[a as usize])
                    .normalize_or_zero()
            })
            .collect::<Vec<_>>();
        let roundoff = 64.0 * f64::from(f32::EPSILON);
        let angular_cosine = look::step::meshing_policy::MeshingPolicy::DEFAULT
            .maximum_angular_deflection
            .cos();
        let surface_curvature: Vec<Option<bool>> = source_samples
            .iter()
            .map(|sample| sample.0.then_some(sample.1))
            .collect();
        let mut legacy_curve = vec![false; triangles.len()];
        let sampled_deviation = source_samples.iter().map(|sample| sample.2).collect();
        let mut neighbor: HashMap<[u32; 2], usize> = HashMap::new();
        for (i, &[a, b, c]) in triangles.iter().enumerate() {
            for [a, b] in [[a, b], [b, c], [c, a]] {
                let edge = [a.min(b), a.max(b)];
                if let Some(&j) = neighbor.get(&edge) {
                    if normals[i].cross(normals[j]).length_squared() > roundoff.powi(2)
                        && normals[i].dot(normals[j]).abs() + roundoff >= angular_cosine
                    {
                        // A known curved patch has the normal/chord guard.
                        // Flat normal samples cannot certify a planar carrier
                        // or its trimmed boundary, so retain this ambiguity
                        // marker for flat and missing evaluator samples.
                        legacy_curve[i] |= surface_curvature[i] != Some(true);
                        legacy_curve[j] |= surface_curvature[j] != Some(true);
                    }
                } else {
                    neighbor.insert(edge, i);
                }
            }
        }
        drop(neighbor);
        let sampled_curve = surface_curvature
            .iter()
            .zip(&legacy_curve)
            .map(|(source, legacy)| source.unwrap_or(false) || *legacy)
            .collect();
        let order = (0..triangles.len()).collect();
        let mut mesh = Self {
            points,
            triangles,
            bounds,
            closed,
            oriented: false,
            epsilon,
            sampled_curve,
            legacy_curve,
            surface_curvature,
            sampled_deviation,
            order,
            nodes: Vec::new(),
        };
        mesh.build(0, mesh.triangles.len());
        mesh.oriented = mesh.geometrically_closed();
        mesh.closed |= six_volume.abs() > bounds.diagonal().powi(3) * 6e-12 && mesh.oriented;
        Ok(mesh)
    }
    // Tessellated faces may sample a common straight edge differently. Check
    // the geometric boundary after splitting only unbalanced edges at existing
    // collinear endpoints. No triangle or position is invented or moved.
    fn geometrically_closed(&self) -> bool {
        let mut balance: HashMap<[u32; 2], i32> = HashMap::new();
        for &[a, b, c] in &self.triangles {
            for [a, b] in [[a, b], [b, c], [c, a]] {
                *balance.entry([a.min(b), a.max(b)]).or_default() += if a < b { 1 } else { -1 };
            }
        }
        balance.retain(|_, count| *count != 0);
        if balance.is_empty() {
            return true;
        }
        let mut endpoints = balance.keys().flatten().copied().collect::<Vec<_>>();
        endpoints.sort_unstable();
        endpoints.dedup();
        let endpoints: HashSet<_> = endpoints.into_iter().collect();
        let mut split_balance: HashMap<[u32; 2], i32> = HashMap::new();
        for ([a, b], count) in balance {
            let origin = self.points[a as usize];
            let direction = self.points[b as usize] - origin;
            let length_squared = direction.length_squared();
            let mut cuts = vec![(0.0, a), (1.0, b)];
            let mut edge_bounds = Bounds::empty();
            edge_bounds.add(origin);
            edge_bounds.add(origin + direction);
            let nearby: HashSet<_> = self
                .candidate_triangles(edge_bounds)
                .into_iter()
                .flat_map(|i| self.triangles[i])
                .filter(|id| endpoints.contains(id))
                .collect();
            for id in nearby {
                if id == a || id == b {
                    continue;
                }
                let point = self.points[id as usize];
                let t = (point - origin).dot(direction) / length_squared;
                if t > 0.0
                    && t < 1.0
                    && point.distance_squared(origin + direction * t) <= self.epsilon.powi(2)
                {
                    cuts.push((t, id));
                }
            }
            cuts.sort_unstable_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
            for interval in cuts.windows(2) {
                let a = interval[0].1;
                let b = interval[1].1;
                *split_balance.entry([a.min(b), a.max(b)]).or_default() +=
                    count * if a < b { 1 } else { -1 };
            }
        }
        split_balance.values().all(|&count| count == 0)
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
    fn cast_ray(
        &self,
        origin: DVec3,
        direction: DVec3,
        limit: f64,
    ) -> (Vec<f64>, bool, i32, usize) {
        let mut ambiguous = false;
        let mut winding = 0;
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
                    if let Some((hit, boundary, sign)) =
                        ray_triangle(origin, direction, self.triangle(t))
                    {
                        if hit >= 0.0 && hit <= limit {
                            ambiguous |= boundary;
                            winding += sign;
                            hits.push(hit);
                        }
                    }
                }
            }
        }
        hits.sort_unstable_by(f64::total_cmp);
        let epsilon = self.epsilon / direction.length();
        let crossing_count = hits.len();
        hits.dedup_by(|a, b| (*a - *b).abs() <= epsilon);
        (hits, ambiguous, winding, crossing_count)
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
    /// Whether the query is near a facet with a gentle nonplanar neighbor.
    /// Such a patch may approximate a curved surface or a shallow crease.
    pub fn nonplanar_near_surface(&self, p: DVec3, tolerance: f64) -> bool {
        self.curve_near_surface(p, tolerance, &self.sampled_curve)
    }
    pub fn legacy_nonplanar_near_surface(&self, p: DVec3, tolerance: f64) -> bool {
        self.curve_near_surface(p, tolerance, &self.legacy_curve)
    }
    fn curve_near_surface(&self, p: DVec3, tolerance: f64, flags: &[bool]) -> bool {
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
                    if flags[t] && point_triangle_distance_squared(p, self.triangle(t)) <= squared {
                        return true;
                    }
                }
            }
        }
        false
    }

    pub fn triangle_surface_sample(&self, triangle: usize) -> (bool, f64) {
        (
            self.surface_curvature[triangle].unwrap_or(self.legacy_curve[triangle]),
            self.sampled_deviation[triangle],
        )
    }

    pub fn nearest_surface_sample(&self, point: DVec3) -> (bool, f64) {
        let mut nearest = f64::INFINITY;
        let mut stack = vec![0];
        while let Some(index) = stack.pop() {
            let node = &self.nodes[index];
            if node.bounds.distance_squared(point) > nearest {
                continue;
            }
            if let Some([a, b]) = node.children {
                let da = self.nodes[a].bounds.distance_squared(point);
                let db = self.nodes[b].bounds.distance_squared(point);
                if da < db {
                    stack.extend([b, a]);
                } else {
                    stack.extend([a, b]);
                }
            } else {
                for &triangle in &self.order[node.start..node.end] {
                    let distance = point_triangle_distance_squared(point, self.triangle(triangle));
                    nearest = nearest.min(distance);
                }
            }
        }
        // Collect the complete linear epsilon shell after finding its center.
        // Updating the sample during the search would discard earlier ties or
        // prune a tied facet merely because another node was visited first.
        let tie_limit = (nearest.sqrt() + self.epsilon).powi(2);
        let mut sample: Option<(bool, f64)> = None;
        let mut stack = vec![0];
        while let Some(index) = stack.pop() {
            let node = &self.nodes[index];
            if node.bounds.distance_squared(point) > tie_limit {
                continue;
            }
            if let Some(children) = node.children {
                stack.extend(children);
            } else {
                for &triangle in &self.order[node.start..node.end] {
                    if point_triangle_distance_squared(point, self.triangle(triangle)) <= tie_limit
                    {
                        let other = self.triangle_surface_sample(triangle);
                        if let Some(sample) = &mut sample {
                            sample.0 &= other.0;
                            sample.1 = sample.1.max(other.1);
                        } else {
                            sample = Some(other);
                        }
                    }
                }
            }
        }
        sample.unwrap_or((false, 0.0))
    }

    pub fn inside(&self, p: DVec3, tolerance: f64) -> bool {
        if !self.bounds.contains(p) || self.near_surface(p, tolerance) {
            return false;
        }
        // Count oriented crossings before distance deduplication. A component
        // may contain touching solids: their coincident exit/entry faces cancel.
        // Deduplicated odd parity incorrectly counts that shared face only once
        // and can classify a point outside both solids as interior.
        // An edge/vertex hit can be a tangency rather than a crossing. Never
        // turn deduplicating such a hit into odd parity: recast in a different
        // direction and use only a ray whose crossings are face-interior hits.
        let mut agreed_inside = false;
        for direction in [
            DVec3::new(1.0, 0.3713906763541037, 0.6947465906068658),
            DVec3::new(0.219513694312, 1.0, 0.51789347219),
            DVec3::new(0.754877666247, 0.324717957245, 1.0),
        ] {
            let (_, ambiguous, winding, crossing_count) =
                self.cast_ray(p, direction, f64::INFINITY);
            if !ambiguous {
                if self.oriented {
                    return winding != 0;
                }
                if crossing_count % 2 == 0 {
                    return false;
                }
                agreed_inside = true;
            }
        }
        agreed_inside
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

fn ray_triangle(o: DVec3, d: DVec3, [a, b, c]: [DVec3; 3]) -> Option<(f64, bool, i32)> {
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
        if det > 0.0 { 1 } else { -1 },
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
    fn near_surface_ties_use_linear_distance_and_ignore_traversal_order() {
        let positions = [
            [0., 0., 0.],
            [1., 0., 0.],
            [0., 1., 0.],
            [0., 0., 0.00005],
            [1., 0., 0.00005],
            [0., 1., 0.00005],
        ];
        let geometry = look::scene::Geometry {
            vertices: positions
                .iter()
                .map(|&position| look::scene::Vertex {
                    position,
                    normal: [0.; 3],
                })
                .collect(),
            surface_normals: None,
            source_attributes: None,
            indices: vec![0, 1, 2, 3, 4, 5],
            bounds: look::scene::Bounds::from_positions(&positions),
            bounding_center: [0.; 3],
            bounding_radius: 0.,
        };
        let mut mesh = Mesh::prepare(&geometry).unwrap();
        mesh.epsilon = 0.0001;
        mesh.surface_curvature = vec![Some(true), Some(false)];
        mesh.sampled_deviation = vec![0.2, 0.1];
        let point = DVec3::new(0.1, 0.1, 1.0);
        assert_eq!(mesh.nearest_surface_sample(point), (false, 0.2));
        mesh.order.reverse();
        assert_eq!(mesh.nearest_surface_sample(point), (false, 0.2));
        // Put the tied facets in separate BVH leaves so nearest-only pruning
        // cannot quietly discard the farther member of the epsilon shell.
        let mut leaves = Vec::new();
        for (start, &triangle) in mesh.order.iter().enumerate() {
            let mut bounds = Bounds::empty();
            for point in mesh.triangle(triangle) {
                bounds.add(point);
            }
            leaves.push(Node {
                bounds,
                start,
                end: start + 1,
                children: None,
            });
        }
        mesh.nodes = vec![Node {
            bounds: mesh.bounds,
            start: 0,
            end: 2,
            children: Some([1, 2]),
        }];
        mesh.nodes.extend(leaves);
        assert_eq!(mesh.nearest_surface_sample(point), (false, 0.2));
        mesh.epsilon *= 0.25;
        assert_eq!(mesh.nearest_surface_sample(point), (false, 0.1));
    }

    #[test]
    fn evaluator_normals_repair_a_reversed_triangle() {
        let positions = [DVec3::ZERO, DVec3::X, DVec3::Y, DVec3::Z];
        let triangles = [[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]];
        let mut vertices = Vec::new();
        let mut normals = Vec::new();
        let mut indices = Vec::new();
        for triangle in triangles {
            let [a, b, c] = triangle.map(|i| positions[i]);
            let normal = (b - a).cross(c - a).normalize().as_vec3().to_array();
            for point in [a, b, c] {
                indices.push(vertices.len() as u32);
                vertices.push(look::scene::Vertex {
                    position: point.as_vec3().to_array(),
                    normal: [0.; 3],
                });
                normals.push(normal);
            }
        }
        indices.swap(0, 1);
        let geometry = look::scene::Geometry {
            surface_normals: Some(normals),
            bounds: look::scene::Bounds::from_positions(
                &vertices.iter().map(|v| v.position).collect::<Vec<_>>(),
            ),
            vertices,
            indices,
            source_attributes: None,
            bounding_center: [0.; 3],
            bounding_radius: 0.,
        };
        let mesh = Mesh::prepare(&geometry).unwrap();
        assert!(mesh.oriented);
        assert!(mesh.inside(DVec3::splat(0.1), mesh.epsilon));
    }

    #[test]
    fn source_surface_normals_mark_a_single_cylindrical_triangle() {
        let angle = 0.5_f32;
        let normals = vec![
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [angle.cos(), angle.sin(), 0.0],
        ];
        let positions = [
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 1.0],
            [angle.cos(), angle.sin(), 0.0],
        ];
        let vertices = positions
            .map(|position| look::scene::Vertex {
                position,
                normal: [0.; 3],
            })
            .to_vec();
        let geometry = look::scene::Geometry {
            surface_normals: Some(normals),
            bounds: look::scene::Bounds::from_positions(&positions),
            vertices,
            indices: vec![0, 1, 2],
            source_attributes: None,
            bounding_center: [0.; 3],
            bounding_radius: 0.,
        };
        let mesh = Mesh::prepare(&geometry).unwrap();
        let point = mesh.triangle(0).iter().sum::<DVec3>() / 3.0;
        assert!(mesh.nonplanar_near_surface(point, 0.01));
        assert!(mesh.triangle_surface_sample(0).1 > 0.02);
    }

    #[test]
    fn flat_normal_samples_keep_the_shared_boundary_ambiguity() {
        let positions = [
            [0., 0., 0.],
            [1., 0., 0.],
            [0., 1., 0.],
            [1., 0., 0.],
            [0., 0., 0.],
            [0., -1., 0.1],
        ];
        let vertices = positions
            .map(|position| look::scene::Vertex {
                position,
                normal: [0.; 3],
            })
            .to_vec();
        let tilted = glam::Vec3::new(0., 0.1, 1.).normalize().to_array();
        let mut geometry = look::scene::Geometry {
            surface_normals: Some(vec![
                [0., 0., 1.],
                [0., 0., 1.],
                [0., 0., 1.],
                tilted,
                tilted,
                tilted,
            ]),
            bounds: look::scene::Bounds::from_positions(&positions),
            vertices,
            indices: vec![0, 1, 2, 3, 4, 5],
            source_attributes: None,
            bounding_center: [0.; 3],
            bounding_radius: 0.,
        };
        let point = DVec3::new(1. / 3., 1. / 3., 0.);
        let mesh = Mesh::prepare(&geometry).unwrap();
        assert!(!mesh.triangle_surface_sample(0).0);
        assert!(mesh.nonplanar_near_surface(point, 0.01));
        assert!(mesh.legacy_nonplanar_near_surface(point, 0.01));
        geometry.surface_normals.as_mut().unwrap()[3..].copy_from_slice(&[
            [0., 0., 1.],
            [0., 0., 1.],
            tilted,
        ]);
        let curved_boundary = Mesh::prepare(&geometry).unwrap();
        assert!(!curved_boundary.triangle_surface_sample(0).0);
        assert!(curved_boundary.legacy_nonplanar_near_surface(point, 0.01));
        geometry.surface_normals = None;
        let fallback = Mesh::prepare(&geometry).unwrap();
        assert!(fallback.nonplanar_near_surface(point, 0.01));
        assert!(fallback.legacy_nonplanar_near_surface(point, 0.01));
    }

    #[test]
    fn touching_solids_do_not_turn_an_exterior_point_into_interior() {
        let rotation = glam::DMat4::from_rotation_z(std::f64::consts::FRAC_PI_4);
        let cube = [
            [-1., -1., -1.],
            [1., -1., -1.],
            [1., 1., -1.],
            [-1., 1., -1.],
            [-1., -1., 1.],
            [1., -1., 1.],
            [1., 1., 1.],
            [-1., 1., 1.],
        ];
        let faces = [
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
        let mut points = Vec::new();
        let mut triangles = Vec::new();
        for offset in [0., 2.] {
            let base = points.len() as u32;
            points.extend(
                cube.map(|p| rotation.transform_point3(DVec3::from_array(p) + DVec3::X * offset)),
            );
            triangles.extend(faces.map(|t| t.map(|i| i + base)));
        }
        let mut bounds = Bounds::empty();
        for &point in &points {
            bounds.add(point);
        }
        let mut mesh = Mesh {
            points,
            triangles,
            bounds,
            closed: true,
            oriented: true,
            epsilon: 1e-7,
            order: (0..24).collect(),
            sampled_curve: vec![false; 24],
            legacy_curve: vec![false; 24],
            surface_curvature: vec![None; 24],
            sampled_deviation: vec![0.0; 24],
            nodes: Vec::new(),
        };
        mesh.build(0, 24);
        let outside = rotation.transform_point3(DVec3::new(0., 1.1, 0.));
        assert!(mesh.bounds.contains(outside));
        assert!(!mesh.inside(outside, mesh.epsilon));
        assert!(mesh.inside(
            rotation.transform_point3(DVec3::new(0., 0., 0.)),
            mesh.epsilon
        ));
    }

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
            oriented: true,
            epsilon: 1e-7,
            order: (0..12).collect(),
            sampled_curve: vec![false; 12],
            legacy_curve: vec![false; 12],
            surface_curvature: vec![None; 12],
            sampled_deviation: vec![0.0; 12],
            nodes: Vec::new(),
        };
        mesh.build(0, 12);
        let direction = DVec3::new(1.0, 0.3713906763541037, 0.6947465906068658);
        let point = (mesh.points[2] + mesh.points[6]) * 0.5 - direction * 0.1;
        assert!(mesh.bounds.contains(point));
        assert!(!mesh.inside(point, mesh.epsilon));
    }
}
