# Optional HEIC decoder

`heif.dll` is libheif 1.23.7 and `libde265.dll` is libde265 1.1.3.
Both are dynamically linked LGPL libraries, kept separate from the application
and replaceable with ABI-compatible builds. License texts are included here.
Corresponding unmodified source archives are included alongside the application
under `native-sources/`, and in this repository under `assets/native-sources/`.

Upstream sources:

- https://github.com/strukturag/libheif/releases/tag/v1.23.7
- https://github.com/strukturag/libde265/releases/tag/v1.1.3

Rebuild with MSVC 2022, CMake and `scripts/build-heif.ps1` from the application
source repository. The script verifies the archives' SHA-256 and records all
configuration flags. Build configuration: shared DLLs, static MSVC CRT,
libde265 decoder enabled, external HEVC/AV1/AVC encoders and plugin loading
disabled. No x265 encoder DLL is distributed with the application.

HEIC decoding is behind Cargo feature `heif`; the Windows desktop app enables
it by default. Without the feature or a compatible decoder, metadata import
continues with a diagnostic warning. AVIF remains metadata-only in this build.
