use std::{char, collections::HashMap, fmt};

use anyhow::{anyhow, Result};
use winnow::{
    ascii::{digit1, multispace0, Caseless},
    combinator::{alt, delimited, opt, separated, separated_pair, trace},
    error::{ContextError, ErrMode, ParserError},
    stream::{AsBStr, AsChar, Compare, FindSlice, ParseSlice, Stream, StreamIsPartial},
    token::take_until,
    PResult, Parser,
};

#[allow(unused)]
#[derive(Debug, Clone, PartialEq)]
enum JsonValue {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Array(Vec<JsonValue>),
    Object(JsonObject),
}

type JsonObject = HashMap<String, JsonValue>;

#[derive(Debug, Clone, PartialEq)]
enum Number {
    Int(i64),
    Float(f64),
}

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
    let input = &mut (&*s);
    let v = parse_json(input)?;
    println!("{:?}", v);

    let s = r#"{
        "name": "John Doe",
        "age": 30,
        "is_student": false,
        "marks": [90, -80, 85],
        "address": {
            "city": "New York",
            "zip": 10001
        }
    }"#;
    let input = &mut (&*s);
    let v = parse_json(input)?;
    println!("{:?}", v);
    Ok(())
}

fn parse_json(input: &str) -> Result<JsonValue> {
    let input = &mut (&*input);
    let v = parse_value(input)
        .map_err(|e: ErrMode<ContextError>| anyhow!("Failed to parse JSON: {:?}", e))?;
    Ok(v)
}

fn parse_null<Input, Error>(input: &mut Input) -> PResult<(), Error>
where
    Input: StreamIsPartial + Stream + Compare<&'static str>,
    Error: ParserError<Input>,
{
    "null".value(()).parse_next(input)
}

fn parse_bool<Input, Error>(input: &mut Input) -> PResult<bool, Error>
where
    Input: StreamIsPartial + Stream + Compare<&'static str>,
    <Input as Stream>::Slice: ParseSlice<bool>,
    Error: ParserError<Input>,
{
    alt(("true", "false")).parse_to().parse_next(input)
}

fn parse_number<Input, Error>(input: &mut Input) -> PResult<Number, Error>
where
    Input: StreamIsPartial
        + Stream
        + Compare<&'static str>
        + Compare<Caseless<&'static str>>
        + Compare<char>
        + AsBStr,
    <Input as Stream>::Slice: ParseSlice<i64> + ParseSlice<f64>,
    <Input as Stream>::Token: AsChar + Clone,
    <Input as Stream>::IterOffsets: Clone,
    Error: ParserError<Input>,
{
    let sign = opt("-").map(|s| s.is_some()).parse_next(input)?;
    let num = digit1.parse_to::<i64>().parse_next(input)?;
    let ret: Result<(), ErrMode<ContextError>> = ".".value(()).parse_next(input);
    if ret.is_ok() {
        let frac = digit1.parse_to::<i64>().parse_next(input)?;
        let v = format!("{}.{}", num, frac).parse::<f64>().unwrap();
        Ok(if sign {
            Number::Float(-v)
        } else {
            Number::Float(v)
        })
    } else {
        Ok(if sign {
            Number::Int(-num)
        } else {
            Number::Int(num)
        })
    }
}

// 可以不写这个解析，因为直接调用 float 解析即可
// fn parse_number<Input, Error>(input: &mut Input) -> PResult<f64, Error>
// where
//     Input: StreamIsPartial + Stream + Compare<Caseless<&'static str>> + Compare<char> + AsBStr,
//     <Input as Stream>::Token: AsChar + Clone,
//     <Input as Stream>::Slice: ParseSlice<f64>,
//     <Input as Stream>::IterOffsets: Clone,
//     Error: ParserError<Input>,
// {
//     float.parse_next(input)
// }

fn parse_string<Input, Error>(input: &mut Input) -> PResult<String, Error>
where
    Input: StreamIsPartial + Stream + Compare<char> + FindSlice<char>,
    <Input as Stream>::Token: AsChar,
    <Input as Stream>::Slice: fmt::Display,
    Error: ParserError<Input>,
{
    let ret = delimited('"', take_until(0.., '"'), '"').parse_next(input)?;
    Ok(ret.to_string())
}

fn parse_array<Input, Error>(input: &mut Input) -> PResult<Vec<JsonValue>, Error>
where
    Input: StreamIsPartial
        + Stream
        + Compare<&'static str>
        + Compare<char>
        + Compare<Caseless<&'static str>>
        + FindSlice<char>
        + AsBStr,
    <Input as Stream>::Token: AsChar + Clone,
    <Input as Stream>::Slice: fmt::Display + ParseSlice<i64> + ParseSlice<f64> + ParseSlice<bool>,
    <Input as Stream>::IterOffsets: Clone,
    Error: ParserError<Input>,
{
    let sep1 = sep_with_space('[');
    let sep2 = sep_with_space(']');
    let sep_comma = sep_with_space(',');
    let parse_values = separated(0.., parse_value, sep_comma);
    delimited(sep1, parse_values, sep2).parse_next(input)
}

