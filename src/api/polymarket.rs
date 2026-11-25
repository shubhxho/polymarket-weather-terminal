use anyhow::Result;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Market {
    pub condition_id: String,
    pub question: String,
    pub slug: String,
    pub description: String,
    pub outcomes: Vec<String>,
    pub outcome_prices: Vec<f64>,
    pub end_date: DateTime<Utc>,
    
    pub latitude: f64,
    pub longitude: f64,
    pub city: String,
    pub station: String,
    pub target_date: String,
    pub bucket_min: Option<f64>,
    pub bucket_max: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBook {
    pub bids: Vec<OrderLevel>,
    pub asks: Vec<OrderLevel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderLevel {
    pub price: f64,
    pub size: f64,
}

pub struct PolymarketClient {
    client: Client,
    gamma_base: String,
    clob_base: String,
}

impl PolymarketClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            gamma_base: "https://gamma-api.polymarket.com".to_string(),
            clob_base: "https://clob.polymarket.com".to_string(),
        }
    }

    pub async fn fetch_weather_markets(&self) -> Result<Vec<Market>> {
        let url = format!("{}/public-search?q=highest%20temperature&limit_per_type=50", self.gamma_base);
        
        let response = self.client.get(&url).send().await?;
        if !response.status().is_success() {
            tracing::warn!("Public search failed, using demo fixtures");
            return load_demo_markets();
        }

        let data: serde_json::Value = response.json().await?;
        let events = data["events"].as_array().ok_or_else(|| anyhow::anyhow!("No events array"))?;

        let mut all_markets = Vec::new();
        
        for event in events {
            let event_title = event["title"].as_str().unwrap_or("");
            tracing::debug!("Checking event: {}", event_title);
            
            if !event_title.to_lowercase().contains("highest temperature") {
                continue;
            }

            tracing::info!("Found temperature event: {}", event_title);
            
            let markets_array = event["markets"].as_array();
            if markets_array.is_none() {
                tracing::warn!("Event has no markets array");
                continue;
            }

            for market_json in markets_array.unwrap() {
                match parse_temperature_market(market_json, event) {
                    Ok(market) => {
                        tracing::info!("Parsed market: {} ({})", market.question, market.city);
                        all_markets.push(market);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse market: {}", e);
                    }
                }
            }
        }

        if all_markets.is_empty() {
            tracing::info!("No temperature markets parsed, loading demo fixtures");
            return load_demo_markets();
        }

        tracing::info!("Found {} live temperature markets", all_markets.len());
        Ok(all_markets)
    }

    pub async fn fetch_order_book(&self, condition_id: &str) -> Result<OrderBook> {
        let url = format!("{}/book?token_id={}", self.clob_base, condition_id);
        
        match self.client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                let book: OrderBook = response.json().await?;
                Ok(book)
            }
            _ => {
                Ok(generate_demo_order_book())
            }
        }
    }
}

fn parse_temperature_market(market_json: &serde_json::Value, event: &serde_json::Value) -> Result<Market> {
    let question = market_json["question"].as_str().unwrap_or("");
    let description = market_json["description"].as_str().unwrap_or("");
    let event_title = event["title"].as_str().unwrap_or("");
    
    let (city, station, latitude, longitude) = parse_location_and_station(event_title, description);
    let target_date = parse_date_from_title(event_title);
    let (bucket_min, bucket_max) = parse_temperature_bucket(question);
    
    let outcomes_str = market_json["outcomes"].as_str().unwrap_or("[\"Yes\", \"No\"]");
    let outcomes: Vec<String> = serde_json::from_str(outcomes_str)?;
    
    let prices_str = market_json["outcomePrices"].as_str().unwrap_or("[\"0.5\", \"0.5\"]");
    let price_strings: Vec<String> = serde_json::from_str(prices_str)?;
    let outcome_prices: Vec<f64> = price_strings.iter()
        .filter_map(|s| s.parse::<f64>().ok())
        .collect();
    
    let end_date = market_json["endDate"].as_str()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(Utc::now);
    
    Ok(Market {
        condition_id: market_json["conditionId"].as_str().unwrap_or("").to_string(),
        question: question.to_string(),
        slug: market_json["slug"].as_str().unwrap_or("").to_string(),
        description: description.to_string(),
        outcomes,
        outcome_prices,
        end_date,
        latitude,
        longitude,
        city,
        station,
        target_date,
        bucket_min,
        bucket_max,
    })
}

