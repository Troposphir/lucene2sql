use std::collections::HashMap;

use serde_json;


pub type ExpressionRuleset = HashMap<String, Vec<(
    serde_json::Value,
    String,
)>>;


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
    Expression(String),
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


fn match_expression(ruleset: &ExpressionRuleset, key: &str, value: &Value) -> Option<String> {
    let rules = ruleset.get(key)?;

    for (case, expression) in rules {
        let found = match (case, value) {
            (serde_json::Value::Number(a), Value::Integer(b)) => a
                .as_i64()
                .map(|a| a == *b)
                .unwrap_or(false),
            (serde_json::Value::Bool(a), Value::Boolean(b)) => *a == *b,
            (serde_json::Value::String(ref a), Value::Text(ref b)) => a == b,
            _ => false,
        };

        if found {
            return Some(expression.to_string());
        }
    }

    None
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


pub fn rename(tree: Term, renames: &HashMap<String, String>) -> Term {
    match tree {
        Term::Named {key, value} => match renames.get(&key) {
            Some(renamed) => Term::Named {
                key: renamed.clone(),
                value,
            },
            None => Term::Named {key, value},
        },
        _ => tree,
    }
}


pub fn replace_expressions(tree: Term, ruleset: &ExpressionRuleset) -> Term {
    match tree {
        Term::Named {key, value} => match_expression(ruleset, &key, &value)
            .map(Term::Expression)
            .unwrap_or(Term::Named {key, value}),
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