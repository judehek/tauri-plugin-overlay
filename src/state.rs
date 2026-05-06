//! Plugin app state.
//!
//! On Windows we wrap an [`overlay_engine::Overlay`] (lazy-built from
//! the user-supplied [`crate::Builder`] options at first attach), plus
//! a tokio task that pumps overlay events into the Tauri event bus.
//!
//! On non-Windows builds the plugin still loads and registers (so
//! consumers can `init()` unconditionally), but every method on
//! [`OverlayPluginState`] returns [`crate::Error::Unsupported`] (or a
//! sensible default like `false` for `is_attached`). This lets
//! consumer code call the plugin API from cross-platform code without
//! cfg-gating every call site.

use std::path::PathBuf;

use tauri::{AppHandle, Manager, Runtime};

use crate::error::Result;
#[cfg(not(target_os = "windows"))]
use crate::error::Error;
use crate::{PanelOptions, Rect};

/// Configuration captured by [`crate::Builder`] and consumed lazily
/// when the user calls `attach`. Parts that depend on the
/// `AppHandle` (e.g. resolving Tauri's resource dir) are evaluated
/// inside the plugin's `setup` hook before this is stashed.
pub struct OverlayConfig {
    pub dll_dir: Option<PathBuf>,
    pub static_dir: Option<PathBuf>,
    /// Override for the WebView2 composition surface size, in
    /// physical pixels. `None` lets the engine fall back to its
    /// internal default. The engine stretches this surface to cover
    /// the game window, so a small surface on a 1080p+ game looks
    /// blurry; size to the user's primary monitor (or the game
    /// window if known) for crisp panels.
    pub surface_size: Option<(u32, u32)>,
    #[cfg(target_os = "windows")]
    pub extra_router: Option<axum::Router>,
}

/// Tauri-managed handle the plugin commands and consumer Rust code
/// reach for. All methods are async and have the same signature on
/// every platform; non-Windows targets short-circuit with
/// [`Error::Unsupported`].
pub struct OverlayPluginState {
    #[cfg(target_os = "windows")]
    inner: imp::Inner,
}

#[cfg(target_os = "windows")]
impl OverlayPluginState {
    pub fn new(config: OverlayConfig) -> Self {
        Self {
            inner: imp::Inner::new(config),
        }
    }

    pub async fn attach<R: Runtime>(&self, app: AppHandle<R>, pid: u32) -> Result<()> {
        self.inner.attach(app, pid).await
    }

    pub async fn detach(&self) -> Result<()> {
        self.inner.detach().await
    }

    pub async fn is_attached(&self) -> bool {
        self.inner.is_attached().await
    }

    pub async fn create_panel(&self, options: PanelOptions) -> Result<()> {
        self.inner.create_panel(options).await
    }

    pub async fn close_panel(&self, id: String) -> Result<()> {
        self.inner.close_panel(id).await
    }

    pub async fn set_panel_bounds(&self, id: String, bounds: Rect) -> Result<()> {
        self.inner.set_panel_bounds(id, bounds).await
    }

    pub async fn set_panel_interactive(&self, id: String, interactive: bool) -> Result<()> {
        self.inner.set_panel_interactive(id, interactive).await
    }

    pub async fn set_panel_z_index(&self, id: String, z_index: i32) -> Result<()> {
        self.inner.set_panel_z_index(id, z_index).await
    }

    pub async fn post_panel_message(
        &self,
        id: String,
        payload: serde_json::Value,
    ) -> Result<()> {
        self.inner.post_panel_message(id, payload).await
    }

    pub async fn ping_shell(&self) -> Result<()> {
        self.inner.ping_shell().await
    }

    pub async fn asset_origin(&self) -> Option<String> {
        self.inner.asset_origin().await
    }

    pub async fn ensure_built(&self) -> Result<()> {
        self.inner.ensure_built().await
    }
}

#[cfg(not(target_os = "windows"))]
impl OverlayPluginState {
    pub fn new(_config: OverlayConfig) -> Self {
        Self {}
    }

