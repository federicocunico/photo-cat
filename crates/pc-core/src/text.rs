//! Confronto di testi senza maiuscole e accenti (ricerche, nomi di luoghi e voci).

/// Minuscole senza segni diacritici: "Città" → "citta", "ÉLITE" → "elite".
pub fn fold(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| match c {
            'à' | 'á' | 'â' | 'ä' | 'ã' | 'å' => 'a',
            'è' | 'é' | 'ê' | 'ë' => 'e',
            'ì' | 'í' | 'î' | 'ï' => 'i',
            'ò' | 'ó' | 'ô' | 'ö' | 'õ' => 'o',
            'ù' | 'ú' | 'û' | 'ü' => 'u',
            'ý' | 'ÿ' => 'y',
            'ç' => 'c',
            'ñ' => 'n',
            c => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::fold;

    #[test]
    fn folds_case_and_accents() {
        assert_eq!(fold("Città"), "citta");
        assert_eq!(fold("ÉLITE"), "elite");
        assert_eq!(fold("Façade Niño"), "facade nino");
        assert_eq!(fold("SIENA"), fold("Siena"));
    }
}
