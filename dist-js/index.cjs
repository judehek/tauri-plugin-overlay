"use strict";
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __export = (target, all) => {
  for (var name in all)
    __defProp(target, name, { get: all[name], enumerable: true });
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toCommonJS = (mod) => __copyProps(__defProp({}, "__esModule", { value: true }), mod);

// guest-js/index.ts
var guest_js_exports = {};
__export(guest_js_exports, {
  Panel: () => Panel,
  attach: () => attach,
  createPanel: () => createPanel,
  default: () => guest_js_default,
  detach: () => detach,
  isAttached: () => isAttached,
  isSupported: () => isSupported,
  onEvent: () => onEvent,
  overlay: () => overlay,
  pingShell: () => pingShell
});
module.exports = __toCommonJS(guest_js_exports);
var import_core = require("@tauri-apps/api/core");
var import_event = require("@tauri-apps/api/event");
var EVENT_NAME = "plugin-overlay://event";
function cmd(name, args) {
  return (0, import_core.invoke)(`plugin:overlay|${name}`, args ?? {});
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
  return (0, import_event.listen)(EVENT_NAME, (e) => handler(e.payload));
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
//# sourceMappingURL=index.cjs.map
