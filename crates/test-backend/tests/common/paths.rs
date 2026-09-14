use rstest::fixture;
use std::path::{Path, PathBuf};

pub fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/ui/scenarios")
        .join(name)
}

#[fixture]
pub fn scratch() -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix("cs-")
        .tempdir_in("/tmp")
        .expect("owned test root")
}
