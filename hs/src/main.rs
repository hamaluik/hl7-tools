use bytes::BytesMut;
use color_eyre::eyre::{Context, Result};
use futures::{SinkExt, StreamExt};
use hl7_mllp_codec::MllpCodec;
use std::fmt::Display;
use std::io::Write;
use std::{net::SocketAddr, time::Duration};
use termcolor::{Color, ColorSpec, StandardStream, WriteColor};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;
use tokio_util::codec::Framed;

mod ack;
mod cli;

fn log<S: Display>(s: S, level: u8, stdout: &mut StandardStream) -> Result<()> {
    let mut colour = ColorSpec::new();
    let colour = match level {
        1 => colour.set_fg(Some(Color::Rgb(156, 207, 216))),
        2 => colour.set_fg(Some(Color::Rgb(246, 193, 119))),
        3 => colour.set_fg(Some(Color::Rgb(49, 116, 143))),
        _ => colour.set_fg(Some(Color::White)),
    };

    stdout
        .set_color(&colour)
        .wrap_err_with(|| "Failed to set terminal colour")?;
    writeln!(stdout, "{}", s).wrap_err_with(|| "Failed to write to stdout/err")?;
    stdout
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

    match cli.command {
        cli::Command::Send {
            wait_time,
            no_parse,
            destination,
            input,
        } => {
            let input = if let Some(input) = &input {
                info!(stdout, loglevel, "Reading input from file: {:?}", input);
                std::fs::read_to_string(input)
                    .wrap_err_with(|| format!("Failed to read input file: {:?}", input.display()))?
            } else {
                info!(stdout, loglevel, "Reading input from stdin");
                use std::io::Read;
                let mut input = String::new();
                std::io::stdin()
                    .read_to_string(&mut input)
                    .wrap_err_with(|| "Failed to read from stdin")?;
                input
            };
            let input = strip_ansi_escapes::strip_str(input);
            trace!(stdout, loglevel, "Read input:\n{:?}", input);

            let input = if cli.no_correct_newlines {
                trace!(stdout, loglevel, "Not correcting newlines");
                input
            } else {
                trace!(stdout, loglevel, "Correcting newlines");
                correct_newlines(&input)
            };
            let input = input.trim_end_matches('\r').to_string();
            trace!(stdout, loglevel, "Corrected input:\n{:?}", input);

            if !no_parse {
                debug!(stdout, loglevel, "Parsing input");
                hl7_parser::ParsedMessage::parse(&input)
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

            debug!(stdout, loglevel, "Sending message");
            transport
                .send(BytesMut::from(input.as_bytes()))
                .await
                .wrap_err_with(|| "Failed to send message")?;
            info!(stdout, loglevel, "Sent message");

            debug!(stdout, loglevel, "Waiting for response");
            let message_response = timeout(Duration::from_secs_f64(wait_time), transport.next())
                .await
                .ok()
                .flatten();
            if let Some(received) = message_response {
                info!(stdout, loglevel, "Received response");
                let received = received.wrap_err_with(|| "Failed to receive response")?;
                trace!(stdout, loglevel, "Response bytes:\n{:?}", received);
                let message = String::from_utf8_lossy(&received);
                print_message(message, &mut stdout).wrap_err_with(|| "Failed to print message")?;
            } else {
                info!(stdout, loglevel, "No response received");
            }
        }
        cli::Command::Listen {
            message_count,
            ack_mode,
            port,
        } => {
            debug!(stdout, loglevel, "Starting to listen on 0.0.0.0:{}", port);
            let addr = SocketAddr::from(([0, 0, 0, 0], port));
            let listener = TcpListener::bind(&addr)
                .await
                .wrap_err_with(|| format!("Failed to start listening on {addr}"))?;
            info!(stdout, loglevel, "Listening on 0.0.0.0:{}", port);

            let mut received_messages: usize = 0;
            'accept: loop {
                let Ok((stream, _)) = listener.accept().await else {
                    info!(stdout, loglevel, "Failed to accept connection");
                    continue 'accept;
                };

                let mut transport = Framed::new(stream, MllpCodec::new());
                'messages: while let Some(result) = transport.next().await {
                    trace!(stdout, loglevel, "Received message");
                    trace!(stdout, loglevel, "Message bytes:\n{:?}", result);
                    let Ok(message) = result else {
                        break 'messages;
                    };
                    let message = String::from_utf8_lossy(&message);
                    let message = if cli.no_correct_newlines {
                        trace!(stdout, loglevel, "Not correcting newlines");
                        message.to_string()
                    } else {
                        trace!(stdout, loglevel, "Correcting newlines");
                        correct_newlines(message.as_ref())
                    };

                    let ack = match ack_mode {
                        cli::AckMode::Success => {
                            debug!(stdout, loglevel, "Generating success ACK");
                            Some(
                                ack::generate_ack(&message, cli::AckMode::Success)
                                    .wrap_err_with(|| "Failed to generate ACK")?,
                            )
                        }
                        cli::AckMode::Error => {
                            debug!(stdout, loglevel, "Generating error ACK");
                            Some(
                                ack::generate_ack(&message, cli::AckMode::Error)
                                    .wrap_err_with(|| "Failed to generate ACK")?,
                            )
                        }
                        cli::AckMode::Ignore => {
                            debug!(stdout, loglevel, "Not generating ACK");
                            None
                        }
                    };
                    if let Some((ack, _parsed_message)) = ack {
                        info!(stdout, loglevel, "Sending ACK");
                        debug!(stdout, loglevel, "ACK:\n{}", ack);
                        transport
                            .send(BytesMut::from(ack.as_bytes()))
                            .await
                            .wrap_err_with(|| "Failed to send ACK")?;
                    }
                    print_message(message, &mut stdout)
                        .wrap_err_with(|| "Failed to print message")?;

                    received_messages += 1;
                    trace!(stdout, loglevel, "Received {} messages so far", received_messages);
                    if let Some(message_count) = message_count {
                        if received_messages >= message_count {
                            info!(stdout, loglevel, "Received {} messages, quitting", received_messages);
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
    StandardStream::stderr(colour)
}

fn correct_newlines(message: &str) -> String {
    message.replace("\r\n", "\r").replace('\n', "\r")
}

fn print_message<S: ToString>(message: S, stdout: &mut StandardStream) -> Result<()> {
    let message = message.to_string().replace('\r', "\n");
    writeln!(stdout, "{}", message).wrap_err_with(|| "Failed to write to stdout")?;
    writeln!(stdout).wrap_err_with(|| "Failed to write to stdout")?;
    Ok(())
}
