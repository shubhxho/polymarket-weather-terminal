use crate::analytics::compute_ensemble_statistics;
use crate::ui::AppData;
use anyhow::Result;
use eframe::egui::{self, Color32, Context, RichText, Stroke};
use egui_plot::{Bar, BarChart, Line, Plot, PlotPoints};
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn run_app(app_data: Arc<RwLock<AppData>>) -> Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Polymarket Weather Terminal")
            .with_inner_size([1600.0, 900.0]),
        ..Default::default()
    };

    let app_data_clone = Arc::clone(&app_data);
    
    eframe::run_native(
        "Polymarket Weather Terminal",
        native_options,
        Box::new(move |_cc| {
            Ok(Box::new(WeatherTerminalApp {
                app_data: app_data_clone,
                frame_times: std::collections::VecDeque::new(),
                last_frame: std::time::Instant::now(),
            }))
        }),
    ).map_err(|e| anyhow::anyhow!("Failed to run app: {}", e))?;

    Ok(())
}

struct WeatherTerminalApp {
    app_data: Arc<RwLock<AppData>>,
    frame_times: std::collections::VecDeque<f32>,
    last_frame: std::time::Instant,
}

impl eframe::App for WeatherTerminalApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        let now = std::time::Instant::now();
        let frame_time = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;
        
        self.frame_times.push_back(frame_time);
        if self.frame_times.len() > 60 {
            self.frame_times.pop_front();
        }

        let data = match self.app_data.try_read() {
            Ok(d) => d,
            Err(_) => {
                ctx.request_repaint();
                return;
            }
        };

        render_ui(ctx, &data, &self.frame_times);
        
        ctx.request_repaint();
    }
}

fn render_ui(ctx: &Context, data: &AppData, frame_times: &std::collections::VecDeque<f32>) {
    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.heading(RichText::new("⛈ Polymarket Weather Terminal").size(20.0).color(Color32::WHITE));
            ui.separator();
            ui.label(format!("Markets: {}", data.markets.len()));
            ui.separator();
            ui.label(format!("Last update: {}", data.last_update.format("%H:%M:%S")));
            
            if data.offline_mode {
                ui.separator();
                ui.label(RichText::new("OFFLINE MODE").color(Color32::LIGHT_RED));
            }
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let avg_frame_time = frame_times.iter().sum::<f32>() / frame_times.len().max(1) as f32;
                let fps = if avg_frame_time > 0.0 { 1.0 / avg_frame_time } else { 0.0 };
                ui.label(RichText::new(format!("FPS: {:.1}", fps)).color(Color32::LIGHT_GREEN));
            });
        });
    });

    egui::SidePanel::left("market_list")
        .default_width(400.0)
        .show(ctx, |ui| {
            ui.heading("Markets");
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                for (idx, market_signal) in data.markets.iter().enumerate() {
                    let is_selected = data.selected_market_idx == Some(idx);
                    let signal = &market_signal.signal;
                    let market = &market_signal.market;

                    let edge_color = if signal.edge > 0.05 {
                        Color32::GREEN
                    } else if signal.edge < -0.05 {
                        Color32::RED
                    } else {
                        Color32::GRAY
                    };

                    let frame = egui::Frame::none()
                        .fill(if is_selected {
                            Color32::from_rgb(40, 40, 60)
                        } else {
                            Color32::from_rgb(25, 25, 35)
                        })
                        .stroke(Stroke::new(1.0_f32, if is_selected { Color32::LIGHT_BLUE } else { Color32::DARK_GRAY }))
                        .inner_margin(8.0);

                    frame.show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.label(RichText::new(&market.city).strong().size(14.0));
                            ui.label(market.question.chars().take(60).collect::<String>());
                            
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(format!("Edge: {:.1}%", signal.edge * 100.0))
                                    .color(edge_color));
                                ui.separator();
                                ui.label(format!("Model: {:.1}%", signal.model_prob * 100.0));
                                ui.separator();
                                ui.label(format!("Market: {:.1}%", signal.market_prob * 100.0));
                            });

                            if signal.kelly_fraction.abs() > 0.001 {
                                let kelly_text = format!("Kelly: {:.2}%", signal.kelly_fraction * 100.0);
                                let kelly_color = if signal.kelly_fraction > 0.0 {
                                    Color32::LIGHT_GREEN
                                } else {
                                    Color32::LIGHT_RED
                                };
                                ui.label(RichText::new(kelly_text).color(kelly_color).small());
                            }
                        });
                    });

                    ui.add_space(4.0);
                }
            });
        });

    egui::CentralPanel::default().show(ctx, |ui| {
        if let Some(idx) = data.selected_market_idx {
            if let Some(market_signal) = data.markets.get(idx) {
                render_market_detail(ui, market_signal);
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("Market not found");
                });
            }
        } else {
            if !data.markets.is_empty() {
                render_overview(ui, data);
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("Loading markets...");
                });
            }
        }
    });
}

fn render_overview(ui: &mut egui::Ui, data: &AppData) {
    ui.heading("Market Overview");
    ui.separator();

    ui.label(format!("Total markets tracked: {}", data.markets.len()));
    
    let positive_edges = data.markets.iter().filter(|m| m.signal.edge > 0.05).count();
    let negative_edges = data.markets.iter().filter(|m| m.signal.edge < -0.05).count();
    
    ui.label(format!("Strong positive edges: {}", positive_edges));
    ui.label(format!("Strong negative edges: {}", negative_edges));
    
    ui.add_space(20.0);

    Plot::new("edge_distribution")
        .height(300.0)
        .legend(egui_plot::Legend::default())
        .show(ui, |plot_ui| {
            let edges: Vec<_> = data.markets.iter().enumerate()
                .map(|(i, m)| [i as f64, m.signal.edge * 100.0])
                .collect();
            
            let line = Line::new(PlotPoints::from(edges))
                .color(Color32::LIGHT_BLUE)
                .name("Edge (%)");
            plot_ui.line(line);
            
            plot_ui.hline(egui_plot::HLine::new(0.0).color(Color32::GRAY));
        });
}

