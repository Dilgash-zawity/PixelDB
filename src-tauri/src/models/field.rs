use serde::{Deserialize, Serialize};

/// Access-style field types and how they are represented in SQLite.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FieldType {
    /// TEXT, optionally limited by a CHECK on length.
    ShortText {
        #[serde(default)]
        max_length: Option<u32>,
    },
    /// TEXT without a length limit.
    LongText,
    /// INTEGER, or REAL when `real` is set.
    Number {
        #[serde(default)]
        real: bool,
    },
    /// 64-bit INTEGER.
    LargeNumber,
    /// INTEGER holding minor units (e.g. cents for `scale = 2`).
    Currency {
        #[serde(default = "default_currency_scale")]
        scale: u8,
    },
    /// INTEGER PRIMARY KEY AUTOINCREMENT.
    AutoNumber,
    /// TEXT in ISO 8601 format.
    DateTime,
    /// INTEGER constrained to 0/1.
    YesNo,
    /// BLOB.
    Attachment,
    /// TEXT.
    Hyperlink,
    /// Generated column.
    Calculated {
        expression: String,
        #[serde(default)]
        stored: bool,
        #[serde(default)]
        result: CalculatedResult,
    },
    /// Foreign key value; requires a matching single-column foreign key.
    Lookup {
        #[serde(default)]
        value_type: LookupValueType,
        #[serde(default)]
        display_column: Option<String>,
    },
    /// A column from a database not created by this application whose
    /// declared type does not map cleanly onto an Access-style type.
    Unknown {
        #[serde(default)]
        declared: String,
    },
}

fn default_currency_scale() -> u8 {
    2
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalculatedResult {
    #[default]
    Any,
    Text,
    Integer,
    Real,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LookupValueType {
    #[default]
    Integer,
    Text,
}

impl FieldType {
    /// The declared SQLite column type.
    pub fn sql_type(&self) -> String {
        match self {
            FieldType::ShortText { .. }
            | FieldType::LongText
            | FieldType::DateTime
            | FieldType::Hyperlink => "TEXT".into(),
            FieldType::Number { real: true } => "REAL".into(),
            FieldType::Number { real: false }
            | FieldType::LargeNumber
            | FieldType::Currency { .. }
            | FieldType::AutoNumber
            | FieldType::YesNo => "INTEGER".into(),
            FieldType::Attachment => "BLOB".into(),
            FieldType::Calculated { result, .. } => match result {
                CalculatedResult::Any => String::new(),
                CalculatedResult::Text => "TEXT".into(),
                CalculatedResult::Integer => "INTEGER".into(),
                CalculatedResult::Real => "REAL".into(),
            },
            FieldType::Lookup { value_type, .. } => match value_type {
                LookupValueType::Integer => "INTEGER".into(),
                LookupValueType::Text => "TEXT".into(),
            },
            FieldType::Unknown { declared } => declared.clone(),
        }
    }

    pub fn is_calculated(&self) -> bool {
        matches!(self, FieldType::Calculated { .. })
    }

    pub fn is_blob(&self) -> bool {
        matches!(self, FieldType::Attachment)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum DefaultValue {
    /// A JSON value converted with the same rules as record input.
    Literal(serde_json::Value),
    CurrentTimestamp,
    CurrentDate,
    CurrentTime,
    /// A constant SQL expression, rendered inside parentheses.
    Expression(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldDef {
    pub name: String,
    pub field_type: FieldType,
    /// NOT NULL.
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub unique: bool,
    #[serde(default)]
    pub primary_key: bool,
    #[serde(default)]
    pub default: Option<DefaultValue>,
    /// Column-level CHECK expression.
    #[serde(default)]
    pub check: Option<String>,
    /// Stored in application metadata only.
    #[serde(default)]
    pub description: Option<String>,
}

impl FieldDef {
    pub fn new(name: &str, field_type: FieldType) -> Self {
        FieldDef {
            name: name.into(),
            field_type,
            required: false,
            unique: false,
            primary_key: false,
            default: None,
            check: None,
            description: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FkAction {
    #[default]
    NoAction,
    Restrict,
    SetNull,
    SetDefault,
    Cascade,
}

impl FkAction {
    pub fn sql(&self) -> &'static str {
        match self {
            FkAction::NoAction => "NO ACTION",
            FkAction::Restrict => "RESTRICT",
            FkAction::SetNull => "SET NULL",
            FkAction::SetDefault => "SET DEFAULT",
            FkAction::Cascade => "CASCADE",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.to_ascii_uppercase().as_str() {
            "RESTRICT" => FkAction::Restrict,
            "SET NULL" => FkAction::SetNull,
            "SET DEFAULT" => FkAction::SetDefault,
            "CASCADE" => FkAction::Cascade,
            _ => FkAction::NoAction,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForeignKeyDef {
    pub columns: Vec<String>,
    pub ref_table: String,
    /// Empty means "the primary key of `ref_table`".
    #[serde(default)]
    pub ref_columns: Vec<String>,
    #[serde(default)]
    pub on_delete: FkAction,
    #[serde(default)]
    pub on_update: FkAction,
}
