/**
 * `@judehek/tauri-plugin-overlay` — host-side JS bindings for
 * `tauri-plugin-overlay`.
 *
 * The plugin renders WebView2 panel iframes into a running Windows
 * game's process. This package exposes a small object-oriented API
 * for the Tauri webview to drive the overlay (`attach`, `createPanel`,
 * `panel.postMessage`, etc.) and an event subscription helper for
 * reacting to messages from inside panels.
 */

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface CreatePanelOptions {
  /** Stable user-chosen id; must be unique per overlay attach. */
  id: string;
  /** URL the iframe loads (e.g. `/notifications.html`). */
  url: string;
  bounds: Rect;
  /** Whether the panel captures input. Default: `false`. */
  interactive?: boolean;
  /** Stacking order. Higher renders on top. Default: `0`. */
  zIndex?: number;
}

export type OverlayEvent =
  | { kind: "shell-ready" }
  | { kind: "panel-loaded"; panel_id: string }
  | { kind: "panel-error"; panel_id: string; error: string }
  | { kind: "panel-message"; panel_id: string; payload: unknown }
  | { kind: "panel-request-close"; panel_id: string }
  | { kind: "engine"; description: string; terminal: boolean };

const EVENT_NAME = "plugin-overlay://event";

function cmd<T>(name: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(`plugin:overlay|${name}`, args ?? {});
}

/** Whether the overlay is supported on this platform (Windows-only). */
export function isSupported(): Promise<boolean> {
  return cmd<boolean>("is_supported");
}

/**
 * Attach to a running game process. The plugin first lazy-builds the
 * underlying overlay (binding the asset server, resolving the DLL
 * dir) on the first call, then injects + brings up the shell.
 */
export function attach(pid: number): Promise<void> {
  return cmd<void>("attach", { pid });
}

/** Tear down the overlay. The next `attach` reuses the same overlay. */
export function detach(): Promise<void> {
  return cmd<void>("detach");
}

/** Whether `attach` has been called (regardless of whether the engine is still up). */
export function isAttached(): Promise<boolean> {
  return cmd<boolean>("is_attached");
}

/** Liveness check: the shell replies with a `pong`. */
export function pingShell(): Promise<void> {
  return cmd<void>("ping_shell");
}

/**
 * Create a panel inside the overlay shell. Returns a [`Panel`]
 * handle scoped to the supplied id. Calling `createPanel` with an
 * id that's already registered rejects with a "panel id already
 * exists" error.
 */
export async function createPanel(options: CreatePanelOptions): Promise<Panel> {
  await cmd<void>("create_panel", {
    args: {
      id: options.id,
      url: options.url,
      bounds: options.bounds,
      interactive: options.interactive ?? false,
      zIndex: options.zIndex ?? 0,
    },
  });
  return new Panel(options.id);
}

/** Subscribe to overlay events. Returns an `unlisten` fn. */
export function onEvent(handler: (event: OverlayEvent) => void): Promise<UnlistenFn> {
  return listen<OverlayEvent>(EVENT_NAME, (e) => handler(e.payload));
}

/**
 * Lightweight handle to a mounted panel. All methods round-trip
 * through Tauri commands (no shared state with the JS side).
 */
export class Panel {
  constructor(public readonly id: string) {}

  setBounds(bounds: Rect): Promise<void> {
    return cmd<void>("set_panel_bounds", { id: this.id, bounds });
  }

  setInteractive(interactive: boolean): Promise<void> {
    return cmd<void>("set_panel_interactive", { id: this.id, interactive });
  }

  setZIndex(zIndex: number): Promise<void> {
    return cmd<void>("set_panel_z_index", { id: this.id, zIndex });
  }

  /** Send a JSON-serialisable payload down to the panel iframe. */
  postMessage(payload: unknown): Promise<void> {
    return cmd<void>("post_panel_message", { id: this.id, payload });
  }

  close(): Promise<void> {
    return cmd<void>("close_panel", { id: this.id });
  }
}

/**
 * Convenience grouping for ergonomic imports:
 *
 * ```ts
 * import { overlay } from '@judehek/tauri-plugin-overlay';
 * await overlay.attach(pid);
 * ```
 */
export const overlay = {
  isSupported,
  attach,
  detach,
  isAttached,
  pingShell,
  createPanel,
  onEvent,
};

export default overlay;
