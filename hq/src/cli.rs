use crate::map::ValueMap;
use clap::{ColorChoice, Parser, ValueEnum};
use hl7_parser::LocationQuery;
use std::{path::PathBuf, str::FromStr};

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

    #[arg(short, long)]
    /// Extract fields from the HL7 message and print the Result
    ///
    /// Query the HL7 message (after any mappings have been applied) and print the result
    /// Multiple queries can be specified and will be reported on separate lines
    pub query: Vec<LocationQuery>,

    #[arg(short, long, default_value_t = false)]
    /// Don't correct newlines in the HL7 message
    ///
    /// By default, \r\n and \n will be converted to \r to separate segments
    pub no_correct_newlines: bool,

    #[arg(short, long, default_value_t = ColorChoice::Auto)]
    /// Colorize output
    pub colour: ColorChoice,

    #[arg(short, long, default_value_t = OutputMode::HL7)]
    /// How to output the HL7 message
    pub output: OutputMode,

    /// The input file to read an HL7 message from
    ///
    /// If not specified, the message will be read from stdin
    pub input: Option<PathBuf>,
}

pub fn cli() -> Cli {
    Cli::parse()
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
#[derive(ValueEnum)]
pub enum OutputMode {
    #[default]
    /// Print the HL7 message as HL7 (the default)
    ///
    /// Note: this will change the newlines in the HL7 message to match the current platform
    ///
    /// Example:
    /// ```text
    /// MSH|^~\&|EPICADT|DH|LABADT|DH|201301011226||ADT^A01|HL7MSG00001|P|2.5
    /// EVN|A01|201301011223
    /// ```
    HL7,
    /// Print the HL7 message as JSON
    ///
    /// Example:
    /// ```json
    /// {
    ///  "MSH": {
    ///    "1": "|",
    ///    "2": "^~\\&",
    ///    "3": "EPICADT",
    ///   }
    /// }
    /// ```
    Json,

    /// Print the HL7 message as a list of rows specifying the field name and value
    ///
    /// Example:
    ///
    /// ```text
    /// MSH.10 1234
    /// MSH.15 AL
    /// ```
    Table,
}

impl std::fmt::Display for OutputMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputMode::HL7 => write!(f, "hl7"),
            OutputMode::Json => write!(f, "json"),
            OutputMode::Table => write!(f, "table"),
        }
    }
}

impl FromStr for OutputMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "hl7" => Ok(OutputMode::HL7),
            "json" => Ok(OutputMode::Json),
            "table" => Ok(OutputMode::Table),
            _ => Err(format!("invalid output mode: {}", s)),
        }
    }
}
