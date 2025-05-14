use mongodb::bson::{Document, doc};
use std::collections::HashMap;

/// Parses query parameters from a URL query string into a MongoDB filter document.
///
/// The function expects query parameters in the format "field_name=value".
/// It will attempt to parse numeric values as i32, otherwise they will be treated as strings.
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
        // Try to parse the value as a number if possible
        if let Ok(num) = field_value.parse::<i32>() {
            filter.insert(field_name, num);
        } else {
            filter.insert(field_name, field_value.to_string());
        }
    }

    Ok(filter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

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
}
