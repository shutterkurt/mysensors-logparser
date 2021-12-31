use chrono::{DateTime, Local};
use nom::error::{ErrorKind, ParseError};
use nom::Finish;

#[macro_use]
extern crate lazy_static;

// so that we can use ? to pass up errors
pub type BoxError = std::boxed::Box<dyn std::error::Error + std::marker::Send + std::marker::Sync>;

#[derive(Debug, PartialEq)]
pub enum SendStatus {
    OK,      //if no prepended char
    ERROR,   //if '!'
    UNKNOWN, //if '?'
}

impl std::str::FromStr for SendStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "!" => Ok(SendStatus::ERROR),
            "?" => Ok(SendStatus::UNKNOWN),
            _ => Err(format!("'{}' is not a valid value for SendStatus", s)),
        }
    }
}

// structure to hold results
// not sure if I'll use this but makes it more flexible
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LogLine {
    pub datetime: Option<DateTime<Local>>,
    pub level: Option<std::string::String>,
    pub msg: std::string::String, //TODO: break down message further if possible
}

impl Default for LogLine {
    fn default() -> Self {
        LogLine {
            datetime: Some(Local::now()),
            level: Some("INFO".to_string()),
            msg: "".to_string(),
        }
    }
}

impl std::fmt::Display for LogLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        //FIXME: expand into more human readable and not just an echo of the input
        let date: String = match self.datetime {
            Some(d) => d.format("%F %H:%M:%S").to_string(),
            None => "".to_string(),
        };
        let level: String = match &self.level {
            Some(l) => l.to_string(),
            None => "".to_string(),
        };

        if self.datetime == None || self.level == None {
            //just write the message - as there was an error in parsing the input
            write!(f, "{}", self.msg)
        } else {
            write!(f, "{} {:6} {}", date, level, self.msg)
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum LogParseError {
    DateTimeError,
    SubSystemError(String, String),
    Nom(String, ErrorKind),
}

impl ParseError<&str> for LogParseError {
    fn from_error_kind(input: &str, kind: ErrorKind) -> Self {
        LogParseError::Nom(input.to_string(), kind)
    }

    fn append(_: &str, _: ErrorKind, other: Self) -> Self {
        other //FIXME: finish
    }
}

use std::fmt;

impl fmt::Display for LogParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LogParseError::DateTimeError => write!(f, "{}", "LogParseError: bad date/time"),
            LogParseError::SubSystemError(found, expected) => write!(
                f,
                "LogParseError: bad subsystem found ({}) expected ({})",
                found, expected
            ),
            LogParseError::Nom(found, kind) => {
                write!(f, "LogParseError: bad parse ({}) ({:?})", found, kind)
            }
        }
    }
}

impl std::error::Error for LogParseError {
    fn description(&self) -> &str {
        "LogParserError" //FIXME: add error text
    }
    //FIXME: add other Error trait fn's
}

pub fn parse_log_line(i: &str) -> LogLine {
    let result = parsers::parse_log(i).finish();
    match result {
        Ok((_, ll)) => return ll,
        Err(_) => {
            //THINK: return error or default LogLine swallowing parse error to caller??
            return LogLine {
                datetime: None,
                level: None,
                msg: i.to_string(),
            };
        } //was Err(e) => return Err(BoxError::from(e)),
    };

    // enum `nom::Err<LogParseError<&str>>`
}

// pub(self) allows other methods within this file to use this module but not externally
pub(self) mod parsers {
    use super::*;
    use chrono::{Datelike, NaiveTime};
    use nom::branch::alt;
    use nom::bytes::complete::tag;
    use nom::character::complete::alpha1;
    use nom::character::complete::space1;
    use std::collections::HashMap;

    mod core_parsers;
    mod gateway_parsers;
    mod xport_function_parsers;
    mod xport_machine_parsers;

    //TODO: consider breaking up the parse_datetime function into smaller items
    // fn parse_month(i: &str) -> nom::IResult<&str, i32, LogParseError<&str>> {}
    // or is it u32s?  see below
    // fn parse_day(i: &str) -> nom::IResult<&str, u32, LogParseError<&str>> {}
    // fn parse_time(i &str) -> nom::IResult<$str, NaiveTime, LogParseError<&str>> {}

