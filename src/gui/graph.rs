use std::collections::{HashMap, VecDeque};
use time::{Duration, OffsetDateTime};

use eframe::egui;
use egui::plot::{Line, Plot};

use crate::config::Color;
use crate::gui::color::egui_color;
use crate::telemetry::{DataPoint, Frame};

struct GraphPlot {
    name: String,
    source_name: String,
    color: egui::Color32,
    data: VecDeque<DataPoint>,
}

pub struct Graph {
    name: String,
    plots: HashMap<String, GraphPlot>,
    window: Duration,
    cursor_group: egui::widgets::plot::LinkedCursorsGroup,
}

impl Graph {
    pub fn new(
        name: &str,
        plots: &[(String, String, Color)],
        window: Duration,
        cursor_group: egui::widgets::plot::LinkedCursorsGroup,
    ) -> Self {
        Self {
            name: name.to_string(),
            plots: plots
                .iter()
                .map(|(name, source_name, color)| {
                    (
                        name.to_string(),
                        GraphPlot {
                            name: name.to_string(),
                            source_name: source_name.to_string(),
                            color: egui_color(*color),
                            data: VecDeque::new(),
                        },
                    )
                })
                .collect(),
            window,
            cursor_group,
        }
    }

    pub fn add_data(&mut self, frame: &Frame) {
        for data_point in frame.data_points.iter() {
            for plot in self.plots.values_mut() {
                if plot.source_name == data_point.name {
                    plot.data.push_back(data_point.clone());
                    while let Some(data_point) = plot.data.front() {
                        if frame.timestamp - data_point.timestamp > self.window {
                            plot.data.pop_front();
                        } else {
                            break;
                        }
                    }
                }
            }
        }
    }

    pub fn reset(&mut self) {
        self.plots.values_mut().for_each(|p| p.data.clear());
    }

    pub fn draw(&self, ui: &mut egui::Ui) {
        let view_width = 280.;
        let view_height = 280.;
        let constant_padding = 1.;
        let padding_factor = 1.2;
        let window_width = self.window;
        let now = OffsetDateTime::now_local().unwrap();

        let plot_data: HashMap<String, Vec<[f64; 2]>> = self
            .plots
            .iter()
            .map(|(n, p)| {
                (
                    n.clone(),
                    p.data
                        .iter()
                        .map(|dp| {
                            [
                                window_width.as_seconds_f64()
                                    - (now - dp.timestamp).as_seconds_f64(),
                                dp.value as f64,
                            ]
                        })
                        .collect(),
                )
            })
            .collect();

        let mut min: f64 = 0.;
        let mut max: f64 = 0.;

        self.plots.values().for_each(|p| {
            let p_min: f64 = p
                .data
                .iter()
                .map(|dp| dp.value)
                .fold(0., |a, b| a.min(b.into()));
            let p_max: f64 = p
                .data
                .iter()
                .map(|dp| dp.value)
                .fold(0., |a, b| a.max(b.into()));
            min = min.min(p_min);
            max = max.max(p_max);
        });

        Plot::new(&self.name)
            .include_y(max * padding_factor + constant_padding)
            .include_y(min * padding_factor - constant_padding)
            .include_x(window_width.as_seconds_f64())
            .include_x(0.)
            .width(view_width)
            .height(view_height)
            .allow_drag(false)
            .allow_scroll(false)
            .allow_zoom(false)
            .allow_boxed_zoom(false)
            .legend(egui::plot::Legend::default().position(egui::widgets::plot::Corner::LeftTop))
            .link_cursor(self.cursor_group.clone())
            .show(ui, |plot_ui| {
                for plot in self.plots.values() {
                    let line = Line::new(
                        plot_data
                            .get(&plot.name)
                            .expect("failed to find plot")
                            .to_owned(),
                    )
                    .color(plot.color)
                    .name(&plot.name);
                    plot_ui.line(line);
                }
            });
    }
}
