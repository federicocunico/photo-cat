// Problemi di accessibilità ancora aperti nei test delle pagine, come prefissi dei
// messaggi di `auditA11y`, ognuno con il task che lo risolve. L'elenco può solo accorciarsi.
export const A11Y_KNOWN = {
  none: [],
  library: [
    // 3.7: frecce delle cartelle nella barra laterale a 20 px.
    'size: button "Mostra le cartelle di',
    'size: button "Nascondi le cartelle di',
    // 3.5: il gruppo di simili diventa un pulsante separato dalla miniatura.
    "nested: span[data-testid=group-badge-",
  ],
  people: [
    // 7.4: "Ripristina" dentro la riga del gruppo ignorato.
    'nested: button "Ripristina" in div "Gruppo di',
  ],
} as const satisfies Record<string, readonly string[]>;
