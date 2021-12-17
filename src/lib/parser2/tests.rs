use nom::Finish;

use super::*;

#[cfg(test)]
mod parse_literal {
    use super::*;

    #[test]
    fn bool() {
        let input = Parser::new_extra("true", ParserCtx::new(PathBuf::new()));

        let (_rest, num_parsed) = parse_literal(input).finish().unwrap();

        assert!(matches!(num_parsed.kind, LiteralKind::Bool(true)));
    }

    #[test]
    fn number() {
        let input = Parser::new_extra("42", ParserCtx::new(PathBuf::new()));

        let (_rest, num_parsed) = parse_literal(input).finish().unwrap();

        assert!(matches!(num_parsed.kind, LiteralKind::Number(42)));
    }

    #[test]
    fn float() {
        let input = Parser::new_extra("42.42", ParserCtx::new(PathBuf::new()));

        let (_rest, num_parsed) = parse_literal(input).finish().unwrap();

        assert!(matches!(num_parsed.kind, LiteralKind::Float(f) if f == 42.42));
    }
}

#[cfg(test)]
mod parse_bool {
    use super::*;

    #[test]
    fn r#true() {
        let input = Parser::new_extra("true", ParserCtx::new(PathBuf::new()));

        let (_rest, parsed) = parse_bool(input).finish().unwrap();

        assert!(matches!(parsed.kind, LiteralKind::Bool(true)));
    }

    #[test]
    fn r#false() {
        let input = Parser::new_extra("false", ParserCtx::new(PathBuf::new()));

        let (_rest, parsed) = parse_bool(input).finish().unwrap();

        assert!(matches!(parsed.kind, LiteralKind::Bool(false)));
    }

    #[test]
    fn invalid() {
        let input = Parser::new_extra("atrue", ParserCtx::new(PathBuf::new()));

        assert!(parse_bool(input).finish().is_err());
    }
}

#[cfg(test)]
mod parse_float {
    use super::*;

    #[test]
    fn valid_with_last_part() {
        let input = Parser::new_extra("42.42", ParserCtx::new(PathBuf::new()));

        let (_rest, parsed) = parse_float(input).finish().unwrap();

        assert!(matches!(parsed.kind, LiteralKind::Float(f) if f == 42.42));
    }

    #[test]
    fn valid_no_last_part() {
        let input = Parser::new_extra("42.", ParserCtx::new(PathBuf::new()));

        let (_rest, parsed) = parse_float(input).finish().unwrap();

        assert!(matches!(parsed.kind, LiteralKind::Float(f) if f == 42.0));
    }

    #[test]
    fn invalid() {
        let input = Parser::new_extra("a42.", ParserCtx::new(PathBuf::new()));

        assert!(parse_float(input).finish().is_err());
    }
}

#[cfg(test)]
mod parse_number {
    use super::*;

    #[test]
    fn valid() {
        let input = Parser::new_extra("42", ParserCtx::new(PathBuf::new()));

        let (_rest, parsed) = parse_number(input).finish().unwrap();

        assert!(matches!(parsed.kind, LiteralKind::Number(42)));
    }

    #[test]
    fn invalid() {
        let input = Parser::new_extra("a42", ParserCtx::new(PathBuf::new()));

        assert!(parse_number(input).finish().is_err());
    }
}
