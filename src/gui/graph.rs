use std::collections::{HashMap, VecDeque};
use time::{Duration, OffsetDateTime};

use eframe::egui;
use egui::plot::{Line, Plot};

use crate::config::Color;
use crate::gui::color::egui_color;

struct GraphPlot {
    name: String,
    source_name: String,
    color: egui::Color32,
    data: VecDeque<(Duration, f32)>,
}

pub struct Graph {
    name: String,
    plots: HashMap<String, GraphPlot>,
    start: Option<OffsetDateTime>,
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
            start: None,
            window,
            cursor_group,
        }
    }

    pub fn add_data(&mut self, name: &str, value: f32) {
        if self.start.is_none() {
            self.start = Some(OffsetDateTime::now_local().unwrap())
        }
        for plot in self.plots.values_mut() {
            if plot.source_name == name {
                let new_duration = OffsetDateTime::now_local().unwrap() - self.start.unwrap();
                plot.data.push_back((new_duration, value));
                while let Some(data_point) = plot.data.front() {
                    if new_duration - data_point.0 > self.window {
                        plot.data.pop_front();
                    } else {
                        break;
                    }
                }
            }
        }
    }

    pub fn reset(&mut self) {
        self.start = None;
        self.plots.values_mut().for_each(|p| p.data.clear());
    }

    pub fn draw(&self, ui: &mut egui::Ui) {
        let view_width = 280.;
        let view_height = 280.;
        let constant_padding = 1.;
        let padding_factor = 1.2;
        let window_width = self.window;

        let plot_data: HashMap<String, Vec<[f64; 2]>> = self
            .plots
            .iter()
            .map(|(n, p)| {
                (
                    n.clone(),
                    p.data
                        .iter()
                        .map(|(ts, v)| [ts.as_seconds_f64(), *v as f64])
                        .collect(),
                )
            })
            .collect();

        let data_width: f64 = plot_data
            .values()
            .map(|p| p.last().map(|[ts, _]| *ts).unwrap_or(0.))
            .fold(0., |a, b| a.max(b));

        let maximum_in_window = self
            .plots
            .values()
            .flat_map(|p| {
                p.data
                    .iter()
                    .filter(|(k, _)| {
                        data_width - k.as_seconds_f64() <= window_width.as_seconds_f64()
                    })
                    .map(|(_, v)| v)
                    .collect::<Vec<_>>()
            })
            .fold(0., |a: f32, b: &f32| a.max(*b));
        let minimum_in_window = self
            .plots
            .values()
            .flat_map(|p| {
                p.data
                    .iter()
                    .filter(|(k, _)| {
                        data_width - k.as_seconds_f64() <= window_width.as_seconds_f64()
                    })
                    .map(|(_, v)| v)
                    .collect::<Vec<_>>()
            })
            .fold(0., |a: f32, b: &f32| a.min(*b));

        Plot::new(&self.name)
            .include_y(maximum_in_window * padding_factor + constant_padding)
            .include_y(minimum_in_window * padding_factor - constant_padding)
            .include_x(data_width)
            .include_x(data_width - window_width.as_seconds_f64())
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
