use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::{stdout, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use itertools::Itertools;
use tantivy::collector::DocSetCollector;
use tantivy::query::{BooleanQuery, FuzzyTermQuery, Occur, PhraseQuery, Query, RegexQuery, TermQuery};
use tantivy::schema::{Field, IndexRecordOption, Schema, TextFieldIndexing, TextOptions, STORED, TEXT};
use tantivy::tokenizer::{LowerCaser, RemoveLongFilter, SimpleTokenizer, TextAnalyzer};
use tantivy::{doc, Document, Index, IndexReader, Term};
use unicode_normalization::UnicodeNormalization;

use crate::error::DictCliError;
use crate::parser;

pub(crate) struct DatabaseSchema {
    schema: Schema,
    lowercase_tokenizer: TextAnalyzer,
    key_lang_left: Field,
    key_lang_right: Field,
    extra_lang_left: Field,
    extra_lang_right: Field,
    pub(crate) lang_left: Field,
    pub(crate) lang_right: Field,
    pub(crate) word_classes: Field,
    pub(crate) subject_labels: Field,
}

impl DatabaseSchema {
    fn new(lang_left: &str, lang_right: &str) -> Self {
        let mut schema_builder = Schema::builder();
        let indexing_options = TEXT.set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer("lowercase")
                .set_index_option(IndexRecordOption::WithFreqsAndPositions),
        ) | STORED;
        let store_options = TextOptions::default()
            .set_indexing_options(TextFieldIndexing::default().set_tokenizer("lowercase"))
            | STORED;

        let key_lang_left = schema_builder.add_text_field(&format!("key_{}", lang_left), indexing_options.clone());
        let key_lang_right = schema_builder.add_text_field(&format!("key_{}", lang_right), indexing_options.clone());
        let extra_lang_left = schema_builder.add_text_field(&format!("extra_{}", lang_left), indexing_options.clone());
        let extra_lang_right = schema_builder.add_text_field(&format!("extra_{}", lang_right), indexing_options);
        let lang_left = schema_builder.add_text_field(lang_left, store_options.clone());
        let lang_right = schema_builder.add_text_field(lang_right, store_options.clone());
        let word_classes = schema_builder.add_text_field("word_classes", store_options.clone());
        let subject_labels = schema_builder.add_text_field("subject_labels", store_options);
        let schema = schema_builder.build();

        let lowercase_tokenizer = TextAnalyzer::from(SimpleTokenizer)
            .filter(RemoveLongFilter::limit(tantivy::tokenizer::MAX_TOKEN_LEN))
            .filter(LowerCaser);

        Self {
            schema,
            lowercase_tokenizer,
            key_lang_left,
            key_lang_right,
            extra_lang_left,
            extra_lang_right,
            lang_left,
            lang_right,
            word_classes,
            subject_labels,
        }
    }
}

fn data_dir() -> Result<PathBuf, DictCliError> {
    let data_dir = dirs::data_local_dir()
        .ok_or(DictCliError::NoDataDirectory)?
        .join("dictcc-cli");
    std::fs::create_dir_all(&data_dir)?;
    Ok(data_dir)
}

fn lang_db_dir(lang_pair: &str) -> Result<PathBuf, DictCliError> {
    Ok(data_dir()?.join(normalized_lang_pair(lang_pair)?))
}

fn read_lang_pair<P: AsRef<Path>>(dictcc_path: P) -> Result<String, DictCliError> {
    let file = OpenOptions::new().read(true).open(&dictcc_path)?;
    let mut buf = BufReader::new(file);
    let mut first_line = String::with_capacity(100);
    buf.read_line(&mut first_line)?;
    let lang_pair = first_line
        .strip_prefix('#')
        .ok_or(DictCliError::NoLanguagePair)?
        .split_whitespace()
        .next()
        .ok_or(DictCliError::NoLanguagePair)?
        .to_string()
        .to_lowercase();
    if lang_pair.bytes().filter(|b| *b == b'-').count() != 1 {
        return Err(DictCliError::InvalidLanguagePair);
    }
    Ok(lang_pair)
}

pub(crate) fn languages(lang_pair: &str) -> Result<(&str, &str), DictCliError> {
    let langs = lang_pair.split_once('-').ok_or(DictCliError::InvalidLanguagePair)?;
    if langs.1.contains('-') {
        return Err(DictCliError::InvalidLanguagePair);
    }
    Ok(langs)
}

