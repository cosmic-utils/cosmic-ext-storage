use rstest::fixture;
use test_backend::{ScenarioBackend, ScenarioRuntime, ScenarioStore};
pub mod paths;
pub use paths::fixture;

#[fixture]
pub async fn scenario(#[default("empty.toml")] file: &str) -> ScenarioRuntime {
    ScenarioRuntime::load(fixture(file), None, None).expect("scenario fixture")
}

#[fixture]
pub fn backend(#[default("empty.toml")] file: &str) -> std::sync::Arc<ScenarioBackend> {
    ScenarioBackend::load(ScenarioStore::new(fixture(file), None, None)).expect("scenario backend")
}
