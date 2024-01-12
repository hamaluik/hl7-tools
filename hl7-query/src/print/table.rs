use crate::cli::Cli;
use color_eyre::eyre::{Context, Result};
use hl7_parser::ParsedMessageOwned;
use std::io::Write;
use termcolor::{Color, ColorSpec, StandardStream, WriteColor};

fn write_path(stdout: &mut StandardStream, path: &[String]) -> Result<()> {
    let mut hl_segment = ColorSpec::new();
    let mut hl_special_char = ColorSpec::new();
    let mut hl_number = ColorSpec::new();
    let mut hl_value = ColorSpec::new();

    hl_segment.set_fg(Some(Color::Cyan));
    hl_special_char.set_fg(Some(Color::Black)).set_intense(true);
    hl_number.set_fg(Some(Color::Yellow));
    hl_value.set_fg(Some(Color::White)).set_intense(true);

    for (i, segment) in path.iter().enumerate() {
        if i > 0 {
            stdout
                .set_color(&hl_special_char)
                .wrap_err_with(|| "Failed to set stdout colour")?;
            write!(stdout, ".").wrap_err_with(|| "Failed to write to stdout")?;
        }
        if i == 0 {
            stdout
                .set_color(&hl_segment)
                .wrap_err_with(|| "Failed to set stdout colour")?;
        } else {
            stdout
                .set_color(&hl_number)
                .wrap_err_with(|| "Failed to set stdout colour")?;
        }
        write!(stdout, "{segment}").wrap_err_with(|| "Failed to write to stdout")?;
    }
    stdout
        .reset()
        .wrap_err_with(|| "Failed to reset stdout colour")?;
    Ok(())
}

fn write_value(stdout: &mut StandardStream, value: &str) -> Result<()> {
    let mut hl_value = ColorSpec::new();
    hl_value.set_fg(Some(Color::White)).set_intense(true);

    stdout
        .set_color(&hl_value)
        .wrap_err_with(|| "Failed to set stdout colour")?;
    write!(stdout, "{value}").wrap_err_with(|| "Failed to write to stdout")?;
    stdout
        .reset()
        .wrap_err_with(|| "Failed to reset stdout colour")?;
    Ok(())
}

fn write_path_value(stdout: &mut StandardStream, path: &[String], value: &str) -> Result<()> {
    write_path(stdout, path)?;
    write!(stdout, "\t")?;
    write_value(stdout, value)?;
    writeln!(stdout)?;
    Ok(())
}

pub fn print_message_table(message: ParsedMessageOwned, cli: &Cli) -> Result<()> {
    let mut stdout = crate::open_stdout(cli);

    let mut current_path: Vec<String> = Vec::new();
    for (segment_name, segments) in message.segments.iter() {
        for (segment_i, segment) in segments.iter().enumerate() {
            let segment_name = if segments.len() > 1 {
                format!("{}[{}]", segment_name, segment_i)
            } else {
                segment_name.to_string()
            };
            current_path.push(segment_name);
            for (field_i, field) in segment.fields.iter().enumerate() {
                let field_name = format!("{}", field_i + 1);
                current_path.push(field_name);
                if field.repeats.is_empty() {
                    let value = field.source(&message.source);
                    if !value.is_empty() {
                        write_path_value(&mut stdout, &current_path, value)?;
                    }
                } else {
                    for (repeat_i, repeat) in field.repeats.iter().enumerate() {
                        if field.repeats.len() > 1 {
                            current_path.push(format!("[{}]", repeat_i + 1));
                        }
                        if repeat.components.is_empty() || repeat.components.len() == 1 {
                            let value = repeat.source(&message.source);
                            if !value.is_empty() {
                                write_path_value(&mut stdout, &current_path, value)?;
                            }
                        } else {
                            for (component_i, component) in repeat.components.iter().enumerate() {
                                current_path.push(format!("{}", component_i + 1));
                                if component.sub_components.is_empty()
                                    || component.sub_components.len() == 1
                                {
                                    let value = component.source(&message.source);
                                    if !value.is_empty() {
                                        write_path_value(&mut stdout, &current_path, value)?;
                                    }
                                } else {
                                    for (sub_component_i, sub_component) in
                                        component.sub_components.iter().enumerate()
                                    {
                                        current_path.push(format!("{}", sub_component_i + 1));
                                        let value = sub_component.source(&message.source);
                                        if !value.is_empty() {
                                            write_path_value(&mut stdout, &current_path, value)?;
                                        }
                                        current_path.pop();
                                    }
                                }
                                current_path.pop();
                            }
                        }
                        if field.repeats.len() > 1 {
                            current_path.pop();
                        }
                    }
                }
                current_path.pop();
            }
            current_path.pop();
        }
    }

    Ok(())
}
