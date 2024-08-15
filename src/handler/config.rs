use kapacitor_udf::proto::{Option as ProtoOption, OptionInfo, OptionValue, ValueType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum IndicatorOptionError {
    #[error("Invalid type for '{0}' option")]
    InvalidOptionType(String),
    #[error("Missing value for '{0}' option")]
    MissingOptionValue(String),
    #[error("Unknown option: {0}")]
    UnknownOption(String),
    #[error("Invalid indicator type: {0}")]
    InvalidIndicatorType(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IndicatorType {
    EMA,
    SMA,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndicatorOptions {
    pub indicator_type: IndicatorType,
    pub period: u32,
    pub field: String,
    pub as_field: String,
    pub ticker_field: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndicatorState {
    pub current_value: Option<f64>,
    pub values: Vec<f64>,
    pub count: u32,
}

impl IndicatorOptions {
    pub fn from_proto_options(options: &[ProtoOption]) -> Result<Self, IndicatorOptionError> {
        let mut indicator_options = IndicatorOptions::default();

        for option in options {
            let value = option
                .values
                .first()
                .ok_or_else(|| IndicatorOptionError::MissingOptionValue(option.name.clone()))?;

            match option.name.as_str() {
                "type" => {
                    if let Some(kapacitor_udf::proto::option_value::Value::StringValue(ref v)) =
                        value.value
                    {
                        indicator_options.indicator_type = match v.to_uppercase().as_str() {
                            "EMA" => IndicatorType::EMA,
                            "SMA" => IndicatorType::SMA,
                            _ => return Err(IndicatorOptionError::InvalidIndicatorType(v.clone())),
                        };
                    } else {
                        return Err(IndicatorOptionError::InvalidOptionType("type".to_string()));
                    }
                }
                "period" => {
                    if let Some(kapacitor_udf::proto::option_value::Value::IntValue(v)) =
                        value.value
                    {
                        indicator_options.period = v as u32;
                    } else {
                        return Err(IndicatorOptionError::InvalidOptionType(
                            "period".to_string(),
                        ));
                    }
                }
                "field" => {
                    if let Some(kapacitor_udf::proto::option_value::Value::StringValue(ref v)) =
                        value.value
                    {
                        indicator_options.field = v.clone();
                    } else {
                        return Err(IndicatorOptionError::InvalidOptionType("field".to_string()));
                    }
                }
                "as" => {
                    if let Some(kapacitor_udf::proto::option_value::Value::StringValue(ref v)) =
                        value.value
                    {
                        indicator_options.as_field = v.clone();
                    } else {
                        return Err(IndicatorOptionError::InvalidOptionType("as".to_string()));
                    }
                }
                "ticker_field" => {
                    if let Some(kapacitor_udf::proto::option_value::Value::StringValue(ref v)) =
                        value.value
                    {
                        indicator_options.ticker_field = v.clone();
                    } else {
                        return Err(IndicatorOptionError::InvalidOptionType(
                            "ticker_field".to_string(),
                        ));
                    }
                }
                _ => {
                    return Err(IndicatorOptionError::UnknownOption(option.name.clone()));
                }
            }
        }

        Ok(indicator_options)
    }

    pub fn to_option_info(&self) -> HashMap<String, OptionInfo> {
        let mut options = HashMap::new();

        options.insert(
            "type".to_string(),
            OptionInfo {
                value_types: vec![ValueType::String as i32],
            },
        );
        options.insert(
            "period".to_string(),
            OptionInfo {
                value_types: vec![ValueType::Int as i32],
            },
        );
        options.insert(
            "field".to_string(),
            OptionInfo {
                value_types: vec![ValueType::String as i32],
            },
        );
        options.insert(
            "as".to_string(),
            OptionInfo {
                value_types: vec![ValueType::String as i32],
            },
        );
        options.insert(
            "ticker_field".to_string(),
            OptionInfo {
                value_types: vec![ValueType::String as i32],
            },
        );

        options
    }

    pub fn to_proto_options(&self) -> Vec<ProtoOption> {
        vec![
            ProtoOption {
                name: "type".to_string(),
                values: vec![OptionValue {
                    r#type: ValueType::String as i32,
                    value: Some(kapacitor_udf::proto::option_value::Value::StringValue(
                        format!("{:?}", self.indicator_type),
                    )),
                }],
            },
            ProtoOption {
                name: "period".to_string(),
                values: vec![OptionValue {
                    r#type: ValueType::Int as i32,
                    value: Some(kapacitor_udf::proto::option_value::Value::IntValue(
                        self.period as i64,
                    )),
                }],
            },
            ProtoOption {
                name: "field".to_string(),
                values: vec![OptionValue {
                    r#type: ValueType::String as i32,
                    value: Some(kapacitor_udf::proto::option_value::Value::StringValue(
                        self.field.clone(),
                    )),
                }],
            },
            ProtoOption {
                name: "as".to_string(),
                values: vec![OptionValue {
                    r#type: ValueType::String as i32,
                    value: Some(kapacitor_udf::proto::option_value::Value::StringValue(
                        self.as_field.clone(),
                    )),
                }],
            },
            ProtoOption {
                name: "ticker_field".to_string(),
                values: vec![OptionValue {
                    r#type: ValueType::String as i32,
                    value: Some(kapacitor_udf::proto::option_value::Value::StringValue(
                        self.ticker_field.clone(),
                    )),
                }],
            },
        ]
    }
}

impl Default for IndicatorOptions {
    fn default() -> Self {
        Self {
            indicator_type: IndicatorType::EMA,
            period: 14,
            field: "value".to_string(),
            as_field: "indicator".to_string(),
            ticker_field: "ticker".to_string(),
        }
    }
}
