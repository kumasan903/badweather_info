use chrono::{Datelike, Duration, TimeZone, Utc};
use std::collections::HashMap;
use std::env;

#[derive(Debug)]
struct MetarTime {
    date: u8,
    hour: u8,
    minute: u8,
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

async fn send_webhook(str: &str) -> Result<reqwest::Response, reqwest::Error> {
    let mut request_body = HashMap::new();
    request_body.insert("content", str);
    let webhook_url = env::var("WEBHOOK_URL").expect("Failed to read WEBHOOK_URL");
    let client = reqwest::Client::new();
    client.post(webhook_url).json(&request_body).send().await
}

fn old(time: &MetarTime) -> bool {
    let dt = if time.date as u32 > Utc::now().day() {
        return true;
    } else {
        Utc.with_ymd_and_hms(
            Utc::now().year(),
            Utc::now().month(),
            time.date as u32,
            time.hour as u32,
            time.minute as u32,
            0,
        )
        .unwrap()
    };
    if Utc::now() - Duration::hours(2) > dt {
        return true;
    }
    false
}

async fn get_metars() -> Result<Vec<String>, reqwest::Error> {
    let body = reqwest::get("https://metar.vatsim.net/RJFK,RJOB,RJSN,RJFU,RJOM,RJOO,RJFO,RJFF,RJCB,RJOS,RJSK,RJTT,RJFT,RJGG,RJAA,RJSS,RJOW,RJCH,RJAF,RJOA,RJCO,RJCK,RJTF,RJCW,RJBD,RJOI,RJCN,RJOT,RJNS,RJOR,RJBE,RJEC,RJOK,RJFM,RJNK,RJCC,RJKA,RJBB,RJCM,ROIG,ROMY,ROAH")
        .await?
        .text()
        .await?;
    Ok(body.split('\n').map(|s| s.to_string()).collect())
}

fn parse_metar(raw_metar: String) -> AirportWeather {
    let mut metar: Vec<&str> = raw_metar.split_whitespace().collect();
    let time = metar[1];
    let time = MetarTime {
        date: time[0..2].parse::<u8>().unwrap(),
        hour: time[2..4].parse::<u8>().unwrap(),
        minute: time[4..6].parse::<u8>().unwrap(),
    };
    if metar[2] == "AUTO" {
        metar.remove(2);
    }
    let wind_str = metar[2];
    // Wind Direction Variable
    if metar[3].len() == 7 {
        metar.remove(3);
    }
    let vis_str = metar[3];
    let wind_speed_kt: &str = if wind_str.ends_with("KT") && wind_str != "/////KT" {
        &wind_str[3..5]
    } else {
        "0"
    };
    let wind_gust_speed_kt: Option<&str> =
        if wind_str.ends_with("KT") && wind_str != "////KT" && wind_str.chars().nth(5) == Some('G')
        {
            Some(&wind_str[6..8])
        } else {
            None
        };
    let visibility_m = if vis_str.len() == 4
        && vis_str != "////"
        && !vis_str.ends_with("SM")
        && vis_str != "CAVOK"
    {
        vis_str
    } else {
        "9999"
    };
    let wind_speed_kt: u8 = wind_speed_kt.parse().unwrap();
    let wind_gust_speed_kt: Option<u8> = wind_gust_speed_kt.and_then(|s| s.parse::<u8>().ok());
    let visibility_m: u16 = visibility_m.parse().unwrap();
    let overcast_ceiling_ft: Option<u16> = if raw_metar.contains("OVC000") {
        Some(0)
    } else if raw_metar.contains("OVC001") {
        Some(100)
    } else if raw_metar.contains("OVC002") {
        Some(200)
    } else if raw_metar.contains("OVC003") {
        Some(300)
    } else {
        None
    };
    let vertical_visibility_ft: Option<u16> = if raw_metar.contains("VV000") {
        Some(0)
    } else if raw_metar.contains("VV001") {
        Some(100)
    } else if raw_metar.contains("VV002") {
        Some(200)
    } else if raw_metar.contains("VV003") {
        Some(300)
    } else {
        None
    };
    AirportWeather {
        time,
        wind_speed_kt,
        wind_gust_speed_kt,
        visibility_m,
        overcast_ceiling_ft,
        vertical_visibility_ft,
        raw_metar: raw_metar.to_string(),
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

#[tokio::main]
async fn main() {
    let metars = get_metars().await.unwrap();
    let mut result = String::new();
    for metar_raw in metars {
        let airport_weather = parse_metar(metar_raw);
        if !old(&airport_weather.time)
            && (strong_wind(&airport_weather)
                || low_visibility(&airport_weather)
                || low_ceiling(&airport_weather))
        {
            result = result + airport_weather.raw_metar.as_str() + "\n";
            println!("{:?}", airport_weather);
        }
    }
    send_webhook(result.as_str()).await.unwrap();
    println!("{}", result);
}
