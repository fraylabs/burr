use glam::{DMat4, DVec3};
use look::scene::CompiledScene;
use serde::Serialize;
#[path = "interference/mesh.rs"]
mod mesh;
use mesh::{Bounds, Mesh};

pub const CHECK_ID: &str = "assembly-interference";
const REPORT_SCHEMA_VERSION: &str = "burr.checks.v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckOutcome {
    Pass,
    Fail,
    Incomplete,
}

#[derive(Clone, Debug, Serialize)]
pub struct CheckReport {
    pub schema_version: &'static str,
    pub model_path: String,
    pub model_version: String,
    pub check_id: &'static str,
    pub outcome: CheckOutcome,
    pub summary: String,
    pub component_count: usize,
    pub checked_pair_count: usize,
    pub candidate_pair_count: usize,
    pub findings: Vec<InterferenceFinding>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub incomplete_reasons: Vec<IncompleteReason>,
}

#[derive(Clone, Debug, Serialize)]
pub struct IncompleteReason {
    pub code: &'static str,
    pub message: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct InterferenceFinding {
    pub id: String,
    pub code: &'static str,
    pub message: String,
    pub components: [ComponentRef; 2],
    pub witness: InterferenceWitness,
}

#[derive(Clone, Debug, Serialize)]
pub struct ComponentRef {
    pub id: String,
    pub occurrence_index: usize,
    pub name: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InterferenceWitness {
    InteriorOverlap {
        point: [f64; 3],
    },
    SurfaceCrossing {
        start: [f64; 3],
        end: [f64; 3],
    },
    Containment {
        point: [f64; 3],
        contained_component: String,
    },
    CoincidentOccurrence,
}

struct Component {
    reference: ComponentRef,
    geometry_index: usize,
    transform: DMat4,
    inverse: DMat4,
    scale: f64,
    bounds: Bounds,
}

impl CheckReport {
    pub fn unsupported(model_path: &str, model_version: &str, message: impl Into<String>) -> Self {
        Self {
            schema_version: REPORT_SCHEMA_VERSION,
            model_path: model_path.to_string(),
            model_version: model_version.to_string(),
            check_id: CHECK_ID,
            outcome: CheckOutcome::Incomplete,
            summary: "Interference check not completed".to_string(),
            component_count: 0,
            checked_pair_count: 0,
            candidate_pair_count: 0,
            findings: Vec::new(),
            incomplete_reasons: vec![IncompleteReason {
                code: "unsupported_model",
                message: message.into(),
            }],
        }
    }
}

pub fn analyze_scene(model_path: &str, model_version: &str, scene: &CompiledScene) -> CheckReport {
    // A flattened / single-part import must never enter mesh preparation.
    if scene.instances.len() < 2 {
        return incomplete(
            model_path,
            model_version,
            scene.instances.len(),
            "assembly_required",
            "The selected STEP file does not expose at least two component occurrences.".into(),
        );
    }
    let mut meshes: Vec<Option<Mesh>> = (0..scene.geometries.len()).map(|_| None).collect();
    let mut components = Vec::with_capacity(scene.instances.len());
    for (index, instance) in scene.instances.iter().enumerate() {
        let prepared: Result<Component, String> = (|| {
            let geometry = scene
                .geometries
                .get(instance.geometry)
                .ok_or_else(|| format!("references missing geometry {}", instance.geometry))?;
            if meshes[instance.geometry].is_none() {
                meshes[instance.geometry] = Some(Mesh::prepare(geometry)?);
            }
            let mesh = meshes[instance.geometry].as_ref().unwrap();
            let transform = instance.transform.as_dmat4();
            let inverse = transform.inverse();
            if !transform.is_finite() || !inverse.is_finite() {
                return Err("has a non-finite or singular transform".into());
            }
            let scales = [
                transform.x_axis.truncate().length(),
                transform.y_axis.truncate().length(),
                transform.z_axis.truncate().length(),
            ];
            // STEP occurrences are rigid (possibly uniformly scaled). Signed
            // distances cannot be converted by one scalar for a sheared mesh.
            let scale = scales.into_iter().fold(0.0, f64::max);
            let min_scale = scales.into_iter().fold(f64::INFINITY, f64::min);
            if scale - min_scale > scale * 1e-5
                || transform
                    .x_axis
                    .truncate()
                    .dot(transform.y_axis.truncate())
                    .abs()
                    > scale * scale * 1e-5
                || transform
                    .x_axis
                    .truncate()
                    .dot(transform.z_axis.truncate())
                    .abs()
                    > scale * scale * 1e-5
                || transform
                    .y_axis
                    .truncate()
                    .dot(transform.z_axis.truncate())
                    .abs()
                    > scale * scale * 1e-5
            {
                return Err("has a non-uniform scale or shear".into());
            }
            let name = instance
                .node_name
                .clone()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| format!("Component {}", index + 1));
            Ok(Component {
                reference: ComponentRef {
                    id: format!("occurrence:{index}"),
                    occurrence_index: index,
                    name,
                },
                geometry_index: instance.geometry,
                transform,
                inverse,
                scale,
                bounds: transformed_bounds(mesh.bounds, transform),
            })
        })();
        match prepared {
            Ok(component) => components.push(component),
            Err(message) => {
                return incomplete(
                    model_path,
                    model_version,
                    scene.instances.len(),
                    "invalid_component_mesh",
                    format!("Component occurrence {index} {message}."),
                )
            }
        }
    }

