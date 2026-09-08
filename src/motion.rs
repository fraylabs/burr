use crate::project::MotionJoint;
use glam::{Mat3, Mat4, Vec3};
use look::{
    config::UpAxis,
    scene::{Bounds, CompiledScene, Geometry},
};
use std::collections::HashMap;

pub const MAX_MOTION_COMPONENTS: usize = 32;
const MOTION_FRAMES_PER_SECOND: usize = 60;

#[derive(Debug)]
pub struct PreparedMotion {
    pub scene: CompiledScene,
    pub instance_ids_base64: String,
    pub frames_base64: String,
    pub frame_count: usize,
    pub instance_count: usize,
    pub duration_ms: u32,
    pub initial_progress: f32,
}

pub fn prepare_motion(
    source: &CompiledScene,
    joints: &[MotionJoint],
    source_up_axis: UpAxis,
    duration_ms: u32,
    initial_progress: f32,
) -> Result<PreparedMotion, String> {
    if source.instances.is_empty() || source.instances.len() > MAX_MOTION_COMPONENTS {
        return Err(format!(
            "Motion requires 1 to {MAX_MOTION_COMPONENTS} named components."
        ));
    }
    if joints.is_empty() {
        return Err("Motion requires at least one rigid joint.".to_string());
    }

    let source_by_name = named_instances(source)?;
    let instance_motions = instance_motions(
        source.instances.len(),
        &source_by_name,
        joints,
        source_up_axis,
    )?;
    let mut geometries = Vec::with_capacity(source.instances.len());
    let mut instance_ids = Vec::new();

    for (instance_index, source_instance) in source.instances.iter().enumerate() {
        let name = source_instance
            .node_name
            .as_deref()
            .ok_or_else(|| "Motion components must have non-empty names.".to_string())?;
        let geometry = source
            .geometries
            .get(source_instance.geometry)
            .ok_or_else(|| format!("Motion source component '{name}' has no geometry."))?;
        instance_ids.extend(std::iter::repeat_n(
            instance_index as f32,
            geometry.vertices.len(),
        ));
        geometries.push(geometry.clone());
    }

    let mut scene = source.clone();
    scene.geometries = geometries;
    for (index, instance) in scene.instances.iter_mut().enumerate() {
        instance.geometry = index;
        instance.transform = Mat4::IDENTITY;
        instance.normal_transform = Mat3::IDENTITY;
    }

    let frame_count = (duration_ms as usize * MOTION_FRAMES_PER_SECOND).div_ceil(1_000) + 1;
    let mut frames = Vec::with_capacity(
        frame_count * scene.instances.len() * Mat4::IDENTITY.to_cols_array().len(),
    );
    let mut animated_bounds = EmptyBounds::new();
    for frame in 0..frame_count {
        let progress = frame as f32 / (frame_count - 1) as f32;
        let eased = progress * progress * (3.0 - 2.0 * progress);
        for (index, source_instance) in source.instances.iter().enumerate() {
            let transform =
                motion_transform(source_instance.transform, &instance_motions[index], eased);
            if !transform.is_finite() {
                return Err("Motion generated a non-finite component transform.".to_string());
            }
            frames.extend(transform.to_cols_array());
            animated_bounds.include_geometry_bounds(&scene.geometries[index], transform);
        }
    }
    scene.bounds = animated_bounds.finish()?;
    scene.fit_radius = bounds_radius(scene.bounds) * 1.3;

    Ok(PreparedMotion {
        scene,
        instance_ids_base64: base64_f32(&instance_ids),
        frames_base64: base64_f32(&frames),
        frame_count,
        instance_count: source.instances.len(),
        duration_ms,
        initial_progress: initial_progress.clamp(0.0, 1.0),
    })
}

#[derive(Clone, Debug)]
enum InstanceMotion {
    Fixed,
    Revolute {
        origin: Vec3,
        axis: Vec3,
        angle_radians: f32,
    },
    Prismatic {
        axis: Vec3,
        distance_mm: f32,
    },
}

