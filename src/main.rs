use std::collections::HashMap;
use std::env;

async fn send_webhook(str :&str) {
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

#[tokio::main]
async fn main() {
    let body = reqwest::get("https://metar.vatsim.net/rj,ro").await.unwrap().text().await.unwrap();
    let metars : Vec<&str> = body.split('\n').collect();
    let mut result =  String::new();
    for metar_raw in metars {
        let mut metar: Vec<&str> = metar_raw.split(' ').collect();
        if metar[2] == "AUTO" {
            metar.remove(0);
        }
        let mut wind;
        if metar[2].ends_with("KT") && metar[2] != "/////KT"{
            wind = metar[2];
            wind = &wind[wind.len()-4..wind.len()-2];
        } else {
            wind = "0";
        }
        let vis;
        if metar[3].len() == 4  && metar[3] != "////" && !metar[3].ends_with("SM") && metar[3] != "CAVOK" {
            vis = metar[3];
        } else {
            vis = "9999";
        }
        let wind: u8 = wind.parse().unwrap();
        let vis: u16 = vis.parse().unwrap();
        if wind > 35 || vis < 1000 || metar_raw.contains("VV00") || metar_raw.contains("OVC000") || metar_raw.contains("OVC001") || metar_raw.contains("OVC002") {
            result = result + metar_raw + "\n";
        }
    }
    send_webhook(result.as_str()).await;
    println!("{}", result);
}
