use std::path::PathBuf;

use clap::builder::{NonEmptyStringValueParser, PathBufValueParser, PossibleValuesParser};
use clap::{arg, crate_description, crate_name, crate_version, ArgMatches, Command};

use crate::database;
use crate::error::DictCliError;

pub(crate) enum Settings {
    Import {
        file: PathBuf,
        force: bool,
    },
    Delete {
        language_pair: String,
    },
    Translate {
        language_pair: String,
        language_from: String,
        fuzzy_distance: u8,
        limit_results: Option<u32>,
        minimum_similarity: Option<u16>,
        completion_type: rustyline::config::CompletionType,
        ascii: bool,
        search: Option<String>,
    },
}

pub(crate) fn parse_settings() -> Result<Settings, DictCliError> {
    let args = parse_args();

    if let Some(import) = args.subcommand_matches("import") {
        return Ok(Settings::Import {
            file: import.get_one::<PathBuf>("FILE").unwrap().to_owned(),
            force: import.get_flag("force"),
        });
    }

    if let Some(delete) = args.subcommand_matches("delete") {
        return Ok(Settings::Delete {
            language_pair: delete.get_one::<String>("LANGUAGE_PAIR").unwrap().to_lowercase(),
        });
    }

    let language_pair = args.get_one::<String>("language-pair").unwrap().to_lowercase();
    let language_from = args.get_one::<String>("from").unwrap().to_lowercase();
    let languages = database::languages(&language_pair)?;

    if language_from != languages.0 && language_from != languages.1 {
        return Err(DictCliError::SearchLanguageNotAvailable(
            language_from,
            format!("{}, {}", languages.0, languages.1),
        ));
    }

    let completion_type = match args
        .get_one::<String>("completion-type")
        .unwrap()
        .to_lowercase()
        .as_str()
    {
        "circular" => rustyline::config::CompletionType::Circular,
        "list" => rustyline::config::CompletionType::List,
        _ => unreachable!(),
    };

    Ok(Settings::Translate {
        language_pair: args.get_one::<String>("language-pair").unwrap().to_lowercase(),
        language_from: args.get_one::<String>("from").unwrap().to_lowercase(),
        fuzzy_distance: *args.get_one::<u8>("distance").unwrap(),
        limit_results: args.get_one::<u32>("limit-results").copied(),
        minimum_similarity: args.get_one::<u16>("min-similarity").copied(),
        completion_type,
        ascii: args.get_flag("ascii"),
        search: args.get_one::<String>("SEARCH").map(|search| search.to_owned()),
    })
}

fn parse_args() -> ArgMatches {
    let mut command = Command::new(crate_name!()).version(crate_version!());
    let description = crate_description!();

    if !description.is_empty() {
        command = command.about(description);
    }

    let available_language_pairs = database::available_language_pairs();
    let available_languages = available_language_pairs
        .as_ref()
        .map(|lang_pairs| database::available_languages(lang_pairs));

    command
        .args_conflicts_with_subcommands(true)
        .subcommand(
            Command::new("import")
                .about("Import a dict.cc file")
                .arg(
                    arg!(
                        -f --force "Overwrite existing database if necessary"
                    )
                    .required(false),
                )
                .arg(
                    arg!(
                        <FILE> "dict.cc file from https://www1.dict.cc/translation_file_request.php"
                    )
                    .required(true)
                    .value_parser(PathBufValueParser::new()),
                ),
        )
        .subcommand(
            Command::new("delete")
                .about("Delete an imported dict.cc database")
                .arg({
                    let arg = arg!(
                        <LANGUAGE_PAIR> "The language pair of the database"
                    )
                    .ignore_case(true)
                    .required(true);
                    if let Some(langs) = available_language_pairs.as_ref() {
                        arg.value_parser(PossibleValuesParser::new(langs.iter()))
                    } else {
                        arg.value_parser(NonEmptyStringValueParser::new())
                    }
                }),
        )
        .arg({
            let arg = arg!(
                -l --"language-pair" <LANGUAGE_PAIR> "Languages to translate between"
            )
            .ignore_case(true)
            .required(true);
            if let Some(langs) = available_language_pairs.as_ref() {
                arg.value_parser(PossibleValuesParser::new(langs.iter()))
            } else {
                arg.value_parser(NonEmptyStringValueParser::new())
            }
        })
        .arg({
            let arg = arg!(
                -f --from <LANGUAGE> "The source language to translate from"
            )
            .ignore_case(true)
            .required(true);
            if let Some(langs) = available_languages.as_ref() {
                arg.value_parser(PossibleValuesParser::new(langs.iter()))
            } else {
                arg.value_parser(NonEmptyStringValueParser::new())
            }
        })
        .arg(
            arg!(
                -d --distance <DISTANCE> "Fuzzy distance to find entries"
            )
            .required(false)
            .value_parser(clap::value_parser!(u8))
            .default_value("0"),
        )
        .arg(
            arg!(
                -r --"limit-results" <LIMIT> "Limit the amount of results"
            )
            .required(false)
            .value_parser(clap::value_parser!(u32).range(1..)),
        )
        .arg(
            arg!(
                -s --"min-similarity" <LIMIT> "Only show results with a specific minimum of similarity [possible values: 0 to 1000]"
            )
            .required(false)
            .value_parser(clap::value_parser!(u16).range(0..=1000)),
        )
        .arg(
            arg!(
                -c --"completion-type" <TYPE> "Tab completion style"
            )
            .required(false)
            .ignore_case(true)
            .value_parser(["circular", "list"])
            .default_value("list"),
        )
        .arg(
            arg!(
                --ascii "Use ASCII tables"
            )
            .required(false),
        )
        .arg(
            arg!(
                [SEARCH] "Search without interactive mode"
            )
            .required(false)
            .value_parser(NonEmptyStringValueParser::new()),
        )
        .get_matches()
}
