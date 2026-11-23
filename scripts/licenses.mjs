#!/usr/bin/env node
// Licenze di terze parti di photo-cat (app-about, "Licenze di terze parti"; ui-usability-audit D7).
// Unisce in assets/licenses/third-party.json:
//   - le dipendenze Rust dell'app (`cargo metadata`, solo dipendenze normali, per tutti i target);
//   - le dipendenze di produzione dell'interfaccia (app/ui/node_modules, dopo `npm ci`);
//   - modelli, dati e librerie native da assets/licenses/manual.json.
// Il testo di ogni licenza viene dal file del pacchetto; se manca, dal modello MIT/BSD con gli
// autori del pacchetto o dai testi in assets/licenses/spdx/. Una licenza non riconosciuta fa
// fallire lo script: va aggiunta a manual.json.
//
// Uso:  node scripts/licenses.mjs            rigenera il file
//       node scripts/licenses.mjs --check    fallisce se il file nel repository è diverso
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const licensesDir = path.join(repo, "assets", "licenses");
const output = path.join(licensesDir, "third-party.json");

const LICENSE_FILE = /^(licen[cs]e|copying|unlicense)([-._].*)?$/i;
const read = (file) => fs.readFileSync(file, "utf8").replace(/\r\n/g, "\n").trim();

/** Testi standard per i pacchetti senza file di licenza. */
const spdx = (id) => {
  const file = path.join(licensesDir, "spdx", `${id}.txt`);
  return fs.existsSync(file) ? read(file) : null;
};
const holders = (authors) =>
  (authors?.length ? authors : ["the authors"]).map((a) => a.replace(/\s*<[^>]*>/, "")).join(", ");
const TEMPLATES = {
  MIT: (who) => `MIT License

Copyright (c) ${who}

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.`,
  "BSD-2-Clause": (who) => `BSD 2-Clause License

Copyright (c) ${who}

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice, this
   list of conditions and the following disclaimer.

2. Redistributions in binary form must reproduce the above copyright notice,
   this list of conditions and the following disclaimer in the documentation
   and/or other materials provided with the distribution.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.`,
  "BSD-3-Clause": (who) => `BSD 3-Clause License

Copyright (c) ${who}

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice, this
   list of conditions and the following disclaimer.

2. Redistributions in binary form must reproduce the above copyright notice,
   this list of conditions and the following disclaimer in the documentation
   and/or other materials provided with the distribution.

3. Neither the name of the copyright holder nor the names of its
   contributors may be used to endorse or promote products derived from
   this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.`,
};

/** Normalizza le vecchie forme "MIT/Apache-2.0" in espressioni SPDX. */
export const normalizeExpression = (expr) =>
  expr
    .replace(/\s*\/\s*/g, " OR ")
    .replace(/\s+/g, " ")
    .trim();

/** Identificatori di licenza in un'espressione SPDX, nell'ordine in cui compaiono. */
export const licenseIds = (expr) =>
  normalizeExpression(expr)
    .replace(/[()]/g, " ")
    .split(/\s+(?:OR|AND|WITH)\s+|\s+/)
    .filter((x) => x && !["OR", "AND", "WITH"].includes(x));

/**
 * Testo per un pacchetto senza file di licenza: la prima licenza dell'espressione per cui c'è un
 * modello o un testo standard. `null` se nessuna è nota (lo script fallisce).
 */
export function fallbackText(expr, authors) {
  const ids = licenseIds(expr);
  const order = ["MIT", ...ids.filter((id) => id !== "MIT")];
  for (const id of order) {
    if (!ids.includes(id)) continue;
    if (TEMPLATES[id]) return TEMPLATES[id](holders(authors));
    const text = spdx(id);
    if (text) return text;
  }
  return null;
}

function packageText(dir, expr, authors) {
  const files = fs
    .readdirSync(dir)
    .filter((f) => LICENSE_FILE.test(f) && fs.statSync(path.join(dir, f)).isFile())
    .sort();
  if (files.length) return files.map((f) => read(path.join(dir, f))).join("\n\n---\n\n");
  return expr ? fallbackText(expr, authors) : null;
}

function rustComponents() {
  const meta = JSON.parse(
    execFileSync("cargo", ["metadata", "--format-version", "1", "--locked"], {
      cwd: repo,
      maxBuffer: 256 * 1024 * 1024,
      encoding: "utf8",
    }),
  );
  const packages = new Map(meta.packages.map((p) => [p.id, p]));
  const nodes = new Map(meta.resolve.nodes.map((n) => [n.id, n]));
  const workspace = new Set(meta.workspace_members);
  const root = meta.packages.find((p) => p.name === "photo-cat").id;
  // Solo le dipendenze normali raggiungibili dall'app, per qualunque target: le dipendenze di
  // build e di sviluppo non finiscono nell'installer.
  const reached = new Set();
  const stack = [root];
  while (stack.length) {
    const id = stack.pop();
    if (reached.has(id)) continue;
    reached.add(id);
    for (const dep of nodes.get(id)?.deps ?? [])
      if (dep.dep_kinds.some((k) => k.kind === null)) stack.push(dep.pkg);
  }
  const out = [];
  for (const id of reached) {
    if (workspace.has(id)) continue;
    const p = packages.get(id);
    const dir = path.dirname(p.manifest_path);
    const expr = p.license ? normalizeExpression(p.license) : null;
    const text =
      p.license_file && fs.existsSync(path.join(dir, p.license_file))
        ? read(path.join(dir, p.license_file))
        : packageText(dir, expr, p.authors);
    out.push({ name: p.name, version: p.version, license: expr ?? "LicenseRef-file", kind: "rust", text });
  }
  return out;
}

