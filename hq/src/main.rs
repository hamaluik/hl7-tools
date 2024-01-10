use cli::Cli;
use color_eyre::eyre::{Context, Result};
use hl7_parser::ParsedMessageOwned;
use std::{io::Write, ops::Range};
use termcolor::{Color, ColorSpec, StandardStream, WriteColor};

mod cli;
mod map;

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

    let input = if cli.no_correct_newlines {
        input
    } else {
        input.replace("\r\n", "\r").replace('\n', "\r")
    };
    let input = input.trim_end_matches('\r').to_string();

    let message = ParsedMessageOwned::parse(input)
        .wrap_err_with(|| "Failed to parse input as HL7 message")?;

    let message = apply_maps(message, &cli).wrap_err_with(|| "Failed to apply value mappings")?;
    print_message(message, &cli).wrap_err_with(|| "Failed to print message")?;
    Ok(())
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

fn print_message(message: ParsedMessageOwned, cli: &Cli) -> Result<()> {
    // theme: rose-pine
    let mut hl_segment = ColorSpec::new();
    hl_segment.set_fg(Some(Color::Rgb(156, 207, 216)));
    let mut hl_special_char = ColorSpec::new();
    hl_special_char.set_fg(Some(Color::Rgb(144, 140, 170)));
    let mut hl_number = ColorSpec::new();
    hl_number.set_fg(Some(Color::Rgb(235, 188, 186)));
    let mut hl_value = ColorSpec::new();
    hl_value.set_fg(Some(Color::Rgb(224, 222, 244)));

    let colour = match cli.colour {
        clap::ColorChoice::Auto => termcolor::ColorChoice::Auto,
        clap::ColorChoice::Always => termcolor::ColorChoice::Always,
        clap::ColorChoice::Never => termcolor::ColorChoice::Never,
    };
    let mut stdout = StandardStream::stdout(colour);

    // this is awful but it basically works
    let mut value_ranges: Vec<Range<usize>> = Vec::new();
    let mut number_ranges: Vec<Range<usize>> = Vec::new();
    let mut segment_identifier_ranges: Vec<Range<usize>> = Vec::new();
    for segments in message.segments.values() {
        for segment in segments.iter() {
            let segment_id_range = segment.range.start..segment.range.start + 3;
            segment_identifier_ranges.push(segment_id_range);
            for field in segment.fields.iter() {
                if field.repeats.is_empty() {
                    value_ranges.push(field.range.clone());
                    if field.source(&message.source).parse::<f64>().is_ok() {
                        number_ranges.push(field.range.clone());
                    }
                }
                for repeat in field.repeats.iter() {
                    if repeat.components.is_empty() {
                        value_ranges.push(repeat.range.clone());
                        if repeat.source(&message.source).parse::<f64>().is_ok() {
                            number_ranges.push(repeat.range.clone());
                        }
                    }
                    for component in repeat.components.iter() {
                        if component.sub_components.is_empty() {
                            value_ranges.push(component.range.clone());
                            if component.source(&message.source).parse::<f64>().is_ok() {
                                number_ranges.push(component.range.clone());
                            }
                        }
                        for sub_component in component.sub_components.iter() {
                            value_ranges.push(sub_component.range.clone());
                            if sub_component.source(&message.source).parse::<f64>().is_ok() {
                                number_ranges.push(sub_component.range.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    for (i, c) in message.source.chars().enumerate() {
        let mut hl = None;
        if segment_identifier_ranges.iter().any(|r| r.contains(&i)) {
            hl = Some(&hl_segment);
        } else if number_ranges.iter().any(|r| r.contains(&i)) {
            hl = Some(&hl_number);
        } else if value_ranges.iter().any(|r| r.contains(&i)) {
            hl = Some(&hl_value);
        } else if message.separators.is_special_char(c) {
            hl = Some(&hl_special_char);
        }

        if let Some(hl) = hl {
            stdout
                .set_color(hl)
                .wrap_err_with(|| "Failed to set stdout colour")?;
        }
        if c == '\r' {
            writeln!(stdout).wrap_err_with(|| "Failed to write new line to stdout")?;
        } else {
            write!(stdout, "{}", c).wrap_err_with(|| "Failed to write character to stdout")?;
        }
        // if hl.is_some() {
        //     stdout
        //         .reset()
        //         .wrap_err_with(|| "Failed to reset stdout colour")?;
        // }
    }
    stdout
        .reset()
        .wrap_err_with(|| "Failed to reset stdout colour")?;
    writeln!(stdout).wrap_err_with(|| "Failed to write new line to stdout")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::*;
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
        };
        let message = ParsedMessageOwned::parse(input).unwrap();
        let message = apply_maps(message, &cli).unwrap();
        assert_eq!(
            message.source,
            "MSH|^~\\&|AccMgr|1|||123||ADT^A01|XXX|P|2.3|||\rPID|1|2|3|4|5|6|7|8|9|10|11|12|13|14|15|16\r"
        );
    }
}
