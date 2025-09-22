use ahash::AHashMap;
use thiserror::Error;
use super::combo::{parse_terms_with_delim, SequenceError, SequenceErrorKind};

pub(crate) type SelectorResult<T> = Result<T, SelectorError>;

#[derive(Error, Debug)]
pub enum SelectorError {
    #[error("invalid operator or: {0}")]
    InvalidOperatorOr(String),

    #[error("unknown group name \"{0}\"")]
    UnknownGroup(String),

    #[error("group and bundle id must be separated by an operator")]
    InvalidGroupAndBundleId(String),
}

/// A lexem is a token in a selector string.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Lexem<'a> {
    Group(&'a str),
    BundleId(&'a str),
    OperatorOr,
}

impl<'a> Lexem<'a> {
    fn parse(token: &'a str) -> Self {
        if token == "|" {
            return Self::OperatorOr;
        }
        if let Some(stripped) = token.strip_prefix('$') {
            return Self::Group(stripped);
        }
        Self::BundleId(token)
    }
}

/// A selector is an app list with groups and bundle ids.
/// It looks like this: `$ide | $browser | com.google.Chrome`.
#[derive(Debug)]
pub(crate) struct Selector<'a>(Vec<Lexem<'a>>);

impl<'a> Selector<'a> {
    /// Materializes the selector into a vector of bundle ids.
    /// Groups are replaced with their bundle ids.
    /// Or operator is ignored.
    pub(crate) fn materialize(
        &self,
        groups: &AHashMap<String, Vec<Box<str>>>,
    ) -> SelectorResult<Vec<Box<str>>> {
        // Pre-allocate at least the number of explicit terms;
        // additional capacity for groups is reserved on demand.
        let mut bundle_ids: Vec<Box<str>> = Vec::with_capacity(self.0.len());
        for token in self.0.iter() {
            match token {
                Lexem::BundleId(bundle_id) => bundle_ids.push((*bundle_id).into()),
                Lexem::Group(group) => {
                    let Some(ids) = groups.get(*group) else {
                        return Err(SelectorError::UnknownGroup(group.to_string()));
                    };
                    bundle_ids.reserve(ids.len());
                    bundle_ids.extend(ids.iter().cloned());
                }
                _ => (),
            }
        }

        Ok(bundle_ids)
    }

