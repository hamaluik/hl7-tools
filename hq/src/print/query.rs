use crate::cli::Cli;
use color_eyre::eyre::{Context, Result};
use hl7_parser::ParsedMessageOwned;
use termcolor::{Color, ColorSpec, WriteColor};
use crate::open_stdout;
use std::io::Write;

pub fn print_query_results(message: ParsedMessageOwned, cli: &Cli) -> Result<()> {
    let mut stdout = open_stdout(cli);
    for query in cli.query.iter() {
        let value = message.query_value(query).wrap_err_with(|| {
            format!(
                "Failed to query message for {:?} (query: {:?})",
                query, query
            )
        })?;
        if let Some(value) = value {
            for c in value.chars() {
                if message.separators.is_special_char(c) {
                    stdout
                        .set_color(ColorSpec::new().set_fg(Some(Color::Rgb(144, 140, 170))))
                        .wrap_err_with(|| "Failed to set stdout colour")?;
                }
                else {
                    stdout
                        .set_color(ColorSpec::new().set_fg(Some(Color::Rgb(224, 222, 244))))
                        .wrap_err_with(|| "Failed to set stdout colour")?;
                }
                write!(stdout, "{c}").wrap_err_with(|| "Failed to write to stdout")?;
            }
            stdout.reset().wrap_err_with(|| "Failed to reset stdout colour")?;
            writeln!(stdout)
                .wrap_err_with(|| "Failed to write to stdout")?;
        } else {
            writeln!(stdout).wrap_err_with(|| "Failed to write to stdout")?;
        }
    }
    Ok(())
}
