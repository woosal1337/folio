//! `/api/account/*` — profile, devices, soft-delete.
//!
//! Tier-2 enrollment endpoints (`/account/tier2-*`) are intentionally
//! omitted here; they ship with the recordings-sync work where they
//! actually apply.

use crate::backend::client::{BackendClient, BackendError};
use crate::backend::types::{AccountUpdateRequest, DeviceDoc, UserDoc};

#[derive(serde::Deserialize)]
struct AccountResponse {
    user: UserDoc,
}

pub async fn get_account(client: &BackendClient) -> Result<UserDoc, BackendError> {
    let resp: AccountResponse = client.get("/account").await?;
    Ok(resp.user)
}

pub async fn update_account(
    client: &BackendClient,
    display_name: Option<&str>,
) -> Result<UserDoc, BackendError> {
    let body = AccountUpdateRequest { display_name };
    let resp: AccountResponse = client.patch("/account", &body).await?;
    Ok(resp.user)
}

#[derive(serde::Deserialize)]
struct DevicesResponse {
    devices: Vec<DeviceDoc>,
}

pub async fn list_devices(client: &BackendClient) -> Result<Vec<DeviceDoc>, BackendError> {
    let resp: DevicesResponse = client.get("/account/devices").await?;
    Ok(resp.devices)
}

pub async fn revoke_device(client: &BackendClient, device_id: &str) -> Result<(), BackendError> {
    let path = format!("/account/devices/{}", device_id);
    client.delete::<()>(&path).await
}

pub async fn soft_delete_account(client: &BackendClient) -> Result<(), BackendError> {
    client.delete::<()>("/account").await
}
