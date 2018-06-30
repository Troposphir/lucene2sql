#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate serde;
extern crate pom;

use std::io::stdin;
use std::collections::{
    HashSet,
    HashMap,
};


mod compose;
mod parser;
mod ast;


#[derive(Debug, Serialize, Deserialize)]
struct InputCommand {
    query: String,
    table: String,
    default_fields: Vec<String>,
    allowed_fields: Option<HashSet<String>>,
    #[serde(default)]
    renames: HashMap<String, String>,
    #[serde(default)]
    expressions: ast::ExpressionRuleset,
}


#[derive(Debug)]
enum Error {
    Serialization(serde_json::Error),
    Parse(pom::Error),
    Deserialization(serde_json::Error),
}


fn main() {
    let InputCommand {
        query,
        table,
        default_fields,
        allowed_fields,
        renames,
        expressions,
    } = serde_json::from_reader(stdin())
        .map_err(Error::Deserialization)
        .unwrap();

    let sql = parser::query()
        .parse(query.as_bytes())
        .map_err(Error::Parse)
        .map(|tree| ast::transform(
            tree,
            &|term| ast::deanonymize(
                term,
                default_fields.as_slice(),
            ),
        ))
        .map(|tree| ast::transform(
            tree,
            &|term| ast::rename(
                term,
                &renames,
            ),
        ))
        .map(|tree| ast::transform(
            tree,
            &|term| ast::replace_expressions(
                term,
                &expressions,
            ),
        ))
        .map(|tree| compose::to_sql(
            &tree,
            table.as_str(),
            allowed_fields,
        ))
        .unwrap();

    let output = serde_json::to_string(&sql)
        .map_err(Error::Serialization)
        .unwrap();

    println!("{}", output);
}
