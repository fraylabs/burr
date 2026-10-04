//! Source BREP evidence, kept separate from float32 tessellation evidence.
//! Unsupported carriers or uncertain occurrence correspondence refuse contact.
use glam::{DMat4, DVec3, DVec4};
use look::scene::CompiledScene;
use std::{collections::HashMap, path::Path, sync::Arc};
use truck_assembly::assy::EdgeEntity;
use truck_stepio::r#in::{
    convert::{AssembleEntity, ProductEntity, ProductShape},
    step_geometry::{
        Conic3D, Curve3D, ElementarySurface, Matrix4, ParametricSurface, Point3, Surface,
    },
    Table,
};
use truck_topology::compress::CompressedShell;

type Shell = CompressedShell<Point3, Curve3D, Surface>;

pub struct SourceEvidence {
    pub occurrences: Vec<SourceOccurrence>,
}

pub struct SourceOccurrence {
    pub name: String,
    world: DMat4,
    boundary: Option<Arc<AnalyticBoundary>>,
}

#[derive(Default)]
struct AnalyticBoundary {
    points: Vec<DVec3>,
    ellipses: Vec<[DVec3; 3]>,
    cylinders: Vec<CylinderSupport>,
    normals: Vec<DVec3>,
}

struct CylinderSupport {
    center: DVec3,
    radial: [DVec3; 2],
    axis: DVec3,
    axial_range: (f64, f64),
}

pub struct ContactProof {
    pub normal: DVec3,
    pub gap: f64,
    pub error: f64,
}

pub fn fallback_name(name: Option<&str>, path: &str) -> String {
    name.and_then(|name| name.rsplit('/').find(|part| meaningful(part)))
        .map(str::trim)
        .map(str::to_owned)
        .unwrap_or_else(|| {
            Path::new(path)
                .file_stem()
                .and_then(|name| name.to_str())
                .unwrap_or("Component")
                .to_owned()
        })
}

fn meaningful(name: &str) -> bool {
    let name = name.trim();
    if name.is_empty() || name.starts_with("=>[") || name.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    let lower = name.to_ascii_lowercase();
    !["solid", "body", "part"].iter().any(|generic| {
        lower.strip_prefix(generic).is_some_and(|suffix| {
            suffix.chars().all(|c| {
                c.is_ascii_digit() || c.is_ascii_whitespace() || c == '#' || c == '_' || c == '-'
            })
        })
    })
}

impl SourceEvidence {
    pub fn read(path: &Path, scene: &CompiledScene) -> Option<Self> {
        let bytes = std::fs::read(path).ok()?;
        if blake3::hash(&bytes).to_hex().as_str() != scene.source_hash {
            return None;
        }
        let text = std::str::from_utf8(&bytes).ok()?;
        let mut exchange = look::step::part21::parse(text).ok()?;
        if exchange.data.len() != 1 {
            return None;
        }
        let table = Table::from_owned_data_section(exchange.data.remove(0));
        let assembly = table.step_assy().ok()?;
        // Match Look's assembly traversal and definition slots. Flat multipart
        // exports are deliberately refused here rather than matched by names.
        for edge in assembly.all_edges() {
            Matrix4::try_from(edge.matrix()).ok()?;
        }
        let mapped = assembly.map(
            |node: &ProductEntity| node.clone(),
            |edge: &AssembleEntity| EdgeEntity {
                matrix: Matrix4::try_from(&edge.matrix).unwrap_or(Matrix4::from_scale(1.0)),
                attrs: edge.attrs.clone(),
            },
        );
        let mut definitions = HashMap::new();
        for node in mapped.all_nodes() {
            let mut shells = Vec::new();
            for shape in node.shape() {
                match shape {
                    ProductShape::Solid(solid, ids) => {
                        if !complete(&table, &solid.boundaries, ids) {
                            return None;
                        }
                        shells.extend(solid.boundaries.iter());
                    }
                    ProductShape::Shells(source_shells, ids) => {
                        if !complete(&table, source_shells, ids) {
                            return None;
                        }
                        shells.extend(source_shells.iter());
                    }
                    ProductShape::Matrix(_) => {}
                }
            }
            if !shells.is_empty() || node.is_terminal() {
                let boundary = AnalyticBoundary::from_shells(&shells).map(Arc::new);
                definitions.insert(node.index(), (definitions.len(), boundary));
            }
        }
        let normalization = DMat4::from_cols(
            DVec4::X,
            DVec4::new(0.0, 0.0, -1.0, 0.0),
            DVec4::Y,
            DVec4::W,
        );
        let mut occurrences = Vec::new();
        for top in mapped.top_nodes() {
            for path_in_graph in mapped.paths_iter(top.index()) {
                let terminal = path_in_graph.terminal_node();
                let Some((geometry, boundary)) = definitions.get(&terminal.index()) else {
                    continue;
                };
                let instance = scene.instances.get(occurrences.len())?;
                if instance.geometry != *geometry {
                    return None;
                }
                let source_world = matrix(&path_in_graph.matrix());
                let expected = glam::Mat4::from_rotation_x(-std::f32::consts::FRAC_PI_2)
                    * glam::Mat4::from_cols_array(
                        &source_world.to_cols_array().map(|value| value as f32),
                    );
                // Reproduce the importer's actual rounding and multiplication;
                // matching by a loose placement threshold could attach source
                // evidence to a different or subsequently moved occurrence.
                if expected.to_cols_array() != instance.transform.to_cols_array() {
                    return None;
                }
                let world = normalization * source_world;
                if !world.is_finite() {
                    return None;
                }
                // Own product, then nearest named ancestor occurrence/product.
                let mut name = None;
                for (index, node) in path_in_graph.nodes().iter().enumerate().rev() {
                    if meaningful(&node.entity().attrs.name) {
                        name = Some(node.entity().attrs.name.trim().to_owned());
                        break;
                    }
                    if index > 0 {
                        let edge = &path_in_graph.edges()[index - 1];
                        if meaningful(&edge.entity().attrs.name) {
                            name = Some(edge.entity().attrs.name.trim().to_owned());
                            break;
                        }
                    }
                }
                let name = name.unwrap_or_else(|| fallback_name(None, &path.to_string_lossy()));
                occurrences.push(SourceOccurrence {
                    name,
                    world,
                    boundary: boundary.clone(),
                });
            }
        }
        (occurrences.len() == scene.instances.len()).then_some(Self { occurrences })
    }

