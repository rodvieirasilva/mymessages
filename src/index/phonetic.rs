/// Bilingual (PT-BR / EN) accent folding, phonetic encoding, and synonym expansion.

pub fn fold_accents(input: &str) -> String {
    input
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' | 'Á' | 'À' | 'Â' | 'Ã' | 'Ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' | 'Î' | 'Ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' | 'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Û' | 'Ü' => 'u',
            'ç' | 'Ç' => 'c',
            'ñ' | 'Ñ' => 'n',
            other => other.to_ascii_lowercase(),
        })
        .collect()
}

/// Lightweight PT-BR + EN phonetic key (e.g. "carolina" / "karolina" / "carolyna" -> "KRLN").
pub fn phonetic_key(word: &str) -> String {
    let folded = fold_accents(word);
    let mut s = String::with_capacity(folded.len());
    for c in folded.chars() {
        if c.is_ascii_alphanumeric() {
            s.push(c);
        }
    }
    if s.is_empty() {
        return String::new();
    }

    let replaced = s
        .replace("ph", "f")
        .replace("qu", "k")
        .replace("ck", "k")
        .replace("ch", "x")
        .replace("sh", "x")
        .replace("lh", "l")
        .replace("nh", "n")
        .replace("th", "t");

    let mut out = String::new();
    let mut prev: Option<char> = None;
    for (idx, ch) in replaced.chars().enumerate() {
        let mapped = match ch {
            'h' => continue,
            'y' => 'i',
            'w' => 'v',
            'c' => 'k',
            'q' => 'k',
            'z' => 's',
            'j' | 'g' if idx == 0 => 'j',
            other => other,
        };
        if idx > 0 && matches!(mapped, 'a' | 'e' | 'i' | 'o' | 'u') {
            continue;
        }
        if prev != Some(mapped) {
            out.push(mapped);
            prev = Some(mapped);
        }
    }
    out.to_ascii_uppercase()
}

const STOP_WORDS: &[&str] = &[
    // PT-BR
    "a", "o", "as", "os", "de", "do", "da", "dos", "das", "em", "no", "na", "nos", "nas",
    "um", "uma", "uns", "umas", "e", "ou", "mas", "que", "com", "por", "para", "se", "ao",
    "aos", "como", "mais", "nao", "sim", "eu", "tu", "ele", "ela", "eles", "elas", "meu",
    "minha", "seu", "sua",
    // EN
    "the", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with", "by",
    "from", "is", "are", "was", "were", "be", "been", "it", "this", "that", "i", "you",
    "he", "she", "we", "they", "my", "your",
];

pub fn is_stop_word(word: &str) -> bool {
    let norm = fold_accents(word.trim());
    STOP_WORDS.contains(&norm.as_str())
}

pub fn filter_stop_words(query: &str) -> Vec<String> {
    let tokens: Vec<String> = query.split_whitespace().map(|s| s.to_string()).collect();
    let filtered: Vec<String> = tokens.iter().filter(|t| !is_stop_word(t)).cloned().collect();
    if filtered.is_empty() { tokens } else { filtered }
}

const SYNONYM_GROUPS: &[&[&str]] = &[
    &["foto", "photo", "imagem", "image", "picture", "pic"],
    &["video", "vídeo", "filme", "movie"],
    &["audio", "áudio", "voz", "voice", "som", "sound"],
    &["pix", "transferencia", "transfer", "pagamento", "payment"],
    &["endereco", "address", "localizacao", "location", "mapa", "map"],
    &["telefone", "phone", "celular", "mobile", "numero", "number"],
    &["reuniao", "meeting", "call", "ligacao"],
    &["documento", "document", "pdf", "arquivo", "file"],
    &["dinheiro", "money", "grana", "cash"],
    &["casa", "home", "house"],
    &["trabalho", "work", "job", "emprego"],
];

pub fn expand_synonyms(term: &str) -> Vec<&'static str> {
    let norm = fold_accents(term.trim());
    for group in SYNONYM_GROUPS {
        if group.iter().any(|&w| fold_accents(w) == norm) {
            return group.to_vec();
        }
    }
    Vec::new()
}

/// Expand a plain search term into an FTS5 OR expression with accent-folded and synonym variants.
pub fn expand_fts_term(term: &str) -> String {
    let trimmed = term.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let mut variants = vec![trimmed.to_string()];
    let folded = fold_accents(trimmed);
    if folded != trimmed {
        variants.push(folded);
    }
    for syn in expand_synonyms(trimmed) {
        if !variants.iter().any(|v| v.eq_ignore_ascii_case(syn)) {
            variants.push(syn.to_string());
        }
    }
    if variants.len() == 1 {
        variants.into_iter().next().unwrap()
    } else {
        format!("({})", variants.join(" OR "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fold_accents_ptbr() {
        assert_eq!(fold_accents("São Paulo - Ação"), "sao paulo - acao");
    }

    #[test]
    fn test_phonetic_key_variants() {
        assert_eq!(phonetic_key("Carolina"), phonetic_key("Karolina"));
        assert_eq!(phonetic_key("Carolina"), phonetic_key("Carolyna"));
    }

    #[test]
    fn test_expand_synonyms_bilingual() {
        let syns = expand_synonyms("foto");
        assert!(syns.contains(&"photo"));
        assert!(syns.contains(&"imagem"));
    }

    #[test]
    fn test_expand_fts_term() {
        let expanded = expand_fts_term("foto");
        assert!(expanded.contains("foto"));
        assert!(expanded.contains("photo"));
        assert!(expanded.contains(" OR "));
    }

    #[test]
    fn test_stop_words_ptbr_and_en() {
        assert!(is_stop_word("não"));
        assert!(is_stop_word("the"));
        assert!(!is_stop_word("carolina"));
        assert_eq!(filter_stop_words("a foto do contrato"), vec!["foto", "contrato"]);
        assert_eq!(filter_stop_words("the a"), vec!["the", "a"]);
    }
}
