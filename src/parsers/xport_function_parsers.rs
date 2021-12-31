use crate::{parsers::ParsedMessage, LogParseError, SendStatus};
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::bytes::complete::take;
use nom::error::ErrorKind;
use std::collections::HashMap;

//look up table helper for converting subsystem into human readable name
lazy_static! {
    static ref SS_LUT: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();
        m.insert("CKU", "CheckUL");
        m.insert("LRT", "LoadRoutingTable");
        m.insert("MSG", "Msg");
        m.insert("PNG", "Ping");
        m.insert("RRT", "Route");
        m.insert("RTE", "Route");
        m.insert("SAN", "Sanity");
        m.insert("SID", "AssignNode");
        m.insert("SIR", "SignalReport");
        m.insert("SND", "RouteSend");
        m.insert("SRT", "SaveRoutingTable");
        m.insert("TDI", "Disable");
        m.insert("TRI", "ReInit");
        m.insert("UPL", "PingGW");
        m.insert("WUR", "WaitUntilReady");
        m
    };
}

lazy_static! {
    static ref MSG_LUT: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();
        m.insert("OK,FCTRL", "UL OK - ping filtered, interval too short");
        m.insert("FAIL", "No Reply recieved");
        m.insert("FPAR PREF FOUND", "Found Preferred parent - static ID");
        m.insert("FPAR INACTIVE", "rvd FindParent Response but no request");
        m.insert("BC", "Broadcast Message Recieved");
        m.insert("GWL OK", "Link to GW OK");
        m.insert("FWD BC MSG", "Controlled Broadcast Msg Fowarding");
        m.insert("RCV CB", "call Receive Callback()");
        m.insert("REL MSG", "Relay Message");
        m.insert("REL MSG,NORP", "Relay Message but NOT a repeater");
        m.insert("SIGN FAIL", "Signing Message Failed");
        m.insert("GWL FAIL", "GW UL Failed");
        m.insert("ID TK INVALID", "Token for ID Request Invalid");
        m.insert("FPAR ACTIVE", "Finding Parent Active, message not sent");
        m.insert("TNR", "Xport Not Ready, message not sent");
        m.insert("TSL", "Xport Sleep");
        m.insert("TPD", "Xport PowerDown");
        m.insert("TRI", "Xport ReInit");
        m.insert("TSB", "Xport Standby");
        m
    };
}

fn parse_msg_into_human(i: &str) -> nom::IResult<&str, &str, crate::LogParseError> {
    //FIXME: finish imple
    Err(nom::Err::Error(crate::LogParseError::Nom(
        "Not Implemented Yet".to_string(),
        ErrorKind::Tag,
    )))
}

fn parse_msg_by_lookup(i: &str) -> nom::IResult<&str, &str, crate::LogParseError> {
    match MSG_LUT.get(i) {
        Some(new_msg) => return Ok(("", new_msg)),
        None => {
            return Err(nom::Err::Error(crate::LogParseError::Nom(
                "Msg not in LUT".to_string(),
                ErrorKind::Tag,
            )))
        }
    };
}

//top level trasnport function parser

pub fn parse_xport_function(i: &str) -> nom::IResult<&str, ParsedMessage, LogParseError> {
    //must start with "TSF:"
    let (remaining, _) = tag("TSF:")(i)?;
    //get the subsystem - always 3 chars
    let (remaining, subsystem) = take(3u32)(remaining)?;
    //now check to see if subsystem is one of the keys above
    if !SS_LUT.contains_key(subsystem) {
        // subsystem is NOT defined - return an error by wrapping a LogParseError in a nom error type:
        return Err(nom::Err::Error(crate::LogParseError::SubSystemError(
            subsystem.to_string(),
            format!("{:?}", SS_LUT.keys()),
        )));
    }

    let (remaining, _) = tag(":")(remaining)?;

    //either try to parse as human readable message (most strict)
    //or try to parse via simple message convert via lookup
    //or just expand system/subsystem and leave message as is
    let result = match alt((parse_msg_into_human, parse_msg_by_lookup))(remaining) {
        Ok((_, converted)) => ParsedMessage {
            send_status: SendStatus::UNKNOWN,
            system: Some("Xport".to_string()),
            subsystem: Some(SS_LUT.get(&subsystem[..]).unwrap().to_string()),
            msg: converted.to_string(),
        },
        Err(_) => ParsedMessage {
            send_status: SendStatus::UNKNOWN,
            system: Some("Xport".to_string()),
            subsystem: Some(SS_LUT.get(&subsystem[..]).unwrap().to_string()),
            msg: remaining.to_string(),
        },
    };

    Ok(("", result)) //consume rest and pass input to output as default
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom::error::ErrorKind;

    #[test]
    fn test_parse_xport_function() {
        assert_eq!(
            parse_xport_function("TSF:UPL:some message"),
            Ok((
                "",
                ParsedMessage {
                    send_status: SendStatus::UNKNOWN,
                    system: Some("Xport".to_string()),
                    subsystem: Some("PingGW".to_string()),
                    msg: "some message".to_string(),
                }
            ))
        );
        assert_eq!(
            parse_xport_function("TSF:RTE:another message"),
            Ok((
                "",
                ParsedMessage {
                    send_status: SendStatus::UNKNOWN,
                    system: Some("Xport".to_string()),
                    subsystem: Some("Route".to_string()),
                    msg: "another message".to_string(),
                }
            ))
        );

        let result_error = parse_xport_function("JUNK").unwrap_err();
        assert_eq!(
            result_error,
            nom::Err::Error(crate::LogParseError::Nom(
                "JUNK".to_string(),
                ErrorKind::Tag
            )),
        );
    }
}
