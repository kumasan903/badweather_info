use chrono::{DateTime, Datelike, Duration, TimeZone, Utc, Timelike};
use std::collections::HashMap;
use std::env;
use std::num::ParseIntError;

#[derive(Debug)]
struct MetarTime {
    date: u8,
    hour: u8,
    minute: u8,
}

impl MetarTime {
    fn to_datetime(&self, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
        let year = now.year();
        let month = now.month();
        if self.date as u32 > now.day() {
            let prev_month = if month == 1 { 12 } else { month - 1 };
            let prev_year = if month == 1 { year - 1 } else { year };
            Utc.with_ymd_and_hms(
                prev_year,
                prev_month,
                self.date as u32,
                self.hour as u32,
                self.minute as u32,
                0,
            )
            .single()
        } else {
            Utc.with_ymd_and_hms(
                year,
                month,
                self.date as u32,
                self.hour as u32,
                self.minute as u32,
                0,
            )
            .single()
        }
    }

    fn from(dt: DateTime<Utc>) -> Self {
        MetarTime {
            date: dt.day() as u8,
            hour: dt.hour() as u8,
            minute: dt.minute() as u8,
        }
    }
}

#[derive(Debug)]
struct AirportWeather {
    time: MetarTime,
    wind_speed_kt: u8,
    wind_gust_speed_kt: Option<u8>,
    visibility_m: u16,
    overcast_ceiling_ft: Option<u16>,
    vertical_visibility_ft: Option<u16>,
    raw_metar: String,
}

async fn send_webhook(message: &str) -> Result<reqwest::Response, reqwest::Error> {
    let mut request_body = HashMap::new();
    request_body.insert("content", message);
    let webhook_url = env::var("WEBHOOK_URL").expect("Failed to read WEBHOOK_URL");
    let client = reqwest::Client::new();
    client.post(webhook_url).json(&request_body).send().await
}

fn old(time: &MetarTime) -> bool {
    let now = Utc::now();
    if let Some(dt) = time.to_datetime(now) {
        now - Duration::hours(2) > dt
    } else {
        true
    }
}

async fn get_metars() -> Result<Vec<String>, reqwest::Error> {
    let body = reqwest::get("https://metar.vatsim.net/RJFK,RJOB,RJSN,RJFU,RJOM,RJOO,RJFO,RJFF,RJCB,RJOS,RJSK,RJTT,RJFT,RJGG,RJAA,RJSS,RJOW,RJCH,RJAF,RJOA,RJCO,RJCK,RJTF,RJCW,RJBD,RJOI,RJCN,RJOT,RJNS,RJOR,RJBE,RJEC,RJOK,RJFM,RJNK,RJCC,RJKA,RJBB,RJCM,ROIG,ROMY,ROAH")
        .await?
        .text()
        .await?;
    Ok(body.split('\n').map(|s| s.to_string()).collect())
}

fn parse_time(metar: &[&str]) -> Result<MetarTime, ParseIntError> {
    let time = metar[1];
    Ok(MetarTime {
        date: time[0..2].parse::<u8>()?,
        hour: time[2..4].parse::<u8>()?,
        minute: time[4..6].parse::<u8>()?,
    })
}

fn parse_wind_speed(metar: &[&str]) -> Result<u8, ParseIntError> {
    let wind_str = metar[2];
    let wind_speed_kt: &str = if wind_str.ends_with("KT") && wind_str != "/////KT" {
        &wind_str[3..5]
    } else {
        "0"
    };
    wind_speed_kt.parse::<u8>()
}

fn parse_wind_gust_speed(metar: &[&str]) -> Option<u8> {
    let wind_str = metar[2];
    let wind_gust_speed_kt: Option<&str> =
    if wind_str.ends_with("KT") && wind_str != "/////KT" && wind_str.chars().nth(5) == Some('G')
    {
        Some(&wind_str[6..8])
    } else {
        None
    };
    wind_gust_speed_kt.and_then(|s| s.parse::<u8>().ok())
}

fn parse_visibility(metar: &[&str]) -> Result<u16, ParseIntError> {
    let vis_str = metar[3];
    let visibility_m = if vis_str.len() == 4
        && vis_str != "////"
        && !vis_str.ends_with("SM")
        && vis_str != "CAVOK"
    {
        vis_str
    } else {
        "9999"
    };
    visibility_m.parse::<u16>()
}

fn parse_ceiling(raw_metar: &str) -> Option<u16> {
    if raw_metar.contains("OVC000") {
        Some(0)
    } else if raw_metar.contains("OVC001") {
        Some(100)
    } else if raw_metar.contains("OVC002") {
        Some(200)
    } else if raw_metar.contains("OVC003") {
        Some(300)
    } else {
        None
    }
}

fn parse_vertical_visibility(raw_metar: &str) -> Option<u16> {
    if raw_metar.contains("VV000") {
        Some(0)
    } else if raw_metar.contains("VV001") {
        Some(100)
    } else if raw_metar.contains("VV002") {
        Some(200)
    } else if raw_metar.contains("VV003") {
        Some(300)
    } else {
        None
    }
}

fn parse_metar(raw_metar: String) -> AirportWeather {
    let mut metar: Vec<&str> = raw_metar.split_whitespace().collect();
    if metar[2] == "AUTO" {
        metar.remove(2);
    }
    // Wind Direction Variable
    if metar[3].len() == 7 {
        metar.remove(3);
    }
    AirportWeather {
        time : parse_time(&metar).unwrap_or(MetarTime::from(Utc::now())),
        wind_speed_kt : parse_wind_speed(&metar).unwrap_or(0),
        wind_gust_speed_kt : parse_wind_gust_speed(&metar),
        visibility_m : parse_visibility(&metar).unwrap_or(9999),
        overcast_ceiling_ft : parse_ceiling(&raw_metar),
        vertical_visibility_ft : parse_vertical_visibility(&raw_metar),
        raw_metar,
    }
}

fn strong_wind(airport_weather: &AirportWeather) -> bool {
    airport_weather.wind_speed_kt > 30
        || matches!(airport_weather.wind_gust_speed_kt, Some(x) if x > 45)
}

fn low_visibility(airport_weather: &AirportWeather) -> bool {
    airport_weather.visibility_m < 500
}

fn low_ceiling(airport_weather: &AirportWeather) -> bool {
    matches!(airport_weather.overcast_ceiling_ft, Some(x) if x <= 200)
        || matches!(airport_weather.vertical_visibility_ft, Some(x) if x <= 200)
}

fn bad_weather(airport_weather: &AirportWeather) -> bool {
    if old(&airport_weather.time) {
        return false;
    }
    if strong_wind(airport_weather) || low_visibility(airport_weather) {
        return true;
    }
    if low_ceiling(airport_weather) {
        return airport_weather.visibility_m < 8000
            || airport_weather.overcast_ceiling_ft == Some(0);
    }
    false
}

#[tokio::main]
async fn main() {
    let metars = get_metars().await.expect("Failed to get METAR");
    let mut result = String::new();
    for metar_raw in metars {
        let airport_weather = parse_metar(metar_raw);
        if bad_weather(&airport_weather) {
            result = result + airport_weather.raw_metar.as_str() + "\n";
            println!("{:?}", airport_weather);
        }
    }
    send_webhook(result.as_str()).await.expect("Failed to send webhook");
    println!("{}", result);
}