    /// Parses the selector string and validates it. Returns a vector of tokens.
    pub(crate) fn parse(input: &'a str) -> SelectorResult<Self> {
        let terms = match parse_terms_with_delim(input, '|') {
            Ok(t) => t,
            Err(SequenceError { rest, kind }) => {
                return Err(match kind {
                    SequenceErrorKind::LeadingOperator
                    | SequenceErrorKind::TrailingOperator
                    | SequenceErrorKind::DoubleOperator => {
                        SelectorError::InvalidOperatorOr(rest.to_string())
                    }
                    SequenceErrorKind::MissingOperatorBetweenTerms => {
                        SelectorError::InvalidGroupAndBundleId(rest.to_string())
                    }
                })
            }
        };

        let selector = terms.into_iter().map(Lexem::parse).collect::<Vec<_>>();

        Ok(Self(selector))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------- tokenizer (next_token)
    #[test]
    fn tokenizer_splits_on_space_and_preserves_rest() {
        use super::super::combo::next_token_with;
        let input = "$ide | com.apple.Safari";
        let (tok, rest) =
            next_token_with(input, '|').expect("should find first token");
        assert_eq!(tok, "$ide");
        assert_eq!(rest, "| com.apple.Safari");
    }

    #[test]
    fn tokenizer_handles_single_token_without_spaces() {
        use super::super::combo::next_token_with;
        let input = "com.apple.Safari";
        let (tok, rest) =
            next_token_with(input, '|').expect("should return single token");
        assert_eq!(tok, "com.apple.Safari");
        assert_eq!(rest, "");
    }

    #[test]
    fn tokenizer_splits_on_pipe_without_spaces() {
        use super::super::combo::next_token_with;
        let input = "$ide|com.apple.Safari";
        let (tok, rest) =
            next_token_with(input, '|').expect("should find first token");
        assert_eq!(tok, "$ide");
        assert_eq!(rest, "|com.apple.Safari");
    }

    #[test]
    fn tokenizer_skips_multiple_spaces() {
        use super::super::combo::next_token_with;
        let input = "$ide   |   com.apple.Safari";
        let (tok, rest) =
            next_token_with(input, '|').expect("should find first token");
        assert_eq!(tok, "$ide");
        assert_eq!(rest, "|   com.apple.Safari");
    }

    // -------- lexer (Lexem::parse)
    #[test]
    fn lexer_parses_operator_or() {
        assert_eq!(Lexem::parse("|"), Lexem::OperatorOr);
    }

    #[test]
    fn lexer_parses_group() {
        assert_eq!(Lexem::parse("$ide"), Lexem::Group("ide"));
    }

    #[test]
    fn lexer_parses_bundle_id() {
        assert_eq!(
            Lexem::parse("com.apple.Safari"),
            Lexem::BundleId("com.apple.Safari")
        );
    }

    // -------- parser (Selector::parse)
    #[test]
    fn parser_accepts_valid_sequence() {
        let s = Selector::parse("$ide | com.apple.Safari | $browser");
        assert!(s.is_ok(), "parser should accept valid selector");
    }

    #[test]
    fn parser_rejects_consecutive_or() {
        let s = Selector::parse("$ide | | com.apple.Safari");
        match s {
            Err(SelectorError::InvalidOperatorOr(_)) => {}
            _ => panic!("expected InvalidOperatorOr"),
        }
    }

    #[test]
    fn parser_requires_operator_between_terms() {
        let s = Selector::parse("$ide com.apple.Safari");
        match s {
            Err(SelectorError::InvalidGroupAndBundleId(_)) => {}
            _ => panic!("expected InvalidGroupAndBundleId"),
        }
    }

    #[test]
    fn parser_rejects_leading_or() {
        let s = Selector::parse("| $ide | com.apple.Safari");
        match s {
            Err(SelectorError::InvalidOperatorOr(_)) => {}
            _ => panic!("expected InvalidOperatorOr"),
        }
    }

    #[test]
    fn parser_rejects_trailing_or() {
        let s = Selector::parse("$ide | com.apple.Safari |");
        match s {
            Err(SelectorError::InvalidOperatorOr(_)) => {}
            _ => panic!("expected InvalidOperatorOr"),
        }
    }

    #[test]
    fn parser_accepts_adjacent_pipes_without_spaces() {
        let s = Selector::parse("$ide|$browser|com.apple.Safari");
        assert!(s.is_ok(), "parser should accept adjacent pipes");
    }

    // -------- materializer (Selector::materialize)
    #[test]
    fn materializer_expands_groups_and_keeps_bundle_ids() {
        let selector =
            Selector::parse("$ide | com.apple.Safari").expect("valid selector");
        let mut groups: AHashMap<String, Vec<Box<str>>> = AHashMap::new();
        groups.insert(
            "ide".to_string(),
            vec!["com.jetbrains.rust".into(), "com.cursor.cursor".into()],
        );

        let ids = selector.materialize(&groups).expect("materialize ok");
        assert_eq!(
            ids,
            vec![
                "com.jetbrains.rust".into(),
                "com.cursor.cursor".into(),
                "com.apple.Safari".into(),
            ]
        );
    }

    #[test]
    fn materializer_errors_on_unknown_group() {
        let selector =
            Selector::parse("$unknown | com.apple.Safari").expect("valid selector");
        let groups: AHashMap<String, Vec<Box<str>>> = AHashMap::new();
        match selector.materialize(&groups) {
            Err(SelectorError::UnknownGroup(name)) => assert_eq!(name, "unknown"),
            _ => panic!("expected UnknownGroup"),
        }
    }
}
