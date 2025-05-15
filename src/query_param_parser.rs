use mongodb::bson::{Bson, Document, bson, doc};
use std::collections::HashMap;

type Number = f64;

#[derive(Debug, Clone)]
pub enum Value {
    Str(String),
    Num(Number),
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Num(a), Value::Num(b)) => a == b,
            _ => false,
        }
    }
}
impl Eq for Value {}

#[derive(Debug, Clone)]
pub enum LexItem {
    ComparisonOperator(String), // 'eq', 'ne', 'lt', 'gt', 'lte', 'gte'
    SpecialChar(char),          // Specoal characters like `(` `)` `,` `.`
    ArrayOp(String),            // "and", "or"
    Symbol(Value),              // Number, String
}

pub struct Lexer {
    input: Vec<char>,
    position: usize,
}

impl PartialEq for LexItem {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (LexItem::ComparisonOperator(a), LexItem::ComparisonOperator(b)) => a == b,
            (LexItem::SpecialChar(a), LexItem::SpecialChar(b)) => a == b,
            (LexItem::Symbol(a), LexItem::Symbol(b)) => a == b,
            _ => false,
        }
    }
}
impl Eq for LexItem {}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Lexer {
            input: input.chars().collect(),
            position: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        if self.position < self.input.len() {
            Some(self.input[self.position])
        } else {
            None
        }
    }

    fn next_char(&mut self) -> Option<char> {
        let result = self.peek();
        self.position += 1;
        result
    }

    fn read_symbol(&mut self) -> String {
        let mut result = String::new();

        while let Some(c) = self.peek() {
            if c.is_alphanumeric() {
                result.push(self.next_char().unwrap());
            } else {
                break;
            }
        }

        result
    }

    fn read_string(&mut self) -> String {
        // Skip the opening quote
        self.next_char();

        let mut result = String::new();

        while let Some(c) = self.peek() {
            if c == '"' {
                // Skip the closing quote
                self.next_char();
                break;
            }
            result.push(self.next_char().unwrap());
        }

        result
    }

    pub fn read_number(&mut self) -> f64 {
        let mut result = String::new();
        let mut has_dot = false;

        match self.peek() {
            Some(c) if c == '-' => {
                result.push(self.next_char().unwrap());
            }
            _ => {}
        }

        while let Some(c) = self.peek() {
            match c {
                '0'..='9' => {
                    result.push(self.next_char().unwrap());
                }
                '.' if !has_dot => {
                    result.push(self.next_char().unwrap());
                    has_dot = true;
                }
                _ => break,
            }
        }

        result.parse().unwrap_or(0.0)
    }

    fn next_token(&mut self) -> Option<LexItem> {
        self.peek().map(|c| {
            match c {
                '(' | ')' | ',' | '.' | '=' => LexItem::SpecialChar(self.next_char().unwrap()),
                '"' => LexItem::Symbol(Value::Str(self.read_string())),
                '0'..='9' | '-' => LexItem::Symbol(Value::Num(self.read_number())),
                _ => {
                    let ident = self.read_symbol();
                    match ident.as_str() {
                        // You would add other operators here
                        "eq" | "lt" | "gt" | "lte" | "gte" => LexItem::ComparisonOperator(ident),
                        "and" | "or" => LexItem::ArrayOp(ident),
                        _ => LexItem::Symbol(Value::Str(ident)),
                    }
                }
            }
        })
    }

    pub fn tokenize(&mut self) -> Vec<LexItem> {
        let mut tokens = Vec::new();
        while let Some(token) = self.next_token() {
            tokens.push(token);
        }
        tokens
    }
}

pub struct Parser {
    tokens: Vec<LexItem>,
    position: usize,
}

impl Parser {
    pub fn new(tokens: Vec<LexItem>) -> Self {
        Parser {
            tokens,
            position: 0,
        }
    }

    fn return_error_msg(&self) -> String {
        format!(
            "Unexpected token at position {:?} | {:?}",
            self.position, self.tokens
        )
    }

    fn peek(&self) -> Option<&LexItem> {
        if self.position < self.tokens.len() {
            Some(&self.tokens[self.position])
        } else {
            None
        }
    }

    fn advance(&mut self) -> Option<LexItem> {
        let token = self.peek().cloned();
        self.position += 1;
        token
    }

