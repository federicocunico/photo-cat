// Problemi di accessibilità ancora aperti nei test delle pagine, come prefissi dei
// messaggi di `auditA11y`, ognuno con il task che lo risolve. L'elenco può solo accorciarsi.
export const A11Y_KNOWN = {
  none: [],
  library: [],
  people: [],
} as const satisfies Record<string, readonly string[]>;
