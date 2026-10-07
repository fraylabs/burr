//! A cached second mesh of a complete source definition. Refinement may add
//! positive evidence; it never replaces the original imported mesh or permits
//! an incomplete, open, or unmatched definition to prove a pair.
use super::{Curve3D, ElementarySurface, Shell, Surface};
use crate::interference::mesh::Mesh;
use look::{
    scene::{Bounds, Geometry, Vertex},
    step::{cone, cylinder, lattice, meshing_policy::MeshingPolicy, policy_geometry, torus_deck},
};
use std::{
    collections::HashSet,
    sync::{Arc, OnceLock},
};
use truck_meshalgo::prelude::*;
use truck_stepio::r#in::Table;

pub(super) const LEVELS: usize = 3;

pub(super) struct Metadata {
    closure: std::collections::HashMap<u64, lattice::SplineAxisClosure>,
    signed_tori: HashSet<u64>,
}

impl Metadata {
    pub fn from_table(table: &Table) -> Self {
        Self {
            closure: lattice::spline_closure_map(table),
            signed_tori: table
                .toroidal_surface
                .iter()
                .filter_map(|(&id, torus)| {
                    (torus.major_radius.is_finite()
                        && torus.major_radius < 0.0
                        && torus.minor_radius.is_finite()
                        && torus.minor_radius > 0.0)
                        .then_some(id)
                })
                .collect(),
        }
    }
}

pub(super) struct Definition {
    shells: Vec<Shell>,
    tolerance: f64,
    metadata: Arc<Metadata>,
    meshes: [OnceLock<Option<Mesh>>; LEVELS],
}

impl Definition {
    pub fn new(shells: &[&Shell], geometry: &Geometry, metadata: Arc<Metadata>) -> Option<Self> {
        if shells.is_empty() {
            return None;
        }
        let diagonal = f64::from(geometry.bounds.size().length());
        // One eighth of the definition-relative tolerance; never below the
        // kernel's documented tolerance floor. Angular refinement is applied
        // as well, so small circles receive more than the default samples.
        let tolerance =
            (diagonal * MeshingPolicy::DEFAULT.relative_linear_deflection / 8.0).max(1e-6);
        if !tolerance.is_finite() {
            return None;
        }
        Some(Self {
            shells: shells.iter().map(|shell| (*shell).clone()).collect(),
            tolerance,
            metadata,
            meshes: std::array::from_fn(|_| OnceLock::new()),
        })
    }

    pub fn requires_trim_confirmation(&self) -> bool {
        // Constant evaluator normals do not certify a straight trim. Only a
        // source definition made exclusively of planes and line edges may
        // bypass confirmation of the meshing allowance.
        self.shells.iter().any(|shell| {
            shell.faces.iter().any(|face| {
                !matches!(
                    face.surface,
                    Surface::ElementarySurface(ElementarySurface::Plane(_))
                )
            }) || shell
                .edges
                .iter()
                .any(|edge| !matches!(edge.curve, Curve3D::Line(_)))
        })
    }

    pub fn mesh(&self, level: usize) -> Option<&Mesh> {
        let tolerance = (self.tolerance / 4_f64.powi(level as i32)).max(1e-6);
        self.meshes
            .get(level)?
            .get_or_init(|| {
                let started = std::time::Instant::now();
                let mesh = self.refine(tolerance, level);
                if std::env::var_os("BURR_REFINEMENT_TRACE").is_some() {
                    eprintln!(
                        "burr refinement {}",
                        serde_json::json!({
                            "faces": self.shells.iter().map(|s| s.faces.len()).sum::<usize>(),
                            "linear_deflection": tolerance,
                            "level": level,
                            "elapsed_s": started.elapsed().as_secs_f64(),
                            "closed_triangles": mesh.as_ref().map(|m| m.triangles.len()),
                        })
                    );
                }
                mesh
            })
            .as_ref()
    }

    fn refine(&self, tolerance: f64, level: usize) -> Option<Mesh> {
        let angular_factor = 2_u32.pow(level as u32 + 1);
        let policy = MeshingPolicy {
            maximum_absolute_deflection: tolerance,
            maximum_angular_deflection: MeshingPolicy::DEFAULT.maximum_angular_deflection
                / f64::from(angular_factor),
            minimum_segments_per_revolution: MeshingPolicy::DEFAULT.minimum_segments_per_revolution
                * angular_factor as usize,
            ..MeshingPolicy::DEFAULT
        };
        let mut vertices = Vec::new();
        let mut normals = Vec::new();
        let mut indices = Vec::new();
        for shell in &self.shells {
            let declared = shell.faces.len();
            let outcome = policy_geometry::wrap_shell_with_source_metadata(
                shell.clone(),
                policy,
                &self.metadata.closure,
                &self.metadata.signed_tori,
            )
            .robust_triangulation_with_inverse_retry_outcome(
                tolerance,
                |s: &policy_geometry::PolicySurface| {
                    lattice::lattice_of_with_closure(s.inner(), s.source_closure())
                },
                |s: &policy_geometry::PolicySurface| lattice::support_schema_of(s.inner()),
                |c: &policy_geometry::PolicyCurve| lattice::curve_schema_of(c.inner()),
                |s: &policy_geometry::PolicySurface| {
                    cylinder::identify_source_cylinder_opt(s.inner())
                },
                |c: &policy_geometry::PolicyCurve| lattice::cylinder_curve_schema_of(c.inner()),
                |c: &policy_geometry::PolicyCurve| lattice::cylinder_curve_family_of(c.inner()),
                |s: &policy_geometry::PolicySurface| cone::identify_source_cone_opt(s.inner()),
                |s: &policy_geometry::PolicySurface| {
                    torus_deck::identify_source_torus_opt(s.inner())
                },
                |s: &policy_geometry::PolicySurface| s.native_inverse_retry(),
            );
            if outcome.shell.faces.len() != declared
                || outcome.face_failures.iter().any(Option::is_some)
            {
                return None;
            }
            for face in &outcome.shell.faces {
                let surface = face.surface.as_ref()?;
                if surface.faces().is_empty() {
                    return None;
                }
                let polygon = if face.orientation {
                    surface.clone()
                } else {
                    surface.inverse()
                };
                for triangle in polygon.tri_faces() {
                    for vertex in triangle {
                        let point = polygon.positions()[vertex.pos];
                        let normal = vertex.nor.and_then(|n| polygon.normals().get(n))?;
                        let normal = [normal.x as f32, normal.y as f32, normal.z as f32];
                        indices.push(u32::try_from(vertices.len()).ok()?);
                        vertices.push(Vertex {
                            position: [point.x as f32, point.y as f32, point.z as f32],
                            normal,
                        });
                        normals.push(normal);
                    }
                }
            }
        }
        let bounds = Bounds::from_position_iter(vertices.iter().map(|v| v.position));
        let geometry = Geometry {
            vertices,
            indices,
            surface_normals: Some(normals),
            source_attributes: None,
            bounds,
            bounding_center: bounds.center().to_array(),
            bounding_radius: bounds.size().length() * 0.5,
        };
        let mut mesh = Mesh::prepare(&geometry).ok()?;
        if !mesh.oriented && !mesh.orient_source_facets() {
            return None;
        }
        if !mesh.closed {
            return None;
        }
        mesh.refined_deflection = Some(tolerance);
        Some(mesh)
    }
}
