#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Operator {
    And,
    Or,
}


#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Term {
    Default(Value),
    Named {
        key: String,
        value: Value,
    },
    Combined {
        left: Box<Term>,
        right: Box<Term>,
        operator: Operator,
        grouping: bool,
    },
}


#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Value {
    Text(String),
    Integer(i64),
    Boolean(bool),
    Range(Boundary, Boundary),
}


#[derive(Debug, Clone, Eq, PartialEq)]
pub enum BoundaryKind {
    Inclusive,
    Exclusive,
}


#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Boundary {
    pub value: i64,
    pub kind: BoundaryKind,
}


pub fn deanonymize(tree: Term, default_fields: &[impl ToString]) -> Term {
    match tree {
        Term::Default(ref value) => default_fields.iter()
            .map(|name| Term::Named {
                key: name.to_string(),
                value: value.clone(),
            })
            .fold(None, |acc, term| match acc {
                Some(acc) => Some(Term::Combined {
                    left: Box::new(acc),
                    right: Box::new(term),
                    operator: Operator::Or,
                    grouping: false,
                }),
                None => Some(term),
            })
            .expect("Deanonymization returned `None`. This is a bug."),
        _ => tree,
    }
}


pub fn transform(tree: Term, visitor: &impl Fn(Term) -> Term) -> Term {
    match tree {
        Term::Combined {left, right, operator, grouping} => Term::Combined {
            operator,
            grouping,
            left: Box::new(transform(*left, visitor)),
            right: Box::new(transform(*right, visitor)),
        },
        _ => visitor(tree),
    }
}