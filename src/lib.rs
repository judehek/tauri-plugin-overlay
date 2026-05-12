//! # tauri-plugin-overlay
//!
//! Tauri 2 plugin wrapping
//! [`overlay-engine`](https://github.com/judehek/overlay-engine).
//! Renders WebView2 panels into running Windows games via DLL
//! injection + composition; exposes the panel lifecycle as Tauri
//! commands and the `chrome.webview` message bus as Tauri events.
//!
//! ## Quick start
//!
//! ```no_run
//! tauri::Builder::default()
//!     .plugin(
//!         tauri_plugin_overlay::Builder::new()
//!             .with_dll_dir_resolver(|app| {
//!                 app.path().resource_dir().unwrap().join("dlls")
//!             })
//!             .with_static_dir_resolver(|app| {
//!                 app.path().resource_dir().unwrap().join("panels")
//!             })
//!             .build(),
//!     )
//!     .run(tauri::generate_context!())
//!     .expect("error while running tauri application");
//! ```
//!
//! From JS:
//!
//! ```ts
//! import { overlay } from '@judehek/tauri-plugin-overlay';
//! await overlay.attach(pid);
//! const panel = await overlay.createPanel({
//!   id: 'notifications',
//!   url: '/notifications.html',
//!   bounds: { x: 0, y: 110, w: 300, h: 100 },
//! });
//! await panel.postMessage({ type: 'show', text: 'hi' });
//! ```

use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{plugin::TauriPlugin, AppHandle, Manager, Runtime};

mod commands;
mod error;
mod state;

pub use error::{Error, Result};
pub use state::{OverlayConfig, OverlayManagerExt, OverlayPluginState};

// Re-export the engine's surface-layout types so consumers don't
// have to add `overlay-engine` as a direct dep just to build a
// layout passed to `with_surface_layout_resolver`.
#[cfg(target_os = "windows")]
pub use overlay_engine::{PercentLength, SurfaceLayout};

/// Pixel-space rectangle on the WebView2 surface. Cross-platform so
/// consumer code (e.g. plugin commands, Rust callers) can construct
/// it without conditionally importing `overlay_engine::Rect`.
///
/// On Windows we provide a `From` conversion into the engine's
/// `Rect`; on other targets the type still exists, but the methods
/// that consume it return [`Error::Unsupported`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

#[cfg(target_os = "windows")]
impl From<Rect> for overlay_engine::Rect {
    fn from(r: Rect) -> Self {
        overlay_engine::Rect {
            x: r.x,
            y: r.y,
            w: r.w,
            h: r.h,
        }
    }
}

/// Configuration for a panel created via
/// [`OverlayPluginState::create_panel`]. Mirrors the shape of
/// `overlay_engine::PanelOptions` but is plugin-owned and
/// cross-platform.
#[derive(Debug, Clone)]
pub struct PanelOptions {
    pub id: String,
    pub url: String,
    pub bounds: Rect,
    pub interactive: bool,
    pub z_index: i32,
}

#[cfg(target_os = "windows")]
impl From<PanelOptions> for overlay_engine::PanelOptions {
    fn from(opts: PanelOptions) -> Self {
        overlay_engine::PanelOptions {
            id: opts.id,
            url: opts.url,
            bounds: opts.bounds.into(),
            interactive: opts.interactive,
            z_index: opts.z_index,
        }
    }
}

/// Tauri event name on which `OverlayEvent`s are emitted to JS.
/// Corresponds to `await listen('plugin-overlay://event', ...)` on
/// the JS side.
pub const EVENT_NAME: &str = "plugin-overlay://event";

/// Resolves a directory path at plugin-setup time. Useful for
/// directories that depend on the running app (e.g. resolving
/// `app.path().resource_dir()` for the DLL or panel assets).
type DirResolver<R> =
    Box<dyn for<'a> Fn(&'a AppHandle<R>) -> PathBuf + Send + Sync + 'static>;

/// Resolves a `(width, height)` pair at plugin-setup time. Used for
/// configuration that depends on the running app's environment
/// (e.g. the user's primary monitor size for the WebView2 surface).
type SizeResolver<R> =
    Box<dyn for<'a> Fn(&'a AppHandle<R>) -> (u32, u32) + Send + Sync + 'static>;

