#![cfg(feature = "test-backend")]

use cosmic_ext_storage::{
    AppRuntime,
    testing::{FixtureSecrets, WorkflowHarness},
};
use rstest::fixture;
use std::path::{Path, PathBuf};

pub fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/ui/scenarios")
        .join(name)
}

#[fixture]
pub fn scenario(#[default("empty.toml")] file: &str) -> AppRuntime {
    AppRuntime::scenario(fixture(file), None, None).expect("scenario runtime")
}

#[fixture]
pub async fn workflow(
    #[default("empty.toml")] file: &str,
    #[default(FixtureSecrets::none())] secrets: FixtureSecrets,
) -> WorkflowHarness {
    WorkflowHarness::from_fixture(file, secrets)
        .await
        .expect("workflow fixture")
}

#[fixture]
pub fn scratch() -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix("cs-")
        .tempdir_in("/tmp")
        .expect("owned test root")
}
