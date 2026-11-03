# Spec Delta

## MODIFIED Requirements

### Requirement: Azione del doppio clic
L'utente SHALL poter scegliere nelle impostazioni cosa fa il doppio clic su una miniatura:
- aprire la foto nel visualizzatore interno di photo-cat;
- aprirla con una delle applicazioni esterne configurate;
- aprirla con l'applicazione predefinita del sistema;
- nessuna azione.

Il valore predefinito SHALL essere l'applicazione predefinita del sistema. Il visualizzatore interno SHALL restare comunque raggiungibile con Spazio o Invio.

#### Scenario: Doppio clic con applicazione configurata
- **WHEN** l'azione del doppio clic è "Lightroom" e l'utente fa doppio clic su una miniatura
- **THEN** la foto viene aperta in Lightroom

#### Scenario: Doppio clic predefinito
- **WHEN** l'utente non ha configurato applicazioni e fa doppio clic su una miniatura
- **THEN** la foto viene aperta con l'applicazione predefinita del sistema per quel formato

#### Scenario: Doppio clic sul visualizzatore interno
- **WHEN** l'azione del doppio clic è "Visualizzatore di photo-cat" e l'utente fa doppio clic su una miniatura
- **THEN** la foto si apre nel visualizzatore interno
