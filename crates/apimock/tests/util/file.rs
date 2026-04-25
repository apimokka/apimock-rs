use std::path::{Path, PathBuf};

/// file path from cargo manifest dir (project root dir)
pub fn file_path_from_cargo_manifest_dir(file_path: &str) -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir).join(file_path).to_path_buf()
}
