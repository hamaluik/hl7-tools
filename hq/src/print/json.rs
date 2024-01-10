use crate::cli::Cli;
use clap::ColorChoice;
use color_eyre::eyre::{Context, Result};
use hl7_parser::ParsedMessageOwned;
use serde_json::{Map, Value};
use std::io::Write;
use syntect::{
    highlighting::{Style, ThemeSet},
    parsing::{SyntaxDefinition, SyntaxSetBuilder},
    util::{as_24_bit_terminal_escaped, LinesWithEndings},
};

const JSON_SYNTAX: &'static str = include_str!("../../assets/JSON.sublime-syntax");
const THEME: &'static [u8] = include_bytes!("../../assets/rose-pine.tmTheme");

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
            let ranges: Vec<(Style, &str)> = highlighter
                .highlight_line(line, &syntaxes)
                .wrap_err_with(|| "Can't highlight line")?;
            let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
            write!(stdout, "{}", escaped)?;
        }
    }
    println!();

    Ok(())
}
