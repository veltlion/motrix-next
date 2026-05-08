//! Native task notifications for background lifecycle events.

use super::config::RuntimeConfig;
use super::monitor::{events, TaskEvent};
use super::notification_i18n::{format_error_message, format_task_message, texts_for_locale};
use tauri::Manager;

#[cfg(not(target_os = "linux"))]
use tauri_plugin_notification::NotificationExt;

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinuxNotificationIdentity {
    pub app_name: &'static str,
    pub icon: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskNotificationKind {
    Complete,
    BtComplete,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskNotificationContent {
    pub kind: TaskNotificationKind,
    pub title: String,
    pub body: String,
    pub locale: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NotificationDispatchResult {
    #[cfg(not(target_os = "linux"))]
    Submitted,
    #[cfg(target_os = "linux")]
    Delivered {
        id: u32,
        identity: LinuxNotificationIdentity,
    },
}

#[cfg(target_os = "linux")]
pub fn linux_notification_identity() -> LinuxNotificationIdentity {
    LinuxNotificationIdentity {
        app_name: "motrixnext",
        icon: "motrix-next",
    }
}

fn kind_for_event(event_name: &str) -> Option<TaskNotificationKind> {
    match event_name {
        events::TASK_COMPLETE => Some(TaskNotificationKind::Complete),
        events::BT_COMPLETE => Some(TaskNotificationKind::BtComplete),
        events::TASK_ERROR => Some(TaskNotificationKind::Error),
        _ => None,
    }
}

fn notification_enabled(kind: TaskNotificationKind, config: &RuntimeConfig) -> bool {
    if !config.task_notification {
        return false;
    }

    match kind {
        TaskNotificationKind::Complete | TaskNotificationKind::BtComplete => {
            config.notify_on_complete
        }
        TaskNotificationKind::Error => true,
    }
}

pub fn build_task_notification(
    event_name: &str,
    event: &TaskEvent,
    config: &RuntimeConfig,
) -> Option<TaskNotificationContent> {
    let kind = kind_for_event(event_name)?;
    if !notification_enabled(kind, config) {
        return None;
    }

    let requested_locale = if config.locale == "auto" {
        sys_locale::get_locale().unwrap_or_else(|| "en-US".to_string())
    } else {
        config.locale.clone()
    };
    let locale = super::notification_i18n::resolve_supported_locale(&requested_locale);
    let texts = texts_for_locale(locale);
    let task_name = event.name.as_str();

    let (title, body) = match kind {
        TaskNotificationKind::Complete => (
            texts.download_complete_title.to_string(),
            format_task_message(texts.download_complete_body, task_name),
        ),
        TaskNotificationKind::BtComplete => (
            texts.bt_complete_title.to_string(),
            format_task_message(texts.bt_complete_body, task_name),
        ),
        TaskNotificationKind::Error => {
            let reason = event
                .error_message
                .as_deref()
                .filter(|message| !message.trim().is_empty())
                .unwrap_or(texts.error_unknown);
            (
                texts.download_failed_title.to_string(),
                format_error_message(texts.download_failed_body, task_name, reason),
            )
        }
    };

    Some(TaskNotificationContent {
        kind,
        title,
        body,
        locale,
    })
}

pub fn send_task_notification(
    app: &tauri::AppHandle,
    event_name: &str,
    event: &TaskEvent,
    config: &RuntimeConfig,
) {
    let Some(kind) = kind_for_event(event_name) else {
        return;
    };

    let Some(content) = build_task_notification(event_name, event, config) else {
        log::debug!(
            "notification:skip reason=preference-disabled type={kind:?} gid={}",
            event.gid
        );
        return;
    };

    log::debug!(
        "notification:send-start type={:?} gid={} locale={} title={:?}",
        content.kind,
        event.gid,
        content.locale,
        content.title
    );

    match send_platform_notification(app, &content) {
        Ok(dispatch) => {
            let webview_alive = app.get_webview_window("main").is_some();
            log_notification_success(&content, event, dispatch, webview_alive);
        }
        Err(e) => {
            log::warn!(
                "notification:failed type={:?} gid={} locale={} error={e}",
                content.kind,
                event.gid,
                content.locale
            );
        }
    }
}

#[cfg(target_os = "linux")]
fn log_notification_success(
    content: &TaskNotificationContent,
    event: &TaskEvent,
    dispatch: NotificationDispatchResult,
    webview_alive: bool,
) {
    match dispatch {
        NotificationDispatchResult::Delivered { id, identity } => {
            log::info!(
                "notification:delivered platform=linux id={} type={:?} gid={} locale={} webview_alive={} app_name={} icon={}",
                id,
                content.kind,
                event.gid,
                content.locale,
                webview_alive,
                identity.app_name,
                identity.icon
            );
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn log_notification_success(
    content: &TaskNotificationContent,
    event: &TaskEvent,
    dispatch: NotificationDispatchResult,
    webview_alive: bool,
) {
    match dispatch {
        NotificationDispatchResult::Submitted => {
            log::info!(
                "notification:submitted type={:?} gid={} locale={} webview_alive={}",
                content.kind,
                event.gid,
                content.locale,
                webview_alive
            );
        }
    }
}

#[cfg(target_os = "linux")]
fn send_platform_notification(
    _app: &tauri::AppHandle,
    content: &TaskNotificationContent,
) -> Result<NotificationDispatchResult, String> {
    let identity = linux_notification_identity();
    let handle = notify_rust::Notification::new()
        .appname(identity.app_name)
        .icon(identity.icon)
        .summary(&content.title)
        .body(&content.body)
        .show()
        .map_err(|error| error.to_string())?;

    Ok(NotificationDispatchResult::Delivered {
        id: handle.id(),
        identity,
    })
}

#[cfg(not(target_os = "linux"))]
fn send_platform_notification(
    app: &tauri::AppHandle,
    content: &TaskNotificationContent,
) -> Result<NotificationDispatchResult, String> {
    app.notification()
        .builder()
        .title(content.title.clone())
        .body(content.body.clone())
        .show()
        .map_err(|error| error.to_string())?;

    Ok(NotificationDispatchResult::Submitted)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> RuntimeConfig {
        RuntimeConfig {
            locale: "en-US".to_string(),
            task_notification: true,
            notify_on_complete: true,
            ..RuntimeConfig::default()
        }
    }

    fn event() -> TaskEvent {
        TaskEvent {
            gid: "g1".to_string(),
            name: "file.zip".to_string(),
            status: "complete".to_string(),
            error_code: None,
            error_message: None,
            dir: "/tmp".to_string(),
            total_length: "1".to_string(),
            completed_length: "1".to_string(),
            info_hash: None,
            is_bt: false,
            files: Vec::new(),
            announce_list: Vec::new(),
        }
    }

    #[test]
    fn builds_localised_complete_notification() {
        let content = build_task_notification(events::TASK_COMPLETE, &event(), &cfg()).unwrap();
        assert_eq!(content.kind, TaskNotificationKind::Complete);
        assert_eq!(content.title, "Download Complete");
        assert_eq!(content.body, "Saved: file.zip");
        assert_eq!(content.locale, "en-US");
    }

    #[test]
    fn builds_localised_bt_complete_notification() {
        let content = build_task_notification(events::BT_COMPLETE, &event(), &cfg()).unwrap();
        assert_eq!(content.kind, TaskNotificationKind::BtComplete);
        assert_eq!(content.title, "BT Download Complete");
        assert_eq!(content.body, "Seeding started: file.zip");
    }

    #[test]
    fn builds_localised_error_notification_with_reason() {
        let mut ev = event();
        ev.error_message = Some("Network error".to_string());
        let content = build_task_notification(events::TASK_ERROR, &ev, &cfg()).unwrap();
        assert_eq!(content.kind, TaskNotificationKind::Error);
        assert_eq!(content.title, "Download Failed");
        assert_eq!(content.body, "file.zip: Network error");
    }

    #[test]
    fn skips_completion_when_complete_notifications_are_disabled() {
        let mut config = cfg();
        config.notify_on_complete = false;
        assert!(build_task_notification(events::TASK_COMPLETE, &event(), &config).is_none());
        assert!(build_task_notification(events::TASK_ERROR, &event(), &config).is_some());
    }

    #[test]
    fn skips_all_when_task_notifications_are_disabled() {
        let mut config = cfg();
        config.task_notification = false;
        assert!(build_task_notification(events::TASK_COMPLETE, &event(), &config).is_none());
        assert!(build_task_notification(events::TASK_ERROR, &event(), &config).is_none());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_notification_identity_avoids_gnome_desktop_entry_hint() {
        let identity = linux_notification_identity();
        assert_eq!(identity.app_name, "motrixnext");
        assert_eq!(identity.icon, "motrix-next");
    }
}
