#!/usr/bin/env bash
# Carica sulla GitHub Release v<versione> il DMG e il checksum prodotti in locale.
# Richiede: macOS Apple Silicon, `gh` autenticato, release già esistente (make release / CI).
# Opzionale: SKIP_BUILD=1 per non ricompilare se manca il DMG (fallisce invece).
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

arch="$(uname -m)"
[[ "$(uname -s)" == "Darwin" && "$arch" == "arm64" ]] || {
  echo "publish-mac solo su macOS Apple Silicon (arm64)" >&2
  exit 1
}

command -v gh >/dev/null || {
  echo "serve la CLI GitHub: brew install gh && gh auth login" >&2
  exit 1
}

version="$(node -p "require('./app/src-tauri/tauri.conf.json').version")"
tag="v${version}"
dmg_name="photo-cat_${version}_aarch64.dmg"
dist="target/release/dist"
dmg="$dist/$dmg_name"
sum="$dist/$dmg_name.sha256"

if [[ ! -f "$dmg" || ! -f "$sum" ]]; then
  if [[ "${SKIP_BUILD:-}" == "1" ]]; then
    echo "mancano $dmg e/o $sum (eseguire make build, oppure omettere SKIP_BUILD=1)" >&2
    exit 1
  fi
  scripts/package-macos.sh
fi

[[ -f "$dmg" && -f "$sum" ]] || {
  echo "artefatti macOS incompleti in $dist" >&2
  exit 1
}

gh release view "$tag" >/dev/null || {
  echo "release $tag assente: prima make release (o attendere la CI sul tag)" >&2
  exit 1
}

gh release upload "$tag" "$dmg" "$sum" --clobber
echo "Caricati su $tag: $dmg_name e $dmg_name.sha256"
echo "https://github.com/federicocunico/photo-cat/releases/tag/$tag"
