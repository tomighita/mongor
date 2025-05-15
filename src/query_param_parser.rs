use mongodb::bson::{Bson, bson};

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

const COMP_OPS: [&str; 5] = ["eq", "lt", "gt", "lte", "gte"];

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

    fn parse_q_value_list(&mut self) -> Result<Vec<Bson>, String> {
        let mut q_values = Vec::new();

        while let Some(token) = self.advance() {
            match token {
                LexItem::SpecialChar(')') => break,
                LexItem::SpecialChar(',') => continue,
                LexItem::Symbol(Value::Str(fieldname)) => {
                    if let Some(LexItem::SpecialChar('.')) = self.advance() {
                        let bson_key = fieldname;
                        let bson_value = self.parse_q_value()?;
                        q_values.push(bson!({ bson_key: bson_value }));
                    }
                }
                _ => {
                    return Err(self.return_error_msg());
                }
            }
        }

        Ok(q_values)
    }

    fn parse_q_value(&mut self) -> Result<Bson, String> {
        match self.peek() {
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
                        return Ok(bson!({ bson_key: bson_value }));
                    }
                    _ => {
                        return Err(self.return_error_msg());
                    }
                }
            }
            Some(LexItem::SpecialChar('(')) => {
                self.advance();
                let bson_elements = self.parse_q_value_list()?;
                Ok(bson!(bson_elements))
            }
            _ => Err(format!(
                "Unexpected token at position {:?} | {:?}",
                self.position, self.tokens
            )),
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
