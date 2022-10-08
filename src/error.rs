use thiserror::Error;

use crate::parser;

#[derive(Error, Debug)]
pub enum DictCliError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Expected a directory, but is not. Path: {0}")]
    NotDirectory(String),
    #[error("The dictionary has already been imported. Use --force to overwrite it.")]
    AlreadyImported,
    #[error("No data directory could be found.")]
    NoDataDirectory,
    #[error("No language pair found in dict.cc file.")]
    NoLanguagePair,
    #[error("Invalid language pair in dict.cc file.")]
    InvalidLanguagePair,
    #[error("Source language {0} not available. Available are: {1}")]
    SearchLanguageNotAvailable(String, String),
    #[error("Parse error: {0}")]
    ParseError(#[from] pest::error::Error<parser::Rule>),
    #[error("Database error: {0}")]
    TantivyError(#[from] tantivy::TantivyError),
    #[error("CSV error: {0}")]
    CsvError(#[from] csv::Error),
}