    fn comparison_op_to_bson_key(operator: &str) -> Result<String, String> {
        match operator {
            "eq" => Ok("$eq".to_string()),
            "lt" => Ok("$lt".to_string()),
            "gt" => Ok("$gt".to_string()),
            "lte" => Ok("$lte".to_string()),
            "gte" => Ok("$gte".to_string()),
            _ => Err(format!("Unknown operator: {}", operator)),
        }
    }

    fn logical_op_to_bson_key(operator: &str) -> Result<String, String> {
        match operator {
            "and" => Ok("$and".to_string()),
            "or" => Ok("$or".to_string()),
            _ => Err(format!("Unknown logical operator: {}", operator)),
        }
    }

    fn parse_inner_filter(&mut self) -> Result<Bson, String> {
        let first_token = self.advance();
        let second_token = self.advance();
        match (first_token, second_token) {
            (Some(LexItem::Symbol(Value::Str(field_name))), Some(LexItem::SpecialChar('.'))) => {
                match self.advance() {
                    // Case Field.ComparisonOp.Value
                    Some(LexItem::ComparisonOperator(operator)) => {
                        let bson_key = Parser::comparison_op_to_bson_key(operator.as_str())?;
                        match (self.advance(), self.advance()) {
                            (Some(LexItem::SpecialChar('.')), Some(LexItem::Symbol(value))) => {
                                let bson_value = match value {
                                    Value::Str(s) => Bson::String(s),
                                    Value::Num(n) => Bson::Double(n),
                                };
                                Ok(bson!({ field_name: { bson_key: bson_value }}))
                            }
                            _ => Err(self.return_error_msg()),
                        }
                    }
                    // Case Field.Value
                    Some(LexItem::Symbol(val)) => {
                        let bson_value = match val {
                            Value::Num(n) => Bson::Double(n),
                            Value::Str(s) => Bson::String(s),
                        };
                        Ok(bson!({ field_name: bson_value}))
                    }
                    _ => Err(self.return_error_msg()),
                }
            }
            (Some(LexItem::ArrayOp(field_name)), Some(LexItem::SpecialChar('='))) => {
                let bson_key = Parser::logical_op_to_bson_key(field_name.as_str())?;
                Ok(bson!({bson_key: self.parse_inner_filters()?}))
            }
            _ => Err(self.return_error_msg()),
        }
    }

    fn parse_inner_filters(&mut self) -> Result<Bson, String> {
        if let Some(LexItem::SpecialChar('(')) = self.peek() {
            self.advance();
            let mut filters = Vec::new();
            while let Some(token) = self.peek() {
                match token {
                    LexItem::SpecialChar(')') => {
                        self.advance();
                        break;
                    }
                    LexItem::SpecialChar(',') => {
                        self.advance();
                        continue;
                    }
                    _ => {
                        if let Ok(obj) = self.parse_inner_filter() {
                            filters.push(obj);
                        } else {
                            return Err(self.return_error_msg());
                        }
                    }
                }
            }
            return Ok(bson!(filters));
        }
        return Err(self.return_error_msg());
    }

    fn parse_top_level_expr(&mut self, key: &str) -> Result<Bson, String> {
        match key {
            // Case TopLevelExpr -> (InnerFilters)
            "and" | "or" => {
                let bson_key = Parser::logical_op_to_bson_key(key)?;
                self.parse_inner_filters()
                    .map(|inner_bson| bson!({bson_key: inner_bson}))
            }
            _ => match self.peek().cloned() {
                // Case TopLevelExpr -> Field=Value
                Some(LexItem::Symbol(val)) => {
                    self.advance();
                    let bson_value = match val {
                        Value::Str(s) => Bson::String(s.clone()),
                        Value::Num(n) => Bson::Double(n),
                    };
                    return Ok(bson!({key: bson_value}));
                }
                // Case TopLevelExpr -> Field=ComparisonOp.Value
                Some(LexItem::ComparisonOperator(_)) => {
                    match (self.advance(), self.advance(), self.advance()) {
                        (
                            Some(LexItem::ComparisonOperator(op)),
                            Some(LexItem::SpecialChar('.')),
                            Some(LexItem::Symbol(value)),
                        ) => {
                            let mql_comparison_op = Parser::comparison_op_to_bson_key(op.as_str())?;
                            let bson_value = match value {
                                Value::Str(s) => Bson::String(s.clone()),
                                Value::Num(n) => Bson::Double(n),
                            };
                            Ok(bson!({ key: {mql_comparison_op: bson_value} }))
                        }
                        _ => Err(self.return_error_msg()),
                    }
                }
                _ => Err(self.return_error_msg()),
            },
        }
    }

