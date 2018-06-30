use std::collections::HashSet;
use std::collections::HashMap;

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
    values: HashMap<String, JsonValue>,
}


impl QueryContext {
    fn add_value(&mut self, value: JsonValue) -> String {
        let key = format!(":v{}", self.values.len());
        self.values.insert(key.clone(), value);
        key
    }
}


#[derive(Debug, Serialize, Deserialize)]
pub struct Query {
    pub body: String,
    pub named_params: HashMap<String, JsonValue>,
}


fn value_to_condition(context: &mut QueryContext, key: &str, value: &Value) -> String {
    let right_hand_side = match value {
        Value::Integer(value) => JsonValue::Number((*value).into()),
        Value::Boolean(value) => JsonValue::Bool(*value),
        Value::Text(value) => return format!(
            "`{}` LIKE CONCAT('%', {}, '%')",
            key,
            context.add_value(JsonValue::String(value.to_string())),
        ),
        Value::Range(
            Boundary { value: start, kind: start_kind },
            Boundary { value: end, kind: end_kind },
        ) => {
            let start_op = match start_kind {
                BoundaryKind::Exclusive => ">",
                BoundaryKind::Inclusive => ">=",
            };

            let end_op = match end_kind {
                BoundaryKind::Exclusive => "<",
                BoundaryKind::Inclusive => "<=",
            };

            return format!(
                "(`{}` {} {} AND `{}` {} {})",
                key,
                start_op,
                context.add_value(JsonValue::Number((*start).into())),
                key,
                end_op,
                context.add_value(JsonValue::Number((*end).into())),
            );
        },
    };

    format!(
        "`{}` = {}",
        key,
        context.add_value(right_hand_side),
    )
}


fn sql_reducer(tree: &Term, context: &mut QueryContext) -> Result<String, String> {
    Ok(match tree {
        Term::Expression(expr) => expr.to_string(),
        Term::Combined {left, right, operator, grouping} => {
            match (sql_reducer(&*left, context), sql_reducer(&*right, context)) {
                (Ok(left), Ok(right)) => {
                    let operator = match operator {
                        Operator::Or => "OR",
                        Operator::And => "AND",
                    };

                    let inner = format!("{} {} {}", left, operator, right);

                    if *grouping {
                        format!("({})", inner)
                    } else {
                        inner
                    }
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
            value_to_condition(context, &key, &value)
        },
        _ => panic!("All terms must be named to generate a SQL query."),
    })
}


pub fn to_sql(
    tree: &Term,
    table: &str,
    allowed_fields: Option<HashSet<String>>,
) -> Result<Query, String> {
    let mut context = QueryContext {
        allowed_fields,
        values: HashMap::new(),
    };

    let conditions = sql_reducer(tree, &mut context);
    // TODO: Reduce allocations here
    let joined_fields = match context.allowed_fields {
        Some(allowed_fields) => format!(
            "`{}`",
            allowed_fields.into_iter()
                .collect::<Vec<String>>()
                .join("`, `"),
        ),
        None => "*".to_string(),
    };
    let values = context.values;

    conditions.map(|conditions| Query {
        body: format!(
            "SELECT {} FROM `{}` WHERE {};",
            joined_fields,
            table,
            conditions,
        ),
        named_params: values,
    })
}