fn reversed_lang_pair(lang_pair: &str) -> Result<String, DictCliError> {
    let (left, right) = languages(lang_pair)?;
    Ok(format!("{}-{}", right, left))
}

fn normalized_lang_pair(lang_pair: &str) -> Result<String, DictCliError> {
    let mut lang_pairs = [lang_pair.to_string(), reversed_lang_pair(lang_pair)?];
    lang_pairs.sort_unstable();
    Ok(std::mem::take(&mut lang_pairs[0]))
}

pub(crate) fn available_language_pairs() -> Option<Box<[String]>> {
    let data_dir = data_dir().ok()?;
    let available_language_pairs: Box<[String]> = std::fs::read_dir(data_dir)
        .ok()?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            if entry.file_type().ok()?.is_dir() {
                let language_pair = entry.file_name().into_string().ok()?;
                if language_pair.split('-').count() != 2 {
                    return None;
                }
                return Some(language_pair);
            }
            None
        })
        .collect();
    Some(available_language_pairs)
}

pub(crate) fn available_languages(language_pairs: &[String]) -> Box<[String]> {
    language_pairs
        .iter()
        .filter_map(|language_pair| {
            let languages: Vec<String> = language_pair.split('-').map(|lang| lang.to_owned()).collect();
            if languages.len() != 2 {
                return None;
            }
            Some(languages)
        })
        .flatten()
        .collect()
}

fn get_csv_reader_from_path<P: AsRef<Path>>(path: P) -> Result<csv::Reader<File>, DictCliError> {
    Ok(csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .has_headers(false)
        .quoting(false)
        .comment(Some(b'#'))
        .from_path(&path)?)
}

fn prepare_import<P: AsRef<Path>>(db_dir: P, force_import: bool) -> Result<(), DictCliError> {
    let path = db_dir.as_ref();

    if path.try_exists()? {
        if !path.is_dir() {
            return Err(DictCliError::NotDirectory(path.to_str().unwrap().to_owned()));
        }

        if !force_import {
            return Err(DictCliError::AlreadyImported);
        } else {
            std::fs::remove_dir_all(path)?;
        }
    }

    std::fs::create_dir_all(path)?;

    Ok(())
}

pub(crate) fn import_dictcc_file<P: AsRef<Path>>(dictcc_path: P, force_import: bool) -> Result<(), DictCliError> {
    const FIELD_LEN: usize = 4;
    const MIN_FIELD_LEN: usize = 2;
    const DATABASE_WRITER_BUFFER_BYTES: usize = 10485760; // 10 MiB

    let mut stdout_lock = stdout().lock();
    writeln!(stdout_lock, "Initializing database...").unwrap();

    let lang_pair = read_lang_pair(&dictcc_path)?;
    let (lang_left, lang_right) = languages(&lang_pair)?;
    let mut input_reader = get_csv_reader_from_path(&dictcc_path)?;
    let db_directory = lang_db_dir(&lang_pair)?;

    // Indexing documents
    // Here we use a buffer that will be split between indexing threads.
    prepare_import(&db_directory, force_import)?;
    let db_schema = DatabaseSchema::new(lang_left, lang_right);
    let index = Index::create_in_dir(&db_directory, db_schema.schema.clone())?;
    index.tokenizers().register("lowercase", db_schema.lowercase_tokenizer);

    let mut index_writer = index.writer(DATABASE_WRITER_BUFFER_BYTES)?;

    let current_pos = input_reader.position().clone();
    let records_count = input_reader.records().count();
    input_reader.seek(current_pos)?;

    for (index, record) in input_reader.into_records().enumerate() {
        write!(stdout_lock, "\r-> Processing {}/{}", index + 1, records_count).unwrap();

        let record = match record {
            Ok(record) => record,
            Err(err) => {
                eprintln!("\n{}", err);
                continue;
            }
        };

        let mut fields: Vec<String> = record
            .into_iter()
            .take(FIELD_LEN)
            .map(|field| html_escape::decode_html_entities(field).nfc().collect())
            .collect();

        if fields.len() < MIN_FIELD_LEN {
            continue;
        }

        let field_lang_left = std::mem::take(&mut fields[0]);
        let field_lang_right = std::mem::take(&mut fields[1]);
        let field_word_classes = fields.get_mut(2).map(std::mem::take).unwrap_or_else(String::new);
        let field_subject_labels = fields.get_mut(3).map(std::mem::take).unwrap_or_else(String::new);

        let normalized_left = match normalized_entry(&field_lang_left, true) {
            Ok(result) => result,
            Err(err) => {
                eprintln!("\n{}", err);
                continue;
            }
        };

        let normalized_right = match normalized_entry(&field_lang_right, true) {
            Ok(result) => result,
            Err(err) => {
                eprintln!("\n{}", err);
                continue;
            }
        };

        if index == records_count - 1 {
            writeln!(stdout_lock).unwrap();
        }

        index_writer.add_document(doc!(
            db_schema.key_lang_left => normalized_left.text,
            db_schema.key_lang_right => normalized_right.text,
            db_schema.extra_lang_left => normalized_left.extra,
            db_schema.extra_lang_right => normalized_right.extra,
            db_schema.lang_left => field_lang_left,
            db_schema.lang_right => field_lang_right,
            db_schema.word_classes => field_word_classes,
            db_schema.subject_labels => field_subject_labels,
        ))?;
    }

    // We need to call .commit() explicitly to force the
    // index_writer to finish processing the documents in the queue,
    // flush the current index to the disk, and advertise
    // the existence of new documents.
    index_writer.commit()?;

    writeln!(stdout_lock, "Initialized database.").unwrap();

    Ok(())
}

