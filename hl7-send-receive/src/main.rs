use bytes::BytesMut;
use color_eyre::eyre::{Context, Result};
use futures::{SinkExt, StreamExt};
use hl7_mllp_codec::MllpCodec;
use std::fmt::Display;
use std::io::{IsTerminal, Write};
use std::time::Duration;
use termcolor::{Color, ColorSpec, StandardStream, WriteColor};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;
use tokio_util::codec::Framed;

mod ack;
mod cli;
mod print;

fn log<S: Display>(s: S, level: u8, stderr: &mut StandardStream) -> Result<()> {
    let mut colour = ColorSpec::new();
    let colour = match level {
        1 => colour.set_fg(Some(Color::Cyan)),
        2 => colour.set_fg(Some(Color::Magenta)),
        3 => colour.set_fg(Some(Color::White)).set_dimmed(true),
        _ => colour.set_fg(Some(Color::White)),
    };

    stderr
        .set_color(colour)
        .wrap_err_with(|| "Failed to set terminal colour")?;
    writeln!(stderr, "{}", s).wrap_err_with(|| "Failed to write to stderr")?;
    stderr
        .reset()
        .wrap_err_with(|| "Failed to reset terminal colour")?;

    Ok(())
}

macro_rules! info {
    ($stdout:ident, $loglevel:ident, $($arg:tt)*) => {
        if $loglevel >= 1 {
            log(format!($($arg)*), 1, &mut $stdout).wrap_err_with(|| "Failed to log info message")?;
        }
    };
}

macro_rules! debug {
    ($stdout:ident, $loglevel:ident, $($arg:tt)*) => {
        if $loglevel >= 2 {
            log(format!($($arg)*), 2, &mut $stdout).wrap_err_with(|| "Failed to log debug message")?;
        }
    };
}

