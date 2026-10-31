# Spec Delta

## ADDED Requirements

### Requirement: Motivo del collegamento non disponibile per piattaforma
Quando un collegamento simbolico non si può creare, il motivo mostrato in anteprima SHALL riferirsi alla causa reale sulla piattaforma in uso; i riferimenti a Modalità sviluppatore o privilegi di Windows SHALL comparire solo su Windows.

#### Scenario: Symlink su Linux in una cartella normale
- **WHEN** su Linux l'utente sceglie symlink con destinazione su un filesystem che li supporta
- **THEN** il metodo è disponibile senza avvisi

#### Scenario: Symlink su Linux verso un filesystem senza symlink
- **WHEN** su Linux la destinazione è su un filesystem che non supporta i symlink (es. FAT o exFAT)
- **THEN** l'anteprima indica che la destinazione non consente collegamenti simbolici, senza citare Windows, e propone hard link o copia
