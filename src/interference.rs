use glam::{DMat4, DVec3};
use look::scene::CompiledScene;
use serde::Serialize;
#[path = "interference/mesh.rs"]
mod mesh;
#[path = "interference/source.rs"]
mod source;
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
    /// False even when the overall verdict is Fail if some pairs are unresolved.
    pub pair_set_complete: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub unresolved_pairs: Vec<UnresolvedPair>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub contact_pairs: Vec<ContactPair>,
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
pub struct UnresolvedPair {
    pub id: String,
    pub code: &'static str,
    pub message: String,
    pub components: [ComponentRef; 2],
    pub coordinate_error_bound: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tessellation_probe_resolution: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mesh_witness: Option<InterferenceWitness>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ContactPair {
    pub id: String,
    pub code: &'static str,
    pub message: String,
    pub components: [ComponentRef; 2],
    pub separating_normal: [f64; 3],
    pub signed_gap: f64,
    pub arithmetic_error_bound: f64,
    pub source_tolerance: f64,
    pub maximum_overlap_depth: f64,
    pub maximum_common_volume: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct ComponentRef {
    pub id: String,
    pub occurrence_index: usize,
    pub name: String,
    pub definition_id: usize,
    pub definition_name: String,
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
    coordinate_error: f64,
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
            pair_set_complete: false,
            unresolved_pairs: Vec::new(),
            contact_pairs: Vec::new(),
            incomplete_reasons: vec![IncompleteReason {
                code: "unsupported_model",
                message: message.into(),
            }],
        }
    }
}

#[cfg(test)]
pub fn analyze_scene(model_path: &str, model_version: &str, scene: &CompiledScene) -> CheckReport {
    analyze_scene_from_source(
        model_path,
        model_version,
        scene,
        std::path::Path::new(model_path),
    )
}

