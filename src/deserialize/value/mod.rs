use std::io::{self, Read};

use crate::{deserialize::whitespace::skip_whitespace, peek_reader::PeekReader, value::Value};

mod array;
mod number;
mod object;
mod string;

use array::parse_array;
use number::parse_number;
use object::{parse_identifier, parse_key_value_pairs_after_key, parse_object};
use string::{parse_byte_string, parse_raw_string, parse_string};

pub fn parse_value<R: Read>(
    reader: &mut PeekReader<R>,
    depth: u8,
    top_level: bool,
) -> io::Result<Value> {
    if depth == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Reached maximum depth",
        ));
    }

    let Some(first_byte) = reader.peek()? else {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "Got EOF when parsing value",
        ));
    };

    match first_byte {
        b'{' => return Ok(Value::Object(parse_object(reader, depth)?)),
        b'[' => return Ok(Value::Array(parse_array(reader, depth)?)),
        b'"' => {
            let string = parse_string(reader)?;
            if top_level {
                skip_whitespace(reader)?;
                if reader.peek()? == Some(b':') {
                    return Ok(Value::Object(parse_key_value_pairs_after_key(
                        reader, string, depth, true,
                    )?));
                }
            }
            return Ok(Value::String(string));
        }
        b'r' => {
            if let Some([_, second_byte]) = reader.peek2()?
                && matches!(second_byte, b'"' | b'#')
            {
                return Ok(Value::String(parse_raw_string(reader)?));
            }
        }
        b'b' => {
            if let Some([_, second_byte]) = reader.peek2()?
                && matches!(second_byte, b'"')
            {
                return Ok(Value::ByteString(parse_byte_string(reader)?));
            }
        }
        _ => {}
    }

    if first_byte.is_ascii_digit() || matches!(first_byte, b'+' | b'-' | b'.') {
        Ok(Value::Number(parse_number(reader)?))
    } else {
        let identifier = parse_identifier(reader)?;
        if top_level {
            skip_whitespace(reader)?;
            if reader.peek()? == Some(b':') {
                return Ok(Value::Object(parse_key_value_pairs_after_key(
                    reader, identifier, depth, true,
                )?));
            }
        }
        match identifier.as_str() {
            "true" => Ok(Value::Bool(true)),
            "false" => Ok(Value::Bool(false)),
            "null" => Ok(Value::Null),
            _ => Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!("Malformed value: {identifier}"),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn test_parse_value() {
        let data = "1";
        let mut reader = PeekReader::new(data.as_bytes());
        assert_eq!(
            parse_value(&mut reader, 100, true).unwrap(),
            Value::Number(1.0)
        );

        let data = "false";
        let mut reader = PeekReader::new(data.as_bytes());
        assert_eq!(
            parse_value(&mut reader, 100, true).unwrap(),
            Value::Bool(false)
        );

        let data = "false: false";
        let mut reader = PeekReader::new(data.as_bytes());
        assert_eq!(
            parse_value(&mut reader, 100, true).unwrap(),
            Value::Object(HashMap::from([("false".to_owned(), Value::Bool(false))]))
        );
    }
}