    let open_components = components
        .iter()
        .filter(|component| !meshes[component.geometry_index].as_ref().unwrap().closed)
        .map(|component| component.reference.name.clone())
        .collect::<Vec<_>>();
    let mut findings = Vec::new();
    // Sweep the axis with the widest spread. checked_pair_count includes
    // pairs rejected by the broad phase, preserving the report's meaning.
    let checked_pair_count = components.len() * (components.len() - 1) / 2;
    let candidates = candidate_pairs(&components);
    let candidate_pair_count = candidates.len();
    for (left_index, right_index) in candidates {
        let left = &components[left_index];
        let right = &components[right_index];
        let left_mesh = meshes[left.geometry_index].as_ref().unwrap();
        let right_mesh = meshes[right.geometry_index].as_ref().unwrap();
        if let Some(witness) = intersection_witness(left, left_mesh, right, right_mesh) {
            findings.push(InterferenceFinding {
                id: format!("{CHECK_ID}:{left_index}:{right_index}"),
                code: "component_interference",
                message: format!("{} overlaps {}.", left.reference.name, right.reference.name),
                components: [left.reference.clone(), right.reference.clone()],
                witness,
            });
        }
    }
    findings.sort_by_key(|finding| {
        (
            finding.components[0].occurrence_index,
            finding.components[1].occurrence_index,
        )
    });

    let incomplete_reasons = if open_components.is_empty() {
        Vec::new()
    } else {
        vec![IncompleteReason {
            code: "open_component_mesh",
            message: format!(
                "Could not prove a clean result because these tessellated components are not closed: {}.",
                open_components.join(", ")
            ),
        }]
    };
    let outcome = if !findings.is_empty() {
        CheckOutcome::Fail
    } else if incomplete_reasons.is_empty() {
        CheckOutcome::Pass
    } else {
        CheckOutcome::Incomplete
    };
    let summary = match outcome {
        CheckOutcome::Pass => {
            format!("No assembly interference detected across {checked_pair_count} pairs")
        }
        CheckOutcome::Fail => format!(
            "{} interfering component pair{} detected",
            findings.len(),
            if findings.len() == 1 { "" } else { "s" }
        ),
        CheckOutcome::Incomplete => "Interference check not completed".to_string(),
    };

