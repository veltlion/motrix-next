use crate::error::AppError;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;
use tokio::sync::Notify;
use url::Url;

/// Base URL for update JSON files on the fixed `updater` GitHub Release tag.
const UPDATER_BASE_URL: &str =
    "https://github.com/AnInsomniacy/motrix-next/releases/download/updater";

/// Serializable update metadata returned to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct UpdateMetadata {
    pub version: String,
    pub body: Option<String>,
    pub date: Option<String>,
}

/// Progress event emitted to the frontend during update download.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", content = "data")]
pub enum UpdateProgressEvent {
    Started {
        content_length: u64,
    },
    Progress {
        chunk_length: usize,
        downloaded: u64,
    },
    Finished,
}

/// Shared state for coordinating update cancellation between commands.
pub struct UpdateCancelState {
    /// Set to `true` when the user requests cancellation.
    cancelled: AtomicBool,
    /// Notified when cancellation is requested, waking the `select!` branch.
    notify: Notify,
}

impl UpdateCancelState {
    pub fn new() -> Self {
        Self {
            cancelled: AtomicBool::new(false),
            notify: Notify::new(),
        }
    }

    /// Arms the cancel state for a new download (resets the flag).
    fn reset(&self) {
        self.cancelled.store(false, Ordering::SeqCst);
    }

    /// Signals cancellation.
    fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        self.notify.notify_waiters();
    }

    /// Returns `true` if cancellation has been requested.
    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

/// Returns the update endpoint URL for the given channel.
fn endpoint_for_channel(channel: &str) -> String {
    let file = if channel == "beta" {
        "beta.json"
    } else {
        "latest.json"
    };
    format!("{}/{}", UPDATER_BASE_URL, file)
}

/// Checks for available updates on the specified channel.
///
/// Returns `Some(UpdateMetadata)` if an update is available, or `None`
/// if the application is already on the latest version for that channel.
#[tauri::command]
pub async fn check_for_update(
    app: AppHandle,
    channel: String,
) -> Result<Option<UpdateMetadata>, AppError> {
    let endpoint = Url::parse(&endpoint_for_channel(&channel))
        .map_err(|e| AppError::Updater(e.to_string()))?;

    let update = app
        .updater_builder()
        .endpoints(vec![endpoint])
        .map_err(|e| AppError::Updater(e.to_string()))?
        .build()
        .map_err(|e| AppError::Updater(e.to_string()))?
        .check()
        .await
        .map_err(|e| AppError::Updater(e.to_string()))?;

    Ok(update.map(|u| UpdateMetadata {
        version: u.version.clone(),
        body: u.body.clone(),
        date: u.date.map(|d| d.to_string()),
    }))
}

/// Downloads and installs the latest update on the specified channel.
///
/// Emits `update-progress` events to the frontend with download progress.
/// The download can be cancelled by calling `cancel_update`.
/// After installation, the frontend should call `relaunch()` to apply.
#[tauri::command]
pub async fn install_update(app: AppHandle, channel: String) -> Result<(), AppError> {
    let cancel_state = app.state::<Arc<UpdateCancelState>>();
    cancel_state.reset();

    let endpoint = Url::parse(&endpoint_for_channel(&channel))
        .map_err(|e| AppError::Updater(e.to_string()))?;

    let update = app
        .updater_builder()
        .endpoints(vec![endpoint])
        .map_err(|e| AppError::Updater(e.to_string()))?
        .build()
        .map_err(|e| AppError::Updater(e.to_string()))?
        .check()
        .await
        .map_err(|e| AppError::Updater(e.to_string()))?;

    let update = match update {
        Some(u) => u,
        None => return Ok(()),
    };

    let app_handle = app.clone();
    let cancel = cancel_state.inner().clone();
    let mut downloaded: u64 = 0;

    let download_fut = update.download_and_install(
        move |chunk_length, content_length| {
            // Stop emitting progress once cancelled (download may still be in flight)
            if cancel.is_cancelled() {
                return;
            }

            downloaded += chunk_length as u64;

            if downloaded == chunk_length as u64 {
                let _ = app_handle.emit(
                    "update-progress",
                    UpdateProgressEvent::Started {
                        content_length: content_length.unwrap_or(0),
                    },
                );
            }

            let _ = app_handle.emit(
                "update-progress",
                UpdateProgressEvent::Progress {
                    chunk_length,
                    downloaded,
                },
            );
        },
        {
            let app_handle = app.clone();
            let cancel = cancel_state.inner().clone();
            move || {
                if !cancel.is_cancelled() {
                    let _ = app_handle.emit("update-progress", UpdateProgressEvent::Finished);
                }
            }
        },
    );

    // Race the download against cancellation
    tokio::select! {
        result = download_fut => {
            if cancel_state.is_cancelled() {
                return Err(AppError::Updater("Update cancelled by user".into()));
            }
            result.map_err(|e| AppError::Updater(e.to_string()))?;
        }
        _ = cancel_state.notify.notified() => {
            return Err(AppError::Updater("Update cancelled by user".into()));
        }
    }

    Ok(())
}

/// Cancels an in-progress update download.
#[tauri::command]
pub fn cancel_update(app: AppHandle) -> Result<(), AppError> {
    let cancel_state = app.state::<Arc<UpdateCancelState>>();
    cancel_state.cancel();
    Ok(())
}