pub(crate) fn remove_database(lang_pair: &str) -> Result<(), DictCliError> {
    std::fs::remove_dir_all(lang_db_dir(lang_pair)?)?;
    Ok(())
}

pub(crate) struct DatabaseSearch {
    pub(crate) schema: DatabaseSchema,
    reader: IndexReader,
    lang_left: String,
    lang_right: String,
}

impl DatabaseSearch {
    pub(crate) fn new(lang_pair: &str) -> Result<Self, DictCliError> {
        let db_dir = lang_db_dir(lang_pair)?;
        let index = Index::open_in_dir(&db_dir)?;
        let reader = index.reader()?;
        let normalized_lang_pair = normalized_lang_pair(lang_pair)?;
        let (lang_left, lang_right) = languages(&normalized_lang_pair)?;
        let schema = DatabaseSchema::new(lang_left, lang_right);
        Ok(Self {
            schema,
            reader,
            lang_left: lang_left.to_owned(),
            lang_right: lang_right.to_owned(),
        })
    }

    pub(crate) fn is_reverse_langs(&self, language_from: &str) -> Result<bool, DictCliError> {
        if language_from == self.lang_left {
            Ok(false)
        } else if language_from == self.lang_right {
            Ok(true)
        } else {
            Err(DictCliError::SearchLanguageNotAvailable(
                language_from.to_owned(),
                format!("{}, {}", self.lang_left, self.lang_right),
            ))
        }
    }

    pub(crate) fn target_language(&self, language_from: &str) -> Result<&str, DictCliError> {
        if language_from == self.lang_left {
            Ok(&self.lang_right)
        } else if language_from == self.lang_right {
            Ok(&self.lang_left)
        } else {
            Err(DictCliError::SearchLanguageNotAvailable(
                language_from.to_owned(),
                format!("{}, {}", self.lang_left, self.lang_right),
            ))
        }
    }

    fn tokenize_search_expression(&self, expression: &str) -> Vec<String> {
        let a = &self.schema.lowercase_tokenizer;
        let mut token_stream = a.token_stream(expression);
        let mut tokens: Vec<String> = Vec::with_capacity(32);
        while token_stream.advance() {
            tokens.push(std::mem::take(&mut token_stream.token_mut().text));
        }
        tokens
    }

