#!/usr/bin/env bash
# Compila il DMG per macOS Apple Silicon.
# Risultato in target/release/dist/: .dmg e .sha256.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

arch="$(uname -m)"
[[ "$arch" == "arm64" ]] || {
  echo "build macOS supportata solo su Apple Silicon (arm64), trovato: $arch" >&2
  exit 1
}

version="$(node -p "require('./app/src-tauri/tauri.conf.json').version")"
dmg_name="photo-cat_${version}_aarch64.dmg"
bundle="target/release/bundle/dmg"
dist="target/release/dist"

if [[ "${1:-}" != "--no-build" ]]; then
  npm --prefix app/ui run tauri -- build --ci --bundles dmg
fi

src=""
for f in "$bundle"/*.dmg; do
  [[ -f "$f" ]] || continue
  src="$f"
  break
done
[[ -n "$src" && -f "$src" ]] || {
  echo "DMG non trovato in $bundle" >&2
  exit 1
}

# Gatekeeper su Apple Silicon tratta come "danneggiata" una firma linker senza
# risorse sigillate: verifica che il .app nel DMG abbia sealed resources.
mount="$(mktemp -d)"
cleanup() { hdiutil detach "$mount" -quiet 2>/dev/null || true; rmdir "$mount" 2>/dev/null || true; }
trap cleanup EXIT
hdiutil attach "$src" -mountpoint "$mount" -nobrowse -readonly -quiet
app="$mount/photo-cat.app"
sig="$(codesign -dv --verbose=2 "$app" 2>&1)"
echo "$sig" | grep -q 'Sealed Resources version=' || {
  echo "firma del bundle incompleta (manca Sealed Resources): impostare bundle.macOS.signingIdentity \"-\" e rifare la build" >&2
  exit 1
}
# Hardened Runtime + ad-hoc blocca dlopen di libonnxruntime.dylib (Library Validation).
if echo "$sig" | grep -E 'flags=.*\([^)]*runtime' >/dev/null; then
  echo "Hardened Runtime ancora attivo: impostare bundle.macOS.hardenedRuntime false e rifare la build" >&2
  exit 1
fi

rm -rf "$dist"
mkdir -p "$dist"
cp "$src" "$dist/$dmg_name"

# Fine riga LF e solo il nome del file: verificabile con `shasum -a 256 -c` dalla cartella dist.
(
  cd "$dist"
  shasum -a 256 "$dmg_name" | awk '{print $1 "  " $2}' > "$dmg_name.sha256"
)
ls -l "$dist"
