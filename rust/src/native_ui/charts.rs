//! Charts module for cost and credits history visualization
//!
//! Provides bar charts similar to the macOS SwiftUI Charts

use eframe::egui::{self, Color32, RichText, Rounding, Stroke, Vec2};

/// A single data point for the chart
#[derive(Clone, Debug)]
pub struct ChartPoint {
    pub date: String,      // "2025-01-15" format
    pub value: f64,        // Cost in USD or credits used
    pub tokens: Option<i64>, // Optional token count
}

/// Cost history chart widget
pub struct CostHistoryChart {
    points: Vec<ChartPoint>,
    selected_index: Option<usize>,
    bar_color: Color32,
    total_cost: Option<f64>,
}

impl CostHistoryChart {
    pub fn new(points: Vec<ChartPoint>, bar_color: Color32) -> Self {
        let total_cost = if points.is_empty() {
            None
        } else {
            Some(points.iter().map(|p| p.value).sum())
        };

        Self {
            points,
            selected_index: None,
            bar_color,
            total_cost,
        }
    }

    /// Render the chart
    pub fn show(&mut self, ui: &mut egui::Ui) {
        if self.points.is_empty() {
            ui.label(
                RichText::new("No cost history data.")
                    .size(11.0)
                    .color(Color32::GRAY),
            );
            return;
        }

        let max_value = self.points.iter().map(|p| p.value).fold(0.0f64, f64::max);
        let peak_index = self.points.iter().enumerate()
            .max_by(|(_, a), (_, b)| a.value.partial_cmp(&b.value).unwrap())
            .map(|(i, _)| i);

        // Chart area
        let chart_height = 100.0;
        let available_width = ui.available_width();
        let bar_width = (available_width / self.points.len() as f32) * 0.8;
        let bar_spacing = (available_width / self.points.len() as f32) * 0.2;

        let (response, painter) = ui.allocate_painter(
            Vec2::new(available_width, chart_height),
            egui::Sense::hover(),
        );

        let rect = response.rect;

        // Draw bars
        for (i, point) in self.points.iter().enumerate() {
            let bar_height = if max_value > 0.0 {
                (point.value / max_value) as f32 * (chart_height - 10.0)
            } else {
                0.0
            };

            let x = rect.left() + (i as f32 * (bar_width + bar_spacing)) + bar_spacing / 2.0;
            let bar_rect = egui::Rect::from_min_size(
                egui::pos2(x, rect.bottom() - bar_height),
                Vec2::new(bar_width, bar_height),
            );

            // Check hover
            let is_hovered = response.hover_pos().map_or(false, |pos| {
                pos.x >= x && pos.x <= x + bar_width
            });

            if is_hovered {
                self.selected_index = Some(i);
            }

            // Bar color - peak gets yellow cap
            let color = if Some(i) == peak_index && bar_height > 5.0 {
                // Draw main bar
                let main_rect = egui::Rect::from_min_size(
                    egui::pos2(x, rect.bottom() - bar_height + 5.0),
                    Vec2::new(bar_width, bar_height - 5.0),
                );
                painter.rect_filled(main_rect, Rounding::same(2.0), self.bar_color);

                // Draw yellow peak cap
                let cap_rect = egui::Rect::from_min_size(
                    egui::pos2(x, rect.bottom() - bar_height),
                    Vec2::new(bar_width, 5.0),
                );
                painter.rect_filled(cap_rect, Rounding::same(2.0), Color32::from_rgb(255, 200, 50));
                continue;
            } else if is_hovered {
                self.bar_color.gamma_multiply(1.2)
            } else {
                self.bar_color
            };

            painter.rect_filled(bar_rect, Rounding::same(2.0), color);
        }

        // Hover selection highlight
        if let Some(idx) = self.selected_index {
            if idx < self.points.len() {
                let x = rect.left() + (idx as f32 * (bar_width + bar_spacing));
                let highlight_rect = egui::Rect::from_min_size(
                    egui::pos2(x, rect.top()),
                    Vec2::new(bar_width + bar_spacing, chart_height),
                );
                painter.rect_filled(highlight_rect, Rounding::ZERO, Color32::from_rgba_unmultiplied(255, 255, 255, 20));
            }
        }

        // Reset selection if not hovering
        if !response.hovered() {
            self.selected_index = None;
        }

        ui.add_space(8.0);

        // Detail text
        if let Some(idx) = self.selected_index {
            if let Some(point) = self.points.get(idx) {
                let date_display = format_date_display(&point.date);
                let cost_display = format!("${:.2}", point.value);

                let detail = if let Some(tokens) = point.tokens {
                    format!("{}: {} · {} tokens", date_display, cost_display, format_tokens(tokens))
                } else {
                    format!("{}: {}", date_display, cost_display)
                };

                ui.label(
                    RichText::new(detail)
                        .size(11.0)
                        .color(Color32::GRAY),
                );
            }
        } else {
            ui.label(
                RichText::new("Hover a bar for details")
                    .size(11.0)
                    .color(Color32::GRAY),
            );
        }

        // Total
        if let Some(total) = self.total_cost {
            ui.add_space(4.0);
            ui.label(
                RichText::new(format!("Total (30d): ${:.2}", total))
                    .size(11.0)
                    .color(Color32::GRAY),
            );
        }
    }
}