    pub fn parse(&mut self, key: &str) -> Result<Bson, String> {
        let result = self.parse_top_level_expr(key)?;
        match self.peek() {
            Some(_) => Err(self.return_error_msg()),
            None => Ok(result),
        }
    }
}

pub fn parse(key: &str, value: &str) -> Result<Bson, String> {
    let mut lexer = Lexer::new(value);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    parser.parse(key)
}

/// Parses query parameters from a URL query string into a MongoDB filter document.
///
/// This function supports two modes of operation:
/// 1. Simple mode: query parameters in the format "field_name=value"
///    It will attempt to parse numeric values as numbers, otherwise they will be treated as strings.
/// 2. Advanced mode: query parameters with comparison operators and logical operators
///    For example: "field.eq.value" or "or=(field1.eq.value1,field2.gt.10)"
///
/// # Arguments
///
/// * `query_params` - A HashMap containing the query parameters from the URL
///
/// # Returns
///
/// * `Result<Document, String>` - A MongoDB filter document or an error message
#[allow(dead_code)]
pub fn parse_query_params(query_params: &HashMap<String, String>) -> Result<Document, String> {
    let mut filter = doc! {};

    for (field_name, field_value) in query_params.iter() {
        match parse(field_name, field_value) {
            Ok(Bson::Document(doc)) => filter.extend(doc),
            Ok(val) => return Err(format!("Unexpected bson: {}", val)),
            Err(err) => return Err(err),
        }
    }

    Ok(filter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mongodb::bson::doc;

    #[test]
    fn test_parse_query_params_empty() {
        let query_params = HashMap::new();
        let result = parse_query_params(&query_params);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), doc! {});
    }

    #[test]
    fn test_parse_query_params_string_value() {
        let mut query_params = HashMap::new();
        query_params.insert("name".to_string(), "john".to_string());

        let result = parse_query_params(&query_params);
        assert!(result.is_ok());

        let filter = result.unwrap();
        assert_eq!(filter.get_str("name").unwrap(), "john");
    }

    #[test]
    fn test_parse_query_params_numeric_value() {
        let mut query_params = HashMap::new();
        query_params.insert("age".to_string(), "30".to_string());

        let result = parse_query_params(&query_params);
        assert!(result.is_ok());

        let filter = result.unwrap();
        assert_eq!(filter, doc! {"age": 30.0});
    }

    #[test]
    fn test_parse_query_params_multiple_fields() {
        let mut query_params = HashMap::new();
        query_params.insert("name".to_string(), "john".to_string());
        query_params.insert("age".to_string(), "30".to_string());

        let result = parse_query_params(&query_params);
        assert!(
            result.is_ok(),
            "Failed to parse query params: {:?}",
            result.err()
        );

        let filter = result.unwrap();
        assert_eq!(filter, doc! {"name": "john", "age": 30.0});
    }

    #[test]
    fn test_parse_query_params_advanced_comparison() {
        let mut query_params = HashMap::new();
        query_params.insert("age".to_string(), "gt.25".to_string());

        let result = parse_query_params(&query_params);
        assert!(result.is_ok());

        let filter = result.unwrap();
        assert!(filter.contains_key("age"));
        let age_doc = filter.get_document("age").unwrap();
        assert!(age_doc.contains_key("$gt"));
        assert_eq!(age_doc.get_f64("$gt").unwrap(), 25.0);
    }

    #[test]
    fn test_parse_query_params_advanced_logical() {
        let mut query_params = HashMap::new();
        query_params.insert("or".to_string(), "(age.gt.25,name.eq.john)".to_string());

        let result = parse_query_params(&query_params);
        assert!(result.is_ok());

        let filter = result.unwrap();
        assert!(filter.contains_key("or") || filter.contains_key("$or"));
    }
}