    pub fn contact(&self, left: usize, right: usize) -> Option<ContactProof> {
        let a = self.occurrences.get(left)?;
        let b = self.occurrences.get(right)?;
        let ab = a.boundary.as_ref()?;
        let bb = b.boundary.as_ref()?;
        for (occurrence, boundary) in [(a, ab), (b, bb)] {
            for &local_normal in &boundary.normals {
                let inverse = occurrence.world.inverse();
                let normal = inverse
                    .transpose()
                    .transform_vector3(local_normal)
                    .normalize();
                if !normal.is_finite() {
                    continue;
                }
                let (amin, amax) = ab.support(a.world, normal)?;
                let (bmin, bmax) = bb.support(b.world, normal)?;
                let gap = (bmin - amax).max(amin - bmax);
                let magnitude = amin
                    .abs()
                    .max(amax.abs())
                    .max(bmin.abs())
                    .max(bmax.abs())
                    .max(1.0);
                // Arithmetic precision only: never the float32 mesh/placement
                // tolerance or nominal tessellation scale. A genuinely shallow
                // source overlap above this bound stays unresolved.
                let error = magnitude * f64::EPSILON * 256.0;
                if gap.abs() <= error {
                    return Some(ContactProof { normal, gap, error });
                }
            }
        }
        None
    }
}

fn complete(table: &Table, shells: &[Shell], ids: &[u64]) -> bool {
    shells.len() == ids.len()
        && shells.iter().zip(ids).all(|(shell, id)| {
            table
                .shell
                .get(id)
                .is_some_and(|source| source.cfs_faces.len() == shell.faces.len())
        })
}

impl AnalyticBoundary {
    fn from_shells(shells: &[&Shell]) -> Option<Self> {
        let mut out = Self::default();
        for shell in shells {
            if shell.faces.is_empty() || shell.edges.is_empty() {
                return None;
            }
            out.points.extend(shell.vertices.iter().copied().map(point));
            for edge in &shell.edges {
                out.add_curve(&edge.curve)?;
            }
            for face in &shell.faces {
                if face.boundaries.is_empty() {
                    return None;
                }
                match &face.surface {
                    Surface::ElementarySurface(ElementarySurface::Plane(plane)) => {
                        out.normals.push(vector(plane.normal()))
                    }
                    Surface::ElementarySurface(ElementarySurface::CylindricalSurface(cylinder)) => {
                        let p0 = point(cylinder.subs(0.0, 0.0));
                        let p90 = point(cylinder.subs(0.0, std::f64::consts::FRAC_PI_2));
                        let p180 = point(cylinder.subs(0.0, std::f64::consts::PI));
                        let center = (p0 + p180) * 0.5;
                        let axis = (point(cylinder.subs(1.0, 0.0)) - p0).normalize();
                        if !axis.is_finite() {
                            return None;
                        }
                        let mut boundary = Self::default();
                        for edge_index in face.boundaries.iter().flatten() {
                            let edge = shell.edges.get(edge_index.index)?;
                            boundary.add_curve(&edge.curve)?;
                            boundary.points.extend([
                                point(*shell.vertices.get(edge.vertices.0)?),
                                point(*shell.vertices.get(edge.vertices.1)?),
                            ]);
                        }
                        let (min, max) = boundary.support(DMat4::IDENTITY, axis)?;
                        out.cylinders.push(CylinderSupport {
                            center,
                            radial: [p0 - center, p90 - center],
                            axis,
                            axial_range: (min - axis.dot(center), max - axis.dot(center)),
                        });
                    }
                    _ => return None,
                }
            }
        }
        Some(out)
    }