function uiComponents() {
  const ui = path.join(repo, "app", "ui");
  const nm = path.join(ui, "node_modules");
  const root = JSON.parse(fs.readFileSync(path.join(ui, "package.json"), "utf8"));
  const seen = new Map();
  const visit = (name, from) => {
    // Risoluzione come Node: prima la cartella annidata del pacchetto che lo richiede.
    const candidates = [from && path.join(from, "node_modules", name), path.join(nm, name)].filter(Boolean);
    const dir = candidates.find((d) => fs.existsSync(path.join(d, "package.json")));
    if (!dir) throw new Error(`pacchetto mancante: ${name} (eseguire npm ci in app/ui)`);
    const pkg = JSON.parse(fs.readFileSync(path.join(dir, "package.json"), "utf8"));
    const key = `${pkg.name}@${pkg.version}`;
    if (seen.has(key)) return;
    const license = typeof pkg.license === "string" ? pkg.license : pkg.license?.type;
    seen.set(key, {
      name: pkg.name,
      version: pkg.version,
      license: license ? normalizeExpression(license) : null,
      kind: "ui",
      text: packageText(dir, license ? normalizeExpression(license) : null, [pkg.author?.name ?? pkg.author].filter(Boolean)),
    });
    for (const dep of Object.keys(pkg.dependencies ?? {})) visit(dep, dir);
    for (const [dep, info] of Object.entries(pkg.peerDependenciesMeta ?? {}))
      if (!info?.optional && pkg.peerDependencies?.[dep]) visit(dep, dir);
  };
  for (const dep of Object.keys(root.dependencies ?? {})) visit(dep, null);
  return [...seen.values()];
}

function manualComponents() {
  const list = JSON.parse(fs.readFileSync(path.join(licensesDir, "manual.json"), "utf8"));
  return list.map((c) => ({
    name: c.name,
    version: c.version,
    license: c.license,
    kind: c.kind,
    text: c.files
      ? c.files.map((f) => read(path.join(licensesDir, f))).join("\n\n---\n\n")
      : c.spdx
        ? spdx(c.spdx)
        : (c.text ?? null),
    sources: c.sources ?? null,
  }));
}

/** Fallisce se un componente non ha una licenza riconosciuta con il suo testo. */
export function checkComponents(all) {
  const missing = all
    .filter((c) => !c.license || !c.text)
    .map((c) => `${c.kind} ${c.name}@${c.version} (${c.license ?? "licenza assente"})`);
  if (missing.length)
    throw new Error(
      `licenze non riconosciute, da aggiungere in assets/licenses/manual.json:\n  ${missing.join("\n  ")}`,
    );
}

export function build() {
  const all = [...manualComponents(), ...uiComponents(), ...rustComponents()];
  checkComponents(all);
  // Testi in comune una volta sola: molti pacchetti hanno la stessa licenza.
  const texts = {};
  const order = { model: 0, native: 1, ui: 2, rust: 3 };
  const components = all
    .map(({ text, ...c }) => {
      const id = createHash("sha256").update(text).digest("hex").slice(0, 16);
      texts[id] = text;
      return { ...c, sources: c.sources ?? null, text: id };
    })
    .sort(
      (a, b) =>
        order[a.kind] - order[b.kind] ||
        a.name.localeCompare(b.name, "en") ||
        a.version.localeCompare(b.version, "en"),
    );
  const sortedTexts = Object.fromEntries(Object.entries(texts).sort(([a], [b]) => a.localeCompare(b)));
  return `${JSON.stringify({ components, texts: sortedTexts }, null, 1)}\n`;
}

if (process.argv[1] && fileURLToPath(import.meta.url) === path.resolve(process.argv[1])) {
  const json = build();
  if (process.argv.includes("--check")) {
    const current = fs.existsSync(output) ? fs.readFileSync(output, "utf8").replace(/\r\n/g, "\n") : "";
    if (current !== json) {
      console.error(
        "assets/licenses/third-party.json non è aggiornato: eseguire node scripts/licenses.mjs e includerlo nel commit",
      );
      process.exit(1);
    }
    console.log("licenze di terze parti aggiornate");
  } else {
    fs.writeFileSync(output, json);
    const data = JSON.parse(json);
    console.log(`${data.components.length} componenti, ${Object.keys(data.texts).length} testi → ${path.relative(repo, output)}`);
  }
}