    fn parse_datetime(i: &str) -> nom::IResult<&str, DateTime<Local>, LogParseError> {
        // stupid look up table helper
        let month_lut = HashMap::from([
            ("Jan", 1),
            ("Feb", 2),
            ("Mar", 3),
            ("Apr", 4),
            ("May", 5),
            ("Jun", 6),
            ("Jul", 7),
            ("Aug", 8),
            ("Sep", 9),
            ("Oct", 10),
            ("Nov", 11),
            ("Dec", 12),
        ]);

        // parse the date time
        // Oct 18 13:34:33
        // first get/take the first 3 chars from input for the Month,
        let (remaining, month_str) = nom::bytes::complete::take(3usize)(i)?;
        //convert to a number month num
        let month = match month_lut.get(month_str) {
            None => 0, //let the date function fail as out of bounds
            Some(m) => *m,
        };
        let (remaining, _) = nom::character::complete::one_of(" ")(remaining)?;
        let (remaining, daystr) = nom::bytes::complete::take(2usize)(remaining)?;
        let day = match daystr.parse::<u32>() {
            Ok(d) => d,
            Err(_) => 0u32, //Let the date function fail as out of bounds
        };
        // create a Date from month, day and current year
        let date = chrono::offset::Local::today();
        let date = match date.with_month(month) {
            Some(d) => d,
            None => chrono::offset::Local::today(), //FIXME: not the right thing to do!
        };
        let date = match date.with_day(day) {
            Some(d) => d,
            None => chrono::offset::Local::today(), //FIXME: not the right thing to do!
        };

        //get the time

        let (remaining, _) = nom::character::complete::one_of(" ")(remaining)?;
        let (remaining, timestr) = nom::bytes::complete::take(8usize)(remaining)?;
        let time = match NaiveTime::parse_from_str(timestr, "%H:%M:%S") {
            Ok(t) => t,
            //FIXME: actually return an error string within DateTimeError!
            Err(_) => return Err(nom::Err::Error(LogParseError::DateTimeError)),
        };

        //finally add time to the date
        let datetime = match date.and_time(time) {
            Some(dt) => dt,
            None => return Err(nom::Err::Error(LogParseError::DateTimeError)),
        };

        // let junk: nom::error::Error<&str> =
        //     nom::error::make_error(remaining, nom::error::ErrorKind::Fail);

        Ok((remaining, datetime))
    }

    //look up table helper for converting system/subsystem into human readable name
    lazy_static! {
        static ref S_SS_LUT: HashMap<&'static str, &'static str> = {
            let mut m = HashMap::new();
            m.insert("GWT:RFC", "Gway:Rcv");
            m.insert("GWT:TSA", "Gway:Xport Avail");
            m.insert("MCO:BGN", "Core:Begin");
            m.insert("TSF:LRT", "Xport:LoadRoutingTable");
            m.insert("TSF:MSG", "Xport:TxMsg");
            m.insert("TSF:SAN", "Xport:Sanity");
            m.insert("TSF:WUR", "Xport:WaitUntilReady");
            m.insert("TSM:READY", "XportSM:Ready");
            m
        };
    }

    #[derive(Debug, PartialEq)]
    pub struct ParsedMessage {
        send_status: SendStatus,
        system: Option<String>,
        subsystem: Option<String>,
        msg: String,
    }

