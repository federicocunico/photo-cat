# Asset distribuiti con photo-cat

Modelli, runtime e dati sono versionati nel repository (ogni file < 100 MB, limite di GitHub) e inclusi nell'installer. All'avvio l'app verifica gli SHA-256 elencati in `models/manifest.json`; un file mancante o corrotto viene ricopiato dagli asset dell'app o, in mancanza, riscaricato dagli URL pubblici del manifest.

| File | Fonte | Licenza |
|---|---|---|
| `models/clip-vit-b32/vision_model_quantized.onnx`, `text_model_quantized.onnx`, `tokenizer.json` | [Xenova/clip-vit-base-patch32](https://huggingface.co/Xenova/clip-vit-base-patch32) (conversione ONNX int8 di OpenAI CLIP ViT-B/32) | MIT |
| `models/yolox-tiny/yolox_tiny.onnx` | [Megvii-BaseDetection/YOLOX release 0.1.1rc0](https://github.com/Megvii-BaseDetection/YOLOX/releases/tag/0.1.1rc0) | Apache-2.0 |
| `models/yunet/face_detection_yunet_2023mar.onnx` | [OpenCV Zoo · YuNet](https://huggingface.co/opencv/face_detection_yunet) | MIT |
| `models/sface/face_recognition_sface_2021dec.onnx` | [OpenCV Zoo · SFace](https://huggingface.co/opencv/face_recognition_sface) | Apache-2.0 |
| `models/geo/cities15000.tsv` | Derivato da GeoNames `cities15000.zip` + nomi italiani da `alternateNamesV2.zip` (scaricati il 2026-10-30) | [CC-BY 4.0](https://creativecommons.org/licenses/by/4.0/), © GeoNames |
| `runtime/win-x64/onnxruntime.dll`, `onnxruntime_providers_shared.dll` | NuGet `Microsoft.ML.OnnxRuntime.DirectML` 1.24.4 | MIT (`licenses/onnxruntime-*`) |
| `runtime/win-x64/DirectML.dll` | NuGet `Microsoft.AI.DirectML` 1.15.4 | Microsoft DirectML (`licenses/DirectML-*`), ridistribuibile all'interno di applicazioni |
| `runtime/win-x64/heif.dll`, `libde265.dll` | libheif 1.23.7 + libde265 1.1.3, compilazione con solo decoder HEVC | LGPL-3.0, licenze e istruzioni in `licenses/HEIF-BUILD.md`; sorgenti corrispondenti in `native-sources/` |
| `runtime/win-x64/vcruntime140*.dll`, `msvcp140*.dll` | Visual Studio 2022, `VC\Redist\MSVC\14.44.35112\x64\Microsoft.VC143.CRT` | Microsoft Visual C++ Runtime, "Distributable Code" (`licenses/VCRedist-NOTICE.md`) |

## Formato di `geo/cities15000.tsv`

Una riga per città con più di 15.000 abitanti, separatore tabulazione, senza intestazione:
`geonameid · nome · nome italiano (vuoto se assente) · latitudine · longitudine · paese ISO · codice admin1 · popolazione`.

Il file derivato non ha un URL pubblico stabile (GeoNames aggiorna i dump ogni giorno): per rigenerarlo si ripete l'estrazione descritta sopra e si aggiorna lo SHA-256 nel manifest.
