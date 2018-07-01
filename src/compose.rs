use std::collections::HashSet;

use serde_json::Value as JsonValue;

use ast::{
    Value,
    Operator,
    Term,
    Boundary,
    BoundaryKind,
};


#[derive(Debug)]
struct QueryContext {
    allowed_fields: Option<HashSet<String>>,
}


#[derive(Debug)]
pub enum Part<'a> {
    String(&'a str),
    Parameter(JsonValue),
}


#[derive(Debug, Serialize, Deserialize)]
pub struct Query {
    pub body: String,
    pub params: Vec<JsonValue>,
}


fn value_to_condition<'a>(
    key: &'a str,
    value: &'a Value,
) -> Vec<Part<'a>> {
    let mut parts = Vec::new();

    let right_hand_side = match value {
        Value::Integer(value) => JsonValue::Number((*value).into()),
        Value::Boolean(value) => JsonValue::Bool(*value),
        Value::Text(value) => {
            parts.push(Part::String("`"));
            parts.push(Part::String(key));
            parts.push(Part::String("` LIKE CONCAT('%', "));
            parts.push(Part::Parameter(JsonValue::String(value.to_string())));
            parts.push(Part::String(", '%')"));
            return parts;
        },
        Value::Range(
            Boundary { value: start, kind: start_kind },
            Boundary { value: end, kind: end_kind },
        ) => {
            parts.push(Part::String("(`"));
            parts.push(Part::String(key));
            parts.push(Part::String("` "));
            parts.push(Part::String(match start_kind {
                BoundaryKind::Exclusive => ">",
                BoundaryKind::Inclusive => ">=",
            }));
            parts.push(Part::String(" "));
            parts.push(Part::Parameter(JsonValue::Number((*start).into())));
            parts.push(Part::String(" AND `"));
            parts.push(Part::String(key));
            parts.push(Part::String("` "));
            parts.push(Part::String(match end_kind {
                BoundaryKind::Exclusive => "<",
                BoundaryKind::Inclusive => "<=",
            }));
            parts.push(Part::String(" "));
            parts.push(Part::Parameter(JsonValue::Number((*end).into())));
            return parts;
        },
    };

    parts.push(Part::String("`"));
    parts.push(Part::String(key));
    parts.push(Part::String("` = "));
    parts.push(Part::Parameter(right_hand_side));
    parts
}


fn sql_reducer<'a>(
    tree: &'a Term,
    context: &'a QueryContext,
) -> Result<Vec<Part<'a>>, String> {
    Ok(match tree {
        Term::Expression(expr) => vec![Part::String(expr)],
        Term::Combined {left, right, operator, grouping} => {
            match (sql_reducer(&*left, context), sql_reducer(&*right, context)) {
                (Ok(mut left), Ok(mut right)) => {
                    let mut parts = Vec::new();

                    if *grouping {
                        parts.push(Part::String("("));
                    }

                    parts.append(&mut left);

                    parts.push(Part::String(" "));
                    parts.push(Part::String(match operator {
                        Operator::Or => "OR",
                        Operator::And => "AND",
                    }));
                    parts.push(Part::String(" "));

                    parts.append(&mut right);

                    if *grouping {
                        parts.push(Part::String(")"));
                    }

                    parts
                }
                (Err(key), Ok(_)) => return Err(key),
                (Ok(_), Err(key)) => return Err(key),
                (Err(a), Err(b)) => return Err(format!("{}, {}", a, b)),
            }
        },
        Term::Named {key, value} => {
            let whitelisted = context.allowed_fields
                .as_ref()
                .map(|fields| fields.contains(key))
                .unwrap_or(true);

            if !whitelisted {
                return Err(key.to_string());
            }

            value_to_condition(&key, &value)
        },
        Term::Negated(inner) => return sql_reducer(&*inner, context)
            .map(|mut inner_parts| {
                let mut parts = vec![Part::String("(NOT (")];
                parts.append(&mut inner_parts);
                parts.push(Part::String("))"));

                parts
            }),
        _ => panic!("All terms must be named to generate a SQL query."),
    })
}


pub fn to_sql(
    tree: &Term,
    table: &str,
    allowed_fields: Option<HashSet<String>>,
) -> Result<Query, String> {
    let context = QueryContext {
        allowed_fields,
    };

    let parts = sql_reducer(tree, &context)?;

    let mut query = Query {
        body: "SELECT * FROM `".to_string(),
        params: Vec::new(),
    };

    query.body.push_str(table);
    query.body.push_str("` WHERE ");

    for part in parts {
        match part {
            Part::String(string) => query.body.push_str(string),
            Part::Parameter(value) => {
                query.body.push_str("?");
                query.params.push(value);
            }
        }
    }

    query.body.push(';');

    Ok(query)
}