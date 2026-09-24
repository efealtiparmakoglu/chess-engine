#!/usr/bin/env node
/* Web UI build: strip types from web/ui.ts, inline into web/dist/index.html. */

import { stripTypeScriptTypes } from "node:module";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";

const ROOT = new URL(".", import.meta.url).pathname;

const uiJs = stripTypeScriptTypes(readFileSync(ROOT + "web/ui.ts", "utf8"), {
  mode: "strip",
});

const template = readFileSync(ROOT + "web/index.template.html", "utf8");
const html = template.replace("/*__GAME__*/", uiJs);

mkdirSync(ROOT + "web/dist", { recursive: true });
writeFileSync(ROOT + "web/dist/index.html", html);
console.log(`web/dist/index.html written (${html.length} bytes) — works offline`);