    fn add_curve(&mut self, curve: &Curve3D) -> Option<()> {
        match curve {
            Curve3D::Line(line) => self.points.extend([point(line.0), point(line.1)]),
            Curve3D::Conic(Conic3D::Circle(ellipse) | Conic3D::Ellipse(ellipse)) => {
                let m = matrix(ellipse.transform());
                self.ellipses.push([
                    m.w_axis.truncate(),
                    m.x_axis.truncate(),
                    m.y_axis.truncate(),
                ]);
            }
            Curve3D::BSplineCurve(curve) => self
                .points
                .extend(curve.control_points().iter().copied().map(point)),
            _ => return None,
        }
        Some(())
    }

    fn support(&self, world: DMat4, normal: DVec3) -> Option<(f64, f64)> {
        let direction = world.transpose().transform_vector3(normal);
        let translation = normal.dot(world.w_axis.truncate());
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        for point in &self.points {
            let d = direction.dot(*point) + translation;
            min = min.min(d);
            max = max.max(d);
        }
        for &[center, u, v] in &self.ellipses {
            let d = direction.dot(center) + translation;
            let radius = direction.dot(u).hypot(direction.dot(v));
            min = min.min(d - radius);
            max = max.max(d + radius);
        }
        // A linear functional on a bounded planar face reaches its extrema
        // on the boundary. A cylinder has no axial interior extrema: its
        // boundary bounds the axial interval. The full radial circle encloses
        // every trimmed patch, including extrema absent from mesh vertices.
        for cylinder in &self.cylinders {
            let center = direction.dot(cylinder.center) + translation;
            let radial = direction
                .dot(cylinder.radial[0])
                .hypot(direction.dot(cylinder.radial[1]));
            let axis = direction.dot(cylinder.axis);
            let a = axis * cylinder.axial_range.0;
            let b = axis * cylinder.axial_range.1;
            min = min.min(center + a.min(b) - radial);
            max = max.max(center + a.max(b) + radial);
        }
        (min.is_finite() && max.is_finite()).then_some((min, max))
    }
}

fn point(p: Point3) -> DVec3 {
    DVec3::new(p.x, p.y, p.z)
}
fn vector(p: truck_stepio::r#in::step_geometry::Vector3) -> DVec3 {
    DVec3::new(p.x, p.y, p.z)
}
fn matrix(m: &Matrix4) -> DMat4 {
    DMat4::from_cols_array_2d(&[m.x.into(), m.y.into(), m.z.into(), m.w.into()])
}

#[cfg(test)]
mod tests {
    use super::*;
    use look::{config::UpAxis, scene::compile_scene, timing::Timings};

    #[test]
    fn generic_names_use_nearest_named_path_or_file() {
        assert_eq!(
            fallback_name(Some("Frame/Body/SOLID"), "assembly.step"),
            "Frame"
        );
        assert_eq!(fallback_name(Some("Part"), "assembly.step"), "assembly");
        assert_eq!(fallback_name(Some("SOLID #2"), "assembly.step"), "assembly");
    }

    #[test]
    fn source_contact_proof_does_not_admit_positive_overlap() {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/interference/touching.step");
        let scene = compile_scene(&path, UpAxis::Z, &mut Timings::default()).unwrap();
        let evidence = SourceEvidence::read(&path, &scene).unwrap();
        assert!(evidence.contact(0, 1).is_some());
        let report =
            super::super::analyze_scene_from_source("touching.step", "fixture", &scene, &path);
        assert_eq!(report.contact_pairs.len(), 1);
        assert!(report.findings.is_empty());
        assert!(report.unresolved_pairs.is_empty());
        assert!(report.pair_set_complete);
        let mut source = evidence;
        // Moving by much less than float32 placement error still invalidates
        // the source separation proof. No mesh threshold is widened.
        source.occurrences[1].world.w_axis.x -= 1e-7;
        assert!(source.contact(0, 1).is_none());
    }
}
