use crate::cli::AckMode;
use chrono::Utc;
use color_eyre::{eyre::Context, Result};
use hl7_parser::ParsedMessage;

pub fn generate_ack(message: &str, ack_mode: AckMode) -> Result<(String, ParsedMessage)> {
    match ack_mode {
        AckMode::Success => {
            let message =
                ParsedMessage::parse(message, false).wrap_err_with(|| "Failed to parse message")?;
            Ok((
                compose_ack(&message, true).wrap_err_with(|| "Failed to compose ACK")?,
                message,
            ))
        }
        AckMode::Error => {
            let message =
                ParsedMessage::parse(message, false).wrap_err_with(|| "Failed to parse message")?;
            Ok((
                compose_ack(&message, false).wrap_err_with(|| "Failed to compose ACK")?,
                message,
            ))
        }
        AckMode::Ignore => Err(color_eyre::eyre::eyre!("ACK mode is set to Ignore")),
    }
}

fn compose_ack(message: &ParsedMessage, success: bool) -> Result<String> {
    let accept_ack = message
        .query_value("MSH.15")
        .expect("valid query")
        .unwrap_or_default();
    let application_ack = message
        .query_value("MSH.16")
        .expect("valid query")
        .unwrap_or_default();

    let accept_ack: Option<AckRequest> =
        AckRequest::from_str(accept_ack).wrap_err_with(|| "Failed to parse accept ACK")?;
    let application_ack: Option<AckRequest> = AckRequest::from_str(application_ack)
        .wrap_err_with(|| "Failed to parse application ACK")?;

    let is_enhanced_mode = accept_ack.is_some() || application_ack.is_some();
    let ack_level = if is_enhanced_mode { 'C' } else { 'A' };

    let control_id = message
        .query_value("MSH.10")
        .expect("valid query")
        .unwrap_or_default();

    let sapp = message
        .query_value("MSH.3")
        .expect("valid query")
        .unwrap_or_default();
    let sfac = message
        .query_value("MSH.4")
        .expect("valid query")
        .unwrap_or_default();
    let rapp = message
        .query_value("MSH.5")
        .expect("valid query")
        .unwrap_or_default();
    let rfac = message
        .query_value("MSH.6")
        .expect("valid query")
        .unwrap_or_default();
    let processing_id = message
        .query_value("MSH.11")
        .expect("valid query")
        .unwrap_or_default();
    let version = message
        .query_value("MSH.12")
        .expect("valid query")
        .unwrap_or_default();
    let trigger = message
        .query_value("MSH.9.2")
        .expect("valid query")
        .unwrap_or_default();

    use rand::distributions::{Alphanumeric, DistString};
    let new_control_id = Alphanumeric.sample_string(&mut rand::thread_rng(), 20);

    let now = Utc::now();
    let now = now.format("%Y%m%d%H%M%S").to_string();

    let msh = format!(
        "MSH|^~\\&|{rapp}|{rfac}|{sapp}|{sfac}|{now}||ACK^{trigger}^ACK|{new_control_id}|{processing_id}|{version}",
    );

    let msa = format!(
        "MSA|{ack_level}{success}|{control_id}|{error_message}",
        success = if success { 'A' } else { 'E' },
        error_message = if success {
            "Message accepted"
        } else {
            "Message rejected"
        },
    );

    Ok(format!("{}\r{}", msh, msa))
}

#[derive(Debug, Copy, Clone)]
enum AckRequest {
    Always,
    Never,
    Success,
    Error,
}

impl AckRequest {
    fn from_str(s: &str) -> Result<Option<AckRequest>> {
        match s {
            "AL" => Ok(Some(AckRequest::Always)),
            "NE" => Ok(Some(AckRequest::Never)),
            "SU" => Ok(Some(AckRequest::Success)),
            "ER" => Ok(Some(AckRequest::Error)),
            "" => Ok(None),
            _ => Err(color_eyre::eyre::eyre!("Invalid ACK request")),
        }
    }
}
