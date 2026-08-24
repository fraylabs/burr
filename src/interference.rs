use look::scene::CompiledScene;
use serde::Serialize;
use truck_meshalgo::prelude::{
    Collision, Faces, IncludingPointInDomain, OptimizingFilter, Point3, PolygonMesh,
    ShellCondition, StandardAttributes, Topology,
};

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

struct ComponentMesh {
    reference: ComponentRef,
    geometry_index: usize,
    transform: [f32; 16],
    mesh: PolygonMesh,
    bounds: WorldBounds,
    closed: bool,
}

#[derive(Clone, Copy)]
struct WorldBounds {
    min: [f64; 3],
    max: [f64; 3],
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
            findings: Vec::new(),
            incomplete_reasons: vec![IncompleteReason {
                code: "unsupported_model",
                message: message.into(),
            }],
        }
    }
}

pub fn analyze_scene(model_path: &str, model_version: &str, scene: &CompiledScene) -> CheckReport {
    let components = scene
        .instances
        .iter()
        .enumerate()
        .map(|(index, instance)| component_mesh(scene, index, instance))
        .collect::<Result<Vec<_>, _>>();
    let components = match components {
        Ok(components) => components,
        Err(message) => {
            return CheckReport {
                schema_version: REPORT_SCHEMA_VERSION,
                model_path: model_path.to_string(),
                model_version: model_version.to_string(),
                check_id: CHECK_ID,
                outcome: CheckOutcome::Incomplete,
                summary: "Interference check not completed".to_string(),
                component_count: scene.instances.len(),
                checked_pair_count: 0,
                findings: Vec::new(),
                incomplete_reasons: vec![IncompleteReason {
                    code: "invalid_component_mesh",
                    message,
                }],
            };
        }
    };

    if components.len() < 2 {
        return CheckReport {
            schema_version: REPORT_SCHEMA_VERSION,
            model_path: model_path.to_string(),
            model_version: model_version.to_string(),
            check_id: CHECK_ID,
            outcome: CheckOutcome::Incomplete,
            summary: "Interference check requires a STEP assembly".to_string(),
            component_count: components.len(),
            checked_pair_count: 0,
            findings: Vec::new(),
            incomplete_reasons: vec![IncompleteReason {
                code: "assembly_required",
                message:
                    "The selected STEP file does not expose at least two component occurrences."
                        .to_string(),
            }],
        };
    }

    let open_components = components
        .iter()
        .filter(|component| !component.closed)
        .map(|component| component.reference.name.clone())
        .collect::<Vec<_>>();
    let mut findings = Vec::new();
    let mut checked_pair_count = 0;
    for left_index in 0..components.len() {
        for right_index in (left_index + 1)..components.len() {
            let left = &components[left_index];
            let right = &components[right_index];
            if !left.bounds.strictly_overlaps(right.bounds) {
                checked_pair_count += 1;
                continue;
            }
            checked_pair_count += 1;
            if let Some(witness) = intersection_witness(left, right) {
                findings.push(InterferenceFinding {
                    id: format!("{CHECK_ID}:{left_index}:{right_index}"),
                    code: "component_interference",
                    message: format!("{} overlaps {}.", left.reference.name, right.reference.name),
                    components: [left.reference.clone(), right.reference.clone()],
                    witness,
                });
            }
        }
    }

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
        findings,
        incomplete_reasons,
    }
}

