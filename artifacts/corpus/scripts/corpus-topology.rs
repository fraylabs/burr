#[allow(dead_code)]
// Copy this probe to examples/corpus-topology.rs before building it.
#[path = "../src/interference/mesh.rs"]
mod mesh;
use std::collections::HashMap;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let filename = std::env::args().nth(1).ok_or("STEP path")?;
    let scene = look::scene::compile_scene(
        std::path::Path::new(&filename),
        look::config::UpAxis::Z,
        &mut look::timing::Timings::default(),
    )?;
    for (index, geometry) in scene.geometries.iter().enumerate() {
        let names = scene
            .instances
            .iter()
            .filter(|instance| instance.geometry == index)
            .filter_map(|instance| instance.node_name.clone())
            .collect::<Vec<_>>();
        let prepared = match mesh::Mesh::prepare(geometry) {
            Ok(prepared) => prepared,
            Err(error) => {
                println!(
                    "{}",
                    serde_json::json!({"geometry":index,"names":names,"error":error})
                );
                continue;
            }
        };
        let mut edges: HashMap<[u32; 2], usize> = HashMap::new();
        for [a, b, c] in &prepared.triangles {
            for [a, b] in [[*a, *b], [*b, *c], [*c, *a]] {
                *edges.entry([a.min(b), a.max(b)]).or_default() += 1;
            }
        }
        let boundary = edges
            .iter()
            .filter(|(_, n)| **n == 1)
            .map(|(e, _)| *e)
            .collect::<Vec<_>>();
        let nonmanifold = edges.values().filter(|n| **n > 2).count();
        let endpoints = boundary
            .iter()
            .flatten()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let mut nearest_vertex_gaps = Vec::new();
        for &i in &endpoints {
            let p = prepared.points[i as usize];
            let nearest = endpoints
                .iter()
                .filter(|&&j| j != i)
                .map(|&j| p.distance(prepared.points[j as usize]))
                .fold(f64::INFINITY, f64::min);
            if nearest.is_finite() {
                nearest_vertex_gaps.push(nearest);
            }
        }
        nearest_vertex_gaps.sort_by(f64::total_cmp);
        println!(
            "{}",
            serde_json::json!({"geometry":index,"names":names,"closed":prepared.closed,"epsilon":prepared.epsilon,"source_triangles":geometry.indices.len()/3,"prepared_triangles":prepared.triangles.len(),"boundary_edges":boundary.len(),"nonmanifold_edges":nonmanifold,"boundary_vertices":endpoints.len(),"nearest_boundary_vertex_gaps":nearest_vertex_gaps})
        );
    }
    Ok(())
}