    pub(crate) fn search_database(
        &self,
        reverse_langs: bool,
        expression: &str,
        fuzzy_distance: u8,
    ) -> Result<Vec<Document>, DictCliError> {
        if expression.trim().is_empty() {
            return Ok(Vec::new());
        }

        let searcher = self.reader.searcher();
        let (key_field, extra_field) = if !reverse_langs {
            (self.schema.key_lang_left, self.schema.extra_lang_left)
        } else {
            (self.schema.key_lang_right, self.schema.extra_lang_right)
        };

        let mut fuzzy_queries: Vec<(Occur, Box<dyn Query>)> = Vec::with_capacity(32);
        let mut extra_terms: Vec<Term> = Vec::with_capacity(32);
        for word in self.tokenize_search_expression(&expression.nfc().collect::<String>()) {
            extra_terms.push(Term::from_field_text(extra_field, &word));
            let term = Term::from_field_text(key_field, &word);
            let query = FuzzyTermQuery::new(term, fuzzy_distance, true);
            fuzzy_queries.push((Occur::Must, Box::new(query)));
        }
        let boolean_query = BooleanQuery::new(fuzzy_queries);

        let fuzzy_results = searcher.search(&boolean_query, &DocSetCollector)?;
        let extra_results = if extra_terms.len() == 1 {
            searcher.search(
                &TermQuery::new(extra_terms.pop().unwrap(), IndexRecordOption::Basic),
                &DocSetCollector,
            )
        } else {
            searcher.search(&PhraseQuery::new(extra_terms), &DocSetCollector)
        }?;

        let results: Vec<Document> = (&fuzzy_results | &extra_results)
            .into_iter()
            .filter_map(|doc_address| {
                if let Ok(doc) = searcher.doc(doc_address) {
                    Some(doc)
                } else {
                    eprintln!("Failed to retrieve document.");
                    None
                }
            })
            .collect();
        Ok(results)
    }

    pub(crate) fn tab_completions(&self, line: &str, reverse_langs: bool) -> Result<HashSet<String>, DictCliError> {
        let line = line.trim();

        if line.is_empty() {
            return Ok(HashSet::new());
        }

        let line: String = line.nfc().collect();
        let searcher = self.reader.searcher();
        let key_field = if !reverse_langs {
            self.schema.key_lang_left
        } else {
            self.schema.key_lang_right
        };

        let mut tokenized_line = self.tokenize_search_expression(&line);
        let last_word = match tokenized_line.pop() {
            Some(word) => word,
            None => return Ok(HashSet::new()),
        };

        let mut start_terms: Vec<Term> = Vec::with_capacity(32);
        for word in tokenized_line {
            start_terms.push(Term::from_field_text(key_field, &word));
        }

        let last_word_results = searcher.search(
            &RegexQuery::from_pattern(&format!("{}.+", last_word), key_field)?,
            &DocSetCollector,
        )?;

        let start_results = if start_terms.is_empty() {
            None
        } else if start_terms.len() == 1 {
            Some(searcher.search(
                &TermQuery::new(start_terms.pop().unwrap(), IndexRecordOption::Basic),
                &DocSetCollector,
            )?)
        } else {
            Some(searcher.search(&PhraseQuery::new(start_terms), &DocSetCollector)?)
        };

        let intersected_results = if let Some(start_results) = &start_results {
            &last_word_results & start_results
        } else {
            last_word_results
        };

        let results: HashSet<String> = intersected_results
            .into_iter()
            .filter_map(|doc_address| {
                if let Ok(doc) = searcher.doc(doc_address) {
                    doc.field_values().iter().find_map(|field_value| {
                        if field_value.field == key_field {
                            field_value.value.as_text().and_then(|text| {
                                if text.starts_with(&line) {
                                    Some(text.to_owned())
                                } else {
                                    None
                                }
                            })
                        } else {
                            None
                        }
                    })
                } else {
                    eprintln!("Failed to retrieve document.");
                    None
                }
            })
            .collect();

        Ok(results)
    }
}

pub(crate) struct NormalizedEntry {
    pub(crate) text: String,
    pub(crate) extra: String,
}

pub(crate) fn normalized_entry(entry: &str, no_angles: bool) -> Result<NormalizedEntry, DictCliError> {
    let nodes = parser::parse_entry(entry)?.next().unwrap().into_inner();

    let text = nodes
        .clone()
        .filter_map(|node| match node.as_rule() {
            parser::Rule::word | parser::Rule::round => Some(node.as_str()),
            _ => None,
        })
        .join(" ");

    let extra = nodes
        .filter_map(|node| match node.as_rule() {
            parser::Rule::angle => {
                let text = node.as_str();
                Some(if no_angles { &text[1..text.len() - 1] } else { text })
            }
            _ => None,
        })
        .join(" ");

    Ok(NormalizedEntry {
        text: remove_multiple_whitespace(&text),
        extra: remove_multiple_whitespace(extra.trim()),
    })
}

/// https://stackoverflow.com/questions/71864137/whats-the-ideal-way-to-trim-extra-spaces-from-a-string
fn remove_multiple_whitespace(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    s.split_whitespace().for_each(|w| {
        if !result.is_empty() {
            result.push(' ');
        }
        result.push_str(w);
    });
    result
}
