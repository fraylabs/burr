//! Source BREP evidence, kept separate from float32 tessellation evidence.
//! Unsupported carriers or uncertain occurrence correspondence refuse contact.
use glam::{DMat4, DVec3, DVec4};
use look::scene::CompiledScene;
use ruststep::ast::{DataSection, EntityInstance, Name, Parameter};
use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, OnceLock},
};
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
    placement_depth: usize,
    absolute_placement: DMat4,
    boundary: Option<Arc<AnalyticBoundary>>,
}

#[derive(Default)]
struct AnalyticBoundary {
    points: Vec<DVec3>,
    ellipses: Vec<[DVec3; 3]>,
    ellipsoids: Vec<[DVec3; 4]>,
    cylinders: Vec<CylinderSupport>,
    normals: Vec<DVec3>,
    source_tolerance: Option<f64>,
    operand_magnitude: f64,
    // Initialized after construction; the shared boundary is immutable.
    local_magnitude: OnceLock<f64>,
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
    pub source_tolerance: f64,
    pub maximum_overlap_depth: f64,
    pub maximum_common_volume: f64,
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
        // Match Look's STEP decoder, including Latin-1 exporter comments and
        // names. A valid imported document must not lose its source ancestry
        // or contact evidence merely because it is not UTF-8.
        let text = std::str::from_utf8(&bytes)
            .map(std::borrow::Cow::Borrowed)
            .unwrap_or_else(|_| bytes.iter().map(|&b| b as char).collect::<String>().into());
        let mut exchange = look::step::part21::parse(&text).ok()?;
        if exchange.data.len() != 1 {
            return None;
        }
        let data = exchange.data.remove(0);
        let millimetre_source = millimetre_units(&data);
        let table = Table::from_owned_data_section(data);
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
            let mut complete_boundary = true;
            for shape in node.shape() {
                match shape {
                    ProductShape::Solid(solid, ids) => {
                        complete_boundary &= complete(&table, &solid.boundaries, ids);
                        shells.extend(solid.boundaries.iter());
                    }
                    ProductShape::Shells(source_shells, ids) => {
                        complete_boundary &= complete(&table, source_shells, ids);
                        shells.extend(source_shells.iter());
                    }
                    ProductShape::Matrix(_) => {}
                }
            }
            if !shells.is_empty() || node.is_terminal() {
                let boundary = (complete_boundary && millimetre_source)
                    .then(|| AnalyticBoundary::from_shells(&shells))
                    .flatten()
                    .map(Arc::new);
                definitions.insert(node.index(), (definitions.len(), boundary));
            }
        }
        let normalizations = [
            (glam::Mat4::IDENTITY, DMat4::IDENTITY),
            (
                glam::Mat4::from_rotation_x(-std::f32::consts::FRAC_PI_2),
                DMat4::from_cols(
                    DVec4::X,
                    DVec4::new(0.0, 0.0, -1.0, 0.0),
                    DVec4::Y,
                    DVec4::W,
                ),
            ),
            (
                glam::Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2),
                DMat4::from_cols(
                    DVec4::Y,
                    DVec4::new(-1.0, 0.0, 0.0, 0.0),
                    DVec4::Z,
                    DVec4::W,
                ),
            ),
        ];
        let mut selected_normalization = None;
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
                let rounded_source = glam::Mat4::from_cols_array(
                    &source_world.to_cols_array().map(|value| value as f32),
                );
                if selected_normalization.is_none() {
                    selected_normalization = normalizations.iter().copied().find(|(import, _)| {
                        (*import * rounded_source).to_cols_array()
                            == instance.transform.to_cols_array()
                    });
                }
                let (import_normalization, normalization) = selected_normalization?;
                let expected = import_normalization * rounded_source;
                // Reproduce the importer's actual rounding and multiplication;
                // matching by a loose placement threshold could attach source
                // evidence to a different or subsequently moved occurrence.
                if expected.to_cols_array() != instance.transform.to_cols_array() {
                    return None;
                }
                let world = normalization * source_world;
                let absolute_placement = absolute_matrix(normalization)
                    * path_in_graph
                        .edges()
                        .iter()
                        .fold(DMat4::IDENTITY, |bound, edge| {
                            bound * absolute_matrix(matrix(&edge.entity().matrix))
                        });
                if !world.is_finite() || !absolute_placement.is_finite() {
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
                    placement_depth: path_in_graph.edges().len(),
                    absolute_placement,
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
            let inverse_transpose = occurrence.world.inverse().transpose();
            for &local_normal in &boundary.normals {
                let normal = inverse_transpose
                    .transform_vector3(local_normal)
                    .normalize();
                if !normal.is_finite() {
                    continue;
                }
                let (amin, amax) = ab.support(a.world, normal)?;
                let (bmin, bmax) = bb.support(b.world, normal)?;
                let gap = (bmin - amax).max(amin - bmax);
                let error = ab.arithmetic_error(a.absolute_placement, normal, a.placement_depth)
                    + bb.arithmetic_error(b.absolute_placement, normal, b.placement_depth);
                let source_tolerance = match (ab.source_tolerance, bb.source_tolerance) {
                    (Some(at), Some(bt)) => (at
                        * a.world.transpose().transform_vector3(normal).length())
                    .min(bt * b.world.transpose().transform_vector3(normal).length()),
                    _ => 0.0,
                };
                let maximum_overlap_depth = (-gap + error).max(0.0);
                if !error.is_finite()
                    || !source_tolerance.is_finite()
                    || source_tolerance <= 0.0
                    || error > source_tolerance
                    || gap > error
                    || maximum_overlap_depth > source_tolerance
                {
                    continue;
                }
                // The intersection lies in a slab of this certified depth.
                // Slice along the dominant normal axis: each fiber has length
                // at most depth / |n_axis|. Analytic support in the other two
                // axes bounds the projected overlap area. This independent
                // volume upper bound prevents a large, shallow true overlap
                // from becoming contact merely because STEP admits its depth.
                let axis = if normal.x.abs() >= normal.y.abs() && normal.x.abs() >= normal.z.abs() {
                    0
                } else if normal.y.abs() >= normal.z.abs() {
                    1
                } else {
                    2
                };
                let mut area = 1.0;
                for index in 0..3 {
                    if index == axis {
                        continue;
                    }
                    let mut direction = DVec3::ZERO;
                    direction[index] = 1.0;
                    let (amin, amax) = ab.support(a.world, direction)?;
                    let (bmin, bmax) = bb.support(b.world, direction)?;
                    let axis_error =
                        ab.arithmetic_error(a.absolute_placement, direction, a.placement_depth)
                            + bb.arithmetic_error(
                                b.absolute_placement,
                                direction,
                                b.placement_depth,
                            );
                    let width = (amax.min(bmax) - amin.max(bmin) + 2.0 * axis_error).max(0.0);
                    area = (area * width).next_up();
                }
                let maximum_common_volume =
                    ((maximum_overlap_depth * area).next_up() / normal[axis].abs()).next_up();
                // The corpus exact gate's absolute positive-volume floor,
                // in source coordinate units cubed. This bounds contact only;
                // it never relaxes the existing interference witness policy.
                if maximum_common_volume <= 1e-6 {
                    return Some(ContactProof {
                        normal,
                        gap,
                        error,
                        source_tolerance,
                        maximum_overlap_depth,
                        maximum_common_volume,
                    });
                }
            }
        }
        None
    }
}

