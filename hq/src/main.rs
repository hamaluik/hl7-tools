use cli::Cli;
use color_eyre::eyre::{Context, Result};
use hl7_parser::ParsedMessageOwned;
use termcolor::StandardStream;

mod cli;
mod map;
mod print;
pub use print::*;

fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = cli::cli();

    let input = if let Some(input) = &cli.input {
        std::fs::read_to_string(input)
            .wrap_err_with(|| format!("Failed to read input file: {:?}", input.display()))?
    } else {
        use std::io::Read;
        let mut input = String::new();
        std::io::stdin()
            .read_to_string(&mut input)
            .wrap_err_with(|| "Failed to read from stdin")?;
        input
    };
    let input = strip_ansi_escapes::strip_str(input);

    let input = if cli.no_correct_newlines {
        input
    } else {
        input.replace("\r\n", "\r").replace('\n', "\r")
    };
    let input = input.trim_end_matches('\r').to_string();

    let message = ParsedMessageOwned::parse(input)
        .wrap_err_with(|| "Failed to parse input as HL7 message")?;

    let message = apply_maps(message, &cli).wrap_err_with(|| "Failed to apply value mappings")?;

    if cli.query.is_empty() {
        match cli.output {
            cli::OutputMode::HL7 => {
                print_message_hl7(message, &cli).wrap_err_with(|| "Failed to print message")
            }
            cli::OutputMode::Json => {
                print_message_json(message, &cli).wrap_err_with(|| "Failed to print queries")
            }
            cli::OutputMode::Table => {
                print_message_table(message, &cli).wrap_err_with(|| "Failed to print queries")
            }
        }
    } else {
        print_query_results(message, &cli).wrap_err_with(|| "Failed to print queries")
    }
}

fn apply_maps(mut message: ParsedMessageOwned, cli: &Cli) -> Result<ParsedMessageOwned> {
    'maps: for map in cli.map.iter() {
        let query = &*map.from;

        if !message.has_segment(&query.segment) {
            continue 'maps;
        }

        let range = message.query(query).wrap_err_with(|| {
            format!(
                "Failed to query message for {:?} (map: {:?})",
                query, map.from
            )
        })?;

        if let Some(range) = range {
            let value = map.to.reify(query);
            message.source.replace_range(range, &value);
            message = ParsedMessageOwned::parse(&message.source)
                .wrap_err_with(|| format!("Failed to re-parse message after applying map {map}"))?;
        }
    }
    Ok(message)
}

fn open_stdout(cli: &Cli) -> StandardStream {
    let colour = match cli.colour {
        clap::ColorChoice::Auto => termcolor::ColorChoice::Auto,
        clap::ColorChoice::Always => termcolor::ColorChoice::Always,
        clap::ColorChoice::Never => termcolor::ColorChoice::Never,
    };
    StandardStream::stdout(colour)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{cli::OutputMode, map::*};
    use clap::ColorChoice;
    use hl7_parser::LocationQuery;

    #[test]
    fn can_map() {
        let input = "MSH|^~\\&|AccMgr|1|||20050110045504||ADT^A01|599102|P|2.3|||\rPID|1|2|3|4|5|6|7|8|9|10|11|12|13|14|15|16\r";
        let cli = Cli {
            map: vec![
                ValueMap {
                    from: ValueMapFrom(LocationQuery::new_field_repeat("MSH", 10, 1).unwrap()),
                    to: ValueMapTo::Explicit("XXX".to_string()),
                },
                ValueMap {
                    from: ValueMapFrom(LocationQuery::new_field_repeat("MSH", 7, 1).unwrap()),
                    to: ValueMapTo::Explicit("123".to_string()),
                },
            ],
            no_correct_newlines: false,
            colour: ColorChoice::Never,
            input: None,
            output: OutputMode::HL7,
            query: vec![],
        };
        let message = ParsedMessageOwned::parse(input).unwrap();
        let message = apply_maps(message, &cli).unwrap();
        assert_eq!(
            message.source,
            "MSH|^~\\&|AccMgr|1|||123||ADT^A01|XXX|P|2.3|||\rPID|1|2|3|4|5|6|7|8|9|10|11|12|13|14|15|16\r"
        );
    }
}
