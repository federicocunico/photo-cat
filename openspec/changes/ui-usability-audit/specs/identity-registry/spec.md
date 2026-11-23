# Spec Delta

## MODIFIED Requirements

### Requirement: Dati di un'identità
Ogni identità SHALL avere un nome visualizzato obbligatorio e univoco, un nome breve per i nomi dei file, alias facoltativi, una cerchia facoltativa (ad esempio "Famiglia" o "Lavoro") e una soglia di somiglianza propria con un valore predefinito. Nell'interfaccia la cerchia SHALL chiamarsi "Cerchia", gli alias "Altri nomi" e la soglia SHALL essere presentata come sensibilità del riconoscimento.

#### Scenario: Nome duplicato
- **WHEN** l'utente crea un'identità "Giulia Bianchi" e ne esiste già una con lo stesso nome
- **THEN** il sistema rifiuta il salvataggio indicando il conflitto

#### Scenario: Nome breve predefinito
- **WHEN** l'utente crea "Giulia Bianchi" senza indicare il nome breve
- **THEN** il nome breve è "Giulia"

#### Scenario: Cerchia nell'elenco
- **WHEN** "Gioia" ha la cerchia "Famiglia" e 26 foto nella libreria
- **THEN** la sua riga nell'elenco delle persone mostra "Famiglia · 26 foto", e senza cerchia mostra solo "26 foto"

### Requirement: Gestione delle identità
L'utente SHALL poter creare, modificare, unire ed eliminare identità dalla sezione "Persone" del modulo Persone, con filtro per cerchia e ricerca su nome e alias. Le modifiche ai dettagli SHALL essere salvate appena fatte, con la conferma "Salvato" accanto. "Unisci a…", "Esporta" ed "Elimina persona…" SHALL stare nel menu delle altre azioni della persona, separati dai dettagli.

#### Scenario: Ricerca per alias
- **WHEN** l'utente cerca "Nonna Rita"
- **THEN** compare l'identità "Rita Rossi" che ha quell'alias

#### Scenario: Unione di identità
- **WHEN** l'utente unisce "G. Bianchi" in "Giulia Bianchi"
- **THEN** volti di riferimento, alias e collegamenti di "G. Bianchi" passano a "Giulia Bianchi" e "G. Bianchi" non esiste più

#### Scenario: Identità senza foto nella libreria
- **WHEN** un'identità non ha volti nella libreria aperta
- **THEN** la sezione "Persone" la mostra con l'indicazione "nessuna foto in questa libreria", e resta modificabile ed esportabile

#### Scenario: Dettagli salvati subito
- **WHEN** l'utente cambia la cerchia di "Gioia" in "Amici" ed esce dal campo
- **THEN** la modifica è salvata senza premere alcun pulsante e accanto compare "Salvato"
