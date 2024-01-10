use crate::map::ValueMap;
use clap::{ColorChoice, Parser};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author = clap::crate_authors!(), version, about, long_about = None, help_template = "\
{before-help}{name} {version}
by {author-with-newline}{about-with-newline}
{usage-heading} {usage}

{all-args}{after-help}
")]
#[command(propagate_version = true)]
pub struct Cli {
    #[arg(
        short,
        long,
        value_parser = clap::value_parser!(ValueMap),
    )]
    /// Map HL7 fields to values (or auto-generate values when using the `<auto>` keyword)
    ///
    /// Format: `hl7_field=<auto>|<now>|value`
    ///
    /// Example: `MSH-10=<auto>`
    ///
    /// Example: `MSH-10=1234`
    ///
    /// The `hl7_field` is a location query, see https://docs.rs/hl7-parser/0.1.0/hl7_parser/struct.LocationQuery.html
    ///
    /// The `value` is a string value to use for the fields
    ///
    /// The `auto` keyword will attempt to generate an appropriate value for the field
    ///  (for example, a date for a date field, a control ID for MSH-10, etc.)
    pub map: Vec<ValueMap>,

    #[arg(short, long, default_value_t = false)]
    pub no_correct_newlines: bool,

    #[arg(short, long, default_value_t = ColorChoice::Auto)]
    pub colour: ColorChoice,

    /// The input file to read an HL7 message from
    /// If not specified, the message will be read from stdin
    pub input: Option<PathBuf>,
}

pub fn cli() -> Cli {
    Cli::parse()
}
