#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate serde;
extern crate pom;

use std::io::stdin;
use std::collections::HashSet;


mod compose;
mod parser;
mod ast;


#[derive(Debug, Serialize, Deserialize)]
struct InputCommand {
    query: String,
    allowed_fields: Option<HashSet<String>>,
    default_fields: Vec<String>,
    table: String,
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
        default_fields,
        allowed_fields,
        table,
    } = serde_json::from_reader(stdin())
        .map_err(Error::Deserialization)
        .unwrap();

    // let query =  b"(draft:false AND deleted:false AND version:2 AND xgms:1) AND ct:[1528766537000 TO 1529976137000] potato land";
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
