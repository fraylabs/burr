use look::{config::UpAxis, scene::compile_scene, timing::Timings};
fn main() {
    let path = std::env::args().nth(1).unwrap();
    let scene = compile_scene(
        std::path::Path::new(&path),
        UpAxis::Z,
        &mut Timings::default(),
    )
    .unwrap();
    let parts=scene.instances.iter().enumerate().map(|(i,p)|{let g=&scene.geometries[p.geometry];let mut min=[f64::INFINITY;3];let mut max=[f64::NEG_INFINITY;3];for v in &g.vertices {let q=p.transform.transform_point3(glam::Vec3::from_array(v.position)).to_array();for k in 0..3{min[k]=min[k].min(q[k] as f64);max[k]=max[k].max(q[k] as f64)}}serde_json::json!({"index":i,"name":p.node_name,"min":min,"max":max,"transform":p.transform.to_cols_array(),"geometry":p.geometry,"sample_points":g.vertices.iter().step_by((g.vertices.len()/32).max(1)).take(32).map(|v|p.transform.transform_point3(glam::Vec3::from_array(v.position)).to_array()).collect::<Vec<_>>()})}).collect::<Vec<_>>();
    println!("{}", serde_json::json!({"parts":parts}));
}
