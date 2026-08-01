// SPDX-License-Identifier: GPL-3.0-only

//! Application composition root for real and scenario storage adapters.

use std::{path::PathBuf, sync::Arc};

use async_trait::async_trait;
use storage_contracts::{
    DesktopServices, ImageWorkflowOperations, RuntimeAdapters, ScenarioControl, StorageError,
    StorageErrorKind, UsageOperations,
};
use storage_types::{
    DesktopImageSelection, ImageAssetRef, ImageAttachment, ImageAttachmentRequest,
    ImageCopyRequest, ImageWorkflowStatus, UsageDeleteRequest, UsageDeleteResponse,
    UsageWorkflowRequest, UsageWorkflowStatus,
};

use crate::operations::{OperationError, StorageOperations};

#[derive(Clone)]
pub struct AppRuntime {
    operations: Arc<StorageOperations>,
    desktop: Arc<dyn DesktopServices>,
    scenario_control: Option<Arc<dyn ScenarioControl>>,
    scenario_marker: Option<String>,
    #[cfg(feature = "test-backend")]
    scenario_control_server: Option<Arc<test_backend::ScenarioControlServer>>,
    /// Production's bootstrap runtime stays owned by the launch runtime so
    /// adapter construction and UI tasks use one Tokio lifecycle.
    #[allow(dead_code)]
    bootstrap: Option<Arc<tokio::runtime::Runtime>>,
}

impl std::fmt::Debug for AppRuntime {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AppRuntime")
            .field("scenario", &self.scenario_control.is_some())
            .field("scenario_marker", &self.scenario_marker)
            .finish_non_exhaustive()
    }
}

impl AppRuntime {
    pub fn production() -> Result<Self, OperationError> {
        let bootstrap = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .map_err(|error| OperationError::Failed(error.to_string()))?,
        );
        let operations = bootstrap.block_on(StorageOperations::new())?;
        let desktop: Arc<dyn DesktopServices> = Arc::new(ProductionDesktopServices);
        Ok(Self {
            operations,
            desktop,
            scenario_control: None,
            scenario_marker: None,
            #[cfg(feature = "test-backend")]
            scenario_control_server: None,
            bootstrap: Some(bootstrap),
        })
    }

    pub fn from_adapters(adapters: RuntimeAdapters) -> Result<Self, OperationError> {
        let desktop = adapters.desktop.clone();
        let scenario_control = adapters.scenario_control.clone();
        let operations = StorageOperations::from_adapters(adapters)?;
        Ok(Self {
            operations,
            desktop,
            scenario_control,
            scenario_marker: None,
            #[cfg(feature = "test-backend")]
            scenario_control_server: None,
            bootstrap: None,
        })
    }

    pub fn operations(&self) -> Arc<StorageOperations> {
        self.operations.clone()
    }
    pub fn desktop(&self) -> Arc<dyn DesktopServices> {
        self.desktop.clone()
    }
    pub fn scenario_control(&self) -> Option<Arc<dyn ScenarioControl>> {
        self.scenario_control.clone()
    }
    pub fn is_scenario(&self) -> bool {
        self.scenario_control.is_some()
    }
    pub fn scenario_marker(&self) -> Option<&str> {
        self.scenario_marker.as_deref()
    }

    pub(crate) fn install(&self) -> Result<(), OperationError> {
        crate::operations::install_selected(Arc::clone(&self.operations))
    }

    #[cfg(feature = "test-backend")]
    pub fn scenario(
        fixture: PathBuf,
        overlay: Option<PathBuf>,
        trace: Option<PathBuf>,
    ) -> Result<Self, OperationError> {
        Self::scenario_with_control(fixture, overlay, trace, None)
    }

    #[cfg(feature = "test-backend")]
    fn scenario_with_control(
        fixture: PathBuf,
        overlay: Option<PathBuf>,
        trace: Option<PathBuf>,
        control: Option<ScenarioControlEndpoint>,
    ) -> Result<Self, OperationError> {
        let scenario = test_backend::ScenarioRuntime::load(fixture, overlay, trace)
            .map_err(OperationError::from)?;
        let marker = scenario.marker();
        let server = match control {
            Some(control) => {
                let token = std::fs::read_to_string(control.token_file).map_err(|error| {
                    OperationError::Failed(format!("cannot read scenario control token: {error}"))
                })?;
                Some(
                    scenario
                        .start_control_server(control.socket, token.trim().into())
                        .map_err(OperationError::from)?,
                )
            }
            None => None,
        };
        let mut runtime = Self::from_adapters(scenario.adapters())?;
        runtime.scenario_marker = Some(marker);
        runtime.scenario_control_server = server;
        Ok(runtime)
    }
}

