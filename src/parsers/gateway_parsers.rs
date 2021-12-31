use crate::{parsers::ParsedMessage, LogParseError, SendStatus};
use nom::branch::alt;
use nom::bytes::complete::tag;
use std::collections::HashMap;

//look up table helper for converting subsystem into human readable name
lazy_static! {
    static ref SS_LUT: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();
        m.insert("IMQ", "MQTT-In");
        m.insert("RFC", "Rcv");
        m.insert("RMQ", "MQTT-Reconnect");
        m.insert("TIN", "XportInit");
        m.insert("TPC", "XportConnect");
        m.insert("TPS", "XportSend");
        m.insert("TRC", "ReadFromClient");
        m.insert("TSA", "XportAvail");
        m
    };
}

fn parse_subsystem(i: &str) -> nom::IResult<&str, &str, LogParseError> {
    // ugh wanted something to extract keys of the LUT to create tag parsers
    // let mut ss_parser:List = SS_LUT
    //     .keys()
    //     .map(|x| tag::<&str, &str, crate::LogParseError>(*x))
    //     .collect();
    // but need something to convert iter into tuple?
    let (remaining, subsystem) = alt((
        tag("IMQ"),
        tag("RFC"),
        tag("RMQ"),
        tag("TIN"),
        tag("TPC"),
        tag("TPS"),
        tag("TRC"),
        tag("TSA"),
    ))(i)?;
    Ok((remaining, subsystem))
}

//top level gateway parser

pub fn parse_gateway(i: &str) -> nom::IResult<&str, ParsedMessage, LogParseError> {
    //must start with "GWT:"
    let (remaining, _) = tag("GWT:")(i)?;
    let (remaining, subsystem) = parse_subsystem(remaining)?; //FIXME: use newer method
    let (remaining, _) = tag(":")(remaining)?;

    // currently all gateway messages are just passed up as is
    Ok((
        "",
        ParsedMessage {
            send_status: SendStatus::UNKNOWN, //overwritten later with correct status
            system: Some("Gway".to_string()),
            subsystem: Some(SS_LUT.get(&subsystem[..]).unwrap().to_string()),
            msg: remaining.to_string(),
        },
    )) //consume rest and pass input to output as default
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom::error::ErrorKind;

    #[test]
    fn test_parse_subsystem() {
        assert_eq!(parse_subsystem("RFC"), Ok(("", "RFC")));
        assert_eq!(parse_subsystem("TSA"), Ok(("", "TSA")));
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
    fn test_parse_gateway() {
        assert_eq!(
            parse_gateway("GWT:RFC:some message"),
            Ok((
                "",
                ParsedMessage {
                    send_status: SendStatus::UNKNOWN,
                    system: Some("Gway".to_string()),
                    subsystem: Some("Rcv".to_string()),
                    msg: "some message".to_string(),
                }
            ))
        );
        assert_eq!(
            parse_gateway("GWT:TSA:another message"),
            Ok((
                "",
                ParsedMessage {
                    send_status: SendStatus::UNKNOWN,
                    system: Some("Gway".to_string()),
                    subsystem: Some("XportAvail".to_string()),
                    msg: "another message".to_string(),
                }
            ))
        );
    }
}
