// SPDX-License-Identifier: GPL-3.0-only

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

use tokio::sync::{Mutex, watch};

use super::{OperationError, StorageOperations, shared};

#[derive(Debug, Clone)]
pub struct OperationStatus {
    pub bytes_completed: u64,
    pub total_bytes: u64,
    pub speed_bytes_per_sec: u64,
}

#[derive(Clone, Copy)]
enum ImageKind {
    Backup,
    Restore,
}

struct ImageOperation {
    progress: Arc<Mutex<OperationStatus>>,
    cancelled: Arc<AtomicBool>,
    completion: watch::Receiver<Result<(), String>>,
}

#[derive(Default)]
pub struct ImageOperationManager {
    operations: Mutex<HashMap<String, ImageOperation>>,
}

#[allow(dead_code)]
impl ImageOperationManager {
    async fn start(
        &self,
        block: Arc<dyn storage_contracts::BlockStorageBackend>,
        kind: ImageKind,
        device: String,
        path: String,
    ) -> Result<String, OperationError> {
        match kind {
            ImageKind::Backup
                if Path::new(&path)
                    .parent()
                    .is_none_or(|parent| !parent.exists()) =>
            {
                return Err(OperationError::InvalidInput(
                    "The image destination directory does not exist".into(),
                ));
            }
            ImageKind::Restore if !Path::new(&path).is_file() => {
                return Err(OperationError::InvalidInput(
                    "The image file does not exist or is not a regular file".into(),
                ));
            }
            _ => {}
        }
        let descriptor = match kind {
            ImageKind::Backup => block.open_for_backup(&device).await?,
            ImageKind::Restore => block.open_for_restore(&device).await?,
        };
        let total = match kind {
            ImageKind::Backup => std::fs::File::from(
                descriptor
                    .try_clone()
                    .map_err(|error| OperationError::Failed(error.to_string()))?,
            )
            .metadata()
            .map(|metadata| metadata.len())
            .unwrap_or(0),
            ImageKind::Restore => std::fs::metadata(&path)
                .map(|metadata| metadata.len())
                .unwrap_or(0),
        };
        let progress = Arc::new(Mutex::new(OperationStatus {
            bytes_completed: 0,
            total_bytes: total,
            speed_bytes_per_sec: 0,
        }));
        let cancelled = Arc::new(AtomicBool::new(false));
        let (sender, receiver) = watch::channel(Ok(()));
        let task_progress = Arc::clone(&progress);
        let task_cancelled = Arc::clone(&cancelled);
        let task_path = PathBuf::from(path);
        tokio::spawn(async move {
            if task_cancelled.load(Ordering::Acquire) {
                let _ = sender.send(Err("Operation cancelled".into()));
                return;
            }
            let started = Instant::now();
            let copied = tokio::task::spawn_blocking(move || match kind {
                ImageKind::Backup => storage_sys::copy_image_to_file(
                    descriptor,
                    &task_path,
                    Some(|bytes| {
                        let elapsed = started.elapsed().as_secs().max(1);
                        let mut progress = task_progress.blocking_lock();
                        progress.bytes_completed = bytes;
                        progress.speed_bytes_per_sec = bytes / elapsed;
                    }),
                ),
                ImageKind::Restore => storage_sys::copy_file_to_image(
                    &task_path,
                    descriptor,
                    Some(|bytes| {
                        let elapsed = started.elapsed().as_secs().max(1);
                        let mut progress = task_progress.blocking_lock();
                        progress.bytes_completed = bytes;
                        progress.speed_bytes_per_sec = bytes / elapsed;
                    }),
                ),
            })
            .await;
            let result = match copied {
                Ok(Ok(_)) if !task_cancelled.load(Ordering::Acquire) => Ok(()),
                Ok(Ok(_)) => Err("Operation cancelled".into()),
                Ok(Err(error)) => Err(error.to_string()),
                Err(error) => Err(error.to_string()),
            };
            let _ = sender.send(result);
        });
        let id = uuid::Uuid::new_v4().to_string();
        self.operations.lock().await.insert(
            id.clone(),
            ImageOperation {
                progress,
                cancelled,
                completion: receiver,
            },
        );
        Ok(id)
    }