macro_rules! trace {
    ($stdout:ident, $loglevel:ident, $($arg:tt)*) => {
        if $loglevel >= 3 {
            log(format!($($arg)*), 3, &mut $stdout).wrap_err_with(|| "Failed to log debug message")?;
        }
    };
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = cli::cli();
    let loglevel = cli.verbose;
    let mut stdout = open_stdout(&cli);
    let mut stderr = open_stderr(&cli);

    match cli.command {
        cli::Command::Send {
            wait_time,
            no_parse,
            destination,
            input,
        } => {
            let input = if let Some(input) = &input {
                info!(stderr, loglevel, "Reading input from file: {:?}", input);
                std::fs::read_to_string(input)
                    .wrap_err_with(|| format!("Failed to read input file: {:?}", input.display()))?
            } else {
                info!(stderr, loglevel, "Reading input from stdin");
                use std::io::Read;
                let mut input = String::new();
                std::io::stdin()
                    .read_to_string(&mut input)
                    .wrap_err_with(|| "Failed to read from stdin")?;
                input
            };
            let input = strip_ansi_escapes::strip_str(input);
            trace!(stderr, loglevel, "Read input:\n{:?}", input);

            let input = if cli.no_correct_newlines {
                trace!(stderr, loglevel, "Not correcting newlines");
                input
            } else {
                trace!(stderr, loglevel, "Correcting newlines");
                correct_newlines(&input)
            };
            let input = input.trim_end_matches('\r').to_string();
            trace!(stderr, loglevel, "Corrected input:\n{:?}", input);

            if !no_parse {
                debug!(stderr, loglevel, "Parsing input");
                hl7_parser::ParsedMessage::parse(&input, false)
                    .wrap_err_with(|| "Failed to parse input message")?;
            }

            debug!(
                stdout,
                loglevel, "Connecting to HL7 destination: {}", destination
            );
            let stream = TcpStream::connect(destination)
                .await
                .with_context(|| "Failed to connect to HL7 destination!")?;
            let mut transport = Framed::new(stream, MllpCodec::new());
            info!(
                stdout,
                loglevel, "Connected to HL7 destination: {}", destination
            );

            debug!(stderr, loglevel, "Sending message");
            transport
                .send(BytesMut::from(input.as_bytes()))
                .await
                .wrap_err_with(|| "Failed to send message")?;
            info!(stderr, loglevel, "Sent message");

            debug!(stderr, loglevel, "Waiting for response");
            let message_response = timeout(Duration::from_secs_f64(wait_time), transport.next())
                .await
                .ok()
                .flatten();
            if let Some(received) = message_response {
                info!(stderr, loglevel, "Received response");
                let received = received.wrap_err_with(|| "Failed to receive response")?;
                trace!(stderr, loglevel, "Response bytes:\n{:?}", received);
                let message = String::from_utf8_lossy(&received);
                if no_parse {
                    print::print_message_nohl(message)
                        .wrap_err_with(|| "Failed to print message")?;
                } else {
                    let message = hl7_parser::ParsedMessage::parse(&message, false)
                        .wrap_err_with(|| "Failed to parse message")?;
                    print::print_message_hl(&mut stdout, message)
                        .wrap_err_with(|| "Failed to print message")?;
                }
            } else {
                info!(stderr, loglevel, "No response received");
            }
        }
        cli::Command::Listen {
            message_count,
            ack_mode,
            bind,
        } => {
            debug!(stderr, loglevel, "Starting to listen on {bind}");
            let listener = TcpListener::bind(&bind)
                .await
                .wrap_err_with(|| format!("Failed to start listening on {bind}"))?;
            info!(stderr, loglevel, "Listening on {bind}");

            let mut received_messages: usize = 0;
            'accept: loop {
                let Ok((stream, remote)) = listener.accept().await else {
                    info!(stderr, loglevel, "Failed to accept connection");
                    continue 'accept;
                };
                trace!(stderr, loglevel, "Remote connection: {:?}", remote);

                let mut transport = Framed::new(stream, MllpCodec::new());
                'messages: while let Some(result) = transport.next().await {
                    trace!(stderr, loglevel, "Received message");
                    trace!(stderr, loglevel, "Message bytes:\n{:?}", result);
                    let Ok(message) = result else {
                        break 'messages;
                    };
                    let message = String::from_utf8_lossy(&message);
                    let message = if cli.no_correct_newlines {
                        trace!(stderr, loglevel, "Not correcting newlines");
                        message.to_string()
                    } else {
                        trace!(stderr, loglevel, "Correcting newlines");
                        correct_newlines(message.as_ref())
                    };

                    let ack = match ack_mode {
                        cli::AckMode::Success => {
                            debug!(stderr, loglevel, "Generating success ACK");
                            Some(
                                ack::generate_ack(&message, cli::AckMode::Success)
                                    .wrap_err_with(|| "Failed to generate ACK")?,
                            )
                        }
                        cli::AckMode::Error => {
                            debug!(stderr, loglevel, "Generating error ACK");
                            Some(
                                ack::generate_ack(&message, cli::AckMode::Error)
                                    .wrap_err_with(|| "Failed to generate ACK")?,
                            )
                        }
                        cli::AckMode::Ignore => {
                            debug!(stderr, loglevel, "Not generating ACK");
                            None
                        }
                    };
                    let parsed_message = if let Some((ack, parsed_message)) = ack {
                        info!(stderr, loglevel, "Sending ACK");
                        debug!(stderr, loglevel, "ACK:\n{}", ack);
                        transport
                            .send(BytesMut::from(ack.as_bytes()))
                            .await
                            .wrap_err_with(|| "Failed to send ACK")?;
                        Some(parsed_message)
                    } else {
                        None
                    };
                    if let Some(parsed_message) = parsed_message {
                        print::print_message_hl(&mut stdout, parsed_message)
                            .wrap_err_with(|| "Failed to print message")?;
                    } else {
                        print::print_message_nohl(&message)
                            .wrap_err_with(|| "Failed to print message")?;
                    }

                    received_messages += 1;
                    trace!(
                        stderr,
                        loglevel,
                        "Received {} messages so far",
                        received_messages
                    );
                    if let Some(message_count) = message_count {
                        if received_messages >= message_count {
                            info!(
                                stderr,
                                loglevel, "Received {} messages, quitting", received_messages
                            );
                            break 'accept;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn open_stdout(cli: &cli::Cli) -> StandardStream {
    let colour = match cli.colour {
        clap::ColorChoice::Auto => termcolor::ColorChoice::Auto,
        clap::ColorChoice::Always => termcolor::ColorChoice::Always,
        clap::ColorChoice::Never => termcolor::ColorChoice::Never,
    };
    let colour = if !std::io::stdout().is_terminal() {
        termcolor::ColorChoice::Never
    } else {
        colour
    };
    StandardStream::stdout(colour)
}

fn open_stderr(cli: &cli::Cli) -> StandardStream {
    let colour = match cli.colour {
        clap::ColorChoice::Auto => termcolor::ColorChoice::Auto,
        clap::ColorChoice::Always => termcolor::ColorChoice::Always,
        clap::ColorChoice::Never => termcolor::ColorChoice::Never,
    };
    let colour = if !std::io::stderr().is_terminal() {
        termcolor::ColorChoice::Never
    } else {
        colour
    };
    StandardStream::stderr(colour)
}

fn correct_newlines(message: &str) -> String {
    message.replace("\r\n", "\r").replace('\n', "\r")
}
