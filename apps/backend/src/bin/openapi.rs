//! Exports the OpenAPI document to stdout or to a file.
//!
//! Used by the frontend type generation step and by CI, so the published
//! contract can be diffed without running the server.

use std::path::PathBuf;

use platform_backend::http::openapi;

fn main() -> anyhow::Result<()> {
    let document = openapi::document();
    let rendered = document.to_pretty_json()?;

    match std::env::args().nth(1) {
        Some(path) => {
            let path = PathBuf::from(path);
            if let Some(parent) = path
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
            {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&path, rendered)?;
            println!("OpenAPI document written to {}", path.display());
        }
        None => println!("{rendered}"),
    }

    Ok(())
}