    pub async fn backup(
        &self,
        block: Arc<dyn storage_contracts::BlockStorageBackend>,
        device: String,
        output: String,
    ) -> Result<String, OperationError> {
        self.start(block, ImageKind::Backup, device, output).await
    }
    pub async fn restore(
        &self,
        block: Arc<dyn storage_contracts::BlockStorageBackend>,
        device: String,
        image: String,
    ) -> Result<String, OperationError> {
        self.start(block, ImageKind::Restore, device, image).await
    }
    pub async fn status(&self, id: &str) -> Result<OperationStatus, OperationError> {
        self.operations
            .lock()
            .await
            .get(id)
            .ok_or_else(|| OperationError::MissingOperation(id.into()))?
            .progress
            .lock()
            .await
            .clone()
            .pipe(Ok)
    }
    pub async fn cancel(&self, id: &str) -> Result<(), OperationError> {
        self.operations
            .lock()
            .await
            .get(id)
            .ok_or_else(|| OperationError::MissingOperation(id.into()))?
            .cancelled
            .store(true, Ordering::Release);
        Ok(())
    }
    pub async fn wait(&self, id: &str) -> Result<(), OperationError> {
        let mut receiver = self
            .operations
            .lock()
            .await
            .get(id)
            .ok_or_else(|| OperationError::MissingOperation(id.into()))?
            .completion
            .clone();
        while receiver.changed().await.is_ok() {}
        receiver.borrow().clone().map_err(OperationError::Failed)
    }
    pub async fn forget(&self, id: &str) -> Result<(), OperationError> {
        self.operations
            .lock()
            .await
            .remove(id)
            .map(|_| ())
            .ok_or_else(|| OperationError::MissingOperation(id.into()))
    }
}

trait Pipe: Sized {
    fn pipe<T>(self, function: impl FnOnce(Self) -> T) -> T {
        function(self)
    }
}
impl<T> Pipe for T {}

#[derive(Clone, Debug)]
pub struct ImageClient(Arc<StorageOperations>);
#[allow(dead_code)]
impl ImageClient {
    pub async fn new() -> Result<Self, OperationError> {
        Ok(Self(shared().await?))
    }
    pub async fn backup_drive(&self, device: &str, output: &str) -> Result<String, OperationError> {
        self.0
            .image_manager
            .backup(
                Arc::clone(&self.0.registry.block),
                device.into(),
                output.into(),
            )
            .await
    }
    pub async fn backup_partition(
        &self,
        device: &str,
        output: &str,
    ) -> Result<String, OperationError> {
        self.backup_drive(device, output).await
    }
    pub async fn restore_drive(&self, device: &str, image: &str) -> Result<String, OperationError> {
        self.0
            .image_manager
            .restore(
                Arc::clone(&self.0.registry.block),
                device.into(),
                image.into(),
            )
            .await
    }
    pub async fn restore_partition(
        &self,
        device: &str,
        image: &str,
    ) -> Result<String, OperationError> {
        self.restore_drive(device, image).await
    }
    pub async fn loop_setup(&self, image: &str) -> Result<String, OperationError> {
        self.0
            .registry
            .block
            .loop_setup(image)
            .await
            .map_err(Into::into)
    }
    pub async fn cancel_operation(&self, id: &str) -> Result<(), OperationError> {
        self.0.image_manager.cancel(id).await
    }
    pub async fn get_operation_status(&self, id: &str) -> Result<OperationStatus, OperationError> {
        self.0.image_manager.status(id).await
    }
    pub async fn wait_for_operation_completion(&self, id: &str) -> Result<(), OperationError> {
        self.0.image_manager.wait(id).await
    }
    pub async fn forget_operation(&self, id: &str) -> Result<(), OperationError> {
        self.0.image_manager.forget(id).await
    }
}
