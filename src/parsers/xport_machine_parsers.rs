use crate::{parsers::ParsedMessage, LogParseError, SendStatus};
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::bytes::complete::{take_until, take_while};
use nom::character::complete::digit1;
use nom::character::is_alphabetic;
use nom::error::ErrorKind;
use std::collections::HashMap;

//look up table helper for converting subsystem into human readable name
lazy_static! {
    static ref SS_LUT: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();
        m.insert("FAIL", "FAIL");
        m.insert("FPAR", "FindParent");
        m.insert("ID", "ID");
        m.insert("INIT", "INIT");
        m.insert("READY", "READY");
        m.insert("UPL", "UPLINK");
        m
    };
}

lazy_static! {
    static ref MSG_LUT: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();
        m.insert("DIS", "Xport Disable");
        m.insert("TSP OK", "Xport Configured & Fully Operational");
        m.insert("TSP PSM", "Xport PassiveMode set");
        m.insert("TSP FAIL", "Xport Init Failed");
        m.insert("SRT", "Save Routing Table");
        m.insert("UPL FAIL,SNP", "Fail count exceeded - search new parent");
        m.insert("FAIL,STATP", "Fail count exceeded - static parent enforced");
        m.insert("OK", "UL OK, GW returned ping");
        m.insert("FAIL", "UL Check FAILED - GW Ping Failed");
        m.insert("NWD REQ", "Send xport network discovery request");
        m
    };
}

fn parse_id_verification_failed(i: &str) -> nom::IResult<&str, String, LogParseError> {
    // FAIL,ID=%d
    let (remaining, _) = tag("FAIL,ID=")(i)?;
    let (remaining, id) = digit1(remaining)?;
    let msg = format!(
        "ID ({}) invalid / verification failed / no ID received from controller",
        id
    );

    Ok((remaining, msg))
}

fn parse_static_id(i: &str) -> nom::IResult<&str, String, LogParseError> {
    // STATID=%d
    let (remaining, _) = tag("STATID=")(i)?;
    let (remaining, id) = digit1(remaining)?;
    let msg = format!("Static ID ({})", id);

    Ok((remaining, msg))
}

fn parse_transition_ready(i: &str) -> nom::IResult<&str, String, LogParseError> {
    // ID=%d,PAR=%d,DIS=%d
    let (remaining, _) = tag("ID=")(i)?;
    let (remaining, id) = digit1(remaining)?;
    let (remaining, _) = tag(",PAR=")(remaining)?;
    let (remaining, parent) = digit1(remaining)?;
    let (remaining, _) = tag(",DIS=")(remaining)?;
    let (remaining, distance) = digit1(remaining)?;

    let msg = format!(
        "READY: node ID ({}) parent ID ({}) GW distance ({})",
        id, parent, distance
    );

    Ok((remaining, msg))
}

fn parse_msg_into_human(i: &str) -> nom::IResult<&str, String, LogParseError> {
    alt((
        parse_id_verification_failed,
        parse_static_id,
        parse_transition_ready,
    ))(i)
}

fn parse_msg_by_lookup(i: &str) -> nom::IResult<&str, String, crate::LogParseError> {
    match MSG_LUT.get(i) {
        Some(new_msg) => return Ok(("", new_msg.to_string())),
        None => {
            return Err(nom::Err::Error(crate::LogParseError::Nom(
                "Msg not in LUT".to_string(),
                ErrorKind::Tag,
            )))
        }
    };
}

fn is_char_alphabetic(chr: char) -> bool {
    return chr.is_ascii() && is_alphabetic(chr as u8);
}

//top level trasnport state machine parser

pub fn parse_xport_machine(i: &str) -> nom::IResult<&str, ParsedMessage, LogParseError> {
    //must start with "TSF:"
    let (remaining, _) = tag("TSM:")(i)?;
    //get the subsystem - in this case, read until the ':'
    // let (remaining, subsystem) = take_until(":")(remaining)?;
    let (remaining, subsystem) = alt((take_until(":"), take_while(is_char_alphabetic)))(remaining)?;
    //now check to see if subsystem is one of the keys above
    if !SS_LUT.contains_key(subsystem) {
        // subsystem is NOT defined - return an error by wrapping a LogParseError in a nom error type:
        return Err(nom::Err::Error(LogParseError::SubSystemError(
            subsystem.to_string(),
            format!("{:?}", SS_LUT.keys()),
        )));
    }

    // handle the case if there is no message remaining
    let final_msg: &str;
    if !remaining.is_empty() {
        let (remaining, _) = tag(":")(remaining)?;
        final_msg = remaining;
    } else {
        final_msg = "State Transition";
    }

    //either try to parse as human readable message (most strict)
    //or try to parse via simple message convert via lookup
    //or just expand system/subsystem and leave message as is
    let result = match alt((parse_msg_into_human, parse_msg_by_lookup))(final_msg) {
        Ok((_, converted)) => ParsedMessage {
            send_status: SendStatus::UNKNOWN,
            system: Some("XportSM".to_string()),
            subsystem: Some(SS_LUT.get(&subsystem[..]).unwrap().to_string()),
            msg: converted.to_string(),
        },
        Err(_) => ParsedMessage {
            send_status: SendStatus::UNKNOWN,
            system: Some("XportSM".to_string()),
            subsystem: Some(SS_LUT.get(&subsystem[..]).unwrap().to_string()),
            msg: final_msg.to_string(),
        },
    };

    Ok(("", result)) //consume rest and pass input to output as default
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom::error::ErrorKind;

    #[test]
    fn test_parse_xport_machine() {
        assert_eq!(
            parse_xport_machine("TSM:UPL:some message"),
            Ok((
                "",
                ParsedMessage {
                    send_status: SendStatus::UNKNOWN,
                    system: Some("XportSM".to_string()),
                    subsystem: Some("UPLINK".to_string()),
                    msg: "some message".to_string(),
                }
            ))
        );
        assert_eq!(
            parse_xport_machine("TSM:INIT:another message"),
            Ok((
                "",
                ParsedMessage {
                    send_status: SendStatus::UNKNOWN,
                    system: Some("XportSM".to_string()),
                    subsystem: Some("INIT".to_string()),
                    msg: "another message".to_string(),
                }
            ))
        );

        assert_eq!(
            parse_xport_machine("TSM:INIT"),
            Ok((
                "",
                ParsedMessage {
                    send_status: SendStatus::UNKNOWN,
                    system: Some("XportSM".to_string()),
                    subsystem: Some("INIT".to_string()),
                    msg: "".to_string(),
                }
            ))
        );

        assert_eq!(
            parse_xport_machine("TSM:INIT:TSP FAIL"),
            Ok((
                "",
                ParsedMessage {
                    send_status: SendStatus::UNKNOWN,
                    system: Some("XportSM".to_string()),
                    subsystem: Some("INIT".to_string()),
                    msg: "Xport Init Failed".to_string(),
                }
            ))
        );

        let result_error = parse_xport_machine("JUNK").unwrap_err();
        assert_eq!(
            result_error,
            nom::Err::Error(crate::LogParseError::Nom(
                "JUNK".to_string(),
                ErrorKind::Tag
            )),
        );
    }
}
