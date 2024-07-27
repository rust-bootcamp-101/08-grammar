use std::collections::HashMap;

use anyhow::{anyhow, Result};

use pest::{iterators::Pair, Parser};

#[derive(Debug, pest_derive::Parser)]
#[grammar = "../examples/json.pest"]
struct JsonParser;

#[derive(Debug, Clone, PartialEq)]
enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(JsonObject),
}

type JsonObject = HashMap<String, JsonValue>;

// #[derive(Debug, Clone, PartialEq)]
// enum Number {
//     Int(i64),
//     Float(f64),
// }

fn main() -> Result<()> {
    let s = r#"{
        "name": "John Doe",
        "age": 30,
        "is_student": false,
        "marks": [90.0, -80.1, 85.2],
        "address": {
            "city": "New York",
            "zip": 10001
        }
    }"#;
    let parsed = JsonParser::parse(Rule::json, s)?
        .next()
        .ok_or_else(|| anyhow!("json has no value"))?;
    let parsed = parse_value(parsed)?;
    println!("{:#?}", parsed);
    Ok(())
}

fn parse_value(pair: Pair<Rule>) -> Result<JsonValue> {
    match pair.as_rule() {
        Rule::null => Ok(JsonValue::Null),
        Rule::bool => Ok(JsonValue::Bool(pair.as_str().parse()?)),
        Rule::number => Ok(JsonValue::Number(pair.as_str().parse()?)),
        Rule::chars => Ok(JsonValue::String(pair.as_str().to_string())),
        Rule::array => Ok(JsonValue::Array(parse_array(pair)?)),
        Rule::object => Ok(JsonValue::Object(parse_object(pair)?)),
        Rule::value => {
            let inner = pair
                .into_inner()
                .next()
                .ok_or_else(|| anyhow!("expected value, found none"))?;
            parse_value(inner)
        }
        _ => unreachable!(),
    }
}

fn parse_array(pair: Pair<Rule>) -> Result<Vec<JsonValue>> {
    pair.into_inner()
        .map(parse_value)
        .collect::<Result<Vec<_>>>()
}

fn parse_object(pair: Pair<Rule>) -> Result<HashMap<String, JsonValue>> {
    let inner = pair.into_inner();
    let values = inner.map(|pair| {
        let mut inner = pair.into_inner();
        let key = inner
            .next()
            .map(|p| p.as_str().to_string())
            .ok_or_else(|| anyhow!("expected key in object, found none"))?;

        let pair = inner
            .next()
            .ok_or_else(|| anyhow!("expected value in object, found none"))?;
        let value = parse_value(pair)?;
        Ok((key, value))
    });
    values.collect::<Result<HashMap<_, _>>>()
}

#[cfg(test)]
mod tests {
    use pest::consumes_to;
    use pest::parses_to;

    use super::*;

    #[test]
    fn pest_parse_null_should_work() -> Result<()> {
        let input = "null";
        let parsed = JsonParser::parse(Rule::null, input)?.next().unwrap();
        let result = parse_value(parsed)?;
        assert_eq!(result, JsonValue::Null);
        Ok(())
    }

    #[test]
    fn pest_parse_bool_should_work() -> Result<()> {
        let input = "false";
        let parsed = JsonParser::parse(Rule::bool, input)?.next().unwrap();
        let result = parse_value(parsed)?;
        assert_eq!(result, JsonValue::Bool(false));

        let input = "true";
        let parsed = JsonParser::parse(Rule::bool, input)?.next().unwrap();
        let result = parse_value(parsed)?;
        assert_eq!(result, JsonValue::Bool(true));
        Ok(())
    }

    #[test]
    fn pest_parse_number_should_work() -> Result<()> {
        let input = "1.23";
        let parsed = JsonParser::parse(Rule::number, input)?.next().unwrap();
        let result = parse_value(parsed)?;
        assert_eq!(result, JsonValue::Number(1.23));

        let input = "-1.23";
        let parsed = JsonParser::parse(Rule::number, input)?.next().unwrap();
        let result = parse_value(parsed)?;
        assert_eq!(result, JsonValue::Number(-1.23));
        Ok(())
    }

    #[test]
    fn pest_parse_string_should_work() -> Result<()> {
        let input = r#""hello \"world\"""#;
        let parsed = JsonParser::parse(Rule::string, input)?.next().unwrap();
        let result = parse_value(parsed)?;
        assert_eq!(result, JsonValue::String("hello \\\"world\\\"".to_string()));
        Ok(())
    }

    #[test]
    fn pest_parse_array_should_work() -> Result<()> {
        let input = r#"[1,2,3]"#;
        let parsed = JsonParser::parse(Rule::array, input)?.next().unwrap();
        let result = parse_value(parsed)?;
        assert_eq!(
            result,
            JsonValue::Array(vec![
                JsonValue::Number(1.0),
                JsonValue::Number(2.0),
                JsonValue::Number(3.0),
            ])
        );
        Ok(())
    }

    #[test]
    fn pest_parse_object_should_work() -> Result<()> {
        let input = r#"{"a": 123}"#;
        let parsed = JsonParser::parse(Rule::object, input)?.next().unwrap();
        let result = parse_value(parsed)?;
        let mut expect = HashMap::new();
        expect.insert("a".to_string(), JsonValue::Number(123.0));
        assert_eq!(result, JsonValue::Object(expect));
        Ok(())
    }

    #[test]
    fn pest_parse_rule_should_work() -> Result<()> {
        parses_to! {
            parser: JsonParser,
            input: r#"{"hello":"world"}"#,
            rule: Rule::json,
            tokens: [
                object(0, 17, [
                    pair(1, 16, [
                        chars(2, 7),
                        value(9, 16, [
                            chars(10, 15),
                        ])
                    ])
                ])
            ]
        };
        Ok(())
    }
}