// The exact gate's absolute volume floor is in mm³. Never apply it to
// metres/inches, mixed or unspecified contexts as if those were millimetres.
// Refusing contact leaves source ancestry available for useful display names.
fn millimetre_units(data: &DataSection) -> bool {
    let mut lengths = std::collections::HashSet::new();
    for entity in &data.entities {
        if let EntityInstance::Complex { id, subsuper } = entity {
            let records = &subsuper.0;
            if !records.iter().any(|record| record.name == "LENGTH_UNIT") {
                continue;
            }
            let Some(unit) = records.iter().find(|record| record.name == "SI_UNIT") else {
                return false;
            };
            let Parameter::List(parameters) = &unit.parameter else {
                return false;
            };
            if !matches!(parameters.as_slice(), [Parameter::Enumeration(prefix), Parameter::Enumeration(unit)]
                if prefix == "MILLI" && unit == "METRE")
            {
                return false;
            }
            if records
                .iter()
                .any(|record| record.name == "CONVERSION_BASED_UNIT")
            {
                return false;
            }
            lengths.insert(*id);
        }
    }
    if lengths.is_empty() {
        return false;
    }
    let mut contexts = 0;
    for entity in &data.entities {
        match entity {
            EntityInstance::Complex { subsuper, .. } => {
                let records = &subsuper.0;
                if !records
                    .iter()
                    .any(|record| record.name == "GEOMETRIC_REPRESENTATION_CONTEXT")
                {
                    continue;
                }
                // PCURVE parameter domains have unitless 2D contexts.
                if records
                    .iter()
                    .any(|record| record.name == "PARAMETRIC_REPRESENTATION_CONTEXT")
                {
                    continue;
                }
                let Some(context) = records
                    .iter()
                    .find(|record| record.name == "GLOBAL_UNIT_ASSIGNED_CONTEXT")
                else {
                    return false;
                };
                let Parameter::List(parameters) = &context.parameter else {
                    return false;
                };
                let [Parameter::List(units)] = parameters.as_slice() else {
                    return false;
                };
                if !units.iter().any(
                    |unit| matches!(unit, Parameter::Ref(Name::Entity(id)) if lengths.contains(id)),
                ) {
                    return false;
                }
                contexts += 1;
            }
            EntityInstance::Simple { record, .. }
                if record.name == "UNCERTAINTY_MEASURE_WITH_UNIT" =>
            {
                let Parameter::List(parameters) = &record.parameter else {
                    return false;
                };
                if !matches!(parameters.get(1), Some(Parameter::Ref(Name::Entity(id))) if lengths.contains(id))
                {
                    return false;
                }
                if let Some(Parameter::Typed { keyword, .. }) = parameters.first() {
                    if keyword != "LENGTH_MEASURE" {
                        return false;
                    }
                }
            }
            _ => {}
        }
    }
    contexts > 0
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
        let source_tolerance = shells
            .iter()
            .map(|shell| shell.source_geometric_uncertainty)
            .collect::<Option<Vec<_>>>()
            .and_then(|values| {
                if values.is_empty()
                    || values
                        .iter()
                        .any(|value| !value.is_finite() || *value <= 0.0)
                {
                    return None;
                }
                values.into_iter().reduce(f64::min)
            });
        let mut out = Self {
            source_tolerance,
            ..Self::default()
        };
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
                        let normal = vector(plane.normal()).normalize();
                        if !normal.is_finite() {
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
                        let (min, max) = boundary.support(DMat4::IDENTITY, normal)?;
                        let origin = point(plane.origin());
                        out.operand_magnitude = out.operand_magnitude.max(
                            origin.abs().max_element()
                                + vector(plane.u_axis()).length()
                                + vector(plane.v_axis()).length(),
                        );
                        let offset = normal.dot(origin);
                        let error = boundary.arithmetic_error(DMat4::IDENTITY, normal, 0)
                            + origin.abs().max_element().max(1.0) * f64::EPSILON * 256.0;
                        // A planar support proof needs a coherent planar trim,
                        // not a boundary displaced within a healing tolerance.
                        if !error.is_finite()
                            || (min - offset).abs().max((max - offset).abs()) > error
                        {
                            return None;
                        }
                        if !out.normals.contains(&normal) && !out.normals.contains(&-normal) {
                            out.normals.push(normal);
                        }
                    }
                    Surface::ElementarySurface(ElementarySurface::Sphere(sphere)) => {
                        let m = matrix(sphere.transform());
                        let center = m.transform_point3(point(sphere.entity().0.center()));
                        let radius = sphere.entity().0.radius();
                        if !radius.is_finite() || radius <= 0.0 {
                            return None;
                        }
                        // The complete analytic sphere (an ellipsoid after an
                        // affine placement) contains every source trim.
                        out.ellipsoids.push([
                            center,
                            m.x_axis.truncate() * radius,
                            m.y_axis.truncate() * radius,
                            m.z_axis.truncate() * radius,
                        ]);
                    }
                    Surface::ElementarySurface(ElementarySurface::CylindricalSurface(cylinder)) => {
                        // STEP's cylinder processor reverses surface orientation
                        // by swapping u/v. Read the underlying revolution so
                        // the axial and angular parameters cannot be confused.
                        let placement = matrix(cylinder.transform());
                        let sample =
                            |u, v| placement.transform_point3(point(cylinder.entity().subs(u, v)));
                        let p0 = sample(0.0, 0.0);
                        let p90 = sample(0.0, std::f64::consts::FRAC_PI_2);
                        let p180 = sample(0.0, std::f64::consts::PI);
                        let center = (p0 + p180) * 0.5;
                        // Read the authored direction rather than subtracting
                        // two translated samples: that subtraction can lose or
                        // rotate the axis at large carrier coordinates. Axial
                        // projection requires a rigid carrier frame; refuse a
                        // sheared/scaled processor rather than assume its
                        // radial directions remain perpendicular to the axis.
                        let axis = rigid_carrier_axis(placement, vector(cylinder.entity().axis()))?;
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
        if !out.operand_magnitude.is_finite()
            || !out.points.iter().all(|point| point.is_finite())
            || !out
                .ellipses
                .iter()
                .flatten()
                .all(|vector| vector.is_finite())
            || !out
                .ellipsoids
                .iter()
                .flatten()
                .all(|vector| vector.is_finite())
            || !out.cylinders.iter().all(|cylinder| {
                cylinder.center.is_finite()
                    && cylinder.radial.iter().all(|vector| vector.is_finite())
                    && cylinder.axis.is_finite()
                    && cylinder.axial_range.0.is_finite()
                    && cylinder.axial_range.1.is_finite()
            })
        {
            return None;
        }
        let local = out.compute_local_magnitude();
        if !local.is_finite() {
            return None;
        }
        out.local_magnitude.set(local).ok()?;
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

    fn compute_local_magnitude(&self) -> f64 {
        let mut local = self
            .points
            .iter()
            .map(|point| point.abs().max_element())
            .fold(self.operand_magnitude, f64::max);
        for &[center, u, v] in &self.ellipses {
            local = local.max(center.abs().max_element() + u.length() + v.length());
        }
        for &[center, u, v, w] in &self.ellipsoids {
            local = local.max(center.abs().max_element() + u.length() + v.length() + w.length());
        }
        for cylinder in &self.cylinders {
            local = local.max(
                cylinder.center.abs().max_element()
                    + cylinder.radial[0].length()
                    + cylinder.radial[1].length()
                    + cylinder
                        .axial_range
                        .0
                        .abs()
                        .max(cylinder.axial_range.1.abs()),
            );
        }
        local
    }

    fn arithmetic_error(&self, world: DMat4, direction: DVec3, placement_depth: usize) -> f64 {
        // Bound arithmetic using operand magnitudes, not just the final
        // coordinates: large local coordinates can cancel a large placement.
        // `world` is the product of absolute source placement matrices.
        // Intermediate placement cancellation cannot shrink this operand bound.
        // Account for that chain as well as carrier/support evaluation.
        let local = *self
            .local_magnitude
            .get_or_init(|| self.compute_local_magnitude());
        let operands = world.x_axis.truncate().abs() * local
            + world.y_axis.truncate().abs() * local
            + world.z_axis.truncate().abs() * local
            + world.w_axis.truncate().abs();
        let magnitude = direction.abs().dot(operands);
        if !magnitude.is_finite() {
            return f64::INFINITY;
        }
        magnitude.max(1.0) * f64::EPSILON * (256.0 + 64.0 * placement_depth as f64)
    }

    fn support(&self, world: DMat4, normal: DVec3) -> Option<(f64, f64)> {
        let direction = world.transpose().transform_vector3(normal);
        let translation = normal.dot(world.w_axis.truncate());
        if !direction.is_finite() || !translation.is_finite() {
            return None;
        }
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        for point in &self.points {
            let d = direction.dot(*point) + translation;
            if !d.is_finite() {
                return None;
            }
            min = min.min(d);
            max = max.max(d);
        }
        for &[center, u, v] in &self.ellipses {
            let d = direction.dot(center) + translation;
            let radius = direction.dot(u).hypot(direction.dot(v));
            if !d.is_finite() || !radius.is_finite() {
                return None;
            }
            min = min.min(d - radius);
            max = max.max(d + radius);
        }
        for &[center, u, v, w] in &self.ellipsoids {
            let d = direction.dot(center) + translation;
            let radius = direction
                .dot(u)
                .hypot(direction.dot(v))
                .hypot(direction.dot(w));
            if !d.is_finite() || !radius.is_finite() {
                return None;
            }
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
            let lower = center + a.min(b) - radial;
            let upper = center + a.max(b) + radial;
            if !lower.is_finite() || !upper.is_finite() {
                return None;
            }
            min = min.min(lower);
            max = max.max(upper);
        }
        (min.is_finite() && max.is_finite()).then_some((min, max))
    }
}

fn absolute_matrix(matrix: DMat4) -> DMat4 {
    DMat4::from_cols_array(&matrix.to_cols_array().map(f64::abs))
}

fn rigid_carrier_axis(placement: DMat4, local_axis: DVec3) -> Option<DVec3> {
    let columns = [
        placement.x_axis.truncate(),
        placement.y_axis.truncate(),
        placement.z_axis.truncate(),
    ];
    let error = 16.0 * f64::EPSILON;
    if !placement.is_finite()
        || !local_axis.is_finite()
        || placement.x_axis.w != 0.0
        || placement.y_axis.w != 0.0
        || placement.z_axis.w != 0.0
        || placement.w_axis.w != 1.0
        || (local_axis.length_squared() - 1.0).abs() > error
        || columns
            .iter()
            .any(|axis| (axis.length_squared() - 1.0).abs() > error)
        || columns[0].dot(columns[1]).abs() > error
        || columns[0].dot(columns[2]).abs() > error
        || columns[1].dot(columns[2]).abs() > error
    {
        return None;
    }
    let axis = placement.transform_vector3(local_axis).normalize();
    axis.is_finite().then_some(axis)
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
    fn source_names_and_contact_survive_every_supported_up_axis() {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/interference/touching.step");
        for axis in [UpAxis::X, UpAxis::Y, UpAxis::Z] {
            let scene = compile_scene(&path, axis, &mut Timings::default()).unwrap();
            let source = SourceEvidence::read(&path, &scene).unwrap();
            assert_eq!(source.occurrences[0].name, "fixed");
            assert!(source.contact(0, 1).is_some());
        }
    }

    #[test]
    fn latin1_source_preserves_names_and_contact_evidence() {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/interference/touching.step");
        let text = std::fs::read_to_string(path).unwrap();
        assert!(text.is_ascii());
        let renamed = text.replace("'fixed'", "'é fixed'");
        assert_ne!(text, renamed);
        let bytes = renamed.chars().map(|c| c as u8).collect::<Vec<_>>();
        let file = tempfile::Builder::new()
            .suffix(".step")
            .tempfile_in(std::env::temp_dir())
            .unwrap();
        std::fs::write(file.path(), bytes).unwrap();
        let scene = compile_scene(file.path(), UpAxis::Z, &mut Timings::default()).unwrap();
        let source = SourceEvidence::read(file.path(), &scene).unwrap();
        assert_eq!(source.occurrences[0].name, "é fixed");
        assert!(source.contact(0, 1).is_some());
    }

    #[test]
    fn contact_volume_floor_requires_declared_millimetres() {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/interference/touching.step");
        let text = std::fs::read_to_string(path).unwrap();
        let exchange = look::step::part21::parse(&text).unwrap();
        assert!(millimetre_units(&exchange.data[0]));
        let metres = text.replace(".MILLI.,.METRE.", "$,.METRE.");
        assert_ne!(metres, text);
        let exchange = look::step::part21::parse(&metres).unwrap();
        assert!(!millimetre_units(&exchange.data[0]));
    }

    #[test]
    fn nonfinite_support_arithmetic_is_refused() {
        let boundary = AnalyticBoundary {
            points: vec![DVec3::ZERO, DVec3::splat(f64::MAX)],
            ..Default::default()
        };
        let scale = DMat4::from_scale(DVec3::splat(2.0));
        assert!(boundary
            .support(scale, DVec3::new(1.0, -1.0, 0.0))
            .is_none());
        assert!(boundary.arithmetic_error(scale, DVec3::X, 1).is_infinite());
    }

    #[test]
    fn cancelled_placements_keep_their_arithmetic_uncertainty() {
        let forward = DMat4::from_translation(DVec3::splat(1e16));
        let backward = DMat4::from_translation(DVec3::splat(-1e16));
        assert_eq!((forward * backward).w_axis, DVec4::W);
        let operands = absolute_matrix(forward) * absolute_matrix(backward);
        assert_eq!(operands.w_axis.x, 2e16);
        let boundary = AnalyticBoundary {
            source_tolerance: Some(1e-7),
            points: vec![DVec3::ZERO, DVec3::ONE],
            ..Default::default()
        };
        assert!(boundary.arithmetic_error(operands, DVec3::X, 2) > 1e-7);
    }

    #[test]
    fn cylinder_support_keeps_axial_and_angular_parameters_distinct() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/interference/curved-contact.step");
        let scene = compile_scene(&path, UpAxis::Z, &mut Timings::default()).unwrap();
        let evidence = SourceEvidence::read(&path, &scene).unwrap();
        let cylinders = evidence
            .occurrences
            .iter()
            .filter_map(|occurrence| occurrence.boundary.as_ref())
            .flat_map(|boundary| boundary.cylinders.iter())
            .collect::<Vec<_>>();
        assert!(!cylinders.is_empty());
        for cylinder in cylinders {
            assert!(cylinder.radial[0].length() > 0.0);
            assert!((cylinder.radial[0].length() - cylinder.radial[1].length()).abs() < 1e-10);
            assert!(cylinder.radial[0].dot(cylinder.axis).abs() < 1e-10);
            assert!(cylinder.radial[1].dot(cylinder.axis).abs() < 1e-10);
            assert!(cylinder.radial[0].dot(cylinder.radial[1]).abs() < 1e-10);
        }
    }

    #[test]
    fn carrier_axis_survives_translation_and_refuses_shear() {
        let placement = DMat4::from_translation(DVec3::splat(1e16));
        assert_eq!(rigid_carrier_axis(placement, DVec3::Z), Some(DVec3::Z));
        let mut shear = DMat4::IDENTITY;
        shear.z_axis.x = 0.1;
        assert!(rigid_carrier_axis(shear, DVec3::Z).is_none());
        assert!(rigid_carrier_axis(DMat4::from_scale(DVec3::splat(2.0)), DVec3::Z).is_none());
    }

    #[test]
    fn source_support_refuses_overlap_above_precision_or_volume_floor() {
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
        // Even inside the declared length tolerance, the broad common area
        // makes this 2e-8-deep overlap larger than the exact volume floor.
        source.occurrences[1].world.w_axis.x += 8e-8;
        assert!(source.contact(0, 1).is_none());
    }
}
