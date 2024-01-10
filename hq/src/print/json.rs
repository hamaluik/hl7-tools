use hl7_parser::ParsedMessageOwned;
use crate::cli::Cli;
use color_eyre::eyre::Result;

pub fn print_message_json(_message: ParsedMessageOwned, _cli: &Cli) -> Result<()> {
    todo!()
}

