#[cfg(test)]
mod tests {
    use mongodb::bson::bson;
    use mongor::query_param_parser::{LexItem, Lexer, Parser, Value};

    #[test]
    fn test_read_number() {
        let test_cases = [
            ("123", 123.0),
            ("42.5", 42.5),
            ("0.123", 0.123),
            ("123rest", 123.0), // Should read only the number
            (".123", 0.123),
            ("-.123", -0.123),
            ("-5.123", -5.123),
        ];

        // println!("Parsed: {}", ".123".parse::<f64>().expect("Should work"));
        for (input, expected) in test_cases {
            let mut lexer = Lexer::new(input);
            println!("Lexer input: {}", input);
            assert_eq!(lexer.read_number(), expected);
        }
    }

    #[test]
    fn test_tokenize_empty() {
        let mut lexer = Lexer::new("");
        assert_eq!(lexer.tokenize(), vec![]);
    }

    #[test]
    fn test_tokenize_punctuation() {
        let mut lexer = Lexer::new(".,()");
        assert_eq!(
            lexer.tokenize(),
            vec![
                LexItem::SpecialChar('.'),
                LexItem::SpecialChar(','),
                LexItem::SpecialChar('('),
                LexItem::SpecialChar(')'),
            ]
        );
    }

    #[test]
    fn test_tokenize_values() {
        let test_cases = [
            (
                "eq.2.05",
                vec![
                    LexItem::ComparisonOperator("eq".to_string()),
                    LexItem::SpecialChar('.'),
                    LexItem::Symbol(Value::Num(2.05)),
                ],
            ),
            (
                "\"a.b.c\".2.05,\"bac25.34\".eq.\"hello\"",
                vec![
                    LexItem::Symbol(Value::Str("a.b.c".to_string())),
                    LexItem::SpecialChar('.'),
                    LexItem::Symbol(Value::Num(2.05)),
                    LexItem::SpecialChar(','),
                    LexItem::Symbol(Value::Str("bac25.34".to_string())),
                    LexItem::SpecialChar('.'),
                    LexItem::ComparisonOperator("eq".to_string()),
                    LexItem::SpecialChar('.'),
                    LexItem::Symbol(Value::Str("hello".to_string())),
                ],
            ),
            (
                "(field1.eq.\"test\",field2.lt.24.5,field3.gte.-2,field4.eq.hello)",
                vec![
                    LexItem::SpecialChar('('),
                    LexItem::Symbol(Value::Str("field1".to_string())),
                    LexItem::SpecialChar('.'),
                    LexItem::ComparisonOperator("eq".to_string()),
                    LexItem::SpecialChar('.'),
                    LexItem::Symbol(Value::Str("test".to_string())),
                    LexItem::SpecialChar(','),
                    LexItem::Symbol(Value::Str("field2".to_string())),
                    LexItem::SpecialChar('.'),
                    LexItem::ComparisonOperator("lt".to_string()),
                    LexItem::SpecialChar('.'),
                    LexItem::Symbol(Value::Num(24.5)),
                    LexItem::SpecialChar(','),
                    LexItem::Symbol(Value::Str("field3".to_string())),
                    LexItem::SpecialChar('.'),
                    LexItem::ComparisonOperator("gte".to_string()),
                    LexItem::SpecialChar('.'),
                    LexItem::Symbol(Value::Num(-2.0)),
                    LexItem::SpecialChar(','),
                    LexItem::Symbol(Value::Str("field4".to_string())),
                    LexItem::SpecialChar('.'),
                    LexItem::ComparisonOperator("eq".to_string()),
                    LexItem::SpecialChar('.'),
                    LexItem::Symbol(Value::Str("hello".to_string())),
                    LexItem::SpecialChar(')'),
                ],
            ),
        ];
        for (input, expected) in test_cases {
            let mut lexer = Lexer::new(input);
            assert_eq!(lexer.tokenize(), expected);
        }
    }

    #[test]
    fn test_simple_top_level_expr() {
        let input_key = "test".as_ref();

        let test_cases = [
            (
                "eq.2.05",
                bson!({
                    input_key: {"$eq": 2.05}
                }),
            ),
            (
                "lt.test",
                bson!({
                    input_key: {
                        "$lt": "test"
                    }
                }),
            ),
        ];
        for (input, expected) in test_cases {
            let mut lexer = Lexer::new(input);
            let mut parser = Parser::new(lexer.tokenize());
            assert_eq!(parser.parse(input_key).expect("Not good"), bson!(expected));
        }
    }

    #[test]
    fn test_arr_top_level_expr() {
        let test_cases = [
            (
                "(field1.eq.\"test\",field2.lt.24.5,field3.gte.-2,field4.eq.hello)",
                bson!({"$or": [
                    {
                        "field1": {
                            "$eq": "test"
                        }
                    },
                    {
                        "field2": {
                            "$lt": 24.5
                        }
                    },
                    {
                        "field3": {
                            "$gte": -2.0
                        }
                    },
                    {
                        "field4": {
                            "$eq": "hello"
                        }
                    }
                ]}),
            ),
            (
                "(field1.eq.\"test\",or=(field2.lt.24,field3.gte.-2,field4.eq.hello))",
                bson!({"$or": [
                    {
                        "field1": {
                            "$eq": "test"
                        }
                    },
                    {"$or": [
                        {
                            "field2": {
                                "$lt": 24.0
                            }
                        },
                        {
                            "field3": {
                                "$gte": -2.0
                            }
                        },
                        {
                            "field4": {
                                "$eq": "hello"
                            }
                        }
                    ]}
                ]}),
            ),
        ];

        for (input, expected) in test_cases {
            let mut lexer = Lexer::new(input);
            let mut parser = Parser::new(lexer.tokenize());
            assert_eq!(parser.parse("or").unwrap(), bson!(expected));
        }
    }
}
