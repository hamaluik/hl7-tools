use crate::cli::Cli;
use color_eyre::eyre::{Context, Result};
use hl7_parser::ParsedMessageOwned;
use termcolor::{Color, ColorSpec, WriteColor};
use crate::open_stdout;
use std::ops::Range;
use std::io::Write;

pub fn print_message_hl7(message: ParsedMessageOwned, cli: &Cli) -> Result<()> {
    // theme: rose-pine
    let mut hl_segment = ColorSpec::new();
    hl_segment.set_fg(Some(Color::Rgb(156, 207, 216)));
    let mut hl_special_char = ColorSpec::new();
    hl_special_char.set_fg(Some(Color::Rgb(144, 140, 170)));
    let mut hl_number = ColorSpec::new();
    hl_number.set_fg(Some(Color::Rgb(235, 188, 186)));
    let mut hl_value = ColorSpec::new();
    hl_value.set_fg(Some(Color::Rgb(224, 222, 244)));

    let mut stdout = open_stdout(cli);

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
            write!(stdout, "{c}").wrap_err_with(|| "Failed to write character to stdout")?;
        }
    }
    stdout
        .reset()
        .wrap_err_with(|| "Failed to reset stdout colour")?;
    println!();

    Ok(())
}

