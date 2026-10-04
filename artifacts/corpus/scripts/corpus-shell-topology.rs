//! Count topology per source shell using Burr's production mesh preparation.
//! Copy to examples/corpus-shell-topology.rs and pass private diagnostic JSON
//! shell dumps containing face positions, triangle indices and orientations.
//! Output contains counts and source provenance; it contains no CAD vertices.
#[allow(dead_code)]
#[path = "../src/interference/mesh.rs"]
mod mesh;
use look::scene::{Bounds, Geometry, Vertex};
use serde::Deserialize;
use std::collections::HashMap;
#[derive(Deserialize)]
struct Shell {
    faces: Vec<Face>,
}
#[derive(Deserialize)]
struct Face {
    id: String,
    orientation: Option<bool>,
    positions: Vec<[f64; 3]>,
    faces: Vec<Vec<usize>>,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    'files: for path in std::env::args().skip(1) {
        let shell: Shell = serde_json::from_slice(&std::fs::read(&path)?)?;
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        for face in &shell.faces {
            for triangle in &face.faces {
                if triangle.len() != 3 {
                    continue;
                }
                let order = if face.orientation.unwrap_or(true) {
                    [0, 1, 2]
                } else {
                    [0, 2, 1]
                };
                for i in order {
                    let Some(&p) = face.positions.get(triangle[i]) else {
                        println!(
                            "{}",
                            serde_json::json!({"file":path,"error":"triangle index out of range"})
                        );
                        continue 'files;
                    };
                    indices.push(vertices.len() as u32);
                    vertices.push(Vertex {
                        position: p.map(|x| x as f32),
                        normal: [0.; 3],
                    });
                }
            }
        }
        let bounds =
            Bounds::from_positions(&vertices.iter().map(|v| v.position).collect::<Vec<_>>());
        let geometry = Geometry {
            surface_normals: None,
            vertices,
            indices,
            source_attributes: None,
            bounds,
            bounding_center: [0.; 3],
            bounding_radius: 0.,
        };
        let prepared = match mesh::Mesh::prepare(&geometry) {
            Ok(p) => p,
            Err(error) => {
                println!("{}", serde_json::json!({"file":path,"error":error}));
                continue;
            }
        };
        let mut edges: HashMap<[u32; 2], (usize, i32)> = HashMap::new();
        for t in &prepared.triangles {
            for [a, b] in [[t[0], t[1]], [t[1], t[2]], [t[2], t[0]]] {
                let key = [a.min(b), a.max(b)];
                let e = edges.entry(key).or_default();
                e.0 += 1;
                e.1 += if a < b { 1 } else { -1 };
            }
        }
        println!(
            "{}",
            serde_json::json!({"file":path,"face_ids":shell.faces.iter().map(|f|&f.id).collect::<Vec<_>>(),"closed":prepared.closed,"epsilon":prepared.epsilon,"triangles":prepared.triangles.len(),"boundary_edges":edges.values().filter(|e|e.0==1).count(),"nonmanifold_edges":edges.values().filter(|e|e.0>2).count(),"inconsistent_oriented_edges":edges.values().filter(|e|e.1!=0).count()})
        );
    }
    Ok(())
}