/// Resolves a [`SurfaceLayout`] at plugin-setup time. Returning
/// `None` keeps the engine's default top-left layout.
///
/// [`SurfaceLayout`]: overlay_engine::SurfaceLayout
#[cfg(target_os = "windows")]
type LayoutResolver<R> = Box<
    dyn for<'a> Fn(&'a AppHandle<R>) -> Option<overlay_engine::SurfaceLayout>
        + Send
        + Sync
        + 'static,
>;

/// Resolves an extra `axum::Router` at plugin-setup time. Use this
/// when the routes need an `AppHandle` to construct — e.g. routes
/// that serve Tauri's embedded `frontendDist` blob via
/// `app.asset_resolver()` to the injected WebView2.
#[cfg(target_os = "windows")]
type RouterResolver<R> =
    Box<dyn for<'a> Fn(&'a AppHandle<R>) -> axum::Router + Send + Sync + 'static>;


/// Plugin builder. Configure DLL + panel-asset locations, then call
/// [`Builder::build`] to install the plugin into a `tauri::Builder`.
///
/// Generic over the Tauri runtime; defaults to `tauri::Wry`. The
/// runtime here MUST match the runtime of the host `tauri::Builder`
/// the plugin gets passed to.
pub struct Builder<R: Runtime = tauri::Wry> {
    dll_dir_resolver: Option<DirResolver<R>>,
    static_dir_resolver: Option<DirResolver<R>>,
    user_data_folder_resolver: Option<DirResolver<R>>,
    surface_size_resolver: Option<SizeResolver<R>>,
    #[cfg(target_os = "windows")]
    surface_layout_resolver: Option<LayoutResolver<R>>,
    #[cfg(target_os = "windows")]
    extra_router: Option<axum::Router>,
    #[cfg(target_os = "windows")]
    extra_router_resolver: Option<RouterResolver<R>>,
    #[cfg(target_os = "windows")]
    document_created_scripts: Vec<String>,
}

impl<R: Runtime> Default for Builder<R> {
    fn default() -> Self {
        Self {
            dll_dir_resolver: None,
            static_dir_resolver: None,
            user_data_folder_resolver: None,
            surface_size_resolver: None,
            #[cfg(target_os = "windows")]
            surface_layout_resolver: None,
            #[cfg(target_os = "windows")]
            extra_router: None,
            #[cfg(target_os = "windows")]
            extra_router_resolver: None,
            #[cfg(target_os = "windows")]
            document_created_scripts: Vec::new(),
        }
    }
}