    CheckReport {
        schema_version: REPORT_SCHEMA_VERSION,
        model_path: model_path.to_string(),
        model_version: model_version.to_string(),
        check_id: CHECK_ID,
        outcome,
        summary,
        component_count: components.len(),
        checked_pair_count,
        candidate_pair_count,
        findings,
        incomplete_reasons,
    }
}

fn incomplete(
    model_path: &str,
    model_version: &str,
    count: usize,
    code: &'static str,
    message: String,
) -> CheckReport {
    CheckReport {
        schema_version: REPORT_SCHEMA_VERSION,
        model_path: model_path.into(),
        model_version: model_version.into(),
        check_id: CHECK_ID,
        outcome: CheckOutcome::Incomplete,
        summary: "Interference check not completed".into(),
        component_count: count,
        checked_pair_count: 0,
        candidate_pair_count: 0,
        findings: Vec::new(),
        incomplete_reasons: vec![IncompleteReason { code, message }],
    }
}

fn transformed_bounds(bounds: Bounds, transform: DMat4) -> Bounds {
    let mut result = Bounds::empty();
    for x in [bounds.min.x, bounds.max.x] {
        for y in [bounds.min.y, bounds.max.y] {
            for z in [bounds.min.z, bounds.max.z] {
                result.add(transform.transform_point3(DVec3::new(x, y, z)));
            }
        }
    }
    result
}

fn candidate_pairs(components: &[Component]) -> Vec<(usize, usize)> {
    let mut bounds = Bounds::empty();
    for component in components {
        bounds.add(component.bounds.min);
        bounds.add(component.bounds.max);
    }
    let extent = bounds.max - bounds.min;
    let axis = if extent.x >= extent.y && extent.x >= extent.z {
        0
    } else if extent.y >= extent.z {
        1
    } else {
        2
    };
    let mut order: Vec<_> = (0..components.len()).collect();
    order.sort_unstable_by(|&a, &b| {
        components[a].bounds.min[axis].total_cmp(&components[b].bounds.min[axis])
    });
    let mut pairs = Vec::new();
    for (position, &a) in order.iter().enumerate() {
        for &b in &order[position + 1..] {
            if components[b].bounds.min[axis] >= components[a].bounds.max[axis] {
                break;
            }
            if components[a].bounds.overlaps(components[b].bounds, 0.0) {
                pairs.push((a.min(b), a.max(b)));
            }
        }
    }
    pairs
}

fn intersection_witness(
    left: &Component,
    left_mesh: &Mesh,
    right: &Component,
    right_mesh: &Mesh,
) -> Option<InterferenceWitness> {
    if left.geometry_index == right.geometry_index
        && left_mesh.closed
        && left
            .transform
            .to_cols_array()
            .into_iter()
            .zip(right.transform.to_cols_array())
            .all(|(a, b)| (a - b).abs() < 1e-9)
    {
        return Some(InterferenceWitness::CoincidentOccurrence);
    }
    // A strictly interior surface point proves positive overlap volume (a
    // neighborhood on the solid side also lies inside the other component).
    // Use the mesh's coordinate/weld accuracy here: a global chord-deflection
    // threshold would erase shallow real overlaps on small assembly parts.
    let tolerance = left_mesh.epsilon * left.scale + right_mesh.epsilon * right.scale;
    if !left.bounds.overlaps(right.bounds, tolerance) {
        return None;
    }
    penetrating_surface(left, left_mesh, right, right_mesh, tolerance)
        .or_else(|| penetrating_surface(right, right_mesh, left, left_mesh, tolerance))
}

fn penetrating_surface(
    source: &Component,
    source_mesh: &Mesh,
    target: &Component,
    target_mesh: &Mesh,
    tolerance: f64,
) -> Option<InterferenceWitness> {
    if !source_mesh.closed || !target_mesh.closed {
        return None;
    }
    let to_target = target.inverse * source.transform;
    let local_tolerance = (tolerance / target.scale).max(target_mesh.epsilon);
    let candidates = source_mesh.candidate_triangles(transformed_bounds(
        target_mesh.bounds,
        source.inverse * target.transform,
    ));
    // A source vertex inside the target is a cheap positive-volume witness.
    if let Some(&point) = source_mesh.points.first() {
        if target_mesh.inside(to_target.transform_point3(point), local_tolerance) {
            return Some(InterferenceWitness::Containment {
                point: source.transform.transform_point3(point).to_array(),
                contained_component: source.reference.id.clone(),
            });
        }
    }
    for triangle in candidates {
        let local = source_mesh.triangle(triangle);
        let vertices = local.map(|p| to_target.transform_point3(p));
        for point in vertices
            .into_iter()
            .chain([vertices.iter().sum::<DVec3>() / 3.0])
        {
            if target_mesh.inside(point, local_tolerance) {
                let world = target.transform.transform_point3(point).to_array();
                return Some(InterferenceWitness::InteriorOverlap { point: world });
            }
        }
        // Crossing solids need not contain each other's vertices. Split each
        // triangle edge at target surface hits and test the interior intervals.
        for [a, b] in [
            [vertices[0], vertices[1]],
            [vertices[1], vertices[2]],
            [vertices[2], vertices[0]],
        ] {
            let direction = b - a;
            if direction.length() <= 2.0 * local_tolerance {
                continue;
            }
            let mut cuts = target_mesh.ray_hits(a, direction, 1.0);
            cuts.insert(0, 0.0);
            cuts.push(1.0);
            for interval in cuts.windows(2) {
                if (interval[1] - interval[0]) * direction.length() <= 2.0 * local_tolerance {
                    continue;
                }
                let point = a + direction * ((interval[0] + interval[1]) * 0.5);
                if target_mesh.inside(point, local_tolerance) {
                    return Some(InterferenceWitness::SurfaceCrossing {
                        start: target
                            .transform
                            .transform_point3(a + direction * interval[0])
                            .to_array(),
                        end: target
                            .transform
                            .transform_point3(a + direction * interval[1])
                            .to_array(),
                    });
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use look::{config::UpAxis, scene::compile_scene, timing::Timings};
    use std::path::{Path, PathBuf};

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/interference")
            .join(name)
    }

    fn report(name: &str) -> CheckReport {
        let path = fixture(name);
        let mut timings = Timings::default();
        let scene = compile_scene(&path, UpAxis::Z, &mut timings).unwrap();
        analyze_scene(name, "fixture", &scene)
    }

    #[test]
    fn separated_assembly_passes() {
        let report = report("separated.step");
        assert_eq!(report.outcome, CheckOutcome::Pass);
        assert!(report.findings.is_empty());
        assert_eq!(report.component_count, 2);
        assert_eq!(report.checked_pair_count, 1);
    }

    #[test]
    fn touching_assembly_does_not_fail() {
        let report = report("touching.step");
        assert_eq!(report.outcome, CheckOutcome::Pass);
        assert!(report.findings.is_empty());
    }

    #[test]
    fn curved_mesh_contact_is_not_interference() {
        // Two valid robot-cover solids: OCCT Common has zero shared volume.
        let report = report("mesh-contact-pair.step");
        assert_eq!(report.component_count, 2);
        assert!(report.findings.is_empty(), "{:?}", report.findings);
        assert_eq!(report.outcome, CheckOutcome::Pass);
    }

    #[test]
    fn crossing_assembly_fails_with_component_references() {
        let report = report("intersecting.step");
        assert_eq!(report.outcome, CheckOutcome::Fail);
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].components[0].name, "fixed");
        assert_eq!(report.findings[0].components[1].name, "moving");
        assert!(matches!(
            report.findings[0].witness,
            InterferenceWitness::SurfaceCrossing { .. }
                | InterferenceWitness::InteriorOverlap { .. }
        ));
    }

    #[test]
    fn contained_component_is_interference() {
        let report = report("contained.step");
        assert_eq!(report.outcome, CheckOutcome::Fail);
        assert_eq!(report.findings.len(), 1);
        assert!(matches!(
            report.findings[0].witness,
            InterferenceWitness::Containment { .. }
        ));
    }

    #[test]
    fn single_part_is_incomplete_not_pass() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/viewer/models/enclosure/counterbore.step");
        let mut timings = Timings::default();
        let scene = compile_scene(&path, UpAxis::Z, &mut timings).unwrap();
        let report = analyze_scene("counterbore.step", "fixture", &scene);
        assert_eq!(report.outcome, CheckOutcome::Incomplete);
        assert_eq!(report.incomplete_reasons[0].code, "assembly_required");
    }

    #[test]
    fn single_occurrence_preflight_does_not_prepare_mesh() {
        let mut scene = compile_scene(
            &fixture("intersecting.step"),
            UpAxis::Z,
            &mut Timings::default(),
        )
        .unwrap();
        scene.instances.truncate(1);
        // The malformed mesh would fail preparation if preflight came later.
        let geometry = scene.instances[0].geometry;
        scene.geometries[geometry].indices[0] = u32::MAX;
        let report = analyze_scene("single.step", "fixture", &scene);
        assert_eq!(report.incomplete_reasons[0].code, "assembly_required");
    }

    #[test]
    fn many_occurrences_share_definition_preparation_and_prune_pairs() {
        let mut scene = compile_scene(
            &fixture("separated.step"),
            UpAxis::Z,
            &mut Timings::default(),
        )
        .unwrap();
        let mut instance = scene.instances[0].clone();
        scene.instances.clear();
        for i in 0..2000 {
            instance.transform =
                glam::Mat4::from_translation(glam::Vec3::new(i as f32 * 20.0, 0.0, 0.0));
            scene.instances.push(instance.clone());
        }
        let report = analyze_scene("many.step", "fixture", &scene);
        assert_eq!(report.outcome, CheckOutcome::Pass);
        assert_eq!(report.component_count, 2000);
        assert_eq!(report.checked_pair_count, 1_999_000);
        assert_eq!(report.candidate_pair_count, 0);
        instance.transform = scene.instances[0].transform;
        scene.instances.push(instance);
        let report = analyze_scene("many.step", "fixture", &scene);
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].components[0].occurrence_index, 0);
        assert_eq!(report.findings[0].components[1].occurrence_index, 2000);
    }

    #[test]
    fn rotated_crossing_without_contained_vertices_is_detected() {
        let mut scene = compile_scene(
            &fixture("separated.step"),
            UpAxis::Z,
            &mut Timings::default(),
        )
        .unwrap();
        let index = scene.instances[0].geometry;
        scene.instances[1].geometry = index;
        // A long narrow bar crossed by the same bar at ninety degrees. No
        // vertex of either bar lies inside the other.
        for vertex in &mut scene.geometries[index].vertices {
            vertex.position[0] *= 5.0;
            vertex.position[1] *= 0.2;
        }
        scene.instances[0].transform = glam::Mat4::IDENTITY;
        scene.instances[1].transform = glam::Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2);
        let report = analyze_scene("crossed-bars.step", "fixture", &scene);
        assert_eq!(report.outcome, CheckOutcome::Fail);
        assert_eq!(report.findings.len(), 1);
    }

    #[test]
    fn rotated_face_contact_with_overlapping_bounds_passes() {
        let mut scene = compile_scene(
            &fixture("separated.step"),
            UpAxis::Z,
            &mut Timings::default(),
        )
        .unwrap();
        let rotation = glam::Mat4::from_rotation_z(std::f32::consts::FRAC_PI_4);
        scene.instances[1].geometry = scene.instances[0].geometry;
        scene.instances[0].transform = rotation;
        scene.instances[1].transform =
            rotation * glam::Mat4::from_translation(glam::Vec3::new(10.0, 0.0, 0.0));
        let report = analyze_scene("rotated-contact.step", "fixture", &scene);
        assert_eq!(report.candidate_pair_count, 1);
        assert_eq!(report.outcome, CheckOutcome::Pass);
    }

    #[test]
    fn shallow_positive_volume_is_not_erased_by_assembly_chord_tolerance() {
        let mut scene = compile_scene(
            &fixture("separated.step"),
            UpAxis::Z,
            &mut Timings::default(),
        )
        .unwrap();
        scene.instances[1].geometry = scene.instances[0].geometry;
        scene.instances[0].transform = glam::Mat4::IDENTITY;
        scene.instances[1].transform =
            glam::Mat4::from_translation(glam::Vec3::new(9.999, 0.0, 0.0));
        let report = analyze_scene("shallow-overlap.step", "fixture", &scene);
        assert_eq!(report.outcome, CheckOutcome::Fail);
        assert_eq!(report.findings.len(), 1);
    }

    #[test]
    fn missing_vertex_index_is_incomplete_not_a_panic() {
        let path = fixture("intersecting.step");
        let mut timings = Timings::default();
        let mut scene = compile_scene(&path, UpAxis::Z, &mut timings).unwrap();
        let geometry_index = scene.instances[0].geometry;
        let missing_index = scene.geometries[geometry_index].vertices.len() as u32;
        scene.geometries[geometry_index].indices[0] = missing_index;

        let report = analyze_scene("invalid.step", "fixture", &scene);
        assert_eq!(report.outcome, CheckOutcome::Incomplete);
        assert_eq!(report.incomplete_reasons[0].code, "invalid_component_mesh");
        assert!(report.incomplete_reasons[0]
            .message
            .contains("references missing vertex index"));
    }
}
