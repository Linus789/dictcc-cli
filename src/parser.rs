extern crate pest;

use pest::iterators::Pairs;
use pest::Parser;

use crate::error::DictCliError;

#[derive(Parser)]
#[grammar = "entry.pest"]
struct LangEntryParser;

pub(crate) fn parse_entry(entry: &str) -> Result<Pairs<'_, Rule>, DictCliError> {
    Ok(LangEntryParser::parse(Rule::expr, entry)?)
}
