// Test di scripts/licenses.mjs (node --test scripts/): espressioni SPDX, testi di ripiego e
// fallimento per una licenza non riconosciuta.
import assert from "node:assert/strict";
import { test } from "node:test";
import { checkComponents, fallbackText, licenseIds, normalizeExpression } from "./licenses.mjs";

test("le vecchie forme con la barra diventano espressioni SPDX", () => {
  assert.equal(normalizeExpression("MIT/Apache-2.0"), "MIT OR Apache-2.0");
  assert.equal(normalizeExpression("Apache-2.0 / MIT"), "Apache-2.0 OR MIT");
  assert.deepEqual(licenseIds("(MIT OR Apache-2.0) AND Unicode-3.0"), [
    "MIT",
    "Apache-2.0",
    "Unicode-3.0",
  ]);
});

test("senza file di licenza: MIT con gli autori del pacchetto, se l'espressione lo consente", () => {
  const text = fallbackText("Zlib OR Apache-2.0 OR MIT", ["Mads Marquart <mads@example.com>"]);
  assert.match(text, /^MIT License/);
  assert.match(text, /Copyright \(c\) Mads Marquart$/m);
  assert.doesNotMatch(text, /example\.com/);
  assert.match(fallbackText("BSD-2-Clause", ["Josh Teeter"]), /^BSD 2-Clause License/);
  assert.match(fallbackText("MPL-2.0", []), /^Mozilla Public License Version 2\.0/);
});

test("una licenza non riconosciuta fa fallire lo script con il nome del pacchetto", () => {
  assert.equal(fallbackText("LicenseRef-Proprietaria", []), null);
  assert.throws(
    () =>
      checkComponents([
        { kind: "rust", name: "serde", version: "1.0.0", license: "MIT", text: "MIT License" },
        { kind: "rust", name: "misterioso", version: "0.1.0", license: "LicenseRef-X", text: null },
      ]),
    /misterioso@0\.1\.0[\s\S]*manual\.json|manual\.json[\s\S]*misterioso@0\.1\.0/,
  );
  assert.doesNotThrow(() =>
    checkComponents([{ kind: "ui", name: "react", version: "18.3.1", license: "MIT", text: "x" }]),
  );
});
