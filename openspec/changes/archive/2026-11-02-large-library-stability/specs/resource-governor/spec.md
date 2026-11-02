# Spec Delta

## MODIFIED Requirements

### Requirement: Adattamento alla pressione di memoria
Durante l'esecuzione il sistema SHALL monitorare periodicamente la RAM disponibile del sistema e il consumo dell'app, inclusi i processi dell'interfaccia (WebView); sotto una soglia di pressione SHALL ridurre la concorrenza e, se persiste, mettere in pausa le fasi AI e la decodifica delle immagini fino al rientro.

#### Scenario: Altra applicazione occupa RAM
- **WHEN** durante un'analisi la RAM disponibile del sistema scende sotto la soglia di pressione
- **THEN** il sistema riduce i worker, registra l'evento e riprende il ritmo normale quando la RAM torna disponibile

#### Scenario: Interfaccia che cresce
- **WHEN** durante un import la memoria dei processi della WebView supera la quota prevista
- **THEN** il sistema registra l'evento con i valori misurati, riduce la concorrenza dell'import e chiede all'interfaccia di liberare le cache

#### Scenario: Pressione critica
- **WHEN** la RAM disponibile scende sotto la soglia critica durante un import
- **THEN** la decodifica delle immagini viene sospesa insieme alle fasi AI finché la memoria non rientra, e il job mostra lo stato "in pausa per memoria"
