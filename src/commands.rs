//! Tauri commands callable from JS via
//! `@judehek/tauri-plugin-overlay`. Each command is a thin wrapper
//! over [`crate::state::OverlayPluginState`] (Windows) or returns
//! [`crate::Error::Unsupported`] (non-Windows).
//!
//! Argument shapes are intentionally simple JSON so the JS side
//! doesn't need bespoke ser/de — just plain objects.

use serde::Deserialize;
use tauri::{AppHandle, Runtime, State};

use crate::error::Result;
use crate::{PanelOptions, Rect};

/// Argument body for the `create_panel` command. Wire-compatible
/// with [`PanelOptions`] but with `serde(rename_all = "camelCase")`
/// so JS callers don't need to know about Rust-side snake_case.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePanelArgs {
    pub id: String,
    pub url: String,
    pub bounds: Rect,
    #[serde(default)]
    pub interactive: bool,
    #[serde(default)]
    pub z_index: i32,
}

impl From<CreatePanelArgs> for PanelOptions {
    fn from(args: CreatePanelArgs) -> Self {
        PanelOptions {
            id: args.id,
            url: args.url,
            bounds: args.bounds,
            interactive: args.interactive,
            z_index: args.z_index,
        }
    }
}

#[tauri::command]
pub fn is_supported() -> bool {
    cfg!(target_os = "windows")
}

#[tauri::command]
pub async fn attach<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, crate::state::OverlayPluginState>,
    pid: u32,
) -> Result<()> {
    state.attach(app, pid).await
}

#[tauri::command]
pub async fn detach(state: State<'_, crate::state::OverlayPluginState>) -> Result<()> {
    state.detach().await
}

#[tauri::command]
pub async fn is_attached(
    state: State<'_, crate::state::OverlayPluginState>,
) -> Result<bool> {
    Ok(state.is_attached().await)
}

#[tauri::command]
pub async fn create_panel(
    state: State<'_, crate::state::OverlayPluginState>,
    args: CreatePanelArgs,
) -> Result<()> {
    state.create_panel(args.into()).await
}

#[tauri::command]
pub async fn close_panel(
    state: State<'_, crate::state::OverlayPluginState>,
    id: String,
) -> Result<()> {
    state.close_panel(id).await
}

#[tauri::command]
pub async fn set_panel_bounds(
    state: State<'_, crate::state::OverlayPluginState>,
    id: String,
    bounds: Rect,
) -> Result<()> {
    state.set_panel_bounds(id, bounds).await
}

#[tauri::command]
pub async fn set_panel_interactive(
    state: State<'_, crate::state::OverlayPluginState>,
    id: String,
    interactive: bool,
) -> Result<()> {
    state.set_panel_interactive(id, interactive).await
}

#[tauri::command]
pub async fn set_panel_z_index(
    state: State<'_, crate::state::OverlayPluginState>,
    id: String,
    z_index: i32,
) -> Result<()> {
    state.set_panel_z_index(id, z_index).await
}

#[tauri::command]
pub async fn post_panel_message(
    state: State<'_, crate::state::OverlayPluginState>,
    id: String,
    payload: serde_json::Value,
) -> Result<()> {
    state.post_panel_message(id, payload).await
}

#[tauri::command]
pub async fn ping_shell(state: State<'_, crate::state::OverlayPluginState>) -> Result<()> {
    state.ping_shell().await
}