pub fn analyze_scene_from_source(
    model_path: &str,
    model_version: &str,
    scene: &CompiledScene,
    source_path: &std::path::Path,
) -> CheckReport {
    // Import completeness is known before any mesh preparation or pair work.
    // Keep both diagnoses when structure and face geometry were lost together.
    let mut import_reasons = scene
        .assembly_structure_errors
        .iter()
        .map(|message| IncompleteReason {
            code: "assembly_structure_lost",
            message: message.clone(),
        })
        .collect::<Vec<_>>();
    if let Some(stats) = scene
        .statistics
        .step_import
        .filter(|stats| stats.lost_faces > 0)
    {
        import_reasons.push(IncompleteReason {
            code: "step_faces_lost",
            message: format!(
                "{} of {} STEP faces were lost during import",
                stats.lost_faces, stats.declared_faces
            ),
        });
    }
    if !import_reasons.is_empty() {
        return CheckReport {
            schema_version: REPORT_SCHEMA_VERSION,
            model_path: model_path.to_string(),
            model_version: model_version.to_string(),
            check_id: CHECK_ID,
            outcome: CheckOutcome::Incomplete,
            summary: format!(
                "Interference check incomplete: {}",
                import_reasons
                    .iter()
                    .map(|reason| reason.message.as_str())
                    .collect::<Vec<_>>()
                    .join("; ")
            ),
            component_count: scene.instances.len(),
            checked_pair_count: 0,
            candidate_pair_count: 0,
            findings: Vec::new(),
            pair_set_complete: false,
            unresolved_pairs: Vec::new(),
            contact_pairs: Vec::new(),
            incomplete_reasons: import_reasons,
        };
    }
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
    let source = source::SourceEvidence::read(source_path, scene);
    let definition_names = scene
        .instances
        .iter()
        .enumerate()
        .map(|(index, instance)| {
            source
                .as_ref()
                .and_then(|source| source.occurrences.get(index))
                .map(|occurrence| occurrence.name.clone())
                .unwrap_or_else(|| source::fallback_name(instance.node_name.as_deref(), model_path))
        })
        .collect::<Vec<_>>();
    let mut name_counts = std::collections::HashMap::new();
    for name in &definition_names {
        *name_counts.entry(name.as_str()).or_insert(0usize) += 1;
    }
    let mut siblings = std::collections::HashMap::new();
    let names = definition_names
        .iter()
        .map(|name| {
            let sibling = siblings.entry(name.as_str()).or_insert(0usize);
            *sibling += 1;
            if name_counts[name.as_str()] > 1 {
                format!("{name} #{sibling}")
            } else {
                name.clone()
            }
        })
        .collect::<Vec<_>>();
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
            let mesh = meshes
                .get(instance.geometry)
                .and_then(Option::as_ref)
                .ok_or_else(|| "prepared mesh is missing".to_string())?;
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
            let definition_name = definition_names[index].clone();
            let name = names[index].clone();
            Ok(Component {
                reference: ComponentRef {
                    id: format!("occurrence:{index}"),
                    occurrence_index: index,
                    name,
                    definition_id: instance.geometry,
                    definition_name,
                },
                geometry_index: instance.geometry,
                transform,
                inverse,
                scale,
                bounds: transformed_bounds(mesh.bounds, transform),
                // Both definition coordinates and occurrence placement are f32.
                // Casting the matrix to f64 cannot recover lost placement bits.
                // Include cancellation between large local coordinates and
                // translations, not only the final world-coordinate magnitude.
                coordinate_error: mesh.epsilon * scale
                    + 2.0
                        * f64::from(f32::EPSILON)
                        * (mesh
                            .bounds
                            .min
                            .abs()
                            .max(mesh.bounds.max.abs())
                            .max_element()
                            * scale
                            + transform.w_axis.truncate().abs().max_element()),
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
                );
            }
        }
    }

    let mut open_components = Vec::new();
    // Resolve every occurrence's prepared mesh once, before checking topology
    // or pairs. The narrow phase then holds references, not optional meshes.
    let mut component_meshes = Vec::with_capacity(components.len());
    for component in &components {
        let Some(mesh) = meshes
            .get(component.geometry_index)
            .and_then(Option::as_ref)
        else {
            return incomplete(
                model_path,
                model_version,
                scene.instances.len(),
                "invalid_component_mesh",
                format!(
                    "Prepared mesh for component occurrence {} is missing.",
                    component.reference.occurrence_index
                ),
            );
        };
        if !mesh.closed {
            open_components.push(component.reference.name.clone());
        }
        component_meshes.push(mesh);
    }
    let mut findings = Vec::new();
    let mut unresolved_pairs = Vec::new();
    let mut contact_pairs = Vec::new();
    // Sweep the axis with the widest spread. checked_pair_count includes
    // pairs rejected by the broad phase, preserving the report's meaning.
    let checked_pair_count = components.len() * (components.len() - 1) / 2;
    let candidates = candidate_pairs(&components);
    let candidate_pair_count = candidates.len();
    let mut source_bounds = Bounds::empty();
    for mesh in meshes.iter().flatten() {
        source_bounds.add(mesh.bounds.min);
        source_bounds.add(mesh.bounds.max);
    }
    let source_resolution = 2.0
        * source_bounds.diagonal()
        * look::step::meshing_policy::MeshingPolicy::DEFAULT.relative_linear_deflection;
    for (left_index, right_index) in candidates {
        let left = &components[left_index];
        let right = &components[right_index];
        let left_mesh = component_meshes[left_index];
        let right_mesh = component_meshes[right_index];
        let tolerance = left.coordinate_error + right.coordinate_error;
        // Edge intervals narrower than the mesher's nominal sampling scale
        // cannot establish reliable surface crossing near curved boundaries.
        // Near nonplanar mesh patches, interior samples also need separation
        // beyond this scale. Planar shallow overlaps retain coordinate accuracy.
        let mut probe_resolution = (left_mesh.bounds.diagonal() * left.scale
            + right_mesh.bounds.diagonal() * right.scale)
            * look::step::meshing_policy::MeshingPolicy::DEFAULT.relative_linear_deflection;
        if !left_mesh.oriented && !right_mesh.oriented {
            probe_resolution = probe_resolution.max(source_resolution);
        }
        // Additional broad-phase candidates are for source contact only.
        // Keep the interference narrow phase and unresolved policy unchanged
        // for every previously checked positive-bounds pair.
        if !left.bounds.overlaps(right.bounds, 0.0) {
            if left_mesh.closed && right_mesh.closed {
                if let Some(pair) = source
                    .as_ref()
                    .and_then(|source| contact_pair(source, left, right, left_index, right_index))
                {
                    contact_pairs.push(pair);
                }
            }
            continue;
        }
        if !left_mesh.closed || !right_mesh.closed {
            unresolved_pairs.push(UnresolvedPair {
                id: format!("{CHECK_ID}:unresolved:{left_index}:{right_index}"),
                code: "open_component_mesh",
                message: format!("Could not resolve {} against {} because a tessellated component has an open boundary.",left.reference.name,right.reference.name),
                components: [left.reference.clone(),right.reference.clone()],
                coordinate_error_bound: tolerance,
                tessellation_probe_resolution: None,
                mesh_witness: None,
            });
            continue;
        }
        if let Some(witness) = intersection_witness(
            left,
            left_mesh,
            right,
            right_mesh,
            tolerance,
            probe_resolution,
        ) {
            findings.push(InterferenceFinding {
                id: format!("{CHECK_ID}:{left_index}:{right_index}"),
                code: "component_interference",
                message: format!("{} overlaps {}.", left.reference.name, right.reference.name),
                components: [left.reference.clone(), right.reference.clone()],
                witness,
            });
        } else {
            let local_accuracy = left_mesh.epsilon * left.scale + right_mesh.epsilon * right.scale;
            let mesh_witness =
                intersection_witness(left, left_mesh, right, right_mesh, local_accuracy, 0.0);
            let overlap =
                left.bounds.max.min(right.bounds.max) - left.bounds.min.max(right.bounds.min);
            if mesh_witness.is_some() || overlap.min_element() <= tolerance {
                if let Some(pair) = source
                    .as_ref()
                    .and_then(|source| contact_pair(source, left, right, left_index, right_index))
                {
                    contact_pairs.push(pair);
                    continue;
                }
                let below_sampling = mesh_witness.as_ref().is_some_and(|witness| {
                    let world = match witness {
                        InterferenceWitness::SurfaceCrossing { start, end } => {
                            let a = DVec3::from_array(*start);
                            let b = DVec3::from_array(*end);
                            if a.distance(b) <= probe_resolution {
                                return true;
                            }
                            // A long tangential interval can still have only
                            // sub-resolution penetration at its midpoint.
                            (a + b) * 0.5
                        }
                        InterferenceWitness::InteriorOverlap { point }
                        | InterferenceWitness::Containment { point, .. } => {
                            DVec3::from_array(*point)
                        }
                        InterferenceWitness::CoincidentOccurrence => return false,
                    };
                    let a = left.inverse.transform_point3(world);
                    let b = right.inverse.transform_point3(world);
                    let ta = probe_resolution / left.scale;
                    let tb = probe_resolution / right.scale;
                    left_mesh.near_surface(a, ta)
                        && right_mesh.near_surface(b, tb)
                        && (left_mesh.nonplanar_near_surface(a, ta)
                            || right_mesh.nonplanar_near_surface(b, tb))
                });
                unresolved_pairs.push(UnresolvedPair {
                id: format!("{CHECK_ID}:unresolved:{left_index}:{right_index}"),
                    code: if below_sampling { "below_tessellation_resolution" } else { "below_coordinate_resolution" },
                    message: if below_sampling {
                        format!("{} and {} have only overlap evidence below the mesh sampling resolution near nonplanar boundaries; no reliable positive volume was proven.",left.reference.name,right.reference.name)
                    } else { format!("{} and {} have a possible overlap below the coordinate and placement resolution; no positive volume was proven.",left.reference.name,right.reference.name) },
                    components: [left.reference.clone(),right.reference.clone()],
                    coordinate_error_bound: tolerance,
                tessellation_probe_resolution: below_sampling.then_some(probe_resolution),
                    mesh_witness,
                });
            }
        }
    }
    findings.sort_by_key(|finding| {
        (
            finding.components[0].occurrence_index,
            finding.components[1].occurrence_index,
        )
    });

    unresolved_pairs.sort_by_key(|pair| {
        (
            pair.components[0].occurrence_index,
            pair.components[1].occurrence_index,
        )
    });
    contact_pairs.sort_by_key(|pair| {
        (
            pair.components[0].occurrence_index,
            pair.components[1].occurrence_index,
        )
    });
    let mut incomplete_reasons = if open_components.is_empty() {
        Vec::new()
    } else {
        vec![IncompleteReason {
            code: "open_component_mesh",
            message: format!(
                "Could not prove a clean result because these tessellated components do not form closed solids: {}.",
                open_components.join(", ")
            ),
        }]
    };
    if unresolved_pairs
        .iter()
        .any(|pair| pair.code == "below_coordinate_resolution")
    {
        incomplete_reasons.push(IncompleteReason {
            code: "below_coordinate_resolution",
            message: "Some possible overlaps are smaller than the coordinate and placement resolution. See the unresolved component pairs.".into(),
        });
    }
    if unresolved_pairs
        .iter()
        .any(|pair| pair.code == "below_tessellation_resolution")
    {
        incomplete_reasons.push(IncompleteReason {
            code: "below_tessellation_resolution",
            message: "Some overlap evidence is below the nominal mesh sampling scale. See the unresolved component pairs.".into(),
        });
    }
    let pair_set_complete = incomplete_reasons.is_empty() && unresolved_pairs.is_empty();
    let outcome = if !findings.is_empty() {
        CheckOutcome::Fail
    } else if incomplete_reasons.is_empty() {
        CheckOutcome::Pass
    } else {
        CheckOutcome::Incomplete
    };
    let mut summary = match outcome {
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

    if !unresolved_pairs.is_empty() {
        summary.push_str(&format!(
            "; {} component pair{} unresolved",
            unresolved_pairs.len(),
            if unresolved_pairs.len() == 1 { "" } else { "s" }
        ));
    }
    if !contact_pairs.is_empty() {
        summary.push_str(&format!(
            "; {} contact or separated component pair{}",
            contact_pairs.len(),
            if contact_pairs.len() == 1 { "" } else { "s" }
        ));
    }
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
        pair_set_complete,
        unresolved_pairs,
        contact_pairs,
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
        pair_set_complete: false,
        unresolved_pairs: Vec::new(),
        contact_pairs: Vec::new(),
        incomplete_reasons: vec![IncompleteReason { code, message }],
    }
}

