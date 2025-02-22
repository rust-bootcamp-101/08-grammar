use std::{
    net::{IpAddr, Ipv4Addr},
    str::FromStr,
};

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use winnow::{
    ascii::{digit1, space0},
    combinator::{alt, delimited, separated},
    token::take_until,
    PResult, Parser,
};

#[allow(unused)]
#[derive(Debug)]
enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Head,
    Options,
    Connect,
    Trace,
    Patch,
}

#[allow(unused)]
#[derive(Debug)]
enum HttpProto {
    HTTP1_0,
    HTTP1_1,
    HTTP2_0,
    HTTP3_0,
}

#[allow(unused)]
#[derive(Debug)]
struct NginxLog {
    addr: IpAddr,
    datetime: DateTime<Utc>,
    method: HttpMethod,
    url: String,
    protocol: HttpProto,
    status: u16,
    body_bytes: u64,
    referer: String,
    user_agent: String,
}

fn main() -> Result<()> {
    let s = r#"93.180.71.3 - - [17/May/2015:08:05:32 +0000] "GET /downloads/product_1 HTTP/1.1" 304 0 "-" "Debian APT-HTTP/1.3 (0.8.16~exp12ubuntu10.21)""#;
    let log = parse_nginx_log(s).unwrap();
    println!("{:?}", log);
    Ok(())
}

fn parse_nginx_log(s: &str) -> PResult<NginxLog> {
    let input = &mut (&*s);
    let ip = parse_ip(input)?;
    parse_ignore(input)?;
    parse_ignore(input)?;
    let datetime = parse_datetime(input)?;
    let (method, url, protocol) = parse_http(input)?;
    let status = parse_status(input)?;
    let body_bytes = parse_body_bytes(input)?;
    let referer = parse_quoted_string(input)?;
    let user_agent = parse_quoted_string(input)?;

    Ok(NginxLog {
        addr: ip,
        datetime,
        method,
        url,
        protocol,
        status,
        body_bytes,
        referer,
        user_agent,
    })
}

fn parse_ignore(s: &mut &str) -> PResult<()> {
    "- ".parse_next(s)?;
    Ok(())
}

fn parse_ip(s: &mut &str) -> PResult<IpAddr> {
    let ret: Vec<u8> = separated(4, digit1.parse_to::<u8>(), '.').parse_next(s)?;
    space0(s)?;
    Ok(IpAddr::V4(Ipv4Addr::new(ret[0], ret[1], ret[2], ret[3])))
}

fn parse_datetime(s: &mut &str) -> PResult<DateTime<Utc>> {
    let ret = delimited('[', take_until(1.., ']'), ']').parse_next(s)?;

    space0(s)?;
    Ok(DateTime::parse_from_str(ret, "%d/%b/%Y:%H:%M:%S %z")
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap())
}

fn parse_http(s: &mut &str) -> PResult<(HttpMethod, String, HttpProto)> {
    let parser = (parse_method, parse_url, parse_protocol);
    let ret = delimited('"', parser, '"').parse_next(s)?;
    space0(s)?;
    Ok(ret)
}

fn parse_method(s: &mut &str) -> PResult<HttpMethod> {
    let ret = alt((
        "GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS", "TRACE", "CONNECT",
    ))
    .parse_to()
    .parse_next(s)?;
    space0(s)?;
    Ok(ret)
}

fn parse_url(s: &mut &str) -> PResult<String> {
    let ret = take_until(1.., ' ').parse_next(s)?;
    space0(s)?;
    Ok(ret.to_string())
}

fn parse_protocol(s: &mut &str) -> PResult<HttpProto> {
    let ret = alt(("HTTP/1.0", "HTTP/1.1", "HTTP/2.0", "HTTP/3.0"))
        .parse_to()
        .parse_next(s)?;
    space0(s)?;
    Ok(ret)
}

fn parse_status(s: &mut &str) -> PResult<u16> {
    let ret = digit1.parse_to().parse_next(s)?;
    space0(s)?;
    Ok(ret)
}

fn parse_body_bytes(s: &mut &str) -> PResult<u64> {
    let ret = digit1.parse_to().parse_next(s)?;
    space0(s)?;
    Ok(ret)
}

fn parse_quoted_string(s: &mut &str) -> PResult<String> {
    let ret = delimited('"', take_until(1.., '"'), '"').parse_next(s)?;
    space0(s)?;
    Ok(ret.to_string())
}

impl FromStr for HttpProto {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "HTTP/1.0" => Ok(HttpProto::HTTP1_0),
            "HTTP/1.1" => Ok(HttpProto::HTTP1_1),
            "HTTP/2.0" => Ok(HttpProto::HTTP2_0),
            "HTTP/3.0" => Ok(HttpProto::HTTP3_0),
            _ => Err(anyhow!("Invalid HTTP protocol")),
        }
    }
}

impl FromStr for HttpMethod {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "GET" => Ok(HttpMethod::Get),
            "POST" => Ok(HttpMethod::Post),
            "PUT" => Ok(HttpMethod::Put),
            "DELETE" => Ok(HttpMethod::Delete),
            "HEAD" => Ok(HttpMethod::Head),
            "OPTIONS" => Ok(HttpMethod::Options),
            "CONNECT" => Ok(HttpMethod::Connect),
            "TRACE" => Ok(HttpMethod::Trace),
            "PATCH" => Ok(HttpMethod::Patch),
            _ => Err(anyhow!("Invalid HTTP method")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use chrono::TimeZone;

    #[test]
    fn parse_ip_should_work() -> Result<()> {
        let mut s = "1.1.1.1";
        let ip = parse_ip(&mut s).unwrap();
        assert_eq!(s, "");
        assert_eq!(ip, IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)));
        Ok(())
    }

    #[test]
    fn parse_datetime_should_work() -> Result<()> {
        let mut s = "[04/Jun/2015:01:06:58 +0000]";
        let dt = parse_datetime(&mut s).unwrap();
        assert_eq!(s, "");
        assert_eq!(dt, Utc.with_ymd_and_hms(2015, 6, 4, 1, 6, 58).unwrap());
        Ok(())
    }
}
