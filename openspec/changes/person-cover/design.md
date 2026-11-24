# Design

## Context

- I gruppi (`GroupRow`) hanno già `cover: FaceView | null`, calcolato in `pc-store::people` da `cover_face`: il volto scelto (`person.cover_face_id`) oppure il primo volto in ordine di qualità.
- L'elenco di Persone › Persone non usa i gruppi ma le identità dell'app (`list_identities` → `IdentityRow`), che non hanno copertina: per questo compaiono le iniziali.
- La scheda riceve un gruppo "segnaposto" con `cover: null` quando la persona si apre dall'elenco delle identità.
- `set_cover` (correzione `set_cover`, annullabile) esiste già e cambia `person.cover_face_id`.

## Goals / Non-Goals

**Goals:**
- Volto di copertina per le persone nell'elenco e nella scheda.
- "Usa come copertina" raggiungibile con il clic destro su un volto.

**Non-Goals:**
- Copertina presa dai volti di riferimento per le persone senza volti nella libreria aperta (restano le iniziali).
- Un selettore dedicato della copertina nella scheda.

## Decisions

### D1. La copertina viaggia con `IdentityRow`
`IdentityRow` riceve `cover: Option<FaceView>`. È la copertina della persona collegata nella libreria aperta, calcolata con la stessa funzione dei gruppi, esposta da `pc-store` come `person_cover`.
- La calcola solo `list_identities`. `identity_selection_uids` ("Seleziona tutto") non ne ha bisogno e resta com'è.
- Costo: una query indicizzata per persona collegata (`face_person_rank`), su centinaia di identità al massimo. È lo stesso schema dei gruppi, che lo fanno per pagina.

Alternativa scartata: il frontend chiede `getGroup` per ogni identità. Sarebbero N chiamate IPC e i caricamenti si vedrebbero a scatti.

### D2. Elenco e scheda usano `Cover`
- Nell'elenco delle persone `Cover` sostituisce le iniziali quando c'è una copertina; senza, restano le iniziali.
- La scheda riceve la copertina dal gruppo, oppure dall'identità quando il gruppo è un segnaposto.
- Aprendo una persona dall'elenco, il segnaposto porta già la copertina dell'identità.

### D3. "Usa come copertina" nel menu contestuale
Con un solo volto come destinazione del menu (clic destro su un volto non selezionato, oppure selezione di un volto) il menu contestuale aggiunge "Usa come copertina". È la stessa correzione di "Altro", con lo stesso avviso annullabile.

Dopo la correzione, `refreshPeopleFast` invalida già `identities`, `people-groups` e `people-move-targets`. Così elenco, scheda e selettore si aggiornano senza altro codice.

## Risks / Trade-offs

- [Elenco delle identità più lento su librerie con migliaia di persone] → una query indicizzata per riga, come per le pagine dei gruppi; se servisse, si passa a una query unica per tutte le persone.