fn parse_location_and_station(event_title: &str, description: &str) -> (String, String, f64, f64) {
    let text = format!("{} {}", event_title.to_lowercase(), description.to_lowercase());
    
    let station_coords: HashMap<&str, (&str, &str, f64, f64)> = [
        ("incheon", ("Seoul", "Incheon Intl Airport (RKSI)", 37.4691, 126.4505)),
        ("hong kong", ("Hong Kong", "Hong Kong Intl Airport (VHHH)", 22.3080, 113.9185)),
        ("beijing", ("Beijing", "Beijing Capital Intl Airport (ZBAA)", 40.0799, 116.6031)),
        ("paris", ("Paris", "Charles de Gaulle Airport (LFPG)", 49.0097, 2.5479)),
        ("chengdu", ("Chengdu", "Chengdu Shuangliu Intl Airport (ZUUU)", 30.5785, 103.9468)),
        ("tokyo", ("Tokyo", "Tokyo Haneda Airport (RJTT)", 35.5494, 139.7798)),
        ("shanghai", ("Shanghai", "Shanghai Pudong Intl Airport (ZSPD)", 31.1443, 121.8083)),
        ("shenzhen", ("Shenzhen", "Shenzhen Bao'an Intl Airport (ZGSZ)", 22.6393, 113.8107)),
        ("narita", ("Tokyo", "Tokyo Narita Airport (RJAA)", 35.7647, 140.3864)),
    ].iter().cloned().collect();
    
    for (keyword, (city, station, lat, lon)) in station_coords.iter() {
        if text.contains(keyword) {
            return (city.to_string(), station.to_string(), *lat, *lon);
        }
    }
    
    ("Unknown".to_string(), "Unknown Station".to_string(), 0.0, 0.0)
}

fn parse_date_from_title(title: &str) -> String {
    if title.contains("November 25") || title.contains("Nov 25") {
        "2026-11-25".to_string()
    } else if title.contains("November 26") || title.contains("Nov 26") {
        "2026-11-26".to_string()
    } else if title.contains("November 27") || title.contains("Nov 27") {
        "2026-11-27".to_string()
    } else {
        chrono::Utc::now().format("%Y-%m-%d").to_string()
    }
}

fn parse_temperature_bucket(question: &str) -> (Option<f64>, Option<f64>) {
    let text = question.to_lowercase();
    
    if text.contains("or below") || text.contains("or less") {
        if let Some(temp) = extract_first_number(&text) {
            return (None, Some(temp + 0.5));
        }
    }
    
    if text.contains("or above") || text.contains("or higher") || text.contains("or more") {
        if let Some(temp) = extract_first_number(&text) {
            return (Some(temp + 0.5), None);
        }
    }
    
    if text.contains(" to ") || text.contains("-") {
        let numbers: Vec<f64> = text
            .split(|c: char| !c.is_numeric() && c != '.' && c != '-')
            .filter_map(|s| s.trim_matches('-').parse::<f64>().ok())
            .filter(|&n| n >= -50.0 && n <= 50.0)
            .collect();
        
        if numbers.len() >= 2 {
            return (Some(numbers[0] - 0.5), Some(numbers[1] + 0.5));
        }
    }
    
    (None, None)
}

fn extract_first_number(text: &str) -> Option<f64> {
    text.split_whitespace()
        .find_map(|word| {
            word.trim_matches(|c: char| !c.is_numeric() && c != '.' && c != '-')
                .parse::<f64>().ok()
        })
        .filter(|&n| n >= -50.0 && n <= 50.0)
}

fn load_demo_markets() -> Result<Vec<Market>> {
    let fixtures_path = std::path::Path::new("fixtures/demo_markets.json");
    
    if fixtures_path.exists() {
        let content = std::fs::read_to_string(fixtures_path)?;
        let markets: Vec<Market> = serde_json::from_str(&content)?;
        Ok(markets)
    } else {
        Ok(create_demo_markets())
    }
}

fn create_demo_markets() -> Vec<Market> {
    vec![
        Market {
            condition_id: "demo_nyc_temp_1".to_string(),
            question: "Will NYC high temperature be above 5°C on Nov 25?".to_string(),
            slug: "nyc-temp-nov-25".to_string(),
            description: "Market resolves YES if Central Park high >= 5°C (41°F)".to_string(),
            outcomes: vec!["Yes".to_string(), "No".to_string()],
            outcome_prices: vec![0.65, 0.35],
            end_date: Utc::now() + chrono::Duration::hours(24),
            latitude: 40.7128,
            longitude: -74.0060,
            city: "New York City".to_string(),
            station: "Central Park".to_string(),
            target_date: "2026-11-25".to_string(),
            bucket_min: Some(5.0),
            bucket_max: None,
        },
    ]
}

fn generate_demo_order_book() -> OrderBook {
    OrderBook {
        bids: vec![
            OrderLevel { price: 0.64, size: 150.0 },
            OrderLevel { price: 0.63, size: 200.0 },
        ],
        asks: vec![
            OrderLevel { price: 0.65, size: 100.0 },
            OrderLevel { price: 0.66, size: 180.0 },
        ],
    }
}