    pub async fn attach<R: Runtime>(&self, _app: AppHandle<R>, _pid: u32) -> Result<()> {
        Err(unsupported())
    }

    pub async fn detach(&self) -> Result<()> {
        Err(unsupported())
    }

    pub async fn is_attached(&self) -> bool {
        false
    }

    pub async fn create_panel(&self, _options: PanelOptions) -> Result<()> {
        Err(unsupported())
    }

    pub async fn close_panel(&self, _id: String) -> Result<()> {
        Err(unsupported())
    }

    pub async fn set_panel_bounds(&self, _id: String, _bounds: Rect) -> Result<()> {
        Err(unsupported())
    }

    pub async fn set_panel_interactive(&self, _id: String, _interactive: bool) -> Result<()> {
        Err(unsupported())
    }

    pub async fn set_panel_z_index(&self, _id: String, _z_index: i32) -> Result<()> {
        Err(unsupported())
    }

    pub async fn post_panel_message(
        &self,
        _id: String,
        _payload: serde_json::Value,
    ) -> Result<()> {
        Err(unsupported())
    }

    pub async fn ping_shell(&self) -> Result<()> {
        Err(unsupported())
    }

    pub async fn asset_origin(&self) -> Option<String> {
        None
    }

    pub async fn ensure_built(&self) -> Result<()> {
        Err(unsupported())
    }
}

#[cfg(not(target_os = "windows"))]
fn unsupported() -> Error {
    Error::Unsupported("overlay only supported on Windows")
}

/// Extension trait so consumer Rust code (e.g. inside Ascent) can
/// reach the plugin state without going through a command.
pub trait OverlayManagerExt<R: Runtime>: Manager<R> {
    fn overlay_state(&self) -> tauri::State<'_, OverlayPluginState> {
        self.state::<OverlayPluginState>()
    }
}
impl<R: Runtime, M: Manager<R>> OverlayManagerExt<R> for M {}

// ---------------------------------------------------------------------------
// Windows-only inner implementation
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
mod imp {
    use overlay_engine::{Overlay, OverlayEvent};
    use serde::Serialize;
    use tauri::{AppHandle, Emitter, Runtime};
    use tokio::sync::Mutex;

    use crate::error::{Error, Result};
    use crate::{EVENT_NAME, PanelOptions, Rect};

    use super::OverlayConfig;

    pub(super) struct Inner {
        config: Mutex<Option<OverlayConfig>>,
        overlay: Mutex<Option<Overlay>>,
    }

    impl Inner {
        pub fn new(config: OverlayConfig) -> Self {
            Self {
                config: Mutex::new(Some(config)),
                overlay: Mutex::new(None),
            }
        }

        async fn overlay(&self) -> Result<Overlay> {
            let mut guard = self.overlay.lock().await;
            if let Some(o) = &*guard {
                return Ok(o.clone());
            }
            let mut config_guard = self.config.lock().await;
            let config = config_guard.take().ok_or(Error::NotInitialized)?;
            let mut builder = Overlay::builder();
            if let Some(dir) = config.dll_dir {
                builder = builder.dll_dir(dir);
            }
            if let Some(dir) = config.static_dir {
                builder = builder.static_dir(dir);
            }
            if let Some(router) = config.extra_router {
                builder = builder.extra_router(router);
            }
            if let Some((w, h)) = config.surface_size {
                builder = builder.surface_size(w, h);
            }
            let overlay = builder.build().await?;
            *guard = Some(overlay.clone());
            Ok(overlay)
        }

        async fn require_overlay(&self) -> Result<Overlay> {
            let guard = self.overlay.lock().await;
            guard.clone().ok_or(Error::NotAttached)
        }

        pub async fn attach<R: Runtime>(&self, app: AppHandle<R>, pid: u32) -> Result<()> {
            let overlay = self.overlay().await?;
            let events = overlay.attach(pid).await?;
            tauri::async_runtime::spawn(pump_events(app, events));
            Ok(())
        }

        pub async fn detach(&self) -> Result<()> {
            self.require_overlay().await?.detach().await.map_err(Into::into)
        }

