# Spec Delta

## ADDED Requirements

### Requirement: Eliminazione di più cataloghi
Nella pagina Cataloghi l'utente SHALL poter selezionare più cataloghi ed eliminarli in un'unica operazione, dopo una conferma che ne indica il numero e i nomi. L'eliminazione di un catalogo MUST NOT rimuovere foto dalla libreria né file dal disco. I cataloghi di sistema SHALL poter essere selezionati ma MUST NOT essere eliminati: se la selezione ne contiene, il comando SHALL spiegarlo ed eliminare solo gli altri.

#### Scenario: Tre cataloghi eliminati
- **WHEN** l'utente seleziona tre cataloghi manuali con Ctrl+clic, preme Canc e conferma
- **THEN** i tre cataloghi non esistono più, le loro foto restano in libreria e un avviso dice "3 cataloghi eliminati"

#### Scenario: Selezione con un catalogo di sistema
- **WHEN** la selezione contiene "Cestino" e due cataloghi smart e l'utente sceglie "Elimina"
- **THEN** la conferma indica che il Cestino è un catalogo di sistema e non verrà eliminato, e dopo la conferma vengono eliminati solo i due cataloghi smart
