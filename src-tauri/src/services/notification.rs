//! Native task notifications for background lifecycle events.

use super::config::RuntimeConfig;
use super::monitor::{events, TaskEvent};
use super::notification_i18n::{format_error_message, format_task_message, texts_for_locale};
use tauri::Manager;

#[cfg(target_os = "linux")]
use std::{
    collections::VecDeque,
    sync::Mutex,
    time::{Duration, Instant},
};

#[cfg(not(target_os = "linux"))]
use tauri_plugin_notification::NotificationExt;

#[cfg(target_os = "linux")]
const LINUX_NOTIFICATION_RETENTION_TTL: Duration = Duration::from_secs(120);
#[cfg(target_os = "linux")]
const LINUX_NOTIFICATION_RETENTION_LIMIT: usize = 32;

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinuxNotificationIdentity {
    pub app_name: &'static str,
    pub icon: &'static str,
    pub desktop_entry: &'static str,
    pub urgency: notify_rust::Urgency,
}

#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinuxNotificationRetention {
    pub retained: bool,
    pub id: u32,
    pub registry_size: usize,
    pub retention_limit: usize,
    pub ttl_secs: u64,
    pub pruned_expired: usize,
    pub dropped_over_limit: usize,
}

#[cfg(target_os = "linux")]
pub struct LinuxNotificationRegistry {
    retained: Mutex<VecDeque<RetainedLinuxNotification>>,
}

#[cfg(target_os = "linux")]
struct RetainedLinuxNotification {
    created_at: Instant,
    _handle: notify_rust::NotificationHandle,
}

#[cfg(target_os = "linux")]
impl LinuxNotificationRegistry {
    pub fn new() -> Self {
        Self {
            retained: Mutex::new(VecDeque::new()),
        }
    }

    pub fn retain(&self, handle: notify_rust::NotificationHandle) -> LinuxNotificationRetention {
        let id = handle.id();
        let now = Instant::now();
        let mut retained = self
            .retained
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let pruned_expired =
            prune_expired_linux_notifications(&mut retained, now, LINUX_NOTIFICATION_RETENTION_TTL);

        retained.push_back(RetainedLinuxNotification {
            created_at: now,
            _handle: handle,
        });

        let dropped_over_limit =
            trim_linux_notifications_to_limit(&mut retained, LINUX_NOTIFICATION_RETENTION_LIMIT);

        LinuxNotificationRetention {
            retained: true,
            id,
            registry_size: retained.len(),
            retention_limit: LINUX_NOTIFICATION_RETENTION_LIMIT,
            ttl_secs: LINUX_NOTIFICATION_RETENTION_TTL.as_secs(),
            pruned_expired,
            dropped_over_limit,
        }
    }
}

#[cfg(target_os = "linux")]
impl Default for LinuxNotificationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "linux")]
fn prune_expired_linux_notifications(
    retained: &mut VecDeque<RetainedLinuxNotification>,
    now: Instant,
    ttl: Duration,
) -> usize {
    let original_len = retained.len();
    retained.retain(|notification| now.duration_since(notification.created_at) < ttl);
    original_len - retained.len()
}

#[cfg(target_os = "linux")]
fn trim_linux_notifications_to_limit(
    retained: &mut VecDeque<RetainedLinuxNotification>,
    limit: usize,
) -> usize {
    let original_len = retained.len();
    while retained.len() > limit {
        retained.pop_front();
    }
    original_len - retained.len()
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
        retention: LinuxNotificationRetention,
    },
}

#[cfg(target_os = "linux")]
pub fn linux_notification_identity() -> LinuxNotificationIdentity {
    LinuxNotificationIdentity {
        app_name: "motrixnext",
        icon: "motrix-next",
        desktop_entry: "MotrixNext",
        urgency: notify_rust::Urgency::Normal,
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
        NotificationDispatchResult::Delivered {
            id,
            identity,
            retention,
        } => {
            log::info!(
                "notification:delivered platform=linux id={} type={:?} gid={} locale={} webview_alive={} app_name={} icon={} desktop_entry={} urgency=normal retained=true registry_size={} retention_limit={} ttl_secs={} pruned_expired={} dropped_over_limit={}",
                id,
                content.kind,
                event.gid,
                content.locale,
                webview_alive,
                identity.app_name,
                identity.icon,
                identity.desktop_entry,
                retention.registry_size,
                retention.retention_limit,
                retention.ttl_secs,
                retention.pruned_expired,
                retention.dropped_over_limit
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
    app: &tauri::AppHandle,
    content: &TaskNotificationContent,
) -> Result<NotificationDispatchResult, String> {
    let identity = linux_notification_identity();
    let handle = notify_rust::Notification::new()
        .appname(identity.app_name)
        .icon(identity.icon)
        .hint(notify_rust::Hint::DesktopEntry(
            identity.desktop_entry.to_string(),
        ))
        .urgency(identity.urgency)
        .summary(&content.title)
        .body(&content.body)
        .show()
        .map_err(|error| error.to_string())?;
    let registry = app.state::<LinuxNotificationRegistry>();
    let retention = registry.retain(handle);

    Ok(NotificationDispatchResult::Delivered {
        id: retention.id,
        identity,
        retention,
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
    fn linux_notification_identity_matches_gnome_desktop_entry() {
        let identity = linux_notification_identity();
        assert_eq!(identity.app_name, "motrixnext");
        assert_eq!(identity.icon, "motrix-next");
        assert_eq!(identity.desktop_entry, "MotrixNext");
        assert_eq!(identity.urgency, notify_rust::Urgency::Normal);
    }
}