        pub async fn is_attached(&self) -> bool {
            self.overlay.lock().await.is_some()
        }

        pub async fn create_panel(&self, options: PanelOptions) -> Result<()> {
            let _ = self
                .require_overlay()
                .await?
                .create_panel(options.into())
                .await?;
            Ok(())
        }

        pub async fn close_panel(&self, id: String) -> Result<()> {
            self.require_overlay()
                .await?
                .close_panel(&id)
                .await
                .map_err(Into::into)
        }

        pub async fn set_panel_bounds(&self, id: String, bounds: Rect) -> Result<()> {
            self.require_overlay()
                .await?
                .set_panel_bounds(&id, bounds.into())
                .await
                .map_err(Into::into)
        }

        pub async fn set_panel_interactive(
            &self,
            id: String,
            interactive: bool,
        ) -> Result<()> {
            self.require_overlay()
                .await?
                .set_panel_interactive(&id, interactive)
                .await
                .map_err(Into::into)
        }

        pub async fn set_panel_z_index(&self, id: String, z_index: i32) -> Result<()> {
            self.require_overlay()
                .await?
                .set_panel_z_index(&id, z_index)
                .await
                .map_err(Into::into)
        }

        pub async fn post_panel_message(
            &self,
            id: String,
            payload: serde_json::Value,
        ) -> Result<()> {
            self.require_overlay()
                .await?
                .post_panel_message(&id, payload)
                .await
                .map_err(Into::into)
        }

        pub async fn ping_shell(&self) -> Result<()> {
            self.require_overlay()
                .await?
                .ping_shell()
                .await
                .map_err(Into::into)
        }

        pub async fn asset_origin(&self) -> Option<String> {
            self.overlay.lock().await.as_ref().map(|o| o.asset_origin())
        }

        pub async fn ensure_built(&self) -> Result<()> {
            self.overlay().await?;
            Ok(())
        }
    }

    /// Wire format of overlay events emitted to JS via Tauri's event
    /// bus. We translate the engine's `OverlayEvent` enum into a
    /// flat JSON shape that's nicer to consume on the TS side
    /// (single discriminator, kebab-case `kind`). `Clone` because
    /// `Emitter::emit` requires `Serialize + Clone`.
    #[derive(Clone, Serialize)]
    #[serde(tag = "kind", rename_all = "kebab-case")]
    enum WireEvent {
        ShellReady,
        PanelLoaded { panel_id: String },
        PanelError { panel_id: String, error: String },
        PanelMessage { panel_id: String, payload: serde_json::Value },
        PanelRequestClose { panel_id: String },
        Engine { description: String, terminal: bool },
    }

    async fn pump_events<R: Runtime>(
        app: AppHandle<R>,
        mut rx: tokio::sync::mpsc::Receiver<OverlayEvent>,
    ) {
        while let Some(event) = rx.recv().await {
            let wire = match event {
                OverlayEvent::ShellReady => WireEvent::ShellReady,
                OverlayEvent::PanelLoaded { panel_id } => WireEvent::PanelLoaded { panel_id },
                OverlayEvent::PanelError { panel_id, error } => {
                    WireEvent::PanelError { panel_id, error }
                }
                OverlayEvent::PanelMessage { panel_id, payload } => {
                    WireEvent::PanelMessage { panel_id, payload }
                }
                OverlayEvent::PanelRequestClose { panel_id } => {
                    WireEvent::PanelRequestClose { panel_id }
                }
                OverlayEvent::Engine(engine_event) => {
                    let terminal = matches!(
                        &engine_event,
                        overlay_engine::EngineEvent::Detached { .. }
                    );
                    WireEvent::Engine {
                        description: format!("{engine_event:?}"),
                        terminal,
                    }
                }
            };
            if let Err(err) = app.emit(EVENT_NAME, &wire) {
                log::warn!("[tauri-plugin-overlay] emit failed: {err:?}");
            }
        }
        let _ = app.emit(
            EVENT_NAME,
            WireEvent::Engine {
                description: "event channel closed".to_string(),
                terminal: true,
            },
        );
    }
}