impl<R: Runtime> Builder<R> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Resolve the DLL directory at plugin-setup time (so the resolver
    /// can use the running app's `app.path()` to locate Tauri's
    /// resource directory).
    pub fn with_dll_dir_resolver<F>(mut self, f: F) -> Self
    where
        F: for<'a> Fn(&'a AppHandle<R>) -> PathBuf + Send + Sync + 'static,
    {
        self.dll_dir_resolver = Some(Box::new(f));
        self
    }

    /// Resolve the panel-assets directory at plugin-setup time. This
    /// is what the asset server serves from `/<path>`.
    pub fn with_static_dir_resolver<F>(mut self, f: F) -> Self
    where
        F: for<'a> Fn(&'a AppHandle<R>) -> PathBuf + Send + Sync + 'static,
    {
        self.static_dir_resolver = Some(Box::new(f));
        self
    }

    /// Resolve the WebView2 user-data folder at plugin-setup time.
    /// `None` from the resolver (i.e. omitting this call) keeps
    /// WebView2's default `<exe>.WebView2/EBWebView/` next to the host
    /// process. Set this to a path you also configure for any other
    /// WebView2 in the host (e.g. the Tauri main window via
    /// `WEBVIEW2_USER_DATA_FOLDER`) so a sign-in flow in one webview
    /// is visible to the in-game overlay.
    pub fn with_user_data_folder_resolver<F>(mut self, f: F) -> Self
    where
        F: for<'a> Fn(&'a AppHandle<R>) -> PathBuf + Send + Sync + 'static,
    {
        self.user_data_folder_resolver = Some(Box::new(f));
        self
    }

    /// Mount additional axum routes alongside the built-in shell +
    /// panel-asset routes. Useful for app-specific endpoints (e.g.
    /// a video-streaming endpoint with token gating).
    #[cfg(target_os = "windows")]
    pub fn with_extra_router(mut self, router: axum::Router) -> Self {
        self.extra_router = Some(router);
        self
    }

    /// Resolve an extra `axum::Router` at plugin-setup time, when an
    /// `AppHandle` is available. Use this (rather than
    /// [`Self::with_extra_router`]) when the router's handlers need to
    /// hold an `AppHandle` — e.g. to serve Tauri's embedded
    /// `frontendDist` files to the injected WebView2 via
    /// `app.asset_resolver()`. If both this and `with_extra_router`
    /// are set, the resolver wins.
    #[cfg(target_os = "windows")]
    pub fn with_extra_router_resolver<F>(mut self, f: F) -> Self
    where
        F: for<'a> Fn(&'a AppHandle<R>) -> axum::Router + Send + Sync + 'static,
    {
        self.extra_router_resolver = Some(Box::new(f));
        self
    }

    /// Resolve the WebView2 composition surface size at plugin-setup
    /// time. The returned `(width, height)` is the render-target the
    /// engine creates for the overlay; it's stretched to cover the
    /// game window, so undersizing it produces blurry panels on
    /// high-resolution games. A reasonable default is the user's
    /// primary monitor size:
    ///
    /// ```ignore
    /// .with_surface_size_resolver(|app| {
    ///     app.primary_monitor()
    ///         .ok()
    ///         .flatten()
    ///         .map(|m| (m.size().width, m.size().height))
    ///         .unwrap_or((1920, 1080))
    /// })
    /// ```
    pub fn with_surface_size_resolver<F>(mut self, f: F) -> Self
    where
        F: for<'a> Fn(&'a AppHandle<R>) -> (u32, u32) + Send + Sync + 'static,
    {
        self.surface_size_resolver = Some(Box::new(f));
        self
    }

    /// Resolve where in the game window the overlay surface should
    /// be composited. Default is top-left at native size, which is
    /// the right answer for surfaces sized to cover the whole game
    /// window. Use this for sub-window panels (a phone-shaped HUD
    /// on the right edge, a corner banner, etc.).
    ///
    /// ```ignore
    /// use overlay_engine::{PercentLength, SurfaceLayout};
    ///
    /// .with_surface_layout_resolver(|_app| {
    ///     // Anchor the right-center of the surface to the
    ///     // right-center of the game window, with a 24 px
    ///     // right margin.
    ///     Some(SurfaceLayout {
    ///         position: (PercentLength::Percent(1.0), PercentLength::Percent(0.5)),
    ///         anchor:   (PercentLength::Percent(1.0), PercentLength::Percent(0.5)),
    ///         margin:   (
    ///             PercentLength::ZERO,
    ///             PercentLength::Length(24.0),
    ///             PercentLength::ZERO,
    ///             PercentLength::ZERO,
    ///         ),
    ///     })
    /// })
    /// ```
    #[cfg(target_os = "windows")]
    pub fn with_surface_layout_resolver<F>(mut self, f: F) -> Self
    where
        F: for<'a> Fn(&'a AppHandle<R>) -> Option<overlay_engine::SurfaceLayout>
            + Send
            + Sync
            + 'static,
    {
        self.surface_layout_resolver = Some(Box::new(f));
        self
    }

    /// Inject a JavaScript snippet into every document the WebView2
    /// loads, via WebView2's `AddScriptToExecuteOnDocumentCreated`.
    /// The script runs at the start of every document, before the
    /// page's own scripts. Call multiple times to register multiple
    /// scripts; they execute in registration order.
    ///
    /// Useful for installing a host-controlled UI layer (e.g. a
    /// draggable frame around panel content) that survives top-frame
    /// navigations away from the embedded shell page. The script
    /// communicates with the host via `chrome.webview.postMessage`,
    /// which arrives on the host as a regular overlay panel-message
    /// event.
    #[cfg(target_os = "windows")]
    pub fn with_document_created_script(mut self, script: impl Into<String>) -> Self {
        self.document_created_scripts.push(script.into());
        self
    }

    /// Build the Tauri plugin. Returns a `TauriPlugin` you pass to
    /// `tauri::Builder::plugin(...)`.
    pub fn build(self) -> TauriPlugin<R> {
        // Capture inputs so the `setup` closure can convert them into
        // an `OverlayConfig` once it has an `AppHandle`. The `Mutex`
        // is for `Sync`; the value is taken exactly once on first
        // invocation.
        let pending = Mutex::new(Some(PendingConfig::<R> {
            dll_dir_resolver: self.dll_dir_resolver,
            static_dir_resolver: self.static_dir_resolver,
            user_data_folder_resolver: self.user_data_folder_resolver,
            surface_size_resolver: self.surface_size_resolver,
            #[cfg(target_os = "windows")]
            surface_layout_resolver: self.surface_layout_resolver,
            #[cfg(target_os = "windows")]
            extra_router: self.extra_router,
            #[cfg(target_os = "windows")]
            extra_router_resolver: self.extra_router_resolver,
            #[cfg(target_os = "windows")]
            document_created_scripts: self.document_created_scripts,
        }));

        tauri::plugin::Builder::<R>::new("overlay")
            .invoke_handler(tauri::generate_handler![
                commands::is_supported,
                commands::attach,
                commands::detach,
                commands::is_attached,
                commands::create_panel,
                commands::close_panel,
                commands::set_panel_bounds,
                commands::set_panel_interactive,
                commands::set_panel_z_index,
                commands::post_panel_message,
                commands::ping_shell,
            ])
            .setup(move |app, _api| {
                let pending = pending
                    .lock()
                    .map_err(|e| format!("plugin builder mutex poisoned: {e}"))?
                    .take()
                    .ok_or_else(|| "plugin builder consumed twice".to_string())?;
                let app_handle = app.app_handle();
                let dll_dir = pending.dll_dir_resolver.as_ref().map(|f| f(app_handle));
                let static_dir = pending
                    .static_dir_resolver
                    .as_ref()
                    .map(|f| f(app_handle));
                let user_data_folder = pending
                    .user_data_folder_resolver
                    .as_ref()
                    .map(|f| f(app_handle));
                let surface_size = pending
                    .surface_size_resolver
                    .as_ref()
                    .map(|f| f(app_handle));
                #[cfg(target_os = "windows")]
                let surface_layout = pending
                    .surface_layout_resolver
                    .as_ref()
                    .and_then(|f| f(app_handle));
                #[cfg(target_os = "windows")]
                let extra_router = pending
                    .extra_router_resolver
                    .as_ref()
                    .map(|f| f(app_handle))
                    .or(pending.extra_router);
                let config = OverlayConfig {
                    dll_dir,
                    static_dir,
                    surface_size,
                    user_data_folder,
                    #[cfg(target_os = "windows")]
                    surface_layout,
                    #[cfg(target_os = "windows")]
                    extra_router,
                    #[cfg(target_os = "windows")]
                    document_created_scripts: pending.document_created_scripts,
                };
                app.manage(OverlayPluginState::new(config));
                Ok(())
            })
            .build()
    }
}

struct PendingConfig<R: Runtime> {
    dll_dir_resolver: Option<DirResolver<R>>,
    static_dir_resolver: Option<DirResolver<R>>,
    user_data_folder_resolver: Option<DirResolver<R>>,
    surface_size_resolver: Option<SizeResolver<R>>,
    #[cfg(target_os = "windows")]
    surface_layout_resolver: Option<LayoutResolver<R>>,
    #[cfg(target_os = "windows")]
    extra_router: Option<axum::Router>,
    #[cfg(target_os = "windows")]
    extra_router_resolver: Option<RouterResolver<R>>,
    #[cfg(target_os = "windows")]
    document_created_scripts: Vec<String>,
}

/// Convenience function so callers can write
/// `.plugin(tauri_plugin_overlay::init())` without going through the
/// builder. Equivalent to `Builder::new().build()`.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::<R>::new().build()
}