fn parse_object<Input, Error>(input: &mut Input) -> PResult<JsonObject, Error>
where
    Input: StreamIsPartial
        + Stream
        + Compare<&'static str>
        + Compare<char>
        + Compare<Caseless<&'static str>>
        + FindSlice<char>
        + AsBStr,
    <Input as Stream>::Token: AsChar + Clone,
    <Input as Stream>::Slice: fmt::Display + ParseSlice<i64> + ParseSlice<f64> + ParseSlice<bool>,
    <Input as Stream>::IterOffsets: Clone,
    Error: ParserError<Input>,
{
    let sep1 = sep_with_space('{');
    let sep2 = sep_with_space('}');
    let sep_comma = sep_with_space(',');
    let sep_colon = sep_with_space(':');
    let parse_kv_pair = separated_pair(parse_string, sep_colon, parse_value);
    let parse_kv = separated(1.., parse_kv_pair, sep_comma);
    delimited(sep1, parse_kv, sep2).parse_next(input)
}

fn parse_value<Input, Error>(input: &mut Input) -> PResult<JsonValue, Error>
where
    Input: StreamIsPartial
        + Stream
        + Compare<&'static str>
        + Compare<char>
        + Compare<Caseless<&'static str>>
        + FindSlice<char>
        + AsBStr,
    <Input as Stream>::Token: AsChar + Clone,
    <Input as Stream>::Slice: fmt::Display + ParseSlice<i64> + ParseSlice<f64> + ParseSlice<bool>,
    <Input as Stream>::IterOffsets: Clone,
    Error: ParserError<Input>,
{
    alt((
        parse_null.value(JsonValue::Null),
        parse_bool.map(JsonValue::Bool),
        parse_number.map(JsonValue::Number),
        parse_string.map(JsonValue::String),
        parse_array.map(JsonValue::Array),
        parse_object.map(JsonValue::Object),
    ))
    .parse_next(input)
}

fn sep_with_space<Input, Output, Error, ParseNext>(
    mut parser: ParseNext,
) -> impl Parser<Input, (), Error>
where
    Input: Stream + StreamIsPartial,
    <Input as Stream>::Token: AsChar + Clone,
    Error: ParserError<Input>,
    ParseNext: Parser<Input, Output, Error>,
{
    trace("sep_with_space", move |input: &mut Input| {
        let _ = multispace0.parse_next(input)?;
        let _ = parser.parse_next(input)?;
        let _ = multispace0.parse_next(input)?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_null_should_work() -> PResult<(), ContextError> {
        let input = "null";
        let ret = parse_null(&mut (&*input))?;
        assert_eq!(ret, ());
        Ok(())
    }

    #[test]
    fn test_parse_bool_should_work() -> PResult<(), ContextError> {
        let input = "false";
        let ret = parse_bool(&mut (&*input))?;
        assert_eq!(ret, false);
        let input = "true";
        let ret = parse_bool(&mut (&*input))?;
        assert_eq!(ret, true);
        Ok(())
    }

    #[test]
    fn test_parse_number_should_work() -> PResult<(), ContextError> {
        let input = "90";
        let ret = parse_number(&mut (&*input))?;
        assert_eq!(ret, Number::Int(90));
        let input = "-89";
        let ret = parse_number(&mut (&*input))?;
        assert_eq!(ret, Number::Int(-89));

        let input = "90.0";
        let ret = parse_number(&mut (&*input))?;
        assert_eq!(ret, Number::Float(90.0));
        let input = "-89.1";
        let ret = parse_number(&mut (&*input))?;
        assert_eq!(ret, Number::Float(-89.1));
        Ok(())
    }

    #[test]
    fn test_parse_string_should_work() -> PResult<(), ContextError> {
        let input = r#""a string""#;
        let ret = parse_string(&mut (&*input))?;
        assert_eq!(ret, "a string");

        Ok(())
    }

    #[test]
    fn test_parse_array_should_work() -> PResult<(), ContextError> {
        let input = r#" [ 1,2, -3 ]"#;
        let ret = parse_array(&mut (&*input))?;
        assert_eq!(
            ret,
            [
                JsonValue::Number(Number::Int(1)),
                JsonValue::Number(Number::Int(2)),
                JsonValue::Number(Number::Int(-3))
            ]
        );

        Ok(())
    }

    #[test]
    fn test_parse_object_should_work() -> PResult<(), ContextError> {
        let input = r#"{"a": 123 }"#;
        let ret = parse_object(&mut (&*input))?;
        assert_eq!(ret.len(), 1);
        assert_eq!(ret.get("a"), Some(&JsonValue::Number(Number::Int(123))));
        Ok(())
    }
}