fn contact_pair(
    source: &source::SourceEvidence,
    left: &Component,
    right: &Component,
    left_index: usize,
    right_index: usize,
) -> Option<ContactPair> {
    let proof = source.contact(left_index, right_index)?;
    Some(ContactPair {
        id: format!("{CHECK_ID}:contact:{left_index}:{right_index}"),
        code: "source_non_interference",
        message: format!("{} and {} are in contact or separated. Analytic support bounds any shared interior to a depth of {:.3e} mm and a volume of at most {:.3e} mm³; no interference above that precision is possible.", left.reference.name, right.reference.name, proof.maximum_overlap_depth, proof.maximum_common_volume),
        components: [left.reference.clone(), right.reference.clone()],
        separating_normal: proof.normal.to_array(),
        signed_gap: proof.gap,
        arithmetic_error_bound: proof.error,
        source_tolerance: proof.source_tolerance,
        maximum_overlap_depth: proof.maximum_overlap_depth,
        maximum_common_volume: proof.maximum_common_volume,
    })
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
    let max_error = components
        .iter()
        .map(|component| component.coordinate_error)
        .fold(0.0, f64::max);
    let mut pairs = Vec::new();
    for (position, &a) in order.iter().enumerate() {
        for &b in &order[position + 1..] {
            if components[b].bounds.min[axis]
                > components[a].bounds.max[axis] + components[a].coordinate_error + max_error
            {
                break;
            }
            if components[a].bounds.overlaps(
                components[b].bounds,
                -(components[a].coordinate_error + components[b].coordinate_error),
            ) {
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
    tolerance: f64,
    tessellation_resolution: f64,
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
    // The caller includes both local coordinate and occurrence-placement
    // accuracy. Sub-resolution witnesses are retained as unresolved pairs.
    if !left.bounds.overlaps(right.bounds, tolerance) {
        return None;
    }
    penetrating_surface(
        left,
        left_mesh,
        right,
        right_mesh,
        tolerance,
        tessellation_resolution,
    )
    .or_else(|| {
        penetrating_surface(
            right,
            right_mesh,
            left,
            left_mesh,
            tolerance,
            tessellation_resolution,
        )
    })
}

fn penetrating_surface(
    source: &Component,
    source_mesh: &Mesh,
    target: &Component,
    target_mesh: &Mesh,
    tolerance: f64,
    tessellation_resolution: f64,
) -> Option<InterferenceWitness> {
    if !source_mesh.closed || !target_mesh.closed {
        return None;
    }
    let to_target = target.inverse * source.transform;
    let unoriented_pair =
        !source_mesh.oriented && !target_mesh.oriented && tessellation_resolution > 0.0;
    let uncertain_margin = if unoriented_pair {
        tessellation_resolution * 0.5
    } else {
        0.0
    };
    let local_tolerance = (tolerance / target.scale).max(target_mesh.epsilon);
    let source_tolerance = (tolerance / source.scale).max(source_mesh.epsilon);
    let to_source = source.inverse * target.transform;
    let nominal_resolution = (source_mesh.bounds.diagonal() * source.scale
        + target_mesh.bounds.diagonal() * target.scale)
        * look::step::meshing_policy::MeshingPolicy::DEFAULT.relative_linear_deflection;
    let reliable_inside = |point: DVec3, triangle: Option<usize>| {
        if !target_mesh.inside(point, local_tolerance) {
            return false;
        }
        let source_point = to_source.transform_point3(point);
        let source_sample = triangle
            .map(|t| source_mesh.triangle_surface_sample(t))
            .unwrap_or_else(|| source_mesh.nearest_surface_sample(source_point));
        let target_sample = target_mesh.nearest_surface_sample(point);
        // A flat carrier can have a curved trimmed boundary. Its own normal
        // sample has zero deviation, but neighboring curved facets can move
        // that trim into the other solid. Include their measured chord error;
        // a nearby patch does not make the entire planar component curved.
        let deflection =
            look::step::meshing_policy::MeshingPolicy::DEFAULT.relative_linear_deflection;
        let source_deviation =
            if source_sample.0 {
                source_sample.1
            } else {
                source_sample.1.max(source_mesh.nearby_surface_deviation(
                    source_point,
                    source_mesh.bounds.diagonal() * deflection,
                ))
            };
        let target_deviation = target_sample.1;
        let curved =
            source_sample.0 || target_sample.0 || source_deviation > 0.0 || target_deviation > 0.0;
        // Budget each side independently before adding the errors. A sampled
        // curved carrier keeps its nominal bound; a flat trimmed carrier adds
        // only the measured nearby trim deviation, never its whole extent.
        let source_resolution = if source_sample.0 {
            source_mesh.bounds.diagonal() * source.scale * deflection
        } else {
            0.0
        };
        let target_resolution = if target_sample.0 {
            target_mesh.bounds.diagonal() * target.scale * deflection
        } else {
            0.0
        };
        let local_resolution = source_resolution.max(source_deviation * source.scale)
            + target_resolution.max(target_deviation * target.scale)
            + tolerance;
        let uncertain = unoriented_pair && curved;
        let target_margin = if uncertain {
            local_tolerance.max(uncertain_margin / target.scale)
        } else {
            local_tolerance
        };
        target_mesh.inside(point, target_margin)
            && (!uncertain
                || source_mesh.inside(
                    source_point,
                    source_tolerance.max(uncertain_margin / source.scale),
                ))
            && !(tessellation_resolution > 0.0
                && curved
                && target_mesh.near_surface(point, local_resolution / target.scale))
            && !(tessellation_resolution > 0.0
                && target_mesh.near_surface(point, nominal_resolution / target.scale)
                && (target_mesh
                    .legacy_nonplanar_near_surface(point, nominal_resolution / target.scale)
                    || source_mesh.legacy_nonplanar_near_surface(
                        source_point,
                        nominal_resolution / source.scale,
                    )))
    };
    let candidates = source_mesh.candidate_triangles(transformed_bounds(
        target_mesh.bounds,
        source.inverse * target.transform,
    ));
    // A source vertex inside the target is a cheap positive-volume witness.
    if let Some(&point) = source_mesh.points.first() {
        if reliable_inside(to_target.transform_point3(point), None) {
            return Some(InterferenceWitness::Containment {
                point: source.transform.transform_point3(point).to_array(),
                contained_component: source.reference.id.clone(),
            });
        }
    }
    for &triangle in &candidates {
        let local = source_mesh.triangle(triangle);
        let vertices = local.map(|p| to_target.transform_point3(p));
        for point in vertices
            .into_iter()
            .chain([vertices.iter().sum::<DVec3>() / 3.0])
        {
            if reliable_inside(point, Some(triangle)) {
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
                if (interval[1] - interval[0]) * direction.length()
                    <= (2.0 * local_tolerance).max(tessellation_resolution / target.scale)
                {
                    continue;
                }
                let point = a + direction * ((interval[0] + interval[1]) * 0.5);
                if reliable_inside(point, Some(triangle)) {
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
    // Keep the existing surface/crossing witness priority and pay for interior
    // probes only when surface samples cannot establish overlap.
    for triangle in candidates {
        let local = source_mesh.triangle(triangle);
        // Coincident surfaces have no strictly interior surface sample. Probe
        // both sides because tessellation winding need not point outward, and
        // accept only a point confirmed strictly inside both closed meshes.
        let [a, b, c] = local;
        let normal = (b - a).cross(c - a).normalize_or_zero();
        let centroid = (a + b + c) / 3.0;
        for sign in [-1.0, 1.0] {
            let source_sample = source_mesh.triangle_surface_sample(triangle);
            let target_sample =
                target_mesh.nearest_surface_sample(to_target.transform_point3(centroid));
            let margin = if unoriented_pair && (source_sample.0 || target_sample.0) {
                let uncertainty = uncertain_margin;
                source_tolerance.max(uncertainty / source.scale)
            } else {
                source_tolerance
            };
            let probe = centroid + normal * (sign * 4.0 * margin);
            if source_mesh.inside(probe, source_tolerance)
                && reliable_inside(to_target.transform_point3(probe), Some(triangle))
            {
                return Some(InterferenceWitness::InteriorOverlap {
                    point: source.transform.transform_point3(probe).to_array(),
                });
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
        analyze_scene_from_source(name, "fixture", &scene, &path)
    }

    #[test]
    fn gearmotor_contact_never_reports_interference() {
        let result = report("gearmotor-contact.step");
        assert_eq!(result.component_count, 2);
        assert!(result.findings.is_empty(), "{result:#?}");
        assert_eq!(result.outcome, CheckOutcome::Incomplete);
        assert_eq!(result.unresolved_pairs.len(), 1);
        assert_eq!(
            result.unresolved_pairs[0].code,
            "below_tessellation_resolution"
        );
    }

    #[test]
    fn gearmotor_overlap_beyond_trim_error_is_interference() {
        let mut scene = compile_scene(
            &fixture("gearmotor-contact.step"),
            UpAxis::Z,
            &mut Timings::default(),
        )
        .unwrap();
        scene.instances[0].transform = glam::Mat4::from_translation(glam::Vec3::new(0.5, 0.0, 0.0))
            * scene.instances[0].transform;
        let result = analyze_scene("gearmotor-overlap.step", "fixture", &scene);
        assert_eq!(result.outcome, CheckOutcome::Fail, "{result:#?}");
        assert_eq!(result.findings.len(), 1);
    }

    #[test]
    fn structure_and_face_losses_are_reported_before_mesh_preparation() {
        let mut scene = compile_scene(
            &fixture("separated.step"),
            UpAxis::Z,
            &mut Timings::default(),
        )
        .unwrap();
        scene
            .assembly_structure_errors
            .push("Unresolved occurrence #42".into());
        scene.statistics.step_import = Some(look::step::StepImportStats {
            declared_faces: 13,
            lost_faces: 1,
        });
        // Mesh preparation would reject this geometry reference. The import
        // preflight must return both source diagnoses without reaching it.
        scene.instances[0].geometry = usize::MAX;
        let report = analyze_scene("broken.step", "fixture", &scene);
        assert_eq!(report.outcome, CheckOutcome::Incomplete);
        assert_eq!(report.component_count, 2);
        assert_eq!(report.checked_pair_count, 0);
        assert_eq!(report.candidate_pair_count, 0);
        assert!(report.findings.is_empty());
        assert_eq!(report.incomplete_reasons.len(), 2);
        assert_eq!(report.incomplete_reasons[0].code, "assembly_structure_lost");
        assert_eq!(report.incomplete_reasons[1].code, "step_faces_lost");
        assert!(report.summary.contains("1 of 13 STEP faces"));
    }

    #[test]
    fn lost_assembly_structure_is_incomplete_before_mesh_preparation() {
        let mut scene = compile_scene(
            &fixture("separated.step"),
            UpAxis::Z,
            &mut Timings::default(),
        )
        .unwrap();
        scene
            .assembly_structure_errors
            .push("Unresolved product occurrence #42".into());
        scene.instances[0].geometry = usize::MAX;
        let report = analyze_scene("broken.step", "v1", &scene);
        assert_eq!(report.outcome, CheckOutcome::Incomplete);
        assert_eq!(report.checked_pair_count, 0);
        assert_eq!(report.incomplete_reasons[0].code, "assembly_structure_lost");
        assert!(report.incomplete_reasons[0].message.contains("#42"));
    }

    #[test]
    fn unresolved_source_occurrence_reports_import_reason() {
        let source = std::fs::read_to_string(fixture("separated.step")).unwrap();
        let source = source.replace(
            "#376 = NEXT_ASSEMBLY_USAGE_OCCURRENCE('1','fixed','',#5,#31,$);",
            "#376 = NEXT_ASSEMBLY_USAGE_OCCURRENCE('1','fixed','',#5,#99999,$);",
        );
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("broken.step");
        std::fs::write(&path, source).unwrap();
        let scene = compile_scene(&path, UpAxis::Z, &mut Timings::default()).unwrap();
        let report = analyze_scene("broken.step", "fixture", &scene);
        assert_eq!(report.outcome, CheckOutcome::Incomplete);
        assert_eq!(report.incomplete_reasons[0].code, "assembly_structure_lost");
        assert!(report.incomplete_reasons[0].message.contains("assembly"));
    }

    #[test]
    fn lost_step_faces_make_a_closed_separated_assembly_incomplete() {
        let path = fixture("separated.step");
        let mut timings = Timings::default();
        let mut scene = compile_scene(&path, UpAxis::Z, &mut timings).unwrap();
        scene.statistics.step_import = Some(look::step::StepImportStats {
            declared_faces: 13,
            lost_faces: 1,
        });
        scene.instances[0].geometry = usize::MAX;
        let report = analyze_scene("separated.step", "fixture", &scene);
        assert_eq!(report.checked_pair_count, 0);
        assert_eq!(report.candidate_pair_count, 0);
        assert_eq!(report.outcome, CheckOutcome::Incomplete);
        assert_eq!(report.incomplete_reasons[0].code, "step_faces_lost");
        assert!(report.summary.contains("1 of 13 STEP faces"));
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
    fn duplicate_solid_definitions_at_same_placement_fail() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut scene = compile_scene(
            &fixture("separated.step"),
            UpAxis::Z,
            &mut Timings::default(),
        )?;
        let original = scene.instances[0].geometry;
        let duplicate = scene.geometries.len();
        scene.geometries.push(scene.geometries[original].clone());
        scene.instances[1].geometry = duplicate;
        for instance in &mut scene.instances {
            instance.transform = glam::Mat4::IDENTITY;
        }
        let report = analyze_scene("duplicate-definitions.step", "fixture", &scene);
        assert_eq!(report.outcome, CheckOutcome::Fail);
        assert_eq!(report.findings.len(), 1);
        assert!(matches!(
            report.findings[0].witness,
            InterferenceWitness::InteriorOverlap { .. }
        ));
        Ok(())
    }

    #[test]
    fn self_mapping_cube_rotation_fails() -> Result<(), Box<dyn std::error::Error>> {
        let mut scene = compile_scene(
            &fixture("separated.step"),
            UpAxis::Z,
            &mut Timings::default(),
        )?;
        let definition = scene.instances[0].geometry;
        scene.instances[1].geometry = definition;
        let mut bounds = Bounds::empty();
        for vertex in &scene.geometries[definition].vertices {
            bounds.add(DVec3::from_array(vertex.position.map(f64::from)));
        }
        let center = ((bounds.min + bounds.max) * 0.5).as_vec3();
        scene.instances[0].transform = glam::Mat4::IDENTITY;
        scene.instances[1].transform = glam::Mat4::from_translation(center)
            * glam::Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2)
            * glam::Mat4::from_translation(-center);
        let report = analyze_scene("self-mapping-cube.step", "fixture", &scene);
        assert_eq!(report.outcome, CheckOutcome::Fail);
        assert_eq!(report.findings.len(), 1);
        assert!(matches!(
            report.findings[0].witness,
            InterferenceWitness::InteriorOverlap { .. }
        ));
        Ok(())
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
    fn relaxed_planar_overlap_reports_coordinate_uncertainty() {
        let mut scene = compile_scene(
            &fixture("separated.step"),
            UpAxis::Z,
            &mut Timings::default(),
        )
        .unwrap();
        let geometry = scene.instances[0].geometry;
        scene.instances[1].geometry = geometry;
        scene.geometries[geometry].surface_normals = None;
        scene.geometries[geometry].indices.swap(0, 1);
        assert!(!Mesh::prepare(&scene.geometries[geometry]).unwrap().oriented);
        scene.instances[0].transform =
            glam::Mat4::from_translation(glam::Vec3::new(1_000_000.0, 0.0, 0.0));
        scene.instances[1].transform =
            glam::Mat4::from_translation(glam::Vec3::new(1_000_000.0 + 9.875, 0.0, 0.0));
        let report = analyze_scene("planar-placement-uncertainty.step", "fixture", &scene);
        assert!(report.findings.is_empty());
        assert_eq!(report.unresolved_pairs.len(), 1);
        assert!(matches!(
            report.unresolved_pairs[0].mesh_witness,
            Some(InterferenceWitness::InteriorOverlap { .. })
                | Some(InterferenceWitness::Containment { .. })
        ));
        assert_eq!(
            report.unresolved_pairs[0].code,
            "below_coordinate_resolution"
        );
        assert!(report.unresolved_pairs[0]
            .tessellation_probe_resolution
            .is_none());
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
    fn unoriented_planar_overlap_keeps_coordinate_accuracy() {
        let mut scene = compile_scene(
            &fixture("separated.step"),
            UpAxis::Z,
            &mut Timings::default(),
        )
        .unwrap();
        scene.instances[1].geometry = scene.instances[0].geometry;
        scene.instances[0].transform = glam::Mat4::IDENTITY;
        scene.instances[1].transform = glam::Mat4::from_translation(glam::Vec3::new(0.1, 0.0, 0.0));
        let geometry = scene.instances[0].geometry;
        assert_eq!(
            scene.geometries[geometry]
                .surface_normals
                .as_ref()
                .unwrap()
                .len(),
            scene.geometries[geometry].vertices.len()
        );
        scene.geometries[geometry].surface_normals = None;
        scene.geometries[geometry].indices.swap(0, 1);
        assert!(!Mesh::prepare(&scene.geometries[geometry]).unwrap().oriented);
        let thick = analyze_scene("unoriented-thick.step", "fixture", &scene);
        assert_eq!(
            thick.findings.len(),
            1,
            "a resolved interior still proves overlap"
        );
        for vertex in &mut scene.geometries[geometry].vertices {
            vertex.position[2] *= 0.0001;
        }
        let thin = analyze_scene("unoriented-thin.step", "fixture", &scene);
        assert_eq!(
            thin.findings.len(),
            1,
            "planar overlap retains coordinate accuracy"
        );
    }

    #[test]
    fn phase_offset_cylindrical_contact_is_unresolved_not_interference() {
        let report = report("curved-contact.step");
        assert_eq!(report.component_count, 2);
        assert!(report.findings.is_empty());
        assert_eq!(report.outcome, CheckOutcome::Incomplete);
        assert_eq!(report.unresolved_pairs.len(), 1);
        assert_eq!(
            report.unresolved_pairs[0].code,
            "below_tessellation_resolution"
        );
    }

    #[test]
    fn curved_overlap_beyond_sampling_resolution_is_still_confirmed() {
        let mut scene = compile_scene(
            &fixture("curved-contact.step"),
            UpAxis::Z,
            &mut Timings::default(),
        )
        .unwrap();
        scene.instances[1].transform *= glam::Mat4::from_scale(glam::Vec3::splat(1.05));
        let report = analyze_scene("curved-overlap.step", "fixture", &scene);
        assert_eq!(report.outcome, CheckOutcome::Fail);
        assert_eq!(report.findings.len(), 1);
    }

    #[test]
    fn a_large_planar_extent_does_not_hide_a_resolved_curved_overlap() {
        let mut scene = compile_scene(
            &fixture("multiscale-planar.step"),
            UpAxis::Z,
            &mut Timings::default(),
        )
        .unwrap();
        assert_eq!(scene.instances.len(), 2);
        let report = analyze_scene("multiscale-planar.step", "fixture", &scene);
        assert_eq!(report.outcome, CheckOutcome::Fail);
        assert_eq!(report.findings.len(), 1);
        // The same exact surfaces become tangent after removing the authored
        // 0.2 mm overlap; the curved-side margin must still reject contact.
        scene.instances[1].transform = glam::Mat4::from_translation(glam::Vec3::new(0.2, 0.0, 0.0))
            * scene.instances[1].transform;
        let contact = analyze_scene("multiscale-planar-contact.step", "fixture", &scene);
        assert!(contact.findings.is_empty());
    }

    #[test]
    fn float_rounding_far_from_the_origin_does_not_open_a_closed_definition() {
        let mut scene = compile_scene(
            &fixture("separated.step"),
            UpAxis::Z,
            &mut Timings::default(),
        )
        .unwrap();
        let definition = scene.instances[0].geometry;
        scene.instances[1].geometry = definition;
        scene.instances[0].transform = glam::Mat4::IDENTITY;
        scene.instances[1].transform =
            glam::Mat4::from_translation(glam::Vec3::new(20.0, 0.0, 0.0));
        let vertices = &mut scene.geometries[definition].vertices;
        for vertex in vertices.iter_mut() {
            vertex.position[0] += 1000.0;
        }
        // Two tessellated faces can round a shared endpoint to adjacent f32
        // values. The uncertainty follows the coordinate's magnitude.
        vertices[0].position[0] = f32::from_bits(vertices[0].position[0].to_bits() + 1);
        let report = analyze_scene("offset-rounding.step", "fixture", &scene);
        assert_eq!(report.outcome, CheckOutcome::Pass);
    }

    #[test]
    fn coincident_double_sided_face_is_not_solid_interference() {
        let mut scene = compile_scene(
            &fixture("separated.step"),
            UpAxis::Z,
            &mut Timings::default(),
        )
        .unwrap();
        let definition = scene.instances[0].geometry;
        for instance in &mut scene.instances {
            instance.geometry = definition;
            instance.transform = glam::Mat4::IDENTITY;
        }
        let geometry = &mut scene.geometries[definition];
        geometry.vertices.truncate(3);
        for (vertex, position) in
            geometry
                .vertices
                .iter_mut()
                .zip([[0.0, 0.0, 0.0], [1.0, 0.0, 1.0], [0.0, 1.0, 1.0]])
        {
            vertex.position = position;
        }
        geometry.indices = vec![0, 1, 2, 0, 2, 1];
        // Every edge has two incident faces and all three AABB extents are
        // positive, but this doubled sloping face encloses no volume.
        let report = analyze_scene("double-sided-face.step", "fixture", &scene);
        assert_eq!(report.outcome, CheckOutcome::Incomplete);
        assert!(report.findings.is_empty());
    }

    #[test]
    fn narrow_crossings_below_mesh_sampling_are_kept_as_unresolved() {
        let mut scene = compile_scene(
            &fixture("separated.step"),
            UpAxis::Z,
            &mut Timings::default(),
        )
        .unwrap();
        let definition = scene.instances[0].geometry;
        scene.instances[1].geometry = definition;
        let geometry = &mut scene.geometries[definition];
        let template = geometry.vertices[0];
        // Deliberately use only box corners: a tessellator's face-centre
        // vertex could provide a stronger, genuine interior witness.
        geometry.vertices = [
            [-25., -0.0005, -5.],
            [25., -0.0005, -5.],
            [25., 0.0005, -5.],
            [-25., 0.0005, -5.],
            [-25., -0.0005, 5.],
            [25., -0.0005, 5.],
            [25., 0.0005, 5.],
            [-25., 0.0005, 5.],
        ]
        .into_iter()
        .map(|position| {
            let mut vertex = template;
            vertex.position = position;
            vertex
        })
        .collect();
        geometry.indices = vec![
            0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5, 2, 3, 7, 2, 7,
            6, 3, 0, 4, 3, 4, 7,
        ];
        scene.instances[0].transform = glam::Mat4::IDENTITY;
        scene.instances[1].transform = glam::Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2);
        let report = analyze_scene("thin-crossing.step", "fixture", &scene);
        assert_eq!(report.outcome, CheckOutcome::Incomplete);
        assert!(report.findings.is_empty());
        assert_eq!(report.unresolved_pairs.len(), 1);
        assert_eq!(
            report.unresolved_pairs[0].code,
            "below_tessellation_resolution"
        );
    }

    #[test]
    fn overlap_below_placement_precision_is_reported_as_unresolved() {
        let mut scene = compile_scene(
            &fixture("separated.step"),
            UpAxis::Z,
            &mut Timings::default(),
        )
        .unwrap();
        scene.instances[1].geometry = scene.instances[0].geometry;
        scene.instances[0].transform =
            glam::Mat4::from_translation(glam::Vec3::new(10000., 0., 0.));
        scene.instances[1].transform =
            glam::Mat4::from_translation(glam::Vec3::new(10009.999, 0., 0.));
        let report = analyze_scene("sub-ulp-overlap.step", "fixture", &scene);
        assert_eq!(report.outcome, CheckOutcome::Incomplete);
        assert!(report.findings.is_empty());
    }

    #[test]
    fn a_real_mesh_hole_stays_unresolved_with_component_references() {
        let mut scene = compile_scene(
            &fixture("separated.step"),
            UpAxis::Z,
            &mut Timings::default(),
        )
        .unwrap();
        let definition = scene.instances[0].geometry;
        for instance in &mut scene.instances {
            instance.geometry = definition;
            instance.transform = glam::Mat4::IDENTITY;
        }
        scene.geometries[definition].indices.drain(..3);
        let report = analyze_scene("hole.step", "fixture", &scene);
        assert_eq!(report.outcome, CheckOutcome::Incomplete);
        assert!(report.findings.is_empty());
        assert!(!report.pair_set_complete);
        assert_eq!(report.unresolved_pairs.len(), 1);
        assert_eq!(report.unresolved_pairs[0].code, "open_component_mesh");
        assert_eq!(report.unresolved_pairs[0].components[0].occurrence_index, 0);
        assert_eq!(report.unresolved_pairs[0].components[1].occurrence_index, 1);
    }

    #[test]
    fn a_t_junction_on_a_closed_solid_does_not_hide_a_pair() {
        let mut scene = compile_scene(
            &fixture("separated.step"),
            UpAxis::Z,
            &mut Timings::default(),
        )
        .unwrap();
        let definition = scene.instances[0].geometry;
        for instance in &mut scene.instances {
            instance.geometry = definition;
            instance.transform = glam::Mat4::IDENTITY;
        }
        let geometry = &mut scene.geometries[definition];
        let [a, b, c] = [
            geometry.indices[0],
            geometry.indices[1],
            geometry.indices[2],
        ];
        let mut midpoint = geometry.vertices[a as usize];
        for axis in 0..3 {
            midpoint.position[axis] = (geometry.vertices[a as usize].position[axis]
                + geometry.vertices[b as usize].position[axis])
                * 0.5;
        }
        let m = geometry.vertices.len() as u32;
        geometry.vertices.push(midpoint);
        geometry.indices[..3].copy_from_slice(&[a, m, c]);
        geometry.indices.extend([m, b, c]);
        let report = analyze_scene("t-junction.step", "fixture", &scene);
        assert_eq!(report.findings.len(), 1);
        assert!(report.incomplete_reasons.is_empty());
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