fn component_mesh(
    scene: &CompiledScene,
    occurrence_index: usize,
    instance: &look::scene::Instance,
) -> Result<ComponentMesh, String> {
    let geometry = scene.geometries.get(instance.geometry).ok_or_else(|| {
        format!(
            "Component occurrence {occurrence_index} references missing geometry {}.",
            instance.geometry
        )
    })?;
    if geometry.indices.len() % 3 != 0 {
        return Err(format!(
            "Component occurrence {occurrence_index} has a non-triangular index buffer."
        ));
    }
    let positions = geometry
        .vertices
        .iter()
        .map(|vertex| {
            let point = instance
                .transform
                .transform_point3(glam::Vec3::from_array(vertex.position));
            Point3::new(f64::from(point.x), f64::from(point.y), f64::from(point.z))
        })
        .collect::<Vec<_>>();
    if positions.is_empty() {
        return Err(format!(
            "Component occurrence {occurrence_index} contains no vertices."
        ));
    }
    if let Some(index) = geometry
        .indices
        .iter()
        .copied()
        .find(|index| *index as usize >= geometry.vertices.len())
    {
        return Err(format!(
            "Component occurrence {occurrence_index} references missing vertex index {index}."
        ));
    }
    let faces = geometry
        .indices
        .as_chunks::<3>()
        .0
        .iter()
        .map(|triangle| {
            [
                triangle[0] as usize,
                triangle[1] as usize,
                triangle[2] as usize,
            ]
        })
        .collect::<Faces>();
    let bounds = WorldBounds::from_positions(&positions);
    let mut mesh = PolygonMesh::new(
        StandardAttributes {
            positions,
            ..Default::default()
        },
        faces,
    );
    let weld_tolerance = (bounds.diagonal() * 1.0e-8).max(1.0e-9);
    mesh.put_together_same_attrs(weld_tolerance);
    let closed = mesh.shell_condition() == ShellCondition::Closed;
    let name = instance
        .node_name
        .clone()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| format!("Component {}", occurrence_index + 1));

    Ok(ComponentMesh {
        reference: ComponentRef {
            id: format!("occurrence:{occurrence_index}"),
            occurrence_index,
            name,
        },
        geometry_index: instance.geometry,
        transform: instance.transform.to_cols_array(),
        mesh,
        bounds,
        closed,
    })
}

fn intersection_witness(
    left: &ComponentMesh,
    right: &ComponentMesh,
) -> Option<InterferenceWitness> {
    if left.geometry_index == right.geometry_index
        && transforms_match(left.transform, right.transform)
    {
        return Some(InterferenceWitness::CoincidentOccurrence);
    }
    if let Some((start, end)) = left.mesh.collide_with(&right.mesh) {
        return Some(InterferenceWitness::SurfaceCrossing {
            start: point_array(start),
            end: point_array(end),
        });
    }
    if left.closed && right.closed {
        if let Some(point) = left.mesh.positions().first().copied() {
            if right.mesh.inside(point) {
                return Some(InterferenceWitness::Containment {
                    point: point_array(point),
                    contained_component: left.reference.id.clone(),
                });
            }
        }
        if let Some(point) = right.mesh.positions().first().copied() {
            if left.mesh.inside(point) {
                return Some(InterferenceWitness::Containment {
                    point: point_array(point),
                    contained_component: right.reference.id.clone(),
                });
            }
        }
    }
    None
}

fn transforms_match(left: [f32; 16], right: [f32; 16]) -> bool {
    left.into_iter()
        .zip(right)
        .all(|(left, right)| (left - right).abs() <= 1.0e-6)
}

fn point_array(point: Point3) -> [f64; 3] {
    [point.x, point.y, point.z]
}

impl WorldBounds {
    fn from_positions(positions: &[Point3]) -> Self {
        let mut min = [f64::INFINITY; 3];
        let mut max = [f64::NEG_INFINITY; 3];
        for point in positions {
            min[0] = min[0].min(point.x);
            min[1] = min[1].min(point.y);
            min[2] = min[2].min(point.z);
            max[0] = max[0].max(point.x);
            max[1] = max[1].max(point.y);
            max[2] = max[2].max(point.z);
        }
        Self { min, max }
    }

    fn diagonal(self) -> f64 {
        ((self.max[0] - self.min[0]).powi(2)
            + (self.max[1] - self.min[1]).powi(2)
            + (self.max[2] - self.min[2]).powi(2))
        .sqrt()
    }

    fn strictly_overlaps(self, other: Self) -> bool {
        let scale = self.diagonal().max(other.diagonal()).max(1.0);
        let tolerance = scale * 1.0e-7;
        (0..3).all(|axis| {
            self.max[axis].min(other.max[axis]) - self.min[axis].max(other.min[axis]) > tolerance
        })
    }
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
    fn crossing_assembly_fails_with_component_references() {
        let report = report("intersecting.step");
        assert_eq!(report.outcome, CheckOutcome::Fail);
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].components[0].name, "fixed");
        assert_eq!(report.findings[0].components[1].name, "moving");
        assert!(matches!(
            report.findings[0].witness,
            InterferenceWitness::SurfaceCrossing { .. }
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
