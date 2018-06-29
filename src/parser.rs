use std::str::FromStr;

use pom::{
    self,
    Parser,
    combinator::*,
    char_class::*,
};

use ast::{
    BoundaryKind,
    Boundary,
    Operator,
    Value,
    Term,
};


fn space<'a>() -> Combinator<impl Parser<'a, u8, Output=()>> {
    one_of(b" \t\r\n")
        .repeat(0..)
        .discard()
}

fn phrase<'a>() -> Combinator<impl Parser<'a, u8, Output=String>> {
    let escape_sequence = sym(b'\\') * (
        sym(b'\\')
        | sym(b'"')
        | sym(b'n').map(|_| b'\n')
        | sym(b't').map(|_| b'\t')
    );

    let character_string = (none_of(b"\\\"") | escape_sequence)
        .repeat(1..)
        .convert(String::from_utf8);

    let string = sym(b'"') * character_string.repeat(0..) - sym(b'"');

    string.map(|strings| strings.concat())
}

fn single_term<'a>() -> Combinator<impl Parser<'a, u8, Output=String>> {
    (is_a(alphanum) | one_of(b"-_.")).repeat(1..)
        .collect()
        .convert(|x| String::from_utf8(x.to_vec()))
}

fn text<'a>() -> Combinator<impl Parser<'a, u8, Output=Value>> {
    (phrase() | single_term())
        .map(Value::Text)
}

fn integer<'a>() -> Combinator<impl Parser<'a, u8, Output=i64>> {
    is_a(digit).repeat(0..)
        .convert(String::from_utf8)
        .convert(|s| i64::from_str(&s))
}

fn boolean<'a>() -> Combinator<impl Parser<'a, u8, Output=bool>> {
    seq(b"true").map(|_| true)
    | seq(b"false").map(|_| false)
}

fn range<'a>() -> Combinator<impl Parser<'a, u8, Output=Value>> {
    let open =
        sym(b'[').map(|_| BoundaryKind::Inclusive)
        | sym(b'{').map(|_| BoundaryKind::Exclusive);

    let close =
        sym(b']').map(|_| BoundaryKind::Inclusive)
        | sym(b'}').map(|_| BoundaryKind::Exclusive);

    (open + space() * integer() + space() * seq(b"TO") * space() * integer() + close)
        .map(|(((start_kind, start), end), end_kind)| Value::Range(
            Boundary {
                value: start,
                kind: start_kind,
            },
            Boundary {
                value: end,
                kind: end_kind,
            },
        ))
}

fn value<'a>() -> Combinator<impl Parser<'a, u8, Output=Value>> {
    range()
    | boolean().map(Value::Boolean)
    | integer().map(Value::Integer)
    | text()
}

fn operator<'a>() -> Combinator<impl Parser<'a, u8, Output=Operator>> {
    let core =
        (seq(b"AND") | seq(b"&&")).map(|_| Operator::And)
        | (seq(b"OR") | seq(b"||")).map(|_| Operator::Or)
        | space().map(|_| Operator::Or);

    space() * core - space()
}

fn field<'a>() -> Combinator<impl Parser<'a, u8, Output=Term>> {
    (single_term() - sym(b':') + value())
        .map(|(k, v)| Term::Named {
            key: k,
            value: v,
        })
}

fn default<'a>() -> Combinator<impl Parser<'a, u8, Output=Term>> {
    value().map(Term::Default)
}

fn term<'a>() -> Combinator<impl Parser<'a, u8, Output=Term>> {
    field() | default()
}

fn many<'a>() -> Combinator<impl Parser<'a, u8, Output=Term>> {
    (space() * comb(partial_expr) + (operator() + comb(expr)).repeat(1..) - space())
        .map(|(head, tail)| tail
            .into_iter()
            .fold(head, |left, (operator, right)| Term::Combined {
                left: Box::new(left),
                right: Box::new(right),
                operator,
                grouping: false,
            }),
        )
}

fn group<'a>() -> Combinator<impl Parser<'a, u8, Output=Term>> {
    (sym(b'(') * space() * comb(expr) - space() - sym(b')'))
        .map(|term| match term {
            Term::Combined {left, right, operator, grouping: _} => Term::Combined {
                left,
                right,
                operator,
                grouping: true,
            },
            _ => term,
        })
}

fn partial_expr<'a>(input: &'a [u8], start: usize) -> pom::Result<(Term, usize)> {
    (group() | term()).0.parse(input, start)
}

fn expr<'a>(input: &'a [u8], start: usize) -> pom::Result<(Term, usize)> {
    let opts = many() | comb(partial_expr);

    opts.0.parse(input, start)
}

pub fn query<'a>() -> Combinator<impl Parser<'a, u8, Output=Term>> {
    space() * comb(expr) - space() - end()
}