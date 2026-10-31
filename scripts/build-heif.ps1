# Rebuild the optional LGPL HEIC decoder from the corresponding bundled source.
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$projectRoot = Split-Path -Parent $PSScriptRoot
$sourceRoot = Join-Path $projectRoot 'target/native-src'
$buildRoot = Join-Path $projectRoot 'target/native-build'
$installRoot = Join-Path $projectRoot 'target/native-install'
New-Item -ItemType Directory -Path $sourceRoot -Force | Out-Null
$archives = @(
    @{Name='libde265-1.1.3.tar.gz'; Hash='554228BD17788C99A7E63B37AB5634722190E6E2BF60C1DCB01CEF328E133905'},
    @{Name='libheif-1.23.7.tar.gz'; Hash='F7483E818503EA6224C8863E4A56AC02C7089021A98472C811C12AA19FAF2475'}
)
foreach ($archive in $archives) {
    $path = Join-Path $projectRoot ('assets/native-sources/' + $archive.Name)
    if ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash -ne $archive.Hash) { throw "Source checksum mismatch: $path" }
    & tar -xf $path -C $sourceRoot
    if ($LASTEXITCODE -ne 0) { throw 'Source extraction failed' }
}
& cmake -S (Join-Path $sourceRoot 'libde265-1.1.3') -B (Join-Path $buildRoot 'de265') -G 'Visual Studio 17 2022' -A x64 "-DCMAKE_INSTALL_PREFIX=$installRoot" -DCMAKE_MSVC_RUNTIME_LIBRARY=MultiThreaded -DENABLE_SDL=OFF -DENABLE_DECODER=OFF -DENABLE_ENCODER=OFF -DENABLE_SHERLOCK265=OFF -DBUILD_SHARED_LIBS=ON
if ($LASTEXITCODE -ne 0) { throw 'libde265 configure failed' }
& cmake --build (Join-Path $buildRoot 'de265') --config Release --target INSTALL --parallel 4
if ($LASTEXITCODE -ne 0) { throw 'libde265 build failed' }
& cmake -S (Join-Path $sourceRoot 'libheif-1.23.7') -B (Join-Path $buildRoot 'heif') -G 'Visual Studio 17 2022' -A x64 "-DCMAKE_INSTALL_PREFIX=$installRoot" "-DCMAKE_PREFIX_PATH=$installRoot" -DCMAKE_MSVC_RUNTIME_LIBRARY=MultiThreaded -DWITH_LIBDE265=ON -DWITH_LIBDE265_PLUGIN=OFF -DWITH_X265=OFF -DWITH_X264=OFF -DWITH_OpenH264_DECODER=OFF -DWITH_AOM_DECODER=OFF -DWITH_AOM_ENCODER=OFF -DWITH_DAV1D=OFF -DWITH_LIBSHARPYUV=OFF -DENABLE_PLUGIN_LOADING=OFF -DWITH_EXAMPLES=OFF -DWITH_GDK_PIXBUF=OFF -DBUILD_TESTING=OFF -DBUILD_DOCUMENTATION=OFF -DBUILD_SHARED_LIBS=ON
if ($LASTEXITCODE -ne 0) { throw 'libheif configure failed' }
& cmake --build (Join-Path $buildRoot 'heif') --config Release --target INSTALL --parallel 4
if ($LASTEXITCODE -ne 0) { throw 'libheif build failed' }
foreach ($name in @('heif.dll','libde265.dll')) {
    Copy-Item -LiteralPath (Join-Path $installRoot "bin/$name") -Destination (Join-Path $projectRoot "assets/runtime/win-x64/$name")
}