fn render_market_detail(ui: &mut egui::Ui, market_signal: &crate::ui::MarketWithSignal) {
    let market = &market_signal.market;
    let signal = &market_signal.signal;
    let forecast = &market_signal.forecast;

    ui.heading(&market.city);
    ui.label(&market.question);
    ui.add_space(10.0);

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.heading("Signal");
            ui.label(format!("Edge: {:.2}%", signal.edge * 100.0));
            ui.label(format!("Model Prob: {:.2}%", signal.model_prob * 100.0));
            ui.label(format!("Market Prob: {:.2}%", signal.market_prob * 100.0));
            ui.label(format!("Kelly Fraction: {:.3}%", signal.kelly_fraction * 100.0));
            ui.label(RichText::new("(Illustrative only, not trading advice)").small().italics());
        });

        ui.separator();

        ui.vertical(|ui| {
            ui.heading("Market Info");
            ui.label(format!("Location: {:.4}°N, {:.4}°E", market.latitude, market.longitude));
            ui.label(format!("Target Date: {}", market.target_date));
            if let (Some(min), Some(max)) = (market.bucket_min, market.bucket_max) {
                ui.label(format!("Bucket: {:.1}°C to {:.1}°C", min, max));
            } else if let Some(min) = market.bucket_min {
                ui.label(format!("Bucket: >= {:.1}°C", min));
            } else if let Some(max) = market.bucket_max {
                ui.label(format!("Bucket: < {:.1}°C", max));
            }
        });
    });

    ui.add_space(20.0);

    let stats = compute_ensemble_statistics(&forecast.members);

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.heading("Ensemble Statistics");
            ui.label(format!("Mean: {:.1}°C", stats.mean));
            ui.label(format!("Median: {:.1}°C", stats.median));
            ui.label(format!("Std Dev: {:.1}°C", stats.std_dev));
            ui.label(format!("Range: {:.1}°C to {:.1}°C", stats.min, stats.max));
            ui.label(format!("80% CI: [{:.1}°C, {:.1}°C]", stats.p10, stats.p90));
        });
    });

    ui.add_space(20.0);

    Plot::new("ensemble_histogram")
        .height(250.0)
        .legend(egui_plot::Legend::default())
        .show(ui, |plot_ui| {
            let bin_width = 1.0;
            let min_temp = stats.min.floor();
            let max_temp = stats.max.ceil();
            let num_bins = ((max_temp - min_temp) / bin_width).ceil() as usize;

            let mut bins = vec![0; num_bins];
            for &temp in &forecast.members {
                let bin_idx = ((temp - min_temp) / bin_width).floor() as usize;
                if bin_idx < num_bins {
                    bins[bin_idx] += 1;
                }
            }

            let bars: Vec<_> = bins.iter().enumerate()
                .map(|(i, &count)| {
                    let temp = min_temp + (i as f64 + 0.5) * bin_width;
                    Bar::new(temp, count as f64)
                        .width(bin_width * 0.9)
                        .fill(Color32::LIGHT_BLUE)
                })
                .collect();

            let chart = BarChart::new(bars)
                .color(Color32::LIGHT_BLUE)
                .name("Ensemble Members");
            plot_ui.bar_chart(chart);

            if let Some(min) = market.bucket_min {
                plot_ui.vline(egui_plot::VLine::new(min).color(Color32::RED).width(2.0_f32));
            }
            if let Some(max) = market.bucket_max {
                plot_ui.vline(egui_plot::VLine::new(max).color(Color32::RED).width(2.0_f32));
            }
        });

    ui.add_space(10.0);

    Plot::new("fan_chart")
        .height(200.0)
        .legend(egui_plot::Legend::default())
        .show(ui, |plot_ui| {
            let sorted_members: Vec<_> = {
                let mut m = forecast.members.clone();
                m.sort_by(|a, b| a.partial_cmp(b).unwrap());
                m
            };

            let n = sorted_members.len();
            let percentiles = [
                (0.1, 0.9, Color32::from_rgba_premultiplied(100, 100, 255, 30)),
                (0.25, 0.75, Color32::from_rgba_premultiplied(100, 100, 255, 60)),
            ];

            for (lower_p, upper_p, color) in percentiles {
                let lower_idx = (n as f64 * lower_p) as usize;
                let upper_idx = (n as f64 * upper_p) as usize;
                
                let lower_val = sorted_members[lower_idx.min(n - 1)];
                let upper_val = sorted_members[upper_idx.min(n - 1)];
                
                let polygon_points: Vec<[f64; 2]> = vec![
                    [0.0, lower_val],
                    [1.0, lower_val],
                    [1.0, upper_val],
                    [0.0, upper_val],
                ];
                
                plot_ui.polygon(egui_plot::Polygon::new(PlotPoints::from(polygon_points))
                    .fill_color(color));
            }

            let median_line = Line::new(vec![[0.0, stats.median], [1.0, stats.median]])
                .color(Color32::YELLOW)
                .width(2.0_f32)
                .name("Median");
            plot_ui.line(median_line);
        });
}
