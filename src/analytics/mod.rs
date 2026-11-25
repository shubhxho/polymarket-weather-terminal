use crate::api::{EnsembleForecast, Market, OrderBook};
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct TradingSignal {
    pub edge: f64,
    pub model_prob: f64,
    pub market_prob: f64,
    pub kelly_fraction: f64,
    #[allow(dead_code)]
    pub confidence_interval: (f64, f64),
}

pub fn compute_signal(
    market: &Market,
    forecast: &EnsembleForecast,
    order_book: &OrderBook,
) -> Result<TradingSignal> {
    let model_prob = compute_model_probability(market, forecast)?;
    
    let market_prob = compute_market_probability(market, order_book);
    
    let edge = model_prob - market_prob;
    
    let kelly_fraction = compute_kelly_fraction(model_prob, market_prob);
    
    let confidence_interval = compute_confidence_interval(forecast);
    
    Ok(TradingSignal {
        edge,
        model_prob,
        market_prob,
        kelly_fraction,
        confidence_interval,
    })
}

pub fn compute_model_probability(market: &Market, forecast: &EnsembleForecast) -> Result<f64> {
    if forecast.members.is_empty() {
        return Err(anyhow::anyhow!("No ensemble members"));
    }

    let count_in_bucket = forecast.members.iter().filter(|&&temp| {
        is_in_bucket(temp, market.bucket_min, market.bucket_max)
    }).count();

    let probability = count_in_bucket as f64 / forecast.members.len() as f64;
    
    Ok(probability.clamp(0.001, 0.999))
}

fn is_in_bucket(temp: f64, min: Option<f64>, max: Option<f64>) -> bool {
    match (min, max) {
        (Some(min_val), Some(max_val)) => temp >= min_val && temp < max_val,
        (Some(min_val), None) => temp >= min_val,
        (None, Some(max_val)) => temp < max_val,
        (None, None) => true,
    }
}

fn compute_market_probability(market: &Market, order_book: &OrderBook) -> f64 {
    if market.outcome_prices.is_empty() {
        return 0.5;
    }

    let yes_price = market.outcome_prices[0];
    
    if !order_book.bids.is_empty() && !order_book.asks.is_empty() {
        let best_bid = order_book.bids.iter().map(|b| b.price).fold(f64::NEG_INFINITY, f64::max);
        let best_ask = order_book.asks.iter().map(|a| a.price).fold(f64::INFINITY, f64::min);
        let mid = (best_bid + best_ask) / 2.0;
        return mid.clamp(0.001, 0.999);
    }
    
    yes_price.clamp(0.001, 0.999)
}

fn compute_kelly_fraction(model_prob: f64, market_prob: f64) -> f64 {
    if market_prob >= 0.999 || market_prob <= 0.001 {
        return 0.0;
    }

    let edge = model_prob - market_prob;
    let odds = market_prob / (1.0 - market_prob);
    
    let kelly = edge / odds;
    
    let fractional_kelly = kelly * 0.25;
    
    fractional_kelly.clamp(-0.1, 0.1)
}

fn compute_confidence_interval(forecast: &EnsembleForecast) -> (f64, f64) {
    if forecast.members.is_empty() {
        return (0.0, 0.0);
    }

    let mut sorted = forecast.members.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    
    let p10_idx = (sorted.len() as f64 * 0.10) as usize;
    let p90_idx = (sorted.len() as f64 * 0.90) as usize;
    
    let p10 = sorted[p10_idx.min(sorted.len() - 1)];
    let p90 = sorted[p90_idx.min(sorted.len() - 1)];
    
    (p10, p90)
}

pub fn compute_ensemble_statistics(members: &[f64]) -> EnsembleStats {
    if members.is_empty() {
        return EnsembleStats::default();
    }

    let mean = members.iter().sum::<f64>() / members.len() as f64;
    
    let variance = members.iter()
        .map(|&x| (x - mean).powi(2))
        .sum::<f64>() / members.len() as f64;
    let std_dev = variance.sqrt();
    
    let mut sorted = members.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    
    let median = if sorted.len().is_multiple_of(2) {
        (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2.0
    } else {
        sorted[sorted.len() / 2]
    };
    
    EnsembleStats {
        mean,
        median,
        std_dev,
        min: sorted[0],
        max: sorted[sorted.len() - 1],
        p10: sorted[(sorted.len() as f64 * 0.10) as usize],
        p25: sorted[(sorted.len() as f64 * 0.25) as usize],
        p75: sorted[(sorted.len() as f64 * 0.75) as usize],
        p90: sorted[(sorted.len() as f64 * 0.90) as usize],
    }
}

#[derive(Debug, Clone, Default)]
pub struct EnsembleStats {
    pub mean: f64,
    pub median: f64,
    pub std_dev: f64,
    pub min: f64,
    pub max: f64,
    pub p10: f64,
    #[allow(dead_code)]
    pub p25: f64,
    #[allow(dead_code)]
    pub p75: f64,
    pub p90: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bucket_logic() {
        assert!(is_in_bucket(10.0, Some(5.0), Some(15.0)));
        assert!(!is_in_bucket(3.0, Some(5.0), Some(15.0)));
        assert!(is_in_bucket(20.0, Some(10.0), None));
        assert!(is_in_bucket(5.0, None, Some(10.0)));
    }

    #[test]
    fn test_kelly_fraction() {
        let kelly = compute_kelly_fraction(0.7, 0.5);
        assert!(kelly > 0.0);
        assert!(kelly <= 0.1);
        
        let kelly_negative = compute_kelly_fraction(0.3, 0.5);
        assert!(kelly_negative < 0.0);
    }

    #[test]
    fn test_ensemble_statistics() {
        let members = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let stats = compute_ensemble_statistics(&members);
        
        assert!((stats.mean - 5.5).abs() < 0.01);
        assert!((stats.median - 5.5).abs() < 0.01);
        assert_eq!(stats.min, 1.0);
        assert_eq!(stats.max, 10.0);
    }
}
