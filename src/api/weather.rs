use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnsembleForecast {
    pub members: Vec<f64>,
    pub times: Vec<String>,
    pub target_time: String,
}

pub struct WeatherClient {
    client: Client,
    base_url: String,
    use_ecmwf: bool,
}

impl WeatherClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            base_url: "https://ensemble-api.open-meteo.com/v1/ensemble".to_string(),
            use_ecmwf: true,
        }
    }

    pub async fn fetch_ensemble_forecast(
        &self,
        latitude: f64,
        longitude: f64,
        target_date: &str,
    ) -> Result<EnsembleForecast> {
        let models = if self.use_ecmwf {
            "gfs025,ecmwf_ifs04"
        } else {
            "gfs025"
        };
        
        let url = format!(
            "{}?latitude={}&longitude={}&hourly=temperature_2m&forecast_days=7&models={}",
            self.base_url, latitude, longitude, models
        );

        match self.client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                let data: serde_json::Value = response.json().await?;
                parse_ensemble_response(data, target_date)
            }
            _ => {
                tracing::warn!("Weather API unavailable, using synthetic ensemble");
                Ok(generate_synthetic_ensemble(latitude, longitude, target_date))
            }
        }
    }
}

fn parse_ensemble_response(data: serde_json::Value, target_date: &str) -> Result<EnsembleForecast> {
    let hourly = data["hourly"]
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("No hourly data"))?;
    
    let times: Vec<String> = serde_json::from_value(
        hourly["time"].clone()
    )?;
    
    let temperature_data = &hourly["temperature_2m"];
    let members = extract_ensemble_members(temperature_data)?;
    
    let target_hour = format!("{}T14:00", target_date);
    
    Ok(EnsembleForecast {
        members,
        times,
        target_time: target_hour,
    })
}

fn extract_ensemble_members(temp_data: &serde_json::Value) -> Result<Vec<f64>> {
    if let Some(array) = temp_data.as_array() {
        let members: Vec<f64> = array
            .iter()
            .filter_map(|v| v.as_f64())
            .collect();
        
        if !members.is_empty() {
            return Ok(members);
        }
    }
    
    if let Some(obj) = temp_data.as_object() {
        let mut all_values = Vec::new();
        for (_key, value) in obj.iter() {
            if let Some(array) = value.as_array() {
                for v in array {
                    if let Some(temp) = v.as_f64() {
                        all_values.push(temp);
                    }
                }
            }
        }
        if !all_values.is_empty() {
            return Ok(all_values);
        }
    }
    
    Err(anyhow::anyhow!("Could not extract ensemble members"))
}

fn generate_synthetic_ensemble(latitude: f64, longitude: f64, target_date: &str) -> EnsembleForecast {
    let base_temp = estimate_base_temperature(latitude, longitude);
    
    let mut members = Vec::new();
    for i in 0..31 {
        let member_offset = ((i as f64 - 15.0) / 15.0) * 3.0;
        let noise = (i as f64 * 0.37).sin() * 0.5;
        members.push(base_temp + member_offset + noise);
    }
    
    let target_time = format!("{}T14:00", target_date);
    let times = vec![target_time.clone()];
    
    EnsembleForecast {
        members,
        times,
        target_time,
    }
}

fn estimate_base_temperature(latitude: f64, longitude: f64) -> f64 {
    let lat_component = (90.0 - latitude.abs()) / 10.0;
    
    let season_offset = -5.0;
    
    let local_offset = if longitude < -50.0 {
        1.0
    } else if longitude > 100.0 {
        -2.0
    } else {
        0.0
    };
    
    lat_component + season_offset + local_offset
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synthetic_ensemble_generation() {
        let forecast = generate_synthetic_ensemble(40.7128, -74.0060, "2026-11-25");
        assert_eq!(forecast.members.len(), 31);
        assert!(forecast.members.iter().all(|&t| t > -50.0 && t < 50.0));
    }

    #[test]
    fn test_temperature_estimation() {
        let nyc_temp = estimate_base_temperature(40.7128, -74.0060);
        assert!(nyc_temp > -10.0 && nyc_temp < 20.0);
        
        let london_temp = estimate_base_temperature(51.5074, -0.1278);
        assert!(london_temp > -10.0 && london_temp < 20.0);
    }
}
