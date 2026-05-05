// guest-js/index.ts
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
var EVENT_NAME = "plugin-overlay://event";
function cmd(name, args) {
  return invoke(`plugin:overlay|${name}`, args ?? {});
}
function isSupported() {
  return cmd("is_supported");
}
function attach(pid) {
  return cmd("attach", { pid });
}
function detach() {
  return cmd("detach");
}
function isAttached() {
  return cmd("is_attached");
}
function pingShell() {
  return cmd("ping_shell");
}
async function createPanel(options) {
  await cmd("create_panel", {
    args: {
      id: options.id,
      url: options.url,
      bounds: options.bounds,
      interactive: options.interactive ?? false,
      zIndex: options.zIndex ?? 0
    }
  });
  return new Panel(options.id);
}
function onEvent(handler) {
  return listen(EVENT_NAME, (e) => handler(e.payload));
}
var Panel = class {
  constructor(id) {
    this.id = id;
  }
  setBounds(bounds) {
    return cmd("set_panel_bounds", { id: this.id, bounds });
  }
  setInteractive(interactive) {
    return cmd("set_panel_interactive", { id: this.id, interactive });
  }
  setZIndex(zIndex) {
    return cmd("set_panel_z_index", { id: this.id, zIndex });
  }
  /** Send a JSON-serialisable payload down to the panel iframe. */
  postMessage(payload) {
    return cmd("post_panel_message", { id: this.id, payload });
  }
  close() {
    return cmd("close_panel", { id: this.id });
  }
};
var overlay = {
  isSupported,
  attach,
  detach,
  isAttached,
  pingShell,
  createPanel,
  onEvent
};
var guest_js_default = overlay;
export {
  Panel,
  attach,
  createPanel,
  guest_js_default as default,
  detach,
  isAttached,
  isSupported,
  onEvent,
  overlay,
  pingShell
};
//# sourceMappingURL=index.js.map
