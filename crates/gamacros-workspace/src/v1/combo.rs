#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum SequenceErrorKind {
    LeadingOperator,
    TrailingOperator,
    DoubleOperator,
    MissingOperatorBetweenTerms,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SequenceError<'a> {
    pub rest: &'a str,
    pub kind: SequenceErrorKind,
}

/// Tokenize with a custom one-character delimiter. Returns either the next
/// term (non-empty slice without surrounding whitespace) or the delimiter
/// itself as a one-character slice, plus the remaining input.
pub(crate) fn next_token_with(input: &str, delim: char) -> Option<(&str, &str)> {
    // Skip leading whitespace
    let input = input.trim_start();
    if input.is_empty() {
        return None;
    }

    // If the next character is the delimiter, return it as a separate token
    let chars = input.char_indices();
    if let Some((_, first_ch)) = chars.clone().next() {
        if first_ch == delim {
            let len = first_ch.len_utf8();
            return Some((&input[..len], &input[len..]));
        }
    }

    // Otherwise, read until next whitespace or delimiter
    for (i, ch) in chars {
        if ch == delim {
            return Some((&input[..i], &input[i..]));
        }
        if ch.is_whitespace() {
            // Trim all subsequent whitespace in the rest for stable tokenization
            let rest = input[i..].trim_start();
            return Some((&input[..i], rest));
        }
    }

    Some((input, ""))
}

/// Parse a sequence of terms separated by the given delimiter.
/// Enforces the following rules:
/// - No leading delimiter
/// - No consecutive delimiters
/// - No consecutive terms without a delimiter between them
/// - No trailing delimiter
///
/// Returns the list of term slices (without delimiters or surrounding spaces).
pub(crate) fn parse_terms_with_delim<'a>(
    mut input: &'a str,
    delim: char,
) -> Result<Vec<&'a str>, SequenceError<'a>> {
    #[derive(PartialEq, Eq, Clone, Copy)]
    enum LastTokenKind {
        None,
        Term,
        Operator,
    }

    let mut terms: Vec<&'a str> = Vec::new();
    let mut last = LastTokenKind::None;

    while let Some((token, rest)) = next_token_with(input, delim) {
        input = rest;

        let is_operator = token.chars().count() == 1 && token.starts_with(delim);
        if is_operator {
            match last {
                LastTokenKind::None => {
                    return Err(SequenceError {
                        rest: input,
                        kind: SequenceErrorKind::LeadingOperator,
                    });
                }
                LastTokenKind::Operator => {
                    return Err(SequenceError {
                        rest: input,
                        kind: SequenceErrorKind::DoubleOperator,
                    });
                }
                LastTokenKind::Term => {
                    last = LastTokenKind::Operator;
                }
            }
        } else {
            match last {
                LastTokenKind::Term => {
                    // Two terms in a row without a delimiter
                    return Err(SequenceError {
                        rest: input,
                        kind: SequenceErrorKind::MissingOperatorBetweenTerms,
                    });
                }
                _ => {
                    terms.push(token);
                    last = LastTokenKind::Term;
                }
            }
        }
    }

    if last == LastTokenKind::Operator {
        return Err(SequenceError {
            rest: "",
            kind: SequenceErrorKind::TrailingOperator,
        });
    }

    Ok(terms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizer_with_delim_splits_on_space_and_preserves_rest() {
        let input = "$ide | com.apple.Safari";
        let (tok, rest) =
            next_token_with(input, '|').expect("should find first token");
        assert_eq!(tok, "$ide");
        assert_eq!(rest, "| com.apple.Safari");
    }

    #[test]
    fn tokenizer_with_delim_handles_single_token_without_spaces() {
        let input = "com.apple.Safari";
        let (tok, rest) =
            next_token_with(input, '|').expect("should return single token");
        assert_eq!(tok, "com.apple.Safari");
        assert_eq!(rest, "");
    }

    #[test]
    fn tokenizer_with_delim_splits_on_pipe_without_spaces() {
        let input = "$ide|com.apple.Safari";
        let (tok, rest) =
            next_token_with(input, '|').expect("should find first token");
        assert_eq!(tok, "$ide");
        assert_eq!(rest, "|com.apple.Safari");
    }

    #[test]
    fn tokenizer_with_delim_skips_multiple_spaces() {
        let input = "$ide   |   com.apple.Safari";
        let (tok, rest) =
            next_token_with(input, '|').expect("should find first token");
        assert_eq!(tok, "$ide");
        assert_eq!(rest, "|   com.apple.Safari");
    }

    #[test]
    fn parse_terms_accepts_valid_sequence() {
        let terms =
            parse_terms_with_delim("$ide | com.apple.Safari | $browser", '|')
                .expect("parser should accept valid selector");
        assert_eq!(terms, vec!["$ide", "com.apple.Safari", "$browser"]);
    }

    #[test]
    fn parse_terms_rejects_consecutive_operators() {
        let err =
            parse_terms_with_delim("$ide | | com.apple.Safari", '|').unwrap_err();
        assert_eq!(err.kind, SequenceErrorKind::DoubleOperator);
    }

    #[test]
    fn parse_terms_requires_operator_between_terms() {
        let err = parse_terms_with_delim("$ide com.apple.Safari", '|').unwrap_err();
        assert_eq!(err.kind, SequenceErrorKind::MissingOperatorBetweenTerms);
    }

    #[test]
    fn parse_terms_rejects_leading_operator() {
        let err =
            parse_terms_with_delim("| $ide | com.apple.Safari", '|').unwrap_err();
        assert_eq!(err.kind, SequenceErrorKind::LeadingOperator);
    }

    #[test]
    fn parse_terms_rejects_trailing_operator() {
        let err =
            parse_terms_with_delim("$ide | com.apple.Safari |", '|').unwrap_err();
        assert_eq!(err.kind, SequenceErrorKind::TrailingOperator);
    }

    #[test]
    fn parse_terms_accepts_adjacent_delims_without_spaces() {
        let terms = parse_terms_with_delim("$ide|$browser|com.apple.Safari", '|')
            .expect("parser should accept adjacent pipes");
        assert_eq!(terms, vec!["$ide", "$browser", "com.apple.Safari"]);
    }
}
