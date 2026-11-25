use crate::analytics::TradingSignal;
use crate::api::{EnsembleForecast, Market, OrderBook};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct MarketWithSignal {
    pub market: Market,
    pub forecast: EnsembleForecast,
    #[allow(dead_code)]
    pub order_book: OrderBook,
    pub signal: TradingSignal,
}

pub struct AppData {
    pub markets: Vec<MarketWithSignal>,
    pub selected_market_idx: Option<usize>,
    pub last_update: DateTime<Utc>,
    pub offline_mode: bool,
}

impl AppData {
    pub fn new(offline_mode: bool) -> Self {
        Self {
            markets: Vec::new(),
            selected_market_idx: None,
            last_update: Utc::now(),
            offline_mode,
        }
    }
}
