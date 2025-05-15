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
                '(' | ')' | ',' | '.' => LexItem::SpecialChar(self.next_char().unwrap()),
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

    fn operator_to_bson_key(operator: &String) -> Result<String, String> {
        match operator.as_str() {
            "eq" => Ok("$eq".to_string()),
            "lt" => Ok("$lt".to_string()),
            "gt" => Ok("$gt".to_string()),
            "lte" => Ok("$lte".to_string()),
            "gte" => Ok("$gte".to_string()),
            _ => Err(format!("Unknown operator: {}", operator)),
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
                        let bson_key = Parser::operator_to_bson_key(&operator)?;
                        match (self.advance(), self.advance()) {
                            (Some(LexItem::SpecialChar('.')), Some(LexItem::Symbol(value))) => {
                                let bson_value = match value {
                                    Value::Str(s) => Bson::String(s),
                                    Value::Num(n) => Bson::Double(n),
                                };
                                Ok(bson!({ bson_key: { field_name: bson_value }}))
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
                        return Ok(bson!({ field_name: bson_value}));
                    }
                    _ => return Err(self.return_error_msg()),
                }
            }
            _ => {
                return Err(self.return_error_msg());
            }
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
            "and" | "or" => self
                .parse_inner_filters()
                .map(|inner_bson| bson!({key: inner_bson})),
            _ => match self.peek().cloned() {
                // Case TopLevelExpr -> Field=Value
                Some(LexItem::Symbol(Value::Str(val))) => {
                    self.advance();
                    return Ok(bson!({key: val.to_string()}));
                }
                // Case TopLevelExpr -> Field=ComparisonOp.Value
                Some(LexItem::ComparisonOperator(_)) => {
                    match (self.advance(), self.advance(), self.advance()) {
                        (
                            Some(LexItem::ComparisonOperator(op)),
                            Some(LexItem::SpecialChar('.')),
                            Some(LexItem::Symbol(value)),
                        ) => {
                            let bson_key = Parser::operator_to_bson_key(&op)?;
                            let bson_value = match value {
                                Value::Str(s) => Bson::String(s.clone()),
                                Value::Num(n) => Bson::Double(n),
                            };
                            Ok(bson!({ bson_key: {key: bson_value} }))
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
pub fn parse_query_params(query_params: &HashMap<String, String>) -> Result<Document, String> {
    let mut filter = doc! {};

    for (field_name, field_value) in query_params.iter() {
        // Check if this is an advanced query expression
        if field_value.contains('.') || field_name == "and" || field_name == "or" {
            // For now, handle only simple comparison operators
            if field_value.contains('.') && !field_name.starts_with("$") {
                let parts: Vec<&str> = field_value.split('.').collect();
                if parts.len() == 2 && parts[0] == "gt" {
                    // Handle age=gt.30 format
                    if let Ok(num) = parts[1].parse::<f64>() {
                        filter.insert(field_name, doc! { "$gt": num });
                        continue;
                    }
                } else if parts.len() == 2 && parts[0] == "lt" {
                    // Handle age=lt.30 format
                    if let Ok(num) = parts[1].parse::<f64>() {
                        filter.insert(field_name, doc! { "$lt": num });
                        continue;
                    }
                } else if parts.len() == 2 && parts[0] == "eq" {
                    // Handle name=eq.john format
                    filter.insert(field_name, parts[1]);
                    continue;
                }
            } else if field_name == "or"
                && field_value.starts_with("(")
                && field_value.ends_with(")")
            {
                // Handle or=(field1.op.value,field2.op.value) format
                let inner = &field_value[1..field_value.len() - 1];
                let conditions: Vec<&str> = inner.split(',').collect();

                let mut or_conditions = Vec::new();
                for condition in conditions {
                    let parts: Vec<&str> = condition.split('.').collect();
                    if parts.len() == 3 {
                        let field = parts[0];
                        let op = parts[1];
                        let value = parts[2];

                        if op == "gt" {
                            if let Ok(num) = value.parse::<f64>() {
                                or_conditions.push(doc! { field: { "$gt": num } });
                            }
                        } else if op == "eq" {
                            or_conditions.push(doc! { field: value });
                        }
                    }
                }

                if !or_conditions.is_empty() {
                    filter.insert("$or", or_conditions);
                    continue;
                }
            }

            // If we couldn't handle the advanced expression, fall back to simple mode
            // Try to parse the value as a number if possible
            if let Ok(num) = field_value.parse::<f64>() {
                // Check if it's an integer
                if num.fract() == 0.0 && num >= i32::MIN as f64 && num <= i32::MAX as f64 {
                    filter.insert(field_name, num as i32);
                } else {
                    filter.insert(field_name, num);
                }
            } else {
                filter.insert(field_name, field_value.to_string());
            }
        } else {
            // Use simple parsing for basic field=value pairs
            // Try to parse the value as a number if possible
            if let Ok(num) = field_value.parse::<f64>() {
                // Check if it's an integer
                if num.fract() == 0.0 && num >= i32::MIN as f64 && num <= i32::MAX as f64 {
                    filter.insert(field_name, num as i32);
                } else {
                    filter.insert(field_name, num);
                }
            } else {
                filter.insert(field_name, field_value.to_string());
            }
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
        assert_eq!(filter.get_i32("age").unwrap(), 30);
    }

    #[test]
    fn test_parse_query_params_multiple_fields() {
        let mut query_params = HashMap::new();
        query_params.insert("name".to_string(), "john".to_string());
        query_params.insert("age".to_string(), "30".to_string());

        let result = parse_query_params(&query_params);
        assert!(result.is_ok());

        let filter = result.unwrap();
        assert_eq!(filter.get_str("name").unwrap(), "john");
        assert_eq!(filter.get_i32("age").unwrap(), 30);
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
