use crate::ported::compat::{TokenType, Tokenizer};

#[test]
fn number_scanning_keeps_adjacent_end_keywords() {
    for prefix in ["", "/* π */"] {
        for number in ["13", "1.5", ".5", "1e3", "1E+3", "1e-3", ".5e2"] {
            let tokens = Tokenizer::default()
                .tokenize(&format!("{prefix}{number}END"))
                .unwrap();
            assert_eq!(tokens.len(), 2, "{tokens:?}");
            assert_eq!(tokens[0].token_type, TokenType::Number);
            assert_eq!(tokens[0].text, number);
            assert_eq!(tokens[1].token_type, TokenType::End);
            assert_eq!(tokens[1].text, "END");
        }
    }
}

#[test]
fn scientific_notation_still_requires_exponent_digits() {
    for number in ["1e3", "1E+3", "1e-3", ".5e2", "1.e2"] {
        let tokens = Tokenizer::default().tokenize(number).unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token_type, TokenType::Number);
        assert_eq!(tokens[0].text, number);
    }
    for sql in ["1e", "1E+", ".5e", ".5E-"] {
        let tokens = Tokenizer::default().tokenize(sql).unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Number);
        assert!(!tokens[0].text.ends_with(['e', 'E', '+', '-']));
    }
}
