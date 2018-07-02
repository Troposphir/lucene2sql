Converts (a subset of) the Lucene query syntaxt to SQL queries.

Currently, it supports the following features:

 - Words: single words, like `potato`;
 - Phrases: quoted strings, such as `"some stuff"`;
 - Named words or phrases: a word or phrase prefixed by `<fieldname>:`;
 - Groups: parenthesized queries of any sort;
 - Negation: negates the condition of any term by prefixing it with `-`;
 - Ranges: numeric ranges in the format `{a TO b}` or `[a TO b]`, for exclusive and inclusive ranges.
 
Sequential terms are `OR`ed together. Additionally, explicit `OR` and `AND`s are supported.

It generates correctly typed values for booleans, integers, and otherwise, strings. All strings are searched through `LIKE` with leading and trailing wildcards.

## Purpose

This was developed for the purpose of being used internally in the main [Troposphir](https://github.com/Troposphir/troposphir), hence why it uses standard I/O for input instead of command-line or similar.

Other Lucene features such as weights and fuzzy searches are unimplemented due to not being possible in naive SQL, and/or not used in the game's search feature.

## Input

This program expects a JSON file in the standard input with the following properties:

 - `query`: A string containing the Lucene source;
 - `table`: The table to which search. It will be quoted, so it cannot contain a database name;
 - `default_fields`: An array of field names. If a word or phrase is not named, then search these fields;
 - `allowed_fields` (optional): If provided, should be an array of field names that will be used as a whitelist. If any extraneous fields are found in the query, the program will error out;
 - `renames` (optional): A mapping of field names to field names, which replaces the keys for the values, to allow a different name in the input query than the actual database;
 - `expressions` (optional): This one is a bit tricky. It is a pattern-matching "switch": an object, where keys are field names, and contain an array of arrays. In this array, There should be 2-item arrays, the first being a expected value, and the second being a SQL condition. When a named or default field of that key is encountered, all cases will be tried in order. If the value of the input query matches the expected value of that rule, the whole term is replaced by the SQL condition. If the rule's value is `null`, the rule will _always_ match.

## Output

On the standard output, this program will emit a JSON with the contents:

 - `body`: A string containing the SQL query, using `?` as a parameter placeholder.
 - `params`: The parameters to bind, in order.

## Example

### Input

```json
{
  "query": "(a:true blabla) -foo AND -bar deleted:true",
  "table": "stuff",
  "default_fields": ["b", "c"],
  "allowed_fields": ["a", "b", "c", "deleted"],
  "expressions": {
    "deleted": [
      [true, "`DELETED_AT` IS NULL"],
      [false, "(NOT (`DELETED_AT` IS NULL))"]
    ]
  }
}
```

### Output

```json
{
  "body": "SELECT * FROM `stuff` WHERE (`a` = ? OR (`b` = ? OR `b` = ?)) OR ((NOT (`a` = ? OR `b` = ?)) AND (`a` = ? OR `b` = ?)) OR `DELETED_AT` IS NULL",
  "params": [
    1,
    "blabla",
    "blabla",
    "foo",
    "foo",
    "bar",
    "bar"
  ]
}
```
