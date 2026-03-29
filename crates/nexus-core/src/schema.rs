use std::path::Path;

use pest::Parser;
use pest_derive::Parser;

use crate::error::NexusCoreError;
use crate::types::{Field, FieldType, StructDef};

#[derive(Parser)]
#[grammar = "nxs.pest"]
struct NxsParser;

pub fn parse_nxs(input: &str) -> Result<Vec<StructDef>, NexusCoreError> {
    let pairs = NxsParser::parse(Rule::file, input).map_err(|e| NexusCoreError::SchemaParse {
        path: "<input>".to_string(),
        message: e.to_string(),
    })?;

    let mut structs = Vec::new();

    for pair in pairs {
        if pair.as_rule() == Rule::file {
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::struct_def {
                    structs.push(parse_struct_def(inner)?);
                }
            }
        }
    }

    Ok(structs)
}

fn parse_struct_def(pair: pest::iterators::Pair<Rule>) -> Result<StructDef, NexusCoreError> {
    let mut inner = pair.into_inner();

    let name = inner
        .next()
        .expect("struct_def must have an ident")
        .as_str()
        .to_string();

    let mut fields = Vec::new();
    for field_pair in inner {
        if field_pair.as_rule() == Rule::field_def {
            fields.push(parse_field_def(field_pair)?);
        }
    }

    Ok(StructDef { name, fields })
}

fn parse_field_def(pair: pest::iterators::Pair<Rule>) -> Result<Field, NexusCoreError> {
    let mut inner = pair.into_inner();

    let name = inner
        .next()
        .expect("field_def must have an ident")
        .as_str()
        .to_string();

    let type_pair = inner.next().expect("field_def must have a field_type");
    let typ = parse_field_type(type_pair)?;

    Ok(Field { name, typ })
}

fn parse_field_type(pair: pest::iterators::Pair<Rule>) -> Result<FieldType, NexusCoreError> {
    // field_type rule wraps the actual type variant
    let inner = pair
        .into_inner()
        .next()
        .expect("field_type must have inner");
    parse_field_type_inner(inner)
}

fn parse_field_type_inner(pair: pest::iterators::Pair<Rule>) -> Result<FieldType, NexusCoreError> {
    match pair.as_rule() {
        Rule::primitive_type => parse_primitive(pair.as_str()),
        Rule::bytes_type => {
            let n = pair
                .into_inner()
                .next()
                .expect("bytes_type has a number")
                .as_str()
                .parse::<usize>()
                .expect("number is valid usize");
            Ok(FieldType::Bytes(n))
        }
        Rule::string_type => {
            let n = pair
                .into_inner()
                .next()
                .expect("string_type has a number")
                .as_str()
                .parse::<usize>()
                .expect("number is valid usize");
            Ok(FieldType::StringFixed(n))
        }
        Rule::array_type => {
            let mut inner = pair.into_inner();
            let elem_pair = inner.next().expect("array_type has field_type");
            let len_pair = inner.next().expect("array_type has number");

            let elem = parse_field_type(elem_pair)?;
            let len = len_pair
                .as_str()
                .parse::<usize>()
                .expect("number is valid usize");

            Ok(FieldType::Array {
                elem: Box::new(elem),
                len,
            })
        }
        Rule::ident => Ok(FieldType::Nested(pair.as_str().to_string())),
        other => Err(NexusCoreError::SchemaParse {
            path: "<input>".to_string(),
            message: format!("unexpected rule {:?} in field_type", other),
        }),
    }
}

fn parse_primitive(s: &str) -> Result<FieldType, NexusCoreError> {
    match s {
        "u8" => Ok(FieldType::U8),
        "u16" => Ok(FieldType::U16),
        "u32" => Ok(FieldType::U32),
        "u64" => Ok(FieldType::U64),
        "i8" => Ok(FieldType::I8),
        "i16" => Ok(FieldType::I16),
        "i32" => Ok(FieldType::I32),
        "i64" => Ok(FieldType::I64),
        "f32" => Ok(FieldType::F32),
        "f64" => Ok(FieldType::F64),
        "bool" => Ok(FieldType::Bool),
        other => Err(NexusCoreError::UnknownFieldType(other.to_string())),
    }
}

pub fn parse_nxs_file(path: &Path) -> Result<Vec<StructDef>, NexusCoreError> {
    let content = std::fs::read_to_string(path).map_err(|e| NexusCoreError::FileRead {
        path: path.display().to_string(),
        source: e,
    })?;
    parse_nxs(&content).map_err(|e| match e {
        NexusCoreError::SchemaParse { message, .. } => NexusCoreError::SchemaParse {
            path: path.display().to_string(),
            message,
        },
        other => other,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_struct() {
        let input = "struct Foo {\n    x : u32\n    y : f64\n}\n";
        let structs = parse_nxs(input).expect("parse should succeed");
        assert_eq!(structs.len(), 1);
        assert_eq!(structs[0].name, "Foo");
        assert_eq!(structs[0].fields.len(), 2);
        assert_eq!(structs[0].fields[0].name, "x");
        assert_eq!(structs[0].fields[0].typ, FieldType::U32);
        assert_eq!(structs[0].fields[1].name, "y");
        assert_eq!(structs[0].fields[1].typ, FieldType::F64);
    }

    #[test]
    fn test_parse_nested_struct() {
        let input = "struct Particle {\n    pos : Vec2\n    mass : f32\n}\n";
        let structs = parse_nxs(input).expect("parse should succeed");
        assert_eq!(
            structs[0].fields[0].typ,
            FieldType::Nested("Vec2".to_string())
        );
        assert_eq!(structs[0].fields[1].typ, FieldType::F32);
    }

    #[test]
    fn test_parse_array_field() {
        let input = "struct Buf {\n    data : [u8; 8]\n}\n";
        let structs = parse_nxs(input).expect("parse should succeed");
        assert_eq!(
            structs[0].fields[0].typ,
            FieldType::Array {
                elem: Box::new(FieldType::U8),
                len: 8,
            }
        );
    }

    #[test]
    fn test_parse_bytes_type() {
        let input = "struct Msg {\n    payload : bytes(16)\n}\n";
        let structs = parse_nxs(input).expect("parse should succeed");
        assert_eq!(structs[0].fields[0].typ, FieldType::Bytes(16));
    }

    #[test]
    fn test_parse_string_type() {
        let input = "struct Msg {\n    label : string(32)\n}\n";
        let structs = parse_nxs(input).expect("parse should succeed");
        assert_eq!(structs[0].fields[0].typ, FieldType::StringFixed(32));
    }

    #[test]
    fn test_parse_multiple_structs() {
        let input = concat!(
            "struct Vec2 {\n    x : f32\n    y : f32\n}\n\n",
            "struct Particle {\n    pos : Vec2\n    mass : f32\n}\n"
        );
        let structs = parse_nxs(input).expect("parse should succeed");
        assert_eq!(structs.len(), 2);
        assert_eq!(structs[0].name, "Vec2");
        assert_eq!(structs[1].name, "Particle");
    }

    #[test]
    fn test_parse_with_comments() {
        let input = concat!(
            "# top-level comment\n",
            "struct Foo {\n",
            "    x : u32   # inline comment\n",
            "    y : f32\n",
            "}\n"
        );
        let structs = parse_nxs(input).expect("parse should succeed");
        assert_eq!(structs[0].fields.len(), 2);
    }

    #[test]
    fn test_parse_error() {
        let input = "this is not valid nxs syntax !!!";
        let result = parse_nxs(input);
        assert!(result.is_err());
    }
}
