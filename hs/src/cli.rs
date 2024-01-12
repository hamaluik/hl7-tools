use clap::{ColorChoice, Parser, Subcommand, ValueEnum};
use std::{
    net::{SocketAddr, ToSocketAddrs},
    path::PathBuf,
};

#[derive(Parser, Debug)]
#[command(author = clap::crate_authors!(), version, about, long_about = None, help_template = "\
{before-help}{name} {version}
by {author-with-newline}{about-with-newline}
{usage-heading} {usage}

{all-args}{after-help}
")]
#[command(propagate_version = true)]
pub struct Cli {
    #[arg(short, long, action = clap::ArgAction::Count)]
    /// Increase the level of verbosity
    ///
    /// Use -v to print log information, -vv to print debug information
    pub verbose: u8,

    #[arg(short, long, default_value_t = ColorChoice::Auto)]
    /// Colorize output
    pub colour: ColorChoice,

    #[arg(short, long, default_value_t = false)]
    /// Don't correct newlines in HL7 messages
    ///
    /// By default, \r\n and \n will be converted to \r to separate segments
    pub no_correct_newlines: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Send an HL7 message to a destination via MLLP transport
    ///
    /// The HL7 message will be read from stdin or from a file
    Send {
        #[arg(short, long, default_value_t = 10.0)]
        /// The number of seconds to wait for an ACK response before timing out
        ///
        /// If set to 0, quit immediately after sending the message without
        /// waiting for a response
        ///
        /// If an ACK response is received before the wait time has elapsed, it
        /// will be written to stdout.
        wait_time: f64,

        #[arg(short('p'), long, default_value_t = false)]
        /// Don't parse the input message or ACK response
        ///
        /// By default, both the input message and ACK response (if any) will be
        /// parsed. If either message fails to parse as HL7, the program will
        /// exit with an error.
        no_parse: bool,

        #[arg(value_parser = parse_socket_addr)]
        /// The destination to send the HL7 message to in the form of <host>:<port>
        destination: SocketAddr,

        /// The input file to read an HL7 message from
        ///
        /// If not specified, the message will be read from stdin
        input: Option<PathBuf>,
    },

    /// Listen for HL7 messages via MLLP transport
    ///
    /// The received HL7 messages will be written to stdout
    Listen {
        #[arg(short, long)]
        /// The number of messages to receive before exiting
        ///
        /// If not specified, the server will run until killed
        message_count: Option<usize>,

        #[arg(short, long, default_value_t = AckMode::Success)]
        /// The mode to use for sending ACKs
        ack_mode: AckMode,

        #[arg(short, long, default_value = "127.0.0.1:2575", value_parser = parse_socket_addr)]
        /// The address to bind to in the form of <host>:<port>
        bind: SocketAddr,
    },
}

pub fn cli() -> Cli {
    Cli::parse()
}

fn parse_socket_addr(s: &str) -> Result<SocketAddr, String> {
    s.to_socket_addrs()
        .map_err(|e| e.to_string())?
        .next()
        .ok_or_else(|| format!("{}: no addresses found", s))
}

#[derive(Debug, ValueEnum, Default, Copy, Clone)]
pub enum AckMode {
    /// Don't parse the received messages and don't send ACKs
    Ignore,
    #[default]
    /// Parse the received messages and send ACKs as if the message was processed successfully
    Success,
    /// Parse the received messages and send ACKs as if the message failed to process
    Error,
}

impl std::fmt::Display for AckMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AckMode::Ignore => write!(f, "ignore"),
            AckMode::Success => write!(f, "success"),
            AckMode::Error => write!(f, "error"),
        }
    }
}