#[cfg(feature = "test-backend")]
#[derive(Debug, Clone, PartialEq, Eq)]
struct ScenarioControlEndpoint {
    socket: PathBuf,
    token_file: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeRequest {
    Real,
    Scenario {
        fixture: PathBuf,
        overlay: Option<PathBuf>,
        trace: Option<PathBuf>,
        watch: bool,
        control_socket: Option<PathBuf>,
        control_token_file: Option<PathBuf>,
    },
}

impl RuntimeRequest {
    fn usage() -> &'static str {
        #[cfg(feature = "test-backend")]
        {
            "usage: cosmic-ext-storage [--backend real|scenario --scenario <fixture> --scenario-state <overlay> --scenario-trace <trace> --watch-scenario --scenario-control-socket <socket> --scenario-control-token-file <token>]"
        }
        #[cfg(not(feature = "test-backend"))]
        {
            "usage: cosmic-ext-storage"
        }
    }

    pub fn parse(arguments: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut backend = "real".to_string();
        let mut fixture = None;
        let mut overlay = None;
        let mut trace = None;
        let mut watch = false;
        let mut control_socket = None;
        let mut control_token_file = None;
        let mut arguments = arguments.into_iter();
        while let Some(argument) = arguments.next() {
            match argument.as_str() {
                "--backend" => {
                    backend = arguments
                        .next()
                        .ok_or("--backend requires real or scenario")?
                }
                "--scenario" => {
                    fixture = Some(PathBuf::from(
                        arguments.next().ok_or("--scenario requires a path")?,
                    ))
                }
                "--scenario-state" => {
                    overlay = Some(PathBuf::from(
                        arguments.next().ok_or("--scenario-state requires a path")?,
                    ))
                }
                "--scenario-trace" => {
                    trace = Some(PathBuf::from(
                        arguments.next().ok_or("--scenario-trace requires a path")?,
                    ))
                }
                "--watch-scenario" => watch = true,
                "--scenario-control-socket" => {
                    control_socket = Some(PathBuf::from(
                        arguments
                            .next()
                            .ok_or("--scenario-control-socket requires a path")?,
                    ))
                }
                "--scenario-control-token-file" => {
                    control_token_file = Some(PathBuf::from(
                        arguments
                            .next()
                            .ok_or("--scenario-control-token-file requires a path")?,
                    ))
                }
                "--help" | "-h" => return Err(Self::usage().into()),
                _ => return Err(format!("unknown launch argument: {argument}")),
            }
        }
        match backend.as_str() {
            "real" => {
                if fixture.is_some()
                    || overlay.is_some()
                    || trace.is_some()
                    || watch
                    || control_socket.is_some()
                    || control_token_file.is_some()
                {
                    return Err("scenario arguments require --backend scenario".into());
                }
                Ok(Self::Real)
            }
            "scenario" => {
                if control_socket.is_some() != control_token_file.is_some() {
                    return Err(
                        "scenario control requires both --scenario-control-socket and --scenario-control-token-file"
                            .into(),
                    );
                }
                Ok(Self::Scenario {
                    fixture: fixture.ok_or("--backend scenario requires --scenario <fixture>")?,
                    overlay,
                    trace,
                    watch,
                    control_socket,
                    control_token_file,
                })
            }
            _ => Err("--backend must be real or scenario".into()),
        }
    }
}

