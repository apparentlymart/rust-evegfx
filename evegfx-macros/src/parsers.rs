use nom::{IResult, Parser};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum Token<'a> {
    Literal(&'a [u8]),
    Verb(&'a [u8]),
    Percent(&'a [u8]),
    Null(&'a [u8]),
    Unterminated(&'a [u8]),
    Invalid(&'a [u8]),
}

pub(crate) fn next_token<'a>(input: &'a [u8]) -> (Token<'a>, &'a [u8]) {
    match fmt_token(input) {
        Ok((remain, token)) => (token, remain),
        Err(err) => match err {
            nom::Err::Incomplete(_) => unreachable!(), // we use complete matchers
            nom::Err::Error(err) => (Token::Invalid(err.input), &b""[..]),
            nom::Err::Failure(err) => (Token::Invalid(err.input), &b""[..]),
        },
    }
}

const PERCENT_BYTE: u8 = b'%';
const NULL_BYTE: u8 = 0x00;

fn fmt_literal(i: &[u8]) -> IResult<&[u8], &[u8]> {
    nom::bytes::complete::take_till(|b| (b == PERCENT_BYTE || b == NULL_BYTE))(i)
}

fn fmt_null(i: &[u8]) -> IResult<&[u8], &[u8]> {
    nom::bytes::complete::tag(b"\x00")(i)
}

fn fmt_percent(i: &[u8]) -> IResult<&[u8], &[u8]> {
    nom::bytes::complete::tag(b"%%")(i)
}

fn fmt_verb(i: &[u8]) -> IResult<&[u8], &[u8]> {
    nom::combinator::recognize(nom::sequence::tuple((
        nom::bytes::complete::tag(b"%"),
        nom::bytes::complete::take_till(is_format_verb),
        nom::bytes::complete::take_while_m_n(1, 1, is_format_verb),
    )))(i)
}

fn fmt_verb_unterminated(i: &[u8]) -> IResult<&[u8], &[u8]> {
    nom::combinator::recognize(nom::sequence::tuple((
        nom::bytes::complete::tag(b"%"),
        nom::bytes::complete::take_till(is_format_verb),
    )))(i)
}

fn fmt_token(i: &[u8]) -> IResult<&[u8], Token> {
    nom::branch::alt((
        fmt_percent.map(|bytes| Token::Percent(bytes)),
        fmt_verb.map(|bytes| Token::Verb(bytes)),
        fmt_verb_unterminated.map(|bytes| Token::Unterminated(bytes)),
        fmt_null.map(|bytes| Token::Null(bytes)),
        fmt_literal.map(|bytes| Token::Literal(bytes)),
    ))(i)
}

fn is_format_verb(b: u8) -> bool {
    (b >= b'a' && b <= b'z') || (b >= b'A' && b <= b'Z')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_just_literal() {
        let got = fmt_token(&b"hello"[..]).unwrap();
        let want = (&b""[..], Token::Literal(&b"hello"[..]));
        assert_eq!(got, want);
    }

    #[test]
    fn test_just_null() {
        let got = fmt_token(&b"\x00"[..]).unwrap();
        let want = (&b""[..], Token::Null(&b"\x00"[..]));
        assert_eq!(got, want);
    }

    #[test]
    fn test_just_verb() {
        let got = fmt_token(&b"%05x"[..]).unwrap();
        let want = (&b""[..], Token::Verb(&b"%05x"[..]));
        assert_eq!(got, want);
    }

    #[test]
    fn test_just_escape() {
        let got = fmt_token(&b"%%"[..]).unwrap();
        let want = (&b""[..], Token::Percent(&b"%%"[..]));
        assert_eq!(got, want);
    }

    #[test]
    fn test_literal_then_verb() {
        let got = fmt_token(&b"hello %s"[..]).unwrap();
        let want = (&b"%s"[..], Token::Literal(&b"hello "[..]));
        assert_eq!(got, want);
    }

    #[test]
    fn test_literal_then_null() {
        let got = fmt_token(&b"hello\0world"[..]).unwrap();
        let want = (&b"\0world"[..], Token::Literal(&b"hello"[..]));
        assert_eq!(got, want);
    }

    #[test]
    fn test_verb_then_literal() {
        let got = fmt_token(&b"%-08d items"[..]).unwrap();
        let want = (&b" items"[..], Token::Verb(&b"%-08d"[..]));
        assert_eq!(got, want);
    }

    #[test]
    fn test_multiple_verbs() {
        let got = fmt_token(&b"%-08d%s"[..]).unwrap();
        let want = (&b"%s"[..], Token::Verb(&b"%-08d"[..]));
        assert_eq!(got, want);
    }

    #[test]
    fn test_multiple_escapes() {
        let got = fmt_token(&b"%%%%"[..]).unwrap();
        let want = (&b"%%"[..], Token::Percent(&b"%%"[..]));
        assert_eq!(got, want);
    }

    #[test]
    fn test_just_incomplete_verb() {
        let got = fmt_token(&b"%"[..]).unwrap();
        let want = (&b""[..], Token::Unterminated(&b"%"[..]));
        assert_eq!(got, want);
    }

    #[test]
    fn test_incomplete_verb_and_more() {
        let got = fmt_token(&b"%36435456345%"[..]).unwrap();
        let want = (&b""[..], Token::Unterminated(&b"%36435456345%"[..]));
        assert_eq!(got, want);
    }
}
