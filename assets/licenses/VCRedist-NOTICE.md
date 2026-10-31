# Microsoft Visual C++ Runtime (distribuzione app-local)

`runtime/win-x64/vcruntime140.dll`, `vcruntime140_1.dll`, `msvcp140.dll`, `msvcp140_1.dll`
provengono dalla cartella `VC\Redist\MSVC\14.44.35112\x64\Microsoft.VC143.CRT` di Visual Studio 2022
e sono "Distributable Code" secondo le condizioni di licenza di Visual Studio (elenco in `redist.txt`).

Servono a `onnxruntime.dll` sui sistemi senza Visual C++ Redistributable installato
(installazioni Windows pulite, Windows Sandbox); l'app li carica dalla propria cartella `runtime`.