impl AppRuntime {
    pub fn from_request(request: RuntimeRequest) -> Result<Self, OperationError> {
        match request {
            RuntimeRequest::Real => Self::production(),
            RuntimeRequest::Scenario { .. } => {
                #[cfg(feature = "test-backend")]
                {
                    let RuntimeRequest::Scenario {
                        fixture,
                        overlay,
                        trace,
                        control_socket,
                        control_token_file,
                        ..
                    } = request
                    else {
                        unreachable!()
                    };
                    Self::scenario_with_control(
                        fixture,
                        overlay,
                        trace,
                        control_socket
                            .zip(control_token_file)
                            .map(|(socket, token_file)| ScenarioControlEndpoint {
                                socket,
                                token_file,
                            }),
                    )
                }
                #[cfg(not(feature = "test-backend"))]
                {
                    Err(OperationError::Unsupported("scenario mode is unavailable in this build; rebuild with --features test-backend".into()))
                }
            }
        }
    }
}

struct ProductionDesktopServices;

#[async_trait]
impl DesktopServices for ProductionDesktopServices {
    async fn select_image(&self) -> Result<DesktopImageSelection, StorageError> {
        Ok(DesktopImageSelection::Cancelled)
    }
    async fn reveal(&self, display_reference: &str) -> Result<(), StorageError> {
        open::that_detached(display_reference)
            .map_err(|error| StorageError::new(StorageErrorKind::Other, error.to_string()))
    }
    async fn open_url(&self, url: &str) -> Result<(), StorageError> {
        open::that_detached(url)
            .map_err(|error| StorageError::new(StorageErrorKind::Other, error.to_string()))
    }
}

/// Placeholder production workflow adapter used while legacy UI callers are
/// migrated.  It makes the boundary explicit and never grants a scenario a
/// real implementation.
pub(crate) struct UnavailableWorkflowAdapter;

fn unavailable(operation: &str) -> StorageError {
    StorageError::new(
        StorageErrorKind::Unsupported,
        format!("{operation} is unavailable"),
    )
}

#[async_trait]
impl UsageOperations for UnavailableWorkflowAdapter {
    async fn list_usage_mounts(&self) -> Result<Vec<String>, StorageError> {
        Err(unavailable("usage.list_mounts"))
    }
    async fn authorize_show_all_files(&self) -> Result<bool, StorageError> {
        Err(unavailable("usage.authorize_show_all_files"))
    }
    async fn start_usage_scan(
        &self,
        _request: UsageWorkflowRequest,
    ) -> Result<String, StorageError> {
        Err(unavailable("usage.start_scan"))
    }
    async fn usage_scan_status(&self, _scan_id: &str) -> Result<UsageWorkflowStatus, StorageError> {
        Err(unavailable("usage.scan_status"))
    }
    async fn wait_for_usage_scan(
        &self,
        _scan_id: &str,
    ) -> Result<UsageWorkflowStatus, StorageError> {
        Err(unavailable("usage.wait_for_scan"))
    }
    async fn delete_usage_files(
        &self,
        _request: UsageDeleteRequest,
    ) -> Result<UsageDeleteResponse, StorageError> {
        Err(unavailable("usage.delete_files"))
    }
}

#[async_trait]
impl ImageWorkflowOperations for UnavailableWorkflowAdapter {
    async fn create_image_asset(
        &self,
        _asset: ImageAssetRef,
        _size_bytes: u64,
    ) -> Result<(), StorageError> {
        Err(unavailable("image.create"))
    }
    async fn attach_image(
        &self,
        _request: ImageAttachmentRequest,
    ) -> Result<ImageAttachment, StorageError> {
        Err(unavailable("image.attach"))
    }
    async fn start_image_copy(&self, _request: ImageCopyRequest) -> Result<String, StorageError> {
        Err(unavailable("image.start_copy"))
    }
    async fn image_copy_status(
        &self,
        _operation_id: &str,
    ) -> Result<ImageWorkflowStatus, StorageError> {
        Err(unavailable("image.copy_status"))
    }
    async fn wait_for_image_copy(
        &self,
        _operation_id: &str,
    ) -> Result<ImageWorkflowStatus, StorageError> {
        Err(unavailable("image.wait_for_copy"))
    }
    async fn cancel_image_copy(&self, _operation_id: &str) -> Result<(), StorageError> {
        Err(unavailable("image.cancel_copy"))
    }
    async fn forget_image_copy(&self, _operation_id: &str) -> Result<(), StorageError> {
        Err(unavailable("image.forget_copy"))
    }
}
