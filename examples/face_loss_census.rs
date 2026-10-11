use ruststep::tables::IntoOwned;
use truck_stepio::r#in::{step_geometry::Surface, Table};

fn conversion_error(f: impl FnOnce() -> Option<String>) -> Option<String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).unwrap_or_else(|panic| {
        let message = panic
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| panic.downcast_ref::<&str>().copied())
            .unwrap_or("non-string panic");
        Some(format!("conversion panic: {message}"))
    })
}

fn main() {
    for path in std::env::args().skip(1) {
        eprintln!("MODEL {path}");
        let bytes = std::fs::read(&path).unwrap();
        let text = String::from_utf8(bytes.clone())
            .unwrap_or_else(|_| bytes.iter().map(|&b| b as char).collect());
        let mut exchange = look::step::part21::parse(&text).unwrap();
        let table = Table::from_owned_data_section(exchange.data.swap_remove(0));
        let mut shells = table.shell.iter().collect::<Vec<_>>();
        shells.sort_by_key(|(id, _)| **id);
        for (&id, shell) in shells {
            match table.to_compressed_shell_with_losses(id, shell) {
                Ok((_, losses)) => {
                    for loss in losses {
                        println!(
                            "{}",
                            serde_json::json!({"model":path,"shell":id,"face":loss.provenance.definition_id.map(|v|v.get()),"reason":loss.reason.tag()})
                        );
                        if let Some(face) = loss
                            .provenance
                            .definition_id
                            .and_then(|id| table.face_surface.get(&id.get()))
                        {
                            let error = conversion_error(|| {
                                match face.face_geometry.clone().into_owned(&table) {
                                    Ok(surface) => {
                                        Surface::try_from(&surface).err().map(|e| e.to_string())
                                    }
                                    Err(error) => Some(error.to_string()),
                                }
                            });
                            if let Some(error) = error {
                                println!(
                                    "{}",
                                    serde_json::json!({"model":path,"face":loss.provenance.definition_id.map(|v|v.get()),"surface_error":error})
                                );
                            }
                        }
                    }
                }
                Err(error) => println!(
                    "{}",
                    serde_json::json!({"model":path,"shell":id,"shell_error":error.to_string()})
                ),
            }
        }
        for (&id, edge) in &table.edge_curve {
            let error = conversion_error(|| match edge.clone().into_owned(&table) {
                Ok(edge) => edge.parse_curve3d().err().map(|e| e.to_string()),
                Err(error) => Some(error.to_string()),
            });
            if let Some(error) = error {
                println!(
                    "{}",
                    serde_json::json!({"model":path,"edge":id,"edge_error":error})
                );
            }
        }
    }
}