fn instance_motions(
    instance_count: usize,
    source_by_name: &HashMap<&str, usize>,
    joints: &[MotionJoint],
    source_up_axis: UpAxis,
) -> Result<Vec<InstanceMotion>, String> {
    let mut motions = vec![InstanceMotion::Fixed; instance_count];
    let normalization = normalization_transform(source_up_axis);
    for joint in joints {
        let (components, operation) = match joint {
            MotionJoint::Revolute {
                components,
                origin_mm,
                axis,
                angle_degrees,
            } => (
                components,
                InstanceMotion::Revolute {
                    origin: normalization.transform_point3(Vec3::from_array(*origin_mm)),
                    axis: normalization.transform_vector3(Vec3::from_array(*axis)),
                    angle_radians: angle_degrees.to_radians(),
                },
            ),
            MotionJoint::Prismatic {
                components,
                axis,
                distance_mm,
            } => (
                components,
                InstanceMotion::Prismatic {
                    axis: normalization.transform_vector3(Vec3::from_array(*axis)),
                    distance_mm: *distance_mm,
                },
            ),
        };
        for component in components {
            let index = source_by_name
                .get(component.as_str())
                .copied()
                .ok_or_else(|| {
                    format!("Motion component '{component}' is missing from the source model.")
                })?;
            if !matches!(motions[index], InstanceMotion::Fixed) {
                return Err(format!(
                    "Motion component may be assigned to only one joint: '{component}'."
                ));
            }
            motions[index] = operation.clone();
        }
    }
    Ok(motions)
}

fn normalization_transform(up_axis: UpAxis) -> Mat4 {
    match up_axis {
        UpAxis::Y => Mat4::IDENTITY,
        UpAxis::Z => Mat4::from_rotation_x(-std::f32::consts::FRAC_PI_2),
        UpAxis::X => Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2),
    }
}

fn motion_transform(source: Mat4, motion: &InstanceMotion, progress: f32) -> Mat4 {
    match motion {
        InstanceMotion::Fixed => source,
        InstanceMotion::Revolute {
            origin,
            axis,
            angle_radians,
        } => {
            Mat4::from_translation(*origin)
                * Mat4::from_axis_angle(*axis, angle_radians * progress)
                * Mat4::from_translation(-*origin)
                * source
        }
        InstanceMotion::Prismatic { axis, distance_mm } => {
            Mat4::from_translation(*axis * *distance_mm * progress) * source
        }
    }
}

fn named_instances(scene: &CompiledScene) -> Result<HashMap<&str, usize>, String> {
    let mut names = HashMap::with_capacity(scene.instances.len());
    for (index, instance) in scene.instances.iter().enumerate() {
        let name = instance
            .node_name
            .as_deref()
            .filter(|name| !name.trim().is_empty())
            .ok_or_else(|| "Motion components must have non-empty names.".to_string())?;
        if names.insert(name, index).is_some() {
            return Err(format!("Motion component names must be unique: '{name}'."));
        }
    }
    Ok(names)
}

struct EmptyBounds {
    min: Vec3,
    max: Vec3,
    has_vertex: bool,
}

impl EmptyBounds {
    fn new() -> Self {
        Self {
            min: Vec3::splat(f32::INFINITY),
            max: Vec3::splat(f32::NEG_INFINITY),
            has_vertex: false,
        }
    }

    fn include_geometry_bounds(&mut self, geometry: &Geometry, transform: Mat4) {
        if geometry.vertices.is_empty() {
            return;
        }
        let min = Vec3::from_array(geometry.bounds.min);
        let max = Vec3::from_array(geometry.bounds.max);
        for x in [min.x, max.x] {
            for y in [min.y, max.y] {
                for z in [min.z, max.z] {
                    let position = transform.transform_point3(Vec3::new(x, y, z));
                    self.min = self.min.min(position);
                    self.max = self.max.max(position);
                    self.has_vertex = true;
                }
            }
        }
    }

    fn finish(self) -> Result<Bounds, String> {
        if !self.has_vertex || !self.min.is_finite() || !self.max.is_finite() {
            return Err("Motion source model contains no finite geometry.".to_string());
        }
        Ok(Bounds {
            min: self.min.to_array(),
            max: self.max.to_array(),
        })
    }
}

fn bounds_radius(bounds: Bounds) -> f32 {
    let diagonal = Vec3::from_array(bounds.max) - Vec3::from_array(bounds.min);
    (diagonal.length() * 0.5).max(1.0e-3)
}

fn base64_f32(values: &[f32]) -> String {
    let mut bytes = Vec::with_capacity(values.len() * 4);
    for value in values {
        bytes.extend(value.to_le_bytes());
    }
    base64_encode(&bytes)
}