    impl std::fmt::Display for ParsedMessage {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            if self.system == None || self.subsystem == None {
                write!(f, "{}", self.msg)
            } else {
                // let s_ss = S_SS_LUT.get(&s_ss[..]).unwrap();
                write!(
                    f,
                    "{}:{}:{}",
                    self.system.as_ref().unwrap(),
                    self.subsystem.as_ref().unwrap(),
                    self.msg
                )
            }
        }
    }

    fn parse_message(i: &str) -> nom::IResult<&str, ParsedMessage, LogParseError> {
        // for now just parse this as remaining string
        // parse optional first char of message into SendStatus   nom::IResult<&str, &str>
        let result: nom::IResult<&str, &str> = alt((tag("!"), tag("?")))(i);
        let (remaining, status) = match result {
            Ok((input, o)) => (input, (o.parse() as Result<SendStatus, String>).unwrap()), //convert o into SendStatus Enum
            Err(_) => (i, SendStatus::OK), //return a SendStatus::OK
        };

        //alternate of system parsers or remaining as message
        let result = match alt((
            core_parsers::parse_core,
            gateway_parsers::parse_gateway,
            xport_function_parsers::parse_xport_function,
            xport_machine_parsers::parse_xport_machine,
        ))(remaining)
        {
            Ok((_, parsed)) => ParsedMessage {
                send_status: status,
                system: parsed.system,
                subsystem: parsed.subsystem,
                msg: parsed.msg,
            },
            Err(_) => ParsedMessage {
                send_status: status,
                system: None,
                subsystem: None,
                msg: remaining.to_string(),
            },
        };

        Ok(("", result))
    }

    pub fn parse_log(i: &str) -> nom::IResult<&str, LogLine, LogParseError> {
        // parse the whole log line
        //Oct 18 13:36:52 INFO  Protocol version - 2.3.2
        //Oct 18 13:36:52 DEBUG MCO:BGN:INIT GW,CP=RNNGL---,FQ=NA,REL=255,VER=2.3.2
        // log messages are
        // <datetime><sp><level><sp><msg>
        // <datetime> is <MMM><sp><DD><sp><hh>:<mm>:<ss>
        let (remaining, datetime) = parse_datetime(i)?;
        let (remaining, _) = nom::character::complete::one_of(" ")(remaining)?;

        // take while not whitespace for the level then eat remaining whitespace
        let (remaining, level) = alpha1(remaining)?;
        let (remaining, _) = space1(remaining)?;
        // finally parse the message
        let (_, message) = parse_message(remaining)?; //FIXME: if can't parse message just pass up remaining

        Ok((
            "",
            LogLine {
                datetime: Some(datetime),
                level: Some(level.to_string()),
                msg: message.to_string(),
            },
        ))
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use chrono::naive::NaiveDate;
        use chrono::offset::{Local, TimeZone};

        #[test]
        fn test_parse_datetime() {
            assert_eq!(
                parse_datetime("Oct 18 13:36:52"),
                Ok((
                    "",
                    Local
                        .from_local_datetime(&NaiveDate::from_ymd(2021, 10, 18).and_hms(13, 36, 52))
                        .unwrap(),
                ))
            );
            let result_error = parse_datetime("Some Other Text").unwrap_err();
            assert_eq!(
                result_error,
                nom::Err::Error(LogParseError::Nom(
                    "e Other Text".to_string(),
                    ErrorKind::OneOf
                )),
            );
        }

        //Oct 18 13:36:52 INFO  Protocol version - 2.3.2
        //Oct 18 13:36:52 DEBUG MCO:BGN:INIT GW,CP=RNNGL---,FQ=NA,REL=255,VER=2.3.2

        #[test]
        fn test_parse_log() {
            assert_eq!(
                parse_log("Oct 18 13:36:52 INFO  Protocol version - 2.3.2"),
                Ok((
                    "",
                    LogLine {
                        datetime: Some(
                            Local
                                .from_local_datetime(
                                    &NaiveDate::from_ymd(2021, 10, 18).and_hms(13, 36, 52)
                                )
                                .unwrap()
                        ),
                        level: Some("INFO".to_string()),
                        msg: "Protocol version - 2.3.2".to_string(),
                    }
                ))
            );
            assert_eq!(
                parse_log(
                    "Oct 18 13:36:52 DEBUG MCO:BGN:INIT GW,CP=RNNGL---,FQ=NA,REL=255,VER=2.3.2"
                ),
                Ok((
                    "",
                    LogLine {
                        datetime: Some(
                            Local
                                .from_local_datetime(
                                    &NaiveDate::from_ymd(2021, 10, 18).and_hms(13, 36, 52)
                                )
                                .unwrap()
                        ),
                        level: Some("DEBUG".to_string()),
                        msg: "Core:Begin:INIT GW,CP=RNNGL---,FQ=NA,REL=255,VER=2.3.2".to_string(),
                    }
                ))
            );
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