/// Credits history chart widget
pub struct CreditsHistoryChart {
    points: Vec<ChartPoint>,
    selected_index: Option<usize>,
    total_credits: Option<f64>,
}

impl CreditsHistoryChart {
    pub fn new(points: Vec<ChartPoint>) -> Self {
        let total_credits = if points.is_empty() {
            None
        } else {
            Some(points.iter().map(|p| p.value).sum())
        };

        Self {
            points,
            selected_index: None,
            total_credits,
        }
    }

    /// Render the chart
    pub fn show(&mut self, ui: &mut egui::Ui) {
        if self.points.is_empty() {
            ui.label(
                RichText::new("No credits history data.")
                    .size(11.0)
                    .color(Color32::GRAY),
            );
            return;
        }

        let bar_color = Color32::from_rgb(73, 163, 176); // Teal color for credits
        let max_value = self.points.iter().map(|p| p.value).fold(0.0f64, f64::max);
        let peak_index = self.points.iter().enumerate()
            .max_by(|(_, a), (_, b)| a.value.partial_cmp(&b.value).unwrap())
            .map(|(i, _)| i);

        // Chart area
        let chart_height = 100.0;
        let available_width = ui.available_width();
        let bar_width = (available_width / self.points.len() as f32) * 0.8;
        let bar_spacing = (available_width / self.points.len() as f32) * 0.2;

        let (response, painter) = ui.allocate_painter(
            Vec2::new(available_width, chart_height),
            egui::Sense::hover(),
        );

        let rect = response.rect;

        // Draw bars
        for (i, point) in self.points.iter().enumerate() {
            let bar_height = if max_value > 0.0 {
                (point.value / max_value) as f32 * (chart_height - 10.0)
            } else {
                0.0
            };

            let x = rect.left() + (i as f32 * (bar_width + bar_spacing)) + bar_spacing / 2.0;

            // Check hover
            let is_hovered = response.hover_pos().map_or(false, |pos| {
                pos.x >= x && pos.x <= x + bar_width
            });

            if is_hovered {
                self.selected_index = Some(i);
            }

            // Bar color - peak gets yellow cap
            if Some(i) == peak_index && bar_height > 5.0 {
                // Draw main bar
                let main_rect = egui::Rect::from_min_size(
                    egui::pos2(x, rect.bottom() - bar_height + 5.0),
                    Vec2::new(bar_width, bar_height - 5.0),
                );
                painter.rect_filled(main_rect, Rounding::same(2.0), bar_color);

                // Draw yellow peak cap
                let cap_rect = egui::Rect::from_min_size(
                    egui::pos2(x, rect.bottom() - bar_height),
                    Vec2::new(bar_width, 5.0),
                );
                painter.rect_filled(cap_rect, Rounding::same(2.0), Color32::from_rgb(255, 200, 50));
            } else {
                let bar_rect = egui::Rect::from_min_size(
                    egui::pos2(x, rect.bottom() - bar_height),
                    Vec2::new(bar_width, bar_height),
                );
                let color = if is_hovered {
                    bar_color.gamma_multiply(1.2)
                } else {
                    bar_color
                };
                painter.rect_filled(bar_rect, Rounding::same(2.0), color);
            }
        }

        // Reset selection if not hovering
        if !response.hovered() {
            self.selected_index = None;
        }

        ui.add_space(8.0);

        // Detail text
        if let Some(idx) = self.selected_index {
            if let Some(point) = self.points.get(idx) {
                let date_display = format_date_display(&point.date);
                let detail = format!("{}: {:.2} credits", date_display, point.value);

                ui.label(
                    RichText::new(detail)
                        .size(11.0)
                        .color(Color32::GRAY),
                );
            }
        } else {
            ui.label(
                RichText::new("Hover a bar for details")
                    .size(11.0)
                    .color(Color32::GRAY),
            );
        }

        // Total
        if let Some(total) = self.total_credits {
            ui.add_space(4.0);
            ui.label(
                RichText::new(format!("Total (30d): {:.2} credits", total))
                    .size(11.0)
                    .color(Color32::GRAY),
            );
        }
    }
}

/// Format date from "2025-01-15" to "Jan 15"
fn format_date_display(date_key: &str) -> String {
    let parts: Vec<&str> = date_key.split('-').collect();
    if parts.len() != 3 {
        return date_key.to_string();
    }

    let month = match parts[1] {
        "01" => "Jan",
        "02" => "Feb",
        "03" => "Mar",
        "04" => "Apr",
        "05" => "May",
        "06" => "Jun",
        "07" => "Jul",
        "08" => "Aug",
        "09" => "Sep",
        "10" => "Oct",
        "11" => "Nov",
        "12" => "Dec",
        _ => parts[1],
    };

    let day = parts[2].trim_start_matches('0');
    format!("{} {}", month, day)
}

/// Format token count with K/M suffix
fn format_tokens(tokens: i64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}K", tokens as f64 / 1_000.0)
    } else {
        tokens.to_string()
    }
}
