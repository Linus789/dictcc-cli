#[macro_use]
extern crate pest_derive;

mod cli;
mod database;
mod error;
mod parser;

use std::cmp::Reverse;
use std::collections::HashMap;

use cli::Settings;
use comfy_table::presets::{ASCII_FULL, UTF8_FULL};
use comfy_table::{ContentArrangement, Table};
use database::DatabaseSearch;
use error::DictCliError;
use rustyline::completion::Completer;
use rustyline::config::BellStyle;
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Config, Editor, Helper};
use tantivy::schema::Field;
use tantivy::Document;
use unicode_normalization::UnicodeNormalization;

fn main() -> Result<(), DictCliError> {
    match cli::parse_settings()? {
        Settings::Import { file, force } => {
            database::import_dictcc_file(file, force)?;
        }
        Settings::Delete { language_pair } => {
            database::remove_database(&language_pair)?;
        }
        Settings::Translate {
            language_pair,
            language_from,
            fuzzy_distance,
            limit_results,
            minimum_similarity,
            completion_type,
            ascii,
            search,
        } => {
            let db_search = database::DatabaseSearch::new(&language_pair)?;
            let reverse_langs = db_search.is_reverse_langs(&language_from)?;
            let source_lang_upper = language_from.to_uppercase();
            let target_lang_upper = db_search.target_language(&language_from)?.to_uppercase();

            let (source_field, target_field) = if !reverse_langs {
                (&db_search.schema.lang_left, &db_search.schema.lang_right)
            } else {
                (&db_search.schema.lang_right, &db_search.schema.lang_left)
            };

            let search_translations = SearchTranslations {
                db_search: &db_search,
                source_field,
                target_field,
                reverse_langs,
                fuzzy_distance,
                limit_results,
                minimum_similarity,
                ascii,
                source_lang_upper,
                target_lang_upper,
            };

            if let Some(search) = search {
                search_translations.print_results(&search);
                return Ok(());
            }

            let mut readline_editor = Editor::<TabCompletion>::with_config(
                Config::builder()
                    .completion_type(completion_type)
                    .bell_style(BellStyle::None)
                    .tab_stop(4)
                    .indent_size(4)
                    .build(),
            )
            .unwrap();
            readline_editor.set_helper(Some(TabCompletion {
                db_search: &db_search,
                reverse_langs,
            }));

            loop {
                let readline = readline_editor.readline("> ");

                match readline {
                    Ok(line) => {
                        readline_editor.add_history_entry(&line);
                        search_translations.print_results(&line);
                    }
                    Err(ReadlineError::Interrupted) => {
                        continue;
                    }
                    Err(ReadlineError::Eof) => {
                        break;
                    }
                    Err(err) => {
                        eprintln!("Readline error: {}", err);
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}

struct SearchTranslations<'a> {
    db_search: &'a DatabaseSearch,
    source_field: &'a Field,
    target_field: &'a Field,
    reverse_langs: bool,
    fuzzy_distance: u8,
    limit_results: Option<u32>,
    minimum_similarity: Option<u16>,
    ascii: bool,
    source_lang_upper: String,
    target_lang_upper: String,
}

impl SearchTranslations<'_> {
    fn print_results(&self, line: &str) {
        let results = self
            .db_search
            .search_database(self.reverse_langs, line, self.fuzzy_distance);

        match results {
            Ok(documents) => {
                let sorted_docs = sort_documents(&documents, self.source_field, line, self.minimum_similarity);

                let mut table = Table::new();
                let mut has_content = false;
                table
                    .load_preset(if self.ascii { ASCII_FULL } else { UTF8_FULL })
                    .set_content_arrangement(ContentArrangement::Dynamic)
                    .set_header(vec![&self.source_lang_upper, &self.target_lang_upper]);

                let iter_fn = |field_map: HashMap<Field, &str>| {
                    table.add_row(vec![field_map[self.source_field], field_map[self.target_field]]);
                    has_content = true;
                };

                if let Some(limit) = &self.limit_results {
                    sorted_docs.into_iter().take(*limit as usize).for_each(iter_fn);
                } else {
                    sorted_docs.into_iter().for_each(iter_fn);
                }

                if has_content {
                    println!("{}", table);
                }
            }
            Err(err) => {
                eprintln!("Search database error: {}", err);
            }
        }
    }
}

fn sort_documents<'a>(
    documents: &'a [Document],
    source_field: &Field,
    actual_input: &str,
    min_similarity: Option<u16>,
) -> Vec<HashMap<Field, &'a str>> {
    let actual_input: String = actual_input.to_lowercase().nfc().collect();

    let mut docs_with_fields: Vec<(HashMap<Field, &str>, u16)> = documents
        .iter()
        .filter_map(|document| {
            let mut field_map: HashMap<Field, &str> = HashMap::new();

            for field in document.field_values() {
                if let Some(text) = field.value().as_text() {
                    field_map.insert(field.field(), text);
                }
            }

            let original_field = field_map.get(source_field).unwrap();
            let norm_result = database::normalized_entry(original_field, false);

            let similarity = (match norm_result {
                Ok(normalized) => strsim::sorensen_dice(
                    &normalized.text.to_lowercase().replace('(', "").replace(')', ""),
                    &actual_input,
                )
                .max(strsim::sorensen_dice(&normalized.extra.to_lowercase(), &actual_input)),
                Err(_) => 0.0,
            } * 1000.0) as u16;

            if let Some(min_similarity) = min_similarity {
                if similarity < min_similarity {
                    return None;
                }
            }

            Some((field_map, similarity))
        })
        .collect();

    docs_with_fields.sort_unstable_by_key(|&(_, similarity)| Reverse(similarity));
    docs_with_fields.into_iter().map(|(fields, _)| fields).collect()
}

struct TabCompletion<'a> {
    db_search: &'a DatabaseSearch,
    reverse_langs: bool,
}
impl Helper for TabCompletion<'_> {}
impl Validator for TabCompletion<'_> {}
impl Highlighter for TabCompletion<'_> {}
impl Hinter for TabCompletion<'_> {
    type Hint = String;
}
impl Completer for TabCompletion<'_> {
    type Candidate = String;

    fn complete(
        &self,
        line: &str,
        _pos: usize,
        _ctx: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        match self.db_search.tab_completions(line, self.reverse_langs) {
            Ok(completions) => {
                let mut completions: Vec<String> = completions.into_iter().collect();
                completions.sort_unstable_by(|completion1, completion2| {
                    completion1
                        .split_whitespace()
                        .count()
                        .cmp(&completion2.split_whitespace().count())
                        .then_with(|| completion1.chars().count().cmp(&completion2.chars().count()))
                });
                Ok((0, completions))
            }
            Err(err) => {
                eprintln!("Tab completion error: {}", err);
                Ok((0, Vec::with_capacity(0)))
            }
        }
    }
}
