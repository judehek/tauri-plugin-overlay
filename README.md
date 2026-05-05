# tauri-plugin-overlay

Tauri 2 plugin wrapping [`overlay-engine`](https://github.com/judehek/overlay-engine) — render WebView2 panel iframes into a running Windows game via DLL injection + composition.

Each panel is a regular HTML document loaded from your app's resources (or any URL). Host-to-panel and panel-to-host communication surfaces as Tauri commands and Tauri events; panels themselves use the [`@overlay-engine/client`](https://www.npmjs.com/package/@overlay-engine/client) package to talk back to the host.

## Install

```bash
pnpm add @judehek/tauri-plugin-overlay
```

```toml
# src-tauri/Cargo.toml
[dependencies]
tauri-plugin-overlay = "0.1"
```

## Setup

```rust
fn main() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_overlay::Builder::new()
                .with_dll_dir_resolver(|app| {
                    app.path().resource_dir().unwrap().join("dlls")
                })
                .with_static_dir_resolver(|app| {
                    app.path().resource_dir().unwrap().join("panels")
                })
                .build(),
        )
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

Add the plugin to your capability file:

```json
{
  "permissions": ["overlay:default"]
}
```

## Use from JS

```ts
import { overlay, Panel } from '@judehek/tauri-plugin-overlay';

await overlay.attach(pid);

const notif = await overlay.createPanel({
  id: 'notifications',
  url: '/notifications.html',
  bounds: { x: 0, y: 110, w: 300, h: 100 },
});

await notif.postMessage({ type: 'show', text: 'hi' });

overlay.onEvent((event) => {
  if (event.kind === 'panel-request-close' && event.panel_id === 'notifications') {
    notif.close();
  }
});
```

## Use from inside a panel

Inside the panel's HTML, import [`@overlay-engine/client`](https://www.npmjs.com/package/@overlay-engine/client):

```ts
import { host } from '@overlay-engine/client';

host.onMessage((msg) => console.log(msg));
host.postMessage({ type: 'ack' });
```

## Platform support

Windows only. On other platforms the plugin loads but every command returns `Unsupported`.
