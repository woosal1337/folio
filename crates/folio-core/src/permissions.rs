use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
#[serde(rename_all = "snake_case")]
pub enum PermissionStatus {
    NotDetermined,
    Granted,
    Denied,
    Restricted,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    Microphone,
    ScreenRecording,
    Calendar,
    Reminders,
    Notifications,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/shared/types/")]
pub struct PermissionRow {
    pub permission: Permission,
    pub status: PermissionStatus,
    pub rationale: String,
    pub settings_url: String,
}
