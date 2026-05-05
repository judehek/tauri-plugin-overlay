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
import { type UnlistenFn } from "@tauri-apps/api/event";
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
export type OverlayEvent = {
    kind: "shell-ready";
} | {
    kind: "panel-loaded";
    panel_id: string;
} | {
    kind: "panel-error";
    panel_id: string;
    error: string;
} | {
    kind: "panel-message";
    panel_id: string;
    payload: unknown;
} | {
    kind: "panel-request-close";
    panel_id: string;
} | {
    kind: "engine";
    description: string;
    terminal: boolean;
};
/** Whether the overlay is supported on this platform (Windows-only). */
export declare function isSupported(): Promise<boolean>;
/**
 * Attach to a running game process. The plugin first lazy-builds the
 * underlying overlay (binding the asset server, resolving the DLL
 * dir) on the first call, then injects + brings up the shell.
 */
export declare function attach(pid: number): Promise<void>;
/** Tear down the overlay. The next `attach` reuses the same overlay. */
export declare function detach(): Promise<void>;
/** Whether `attach` has been called (regardless of whether the engine is still up). */
export declare function isAttached(): Promise<boolean>;
/** Liveness check: the shell replies with a `pong`. */
export declare function pingShell(): Promise<void>;
/**
 * Create a panel inside the overlay shell. Returns a [`Panel`]
 * handle scoped to the supplied id. Calling `createPanel` with an
 * id that's already registered rejects with a "panel id already
 * exists" error.
 */
export declare function createPanel(options: CreatePanelOptions): Promise<Panel>;
/** Subscribe to overlay events. Returns an `unlisten` fn. */
export declare function onEvent(handler: (event: OverlayEvent) => void): Promise<UnlistenFn>;
/**
 * Lightweight handle to a mounted panel. All methods round-trip
 * through Tauri commands (no shared state with the JS side).
 */
export declare class Panel {
    readonly id: string;
    constructor(id: string);
    setBounds(bounds: Rect): Promise<void>;
    setInteractive(interactive: boolean): Promise<void>;
    setZIndex(zIndex: number): Promise<void>;
    /** Send a JSON-serialisable payload down to the panel iframe. */
    postMessage(payload: unknown): Promise<void>;
    close(): Promise<void>;
}
/**
 * Convenience grouping for ergonomic imports:
 *
 * ```ts
 * import { overlay } from '@judehek/tauri-plugin-overlay';
 * await overlay.attach(pid);
 * ```
 */
export declare const overlay: {
    isSupported: typeof isSupported;
    attach: typeof attach;
    detach: typeof detach;
    isAttached: typeof isAttached;
    pingShell: typeof pingShell;
    createPanel: typeof createPanel;
    onEvent: typeof onEvent;
};
export default overlay;
