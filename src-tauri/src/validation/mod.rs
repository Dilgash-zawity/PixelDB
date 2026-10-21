//! Input validation: identifiers, file paths, SQL fragments, values and
//! table definitions. Nothing user-provided reaches SQL text without passing
//! through this module.

pub mod expressions;
pub mod identifiers;
pub mod paths;
pub mod table;
pub mod values;

pub use identifiers::{quote_ident, quote_literal, validate_identifier};
