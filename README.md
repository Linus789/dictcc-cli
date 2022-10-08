# dictcc-cli
Offline dictionary via dict.cc database

## Features
* Fuzzy search (kind of)
* Tab completion

## Import database
Go to [https://www1.dict.cc/translation_file_request.php](https://www1.dict.cc/translation_file_request.php) download the file and unpack it, if necessary. Then import it.
```
dictcc-cli import filename.txt
```
After the import has finished, you may delete the file if you want to.

## Translate
Interactive
```
dictcc-cli --language-pair de-en --from en
```
Non-interactive
```
dictcc-cli --language-pair de-en --from en -- Hello
```

## Help menu
```
Usage: dictcc-cli [OPTIONS] --language-pair <LANGUAGE_PAIR> --from <LANGUAGE> [SEARCH]
       dictcc-cli <COMMAND>

Commands:
  import  Import a dict.cc file
  delete  Delete an imported dict.cc database
  help    Print this message or the help of the given subcommand(s)

Arguments:
  [SEARCH]  Search without interactive mode

Options:
  -l, --language-pair <LANGUAGE_PAIR>
          Languages to translate between
  -f, --from <LANGUAGE>
          The source language to translate from
  -d, --distance <DISTANCE>
          Fuzzy distance to find entries [default: 0]
  -r, --limit-results <LIMIT>
          Limit the amount of results
  -s, --min-similarity <LIMIT>
          Only show results with a specific minimum of similarity [possible values: 0 to 1000]
  -c, --completion-type <TYPE>
          Tab completion style [default: list] [possible values: circular, list]
      --ascii
          Use ASCII tables
  -h, --help
          Print help information
  -V, --version
          Print version information
```

## Example
```
$ dictcc-cli -l de-en -f en
> figment
┌──────────────────────────────┬───────────────────────────────────────────────┐
│ EN                           ┆ DE                                            │
╞══════════════════════════════╪═══════════════════════════════════════════════╡
│ figment                      ┆ Erfindung {f}                                 │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ figment                      ┆ Produkt {n} der Einbildung                    │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ figment                      ┆ Gebilde {n} [Phantasie]                       │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ figment                      ┆ Fabelei {f} [oft pej.] [erfundene Geschichte] │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ figment                      ┆ Hirngespinst {n}                              │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ figment of the imagination   ┆ Phantasiegebilde {n}                          │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ figment of the imagination   ┆ Fantasievorstellung {f}                       │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ figment of the imagination   ┆ pure Einbildung {f}                           │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ figment of the imagination   ┆ Ausgeburt {f} der Phantasie / Fantasie        │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ figment of the imagination   ┆ Fantasiegebilde {n}                           │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤
│ a figment of his imagination ┆ ein Produkt {n} seiner Phantasie              │
└──────────────────────────────┴───────────────────────────────────────────────┘
> 
```

## Build from source
* Install `rustup` to get the `rust` compiler installed on your system. [Install rustup](https://www.rust-lang.org/en-US/install.html)
* Rust version 1.63.0 or later is required
* Build in release mode: `cargo build --release`
* The resulting executable can be found at `target/release/dictcc-cli`