fn base64_encode(data: &[u8]) -> String {
    const CHARACTERS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut encoded = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let first = u32::from(chunk[0]);
        let second = chunk.get(1).copied().map_or(0, u32::from);
        let third = chunk.get(2).copied().map_or(0, u32::from);
        let value = (first << 16) | (second << 8) | third;
        encoded.push(CHARACTERS[((value >> 18) & 63) as usize] as char);
        encoded.push(CHARACTERS[((value >> 12) & 63) as usize] as char);
        encoded.push(if chunk.len() > 1 {
            CHARACTERS[((value >> 6) & 63) as usize] as char
        } else {
            '='
        });
        encoded.push(if chunk.len() > 2 {
            CHARACTERS[(value & 63) as usize] as char
        } else {
            '='
        });
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;
    use look::{config::UpAxis, scene::compile_scene, timing::Timings};
    use std::path::Path;

    fn fixture_scene() -> CompiledScene {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/interference/separated.step");
        compile_scene(&path, UpAxis::Z, &mut Timings::default()).unwrap()
    }

    fn revolute_joint(component: &str) -> MotionJoint {
        MotionJoint::Revolute {
            components: vec![component.to_string()],
            origin_mm: [0.0, 0.0, 0.0],
            axis: [0.0, 0.0, 1.0],
            angle_degrees: 90.0,
        }
    }

    #[test]
    fn builds_rigid_frames_from_one_source_scene() {
        let source = fixture_scene();

        let prepared =
            prepare_motion(&source, &[revolute_joint("moving")], UpAxis::Z, 900, 0.25).unwrap();
        assert_eq!(prepared.instance_count, source.instances.len());
        assert_eq!(prepared.frame_count, 55);
        assert_eq!(prepared.initial_progress, 0.25);
        assert!(prepared.instance_ids_base64.len() > 16);
        assert!(prepared.frames_base64.len() > 16);
        assert!(prepared
            .scene
            .instances
            .iter()
            .all(|instance| instance.transform == Mat4::IDENTITY));
    }

    #[test]
    fn revolute_motion_keeps_the_declared_world_pivot_fixed() {
        let source = fixture_scene();
        let moving_index = named_instances(&source).unwrap()["moving"];
        let source_transform = source.instances[moving_index].transform;
        let pivot = Vec3::new(10.0, 2.0, -3.0);
        let pivot_in_component = source_transform.inverse().transform_point3(pivot);
        let operation = InstanceMotion::Revolute {
            origin: pivot,
            axis: Vec3::Z,
            angle_radians: std::f32::consts::FRAC_PI_2,
        };

        let target = motion_transform(source_transform, &operation, 1.0)
            .transform_point3(pivot_in_component);
        assert!((target - pivot).length() < 1.0e-5);
    }

    #[test]
    fn prismatic_motion_reaches_the_declared_distance() {
        let operation = InstanceMotion::Prismatic {
            axis: Vec3::Y,
            distance_mm: 7.5,
        };
        let target = motion_transform(Mat4::IDENTITY, &operation, 1.0).transform_point3(Vec3::ZERO);

        assert!((target - Vec3::new(0.0, 7.5, 0.0)).length() < 1.0e-6);
    }

    #[test]
    fn z_up_joint_coordinates_follow_the_step_scene_normalization() {
        let source = fixture_scene();
        let source_by_name = named_instances(&source).unwrap();
        let joints = [MotionJoint::Revolute {
            components: vec!["moving".to_string()],
            origin_mm: [1.0, 2.0, 3.0],
            axis: [0.0, 0.0, 1.0],
            angle_degrees: 90.0,
        }];
        let motions =
            instance_motions(source.instances.len(), &source_by_name, &joints, UpAxis::Z).unwrap();
        let moving_index = source_by_name["moving"];

        let InstanceMotion::Revolute { origin, axis, .. } = &motions[moving_index] else {
            panic!("moving component did not receive the revolute joint");
        };
        assert!((*origin - Vec3::new(1.0, 3.0, -2.0)).length() < 1.0e-6);
        assert!((*axis - Vec3::Y).length() < 1.0e-6);
    }

    #[test]
    fn long_motions_keep_sixty_frame_per_second_resolution() {
        let source = fixture_scene();
        let prepared =
            prepare_motion(&source, &[revolute_joint("moving")], UpAxis::Z, 10_000, 0.0).unwrap();

        assert_eq!(prepared.frame_count, 601);
    }

    #[test]
    fn rejects_a_joint_component_missing_from_the_source() {
        let source = fixture_scene();

        assert!(
            prepare_motion(&source, &[revolute_joint("missing")], UpAxis::Z, 900, 0.0,)
                .unwrap_err()
                .contains("missing from the source model")
        );
    }
}
