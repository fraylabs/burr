use glam::{Mat3, Mat4, Vec3};
use look::scene::{Bounds, CompiledScene, Geometry};
use std::collections::HashMap;

pub const MAX_MOTION_COMPONENTS: usize = 32;
const MOTION_FRAME_COUNT: usize = 61;

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
    from: &CompiledScene,
    to: &CompiledScene,
    duration_ms: u32,
    initial_progress: f32,
) -> Result<PreparedMotion, String> {
    if from.instances.is_empty() || from.instances.len() > MAX_MOTION_COMPONENTS {
        return Err(format!(
            "Motion requires from 1 to {MAX_MOTION_COMPONENTS} named components."
        ));
    }
    if from.instances.len() != to.instances.len() {
        return Err("Motion poses must contain the same number of components.".to_string());
    }

    let target_by_name = named_instances(to)?;
    let mut targets = Vec::with_capacity(from.instances.len());
    let mut geometries = Vec::with_capacity(from.instances.len());
    let mut instance_ids = Vec::new();

    let source_names = named_instances(from)?;
    if source_names.len() != target_by_name.len() {
        return Err("Motion poses must contain the same named components.".to_string());
    }

    for (instance_index, from_instance) in from.instances.iter().enumerate() {
        let name = from_instance
            .node_name
            .as_deref()
            .ok_or_else(|| "Motion components must have non-empty names.".to_string())?;
        let to_index = target_by_name
            .get(name)
            .copied()
            .ok_or_else(|| format!("Motion target pose is missing component '{name}'."))?;
        let to_instance = &to.instances[to_index];
        let from_geometry = from
            .geometries
            .get(from_instance.geometry)
            .ok_or_else(|| format!("Motion source component '{name}' has no geometry."))?;
        let to_geometry = to
            .geometries
            .get(to_instance.geometry)
            .ok_or_else(|| format!("Motion target component '{name}' has no geometry."))?;
        if !same_geometry(from_geometry, to_geometry) {
            return Err(format!(
                "Motion component '{name}' changes geometry between poses; only rigid transforms are supported."
            ));
        }
        let (from_scale, _, _) = from_instance.transform.to_scale_rotation_translation();
        let (to_scale, _, _) = to_instance.transform.to_scale_rotation_translation();
        if (from_scale - to_scale).abs().max_element() > 1.0e-5 {
            return Err(format!(
                "Motion component '{name}' changes scale between poses; only rigid transforms are supported."
            ));
        }

        instance_ids.extend(std::iter::repeat_n(
            instance_index as f32,
            from_geometry.vertices.len(),
        ));
        geometries.push(from_geometry.clone());
        targets.push(to_instance.transform);
    }

    let mut scene = from.clone();
    scene.geometries = geometries;
    for (index, instance) in scene.instances.iter_mut().enumerate() {
        instance.geometry = index;
        instance.transform = Mat4::IDENTITY;
        instance.normal_transform = Mat3::IDENTITY;
    }
    scene.bounds = union_bounds(from.bounds, to.bounds);
    scene.fit_radius = bounds_radius(scene.bounds) * 1.3;

    let mut frames = Vec::with_capacity(
        MOTION_FRAME_COUNT * scene.instances.len() * Mat4::IDENTITY.to_cols_array().len(),
    );
    for frame in 0..MOTION_FRAME_COUNT {
        let progress = frame as f32 / (MOTION_FRAME_COUNT - 1) as f32;
        let eased = progress * progress * (3.0 - 2.0 * progress);
        for (from_instance, target) in from.instances.iter().zip(&targets) {
            let (from_scale, from_rotation, from_translation) =
                from_instance.transform.to_scale_rotation_translation();
            let (to_scale, to_rotation, to_translation) = target.to_scale_rotation_translation();
            let transform = Mat4::from_scale_rotation_translation(
                from_scale.lerp(to_scale, eased),
                from_rotation.slerp(to_rotation, eased),
                from_translation.lerp(to_translation, eased),
            );
            frames.extend(transform.to_cols_array());
        }
    }

    Ok(PreparedMotion {
        scene,
        instance_ids_base64: base64_f32(&instance_ids),
        frames_base64: base64_f32(&frames),
        frame_count: MOTION_FRAME_COUNT,
        instance_count: from.instances.len(),
        duration_ms,
        initial_progress: initial_progress.clamp(0.0, 1.0),
    })
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

fn same_geometry(left: &Geometry, right: &Geometry) -> bool {
    left.indices == right.indices
        && left.vertices.len() == right.vertices.len()
        && left
            .vertices
            .iter()
            .zip(&right.vertices)
            .all(|(left, right)| {
                left.position
                    .iter()
                    .zip(right.position)
                    .all(|(left, right)| (left - right).abs() <= 1.0e-5)
                    && left
                        .normal
                        .iter()
                        .zip(right.normal)
                        .all(|(left, right)| (left - right).abs() <= 1.0e-5)
            })
}

fn union_bounds(left: Bounds, right: Bounds) -> Bounds {
    Bounds {
        min: [
            left.min[0].min(right.min[0]),
            left.min[1].min(right.min[1]),
            left.min[2].min(right.min[2]),
        ],
        max: [
            left.max[0].max(right.max[0]),
            left.max[1].max(right.max[1]),
            left.max[2].max(right.max[2]),
        ],
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
    use glam::Quat;
    use look::{config::UpAxis, scene::compile_scene, timing::Timings};
    use std::path::Path;

    fn fixture_scene() -> CompiledScene {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/interference/separated.step");
        compile_scene(&path, UpAxis::Z, &mut Timings::default()).unwrap()
    }

    #[test]
    fn builds_rigid_frames_for_matching_named_components() {
        let from = fixture_scene();
        let mut to = from.clone();
        to.instances[1].transform *= Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2);

        let prepared = prepare_motion(&from, &to, 900, 0.25).unwrap();
        assert_eq!(prepared.instance_count, from.instances.len());
        assert_eq!(prepared.frame_count, MOTION_FRAME_COUNT);
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
    fn rejects_geometry_changes_between_poses() {
        let from = fixture_scene();
        let mut to = from.clone();
        to.geometries[to.instances[0].geometry].vertices[0].position[0] += 1.0;

        assert!(prepare_motion(&from, &to, 900, 0.0)
            .unwrap_err()
            .contains("changes geometry"));
    }

    #[test]
    fn generated_quaternion_path_stays_finite() {
        let from = fixture_scene();
        let mut to = from.clone();
        to.instances[0].transform *= Mat4::from_quat(Quat::from_rotation_z(2.5));
        let prepared = prepare_motion(&from, &to, 900, 0.0).unwrap();
        assert!(!prepared.frames_base64.is_empty());
    }

    #[test]
    fn rejects_scale_changes_between_poses() {
        let from = fixture_scene();
        let mut to = from.clone();
        to.instances[0].transform *= Mat4::from_scale(Vec3::splat(1.1));

        assert!(prepare_motion(&from, &to, 900, 0.0)
            .unwrap_err()
            .contains("changes scale"));
    }
}
