use crate::cli::Cli;
use clap::ColorChoice;
use color_eyre::eyre::{Context, Result};
use hl7_parser::ParsedMessageOwned;
use serde_json::{Map, Value};
use std::io::Write;
use syntect::{
    highlighting::{self, Style, ThemeSet},
    parsing::{SyntaxDefinition, SyntaxSetBuilder},
    util::LinesWithEndings,
};
use termcolor::{Color, ColorSpec, WriteColor};

const JSON_SYNTAX: &str = include_str!("../../assets/JSON.sublime-syntax");
const THEME: &[u8] = include_bytes!("../../assets/ansi.tmTheme");

fn sub_components_to_json(
    sub_components: &[hl7_parser::SubComponent],
    source: &str,
) -> Option<Value> {
    if sub_components.len() == 1 {
        let value = sub_components[0].source(source);
        if value.is_empty() {
            None
        } else {
            Some(Value::String(value.to_string()))
        }
    } else {
        Some(Value::Object(
            sub_components
                .iter()
                .enumerate()
                .filter_map(|(i, sub_component)| {
                    let value = sub_component.source(source);
                    if value.is_empty() {
                        None
                    } else {
                        Some((format!("{}", i + 1), Value::String(value.to_string())))
                    }
                })
                .collect(),
        ))
    }
}

fn components_to_json(components: &[hl7_parser::Component], source: &str) -> Option<Value> {
    if components.len() == 1 {
        sub_components_to_json(&components[0].sub_components, source)
    } else {
        Some(Value::Object(
            components
                .iter()
                .enumerate()
                .filter_map(|(i, component)| {
                    sub_components_to_json(&component.sub_components, source)
                        .map(|value| (format!("{}", i + 1), value))
                })
                .collect(),
        ))
    }
}

fn repeats_to_json(repeats: &[hl7_parser::Repeat], source: &str) -> Option<Value> {
    if repeats.len() == 1 {
        components_to_json(&repeats[0].components, source)
    } else {
        Some(Value::Object(
            repeats
                .iter()
                .enumerate()
                .filter_map(|(i, repeat)| {
                    components_to_json(&repeat.components, source)
                        .map(|value| (format!("{}", i + 1), value))
                })
                .collect(),
        ))
    }
}

fn fields_to_json(fields: &[hl7_parser::Field], source: &str) -> Option<Value> {
    if fields.len() == 1 {
        repeats_to_json(&fields[0].repeats, source)
    } else {
        Some(Value::Object(
            fields
                .iter()
                .enumerate()
                .filter_map(|(i, field)| {
                    repeats_to_json(&field.repeats, source)
                        .map(|value| (format!("{}", i + 1), value))
                })
                .collect(),
        ))
    }
}

fn segments_to_json(segments: &[hl7_parser::Segment], source: &str) -> Value {
    if segments.len() == 1 {
        fields_to_json(&segments[0].fields, source).unwrap_or(Value::Null)
    } else {
        Value::Object(
            segments
                .iter()
                .enumerate()
                .filter_map(|(i, segment)| {
                    fields_to_json(&segment.fields, source)
                        .map(|value| (format!("{}", i + 1), value))
                })
                .collect(),
        )
    }
}

fn message_to_json(message: &ParsedMessageOwned) -> Value {
    let tree: Map<String, Value> = message
        .segments
        .iter()
        .map(|(segment_name, segments)| {
            (
                segment_name.to_string(),
                segments_to_json(segments, &message.source),
            )
        })
        .collect();

    Value::Object(tree)
}

pub fn print_message_json(message: ParsedMessageOwned, cli: &Cli) -> Result<()> {
    let json = message_to_json(&message);
    let json = serde_json::to_string_pretty(&json).wrap_err_with(|| "Can't serialize JSON")?;

    if cli.colour == ColorChoice::Never {
        print!("{json}");
    } else {
        // TODO: Do this at build time and figure out why my attempts segfault
        let mut theme = std::io::Cursor::new(THEME);
        let theme = ThemeSet::load_from_reader(&mut theme).wrap_err_with(|| "Can't load themes")?;
        let syntax = SyntaxDefinition::load_from_str(JSON_SYNTAX, false, None)
            .wrap_err_with(|| "Can't load syntax")?;
        let mut syntaxes = SyntaxSetBuilder::new();
        syntaxes.add(syntax);
        let syntaxes = syntaxes.build();

        let syntax = syntaxes
            .find_syntax_by_name("JSON")
            .expect("Can find JSON syntax");
        let mut highlighter = syntect::easy::HighlightLines::new(syntax, &theme);

        let mut stdout = crate::open_stdout(cli);

        for line in LinesWithEndings::from(&json) {
            let spans: Vec<(Style, &str)> = highlighter
                .highlight_line(line, &syntaxes)
                .wrap_err_with(|| "Can't highlight line")?;

            for (style, text) in spans.into_iter() {
                let fg = to_ansi_color(style.foreground);
                let mut spec = ColorSpec::new();
                spec.set_fg(fg);

                stdout
                    .set_color(&spec)
                    .wrap_err_with(|| "Can't set color")?;
                write!(stdout, "{text}").wrap_err_with(|| "Can't write to buffer")?;
            }
        }

        stdout.reset().wrap_err_with(|| "Can't reset stdout")?;
    }
    println!();

    Ok(())
}

/// source: https://github.com/sharkdp/bat/blob/cd81c7fa6bf0d061f455f67aae72dc5537f7851d/src/terminal.rs#L6
fn to_ansi_color(color: highlighting::Color) -> Option<Color> {
    if color.a == 0 {
        Some(match color.r {
            0x00 => Color::Black,
            0x01 => Color::Red,
            0x02 => Color::Green,
            0x03 => Color::Yellow,
            0x04 => Color::Blue,
            0x05 => Color::Magenta,
            0x06 => Color::Cyan,
            0x07 => Color::White,
            n => Color::Ansi256(n),
        })
    } else if color.a == 1 {
        None
    } else {
        Some(Color::Rgb(color.r, color.g, color.b))
    }
}
