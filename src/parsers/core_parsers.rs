use crate::{parsers::ParsedMessage, LogParseError, SendStatus};
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::error::ErrorKind;
use std::collections::HashMap;

//look up table helper for converting subsystem into human readable name
lazy_static! {
    static ref SS_LUT: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();
        m.insert("BGN", "Begin");
        m.insert("NLK", "NodeLock");
        m.insert("PIM", "InternalMsg");
        m.insert("REG", "RegisterNode");
        m.insert("SLP", "Sleep");
        m.insert("SND", "Send");
        m.insert("WAI", "Wait");
        m
    };
}

//look up table helper for converting message into human readable name
lazy_static! {
    static ref MSG_LUT: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();
        m.insert("BFR", "Callback before()");
        m.insert("STP", "Callback setup()");
        m.insert("TSP FAIL", "Xport Init Failed");
        m.insert("TSL", "Xport Sleep");
        m.insert("REQ", "Registration Request");
        m.insert("NTL", "Can't Sleep - no time left");
        m.insert("FWUPD", "Can't Sleep - FW updating");
        m.insert("REP", "Can't Sleep - repeater node");
        m.insert("TNR", "Xport Not Ready - attempting reconnect");
        m
    };
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

fn parse_msg_into_human(i: &str) -> nom::IResult<&str, &str, LogParseError> {
    //FIXME: finish imple
    Err(nom::Err::Error(crate::LogParseError::Nom(
        "JUNK".to_string(),
        ErrorKind::Tag,
    )))
}

fn parse_subsystem(i: &str) -> nom::IResult<&str, &str, LogParseError> {
    // ugh wanted something to extract keys of the LUT to create tag parsers
    // let mut ss_parser:List = SS_LUT
    //     .keys()
    //     .map(|x| tag::<&str, &str, crate::LogParseError>(*x))
    //     .collect();
    // but need something to convert iter into tuple?
    let (remaining, subsystem) = alt((
        tag("BGN"),
        tag("NLK"),
        tag("PIM"),
        tag("REG"),
        tag("SLP"),
        tag("SND"),
        tag("WAI"),
    ))(i)?;
    Ok((remaining, subsystem))
}

//top level core parser

pub fn parse_core(i: &str) -> nom::IResult<&str, ParsedMessage, LogParseError> {
    //must start with "MCO:"
    let (remaining, _) = tag("MCO:")(i)?;
    let (remaining, subsystem) = parse_subsystem(remaining)?; //FIXME: use newer method
    let (remaining, _) = tag(":")(remaining)?;

    //either try to parse as human readable message (most strict)
    //or try to parse via simple message convert via lookup
    //or just expand system/subsystem and leave message as is
    let result = match alt((parse_msg_into_human, parse_msg_by_lookup))(remaining) {
        Ok((_, converted)) => ParsedMessage {
            send_status: SendStatus::UNKNOWN,
            system: Some("Core".to_string()),
            subsystem: Some(SS_LUT.get(&subsystem[..]).unwrap().to_string()),
            msg: converted.to_string(),
        },
        Err(_) => ParsedMessage {
            send_status: SendStatus::UNKNOWN,
            system: Some("Core".to_string()),
            subsystem: Some(SS_LUT.get(&subsystem[..]).unwrap().to_string()),
            msg: remaining.to_string(),
        },
    };

    // let result = format!("Core:{}:{}", SS_LUT.get(&subsystem[..]).unwrap(), remaining);

    Ok(("", result)) //consume rest and pass input to output as default
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom::error::ErrorKind;

    #[test]
    fn test_parse_subsystem() {
        assert_eq!(parse_subsystem("PIM"), Ok(("", "PIM")));
        assert_eq!(parse_subsystem("SLP"), Ok(("", "SLP")));
        let result_error = parse_subsystem("JUNK").unwrap_err();
        assert_eq!(
            result_error,
            nom::Err::Error(crate::LogParseError::Nom(
                "JUNK".to_string(),
                ErrorKind::Tag
            )),
        );
    }

    #[test]
    fn test_parse_core() {
        assert_eq!(
            parse_core("MCO:PIM:some message"),
            Ok((
                "",
                ParsedMessage {
                    send_status: SendStatus::UNKNOWN,
                    system: Some("Core".to_string()),
                    subsystem: Some("InternalMsg".to_string()),
                    msg: "some message".to_string(),
                }
            ))
        );
        assert_eq!(
            parse_core("MCO:WAI:another message"),
            Ok((
                "",
                ParsedMessage {
                    send_status: SendStatus::UNKNOWN,
                    system: Some("Core".to_string()),
                    subsystem: Some("Wait".to_string()),
                    msg: "another message".to_string(),
                }
            ))
        );
    }
}
