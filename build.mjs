import * as esbuild from "esbuild";
import { dirname } from "node:path";
import { fileURLToPath } from "node:url";

const root = dirname(fileURLToPath(import.meta.url));

const baseOpts = {
  entryPoints: [`${root}/guest-js/index.ts`],
  bundle: true,
  target: "es2020",
  sourcemap: true,
  legalComments: "none",
  // The Tauri API is delivered to the webview at runtime — don't
  // bundle it. Mark all `@tauri-apps/api/*` subpaths as external.
  external: ["@tauri-apps/api", "@tauri-apps/api/*"],
};

await Promise.all([
  esbuild.build({ ...baseOpts, format: "esm", outfile: `${root}/dist-js/index.js` }),
  esbuild.build({ ...baseOpts, format: "cjs", outfile: `${root}/dist-js/index.cjs` }),
]);
console.log("[tauri-plugin-overlay] built dist-js/index.{js,cjs}");
