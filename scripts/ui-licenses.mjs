// Raccoglie le licenze delle dipendenze dell'interfaccia aggiunte da ui-ux-overhaul
// (Radix, cmdk, lucide-react) e di tutte le loro dipendenze di produzione, in
// assets/licenses/ui-dependencies-LICENSES.txt (incluso nel pacchetto dell'app).
// Uso: node scripts/ui-licenses.mjs  (dopo `npm ci` in app/ui)
import fs from "node:fs";
import path from "node:path";

const ROOTS = [
  "@radix-ui/react-alert-dialog",
  "@radix-ui/react-collapsible",
  "@radix-ui/react-context-menu",
  "@radix-ui/react-dialog",
  "@radix-ui/react-dropdown-menu",
  "@radix-ui/react-popover",
  "@radix-ui/react-tabs",
  "@radix-ui/react-toggle-group",
  "@radix-ui/react-tooltip",
  "cmdk",
  "lucide-react",
];
// Già presenti nell'app prima di questa change: non si ripetono.
const SKIP = new Set(["react", "react-dom", "scheduler", "loose-envify", "js-tokens"]);

const repo = path.resolve(path.dirname(new URL(import.meta.url).pathname.replace(/^\/(\w:)/, "$1")), "..");
const nm = path.join(repo, "app", "ui", "node_modules");
const seen = new Map();

function visit(name) {
  if (seen.has(name) || SKIP.has(name)) return;
  const dir = path.join(nm, name);
  const pkgFile = path.join(dir, "package.json");
  if (!fs.existsSync(pkgFile)) throw new Error(`pacchetto mancante: ${name} (eseguire npm ci)`);
  const pkg = JSON.parse(fs.readFileSync(pkgFile, "utf8"));
  const licFile = fs
    .readdirSync(dir)
    .find((f) => /^(licen[cs]e|copying)(\.(md|txt))?$/i.test(f));
  seen.set(name, {
    version: pkg.version,
    license: typeof pkg.license === "string" ? pkg.license : JSON.stringify(pkg.license),
    text: licFile ? fs.readFileSync(path.join(dir, licFile), "utf8").replace(/\r\n/g, "\n").trim() : null,
  });
  for (const dep of Object.keys(pkg.dependencies ?? {})) visit(dep);
  for (const [dep, meta] of Object.entries(pkg.peerDependenciesMeta ?? {})) {
    if (!meta?.optional && pkg.peerDependencies?.[dep]) visit(dep);
  }
}
ROOTS.forEach(visit);

const names = [...seen.keys()].sort();
let out =
  "Licenze delle dipendenze dell'interfaccia (Radix UI, cmdk, lucide-react e dipendenze).\n" +
  "Generato da scripts/ui-licenses.mjs: non modificare a mano.\n\n";
out += names.map((n) => `- ${n}@${seen.get(n).version} (${seen.get(n).license})`).join("\n") + "\n";
for (const n of names) {
  const e = seen.get(n);
  out += `\n${"=".repeat(72)}\n${n}@${e.version} — ${e.license}\n${"=".repeat(72)}\n\n`;
  out += e.text ?? `Licenza ${e.license} dichiarata in package.json; il pacchetto non include un file di licenza.`;
  out += "\n";
}
const target = path.join(repo, "assets", "licenses", "ui-dependencies-LICENSES.txt");
fs.writeFileSync(target, out);
console.log(`${names.length} pacchetti → ${path.relative(repo, target)}`);
const bad = names.filter((n) => !/^(MIT|ISC|Apache-2\.0|BSD-[23]-Clause|0BSD)$/.test(seen.get(n).license));
if (bad.length) {
  console.error("Licenze da verificare:", bad.map((n) => `${n} (${seen.get(n).license})`).join(", "));
  process.exit(1);
}
