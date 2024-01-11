use bytes::BytesMut;
use color_eyre::eyre::{Context, Result};
use futures::{SinkExt, StreamExt};
use hl7_mllp_codec::MllpCodec;
use termcolor::StandardStream;
use std::{net::SocketAddr, time::Duration};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;
use tokio_util::codec::Framed;

mod ack;
mod cli;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = cli::cli();
    let mut stdout = open_stdout(&cli);

    match cli.command {
        cli::Command::Send {
            wait_time,
            no_parse,
            destination,
            input,
        } => {
            let input = if let Some(input) = &input {
                std::fs::read_to_string(input)
                    .wrap_err_with(|| format!("Failed to read input file: {:?}", input.display()))?
            } else {
                use std::io::Read;
                let mut input = String::new();
                std::io::stdin()
                    .read_to_string(&mut input)
                    .wrap_err_with(|| "Failed to read from stdin")?;
                input
            };
            let input = strip_ansi_escapes::strip_str(input);

            let input = if cli.no_correct_newlines {
                input
            } else {
                correct_newlines(&input)
            };
            let input = input.trim_end_matches('\r').to_string();

            if !no_parse {
                hl7_parser::ParsedMessage::parse(&input)
                    .wrap_err_with(|| "Failed to parse input message")?;
            }

            let stream = TcpStream::connect(destination)
                .await
                .with_context(|| "Failed to connect to HL7 destination!")?;
            let mut transport = Framed::new(stream, MllpCodec::new());

            transport
                .send(BytesMut::from(input.as_bytes()))
                .await
                .wrap_err_with(|| "Failed to send message")?;

            let message_response = timeout(Duration::from_secs_f64(wait_time), transport.next())
                .await
                .ok()
                .flatten();
            if let Some(received) = message_response {
                let received = received.wrap_err_with(|| "Failed to receive response")?;
                let message = String::from_utf8_lossy(&received);
                print_message(message, &mut stdout).wrap_err_with(|| "Failed to print message")?;
            }
        }
        cli::Command::Listen {
            message_count,
            ack_mode,
            port,
        } => {
            let addr = SocketAddr::from(([0, 0, 0, 0], port));
            let listener = TcpListener::bind(&addr)
                .await
                .wrap_err_with(|| format!("Failed to start listening on {addr}"))?;

            let mut received_messages: usize = 0;
            'accept: loop {
                let Ok((stream, _)) = listener.accept().await else {
                    continue 'accept;
                };

                    let mut transport = Framed::new(stream, MllpCodec::new());
                    'messages: while let Some(result) = transport.next().await {
                        let Ok(message) = result else {
                            break 'messages;
                        };
                        let message = String::from_utf8_lossy(&message);
                        let message = if cli.no_correct_newlines {
                            message.to_string()
                        } else {
                            correct_newlines(message.as_ref())
                        };

                        let ack = match ack_mode {
                            cli::AckMode::Success => {
                                Some(ack::generate_ack(&message, cli::AckMode::Success)
                                    .wrap_err_with(|| "Failed to generate ACK")?)
                            }
                            cli::AckMode::Error => {
                                Some(ack::generate_ack(&message, cli::AckMode::Error)
                                    .wrap_err_with(|| "Failed to generate ACK")?)
                            }
                            cli::AckMode::Ignore => {
                                None
                            }
                        };
                        if let Some((ack, _parsed_message)) = ack {
                            transport
                                .send(BytesMut::from(ack.as_bytes()))
                                .await
                                .wrap_err_with(|| "Failed to send ACK")?;
                        }
                        print_message(message, &mut stdout).wrap_err_with(|| "Failed to print message")?;

                    received_messages += 1;
                    if let Some(message_count) = message_count {
                        if received_messages >= message_count {
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

fn print_message<S: ToString>(message: S, stdout: &mut StandardStream) -> Result<()> {
    use std::io::Write;
    let message = message.to_string().replace('\r', "\n");
    writeln!(stdout, "{}", message).wrap_err_with(|| "Failed to write to stdout")?;
    writeln!(stdout).wrap_err_with(|| "Failed to write to stdout")?;
    Ok(())
}

fn correct_newlines(message: &str) -> String {
    message.replace("\r\n", "\r").replace('\n', "\r")
}
