use chrono::{Datelike, Duration, TimeZone, Utc};
use std::collections::HashMap;
use std::env;

async fn send_webhook(str: &str) {
    let mut request_body = HashMap::new();
    request_body.insert("content", str);
    let webhook_url = env::var("WEBHOOK_URL").unwrap();
    let client = reqwest::Client::new();
    let _ = client
        .post(webhook_url)
        .json(&request_body)
        .send()
        .await
        .unwrap();
}

fn old(time: &str) -> bool {
    let time_date = time[0..2].parse::<u32>().unwrap();
    let time_hour = time[2..4].parse::<u32>().unwrap();
    let time_minutes = time[4..6].parse::<u32>().unwrap();
    let dt = if time_date > Utc::now().day() {
        return true;
    } else {
        Utc.with_ymd_and_hms(
            Utc::now().year(),
            Utc::now().month(),
            time_date,
            time_hour,
            time_minutes,
            0,
        )
        .unwrap()
    };
    if Utc::now() - Duration::hours(2) > dt {
        return true;
    }
    false
}

#[tokio::main]
async fn main() {
    let body = reqwest::get("https://metar.vatsim.net/RJFK,RJOB,RJSN,RJFU,RJOM,RJOO,RJFO,RJFF,RJCB,RJOS,RJSK,RJTT,RJFT,RJGG,RJAA,RJSS,RJOW,RJCH,RJAF,RJOA,RJCO,RJCK,RJTF,RJCW,RJBD,RJOI,RJCN,RJOT,RJNS,RJOR,RJBE,RJEC,RJOK,RJFM,RJNK,RJCC,RJKA,RJBB,RJCM,ROIG,ROMY,ROAH")
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let metars: Vec<&str> = body.split('\n').collect();
    let mut result = String::new();
    for metar_raw in metars {
        let mut metar: Vec<&str> = metar_raw.split(' ').collect();
        if old(metar[1]) {
            continue;
        }
        if metar[2] == "AUTO" {
            metar.remove(0);
        }
        let wind = if metar[2].ends_with("KT") && metar[2] != "/////KT" {
            &metar[2][3..5]
        } else {
            "0"
        };
        let gust = if metar[2].ends_with("KT")
            && metar[2] != "/////KT"
            && metar[2].chars().nth(5) == Some('G')
        {
            &metar[2][6..8]
        } else {
            "0"
        };
        let vis = if metar[3].len() == 4
            && metar[3] != "////"
            && !metar[3].ends_with("SM")
            && metar[3] != "CAVOK"
        {
            metar[3]
        } else {
            "9999"
        };
        let wind: u8 = wind.parse().unwrap();
        let gust: u8 = gust.parse().unwrap();
        let vis: u16 = vis.parse().unwrap();
        if wind > 30
            || gust > 45
            || vis < 500
            || metar_raw.contains("VV000")
            || metar_raw.contains("VV001")
            || metar_raw.contains("VV002")
            || metar_raw.contains("OVC000")
            || metar_raw.contains("OVC001")
            || metar_raw.contains("OVC002")
        {
            result = result + metar_raw + "\n";
        }
    }
    send_webhook(result.as_str()).await;
    println!("{}", result);
}
