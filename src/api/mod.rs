mod polymarket;
mod weather;

pub use polymarket::{PolymarketClient, Market, OrderBook};
pub use weather::{WeatherClient, EnsembleForecast};

use crate::ui::AppData;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};

pub async fn data_refresh_loop(app_data: Arc<RwLock<AppData>>) -> Result<()> {
    let polymarket = PolymarketClient::new();
    let weather = WeatherClient::new();

    loop {
        let offline = {
            let data = app_data.read().await;
            data.offline_mode
        };

        if !offline {
            match fetch_and_update(&polymarket, &weather, &app_data).await {
                Ok(_) => tracing::debug!("Data refresh successful"),
                Err(e) => tracing::warn!("Data refresh failed: {}", e),
            }
        }

        sleep(Duration::from_secs(30)).await;
    }
}

async fn fetch_and_update(
    polymarket: &PolymarketClient,
    weather: &WeatherClient,
    app_data: &Arc<RwLock<AppData>>,
) -> Result<()> {
    let markets = polymarket.fetch_weather_markets().await?;
    
    let mut updated_markets = Vec::new();
    for market in markets {
        let forecast = weather
            .fetch_ensemble_forecast(market.latitude, market.longitude, &market.target_date)
            .await?;
        
        let order_book = polymarket.fetch_order_book(&market.condition_id).await?;
        
        let signal = crate::analytics::compute_signal(&market, &forecast, &order_book)?;
        
        updated_markets.push(crate::ui::MarketWithSignal {
            market,
            forecast,
            order_book,
            signal,
        });
    }

    let mut data = app_data.write().await;
    data.markets = updated_markets;
    data.last_update = chrono::Utc::now();

    Ok(())
}
