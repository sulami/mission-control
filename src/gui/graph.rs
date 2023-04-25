use std::collections::HashMap;
use time::Duration;
use time::OffsetDateTime;

use eframe::egui;
use egui::plot::{Line, Plot};

struct GraphPlot {
    name: String,
    data: Vec<(Duration, f32)>,
}

pub struct Graph {
    name: String,
    plots: HashMap<String, GraphPlot>,
    start: Option<OffsetDateTime>,
    cursor_group: egui::widgets::plot::LinkedCursorsGroup,
}

impl Graph {
    pub fn new(
        name: &str,
        plots: &[&str],
        cursor_group: egui::widgets::plot::LinkedCursorsGroup,
    ) -> Self {
        Self {
            name: name.to_string(),
            plots: plots
                .iter()
                .map(|name| {
                    (
                        name.to_string(),
                        GraphPlot {
                            name: name.to_string(),
                            data: Vec::new(),
                        },
                    )
                })
                .collect(),
            start: None,
            cursor_group,
        }
    }

    pub fn add_data(&mut self, data_points: HashMap<String, f32>) {
        if self.start.is_none() {
            self.start = Some(OffsetDateTime::now_local().unwrap())
        }
        for (k, v) in data_points {
            let plot = self.plots.get_mut(&k).expect("failed to find plot");
            plot.data.push((
                OffsetDateTime::now_local().unwrap() - self.start.unwrap(),
                v,
            ));
        }
    }

    pub fn reset(&mut self) {
        self.start = None;
        self.plots.values_mut().for_each(|p| p.data.clear());
    }

    pub fn draw(&self, ui: &mut egui::Ui) {
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

        let width: f64 = plot_data
            .values()
            .map(|p| p.last().map(|[ts, _]| *ts).unwrap_or(0.))
            .fold(0., |a, b| a.max(b));

        let extreme = self
            .plots
            .values()
            .flat_map(|p| p.data.iter().map(|(_, v)| v.abs()).collect::<Vec<_>>())
            .fold(0., |a: f32, b: f32| a.max(b));

        let view_width = 280.;
        let view_height = 200.;

        Plot::new(&self.name)
            .data_aspect(if width == 0. {
                1.
            } else {
                width as f32 / (extreme * 2. * (view_width / view_height) * 1.1)
            })
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
                    .name(&plot.name);
                    plot_ui.line(line);
                }
            });
    }
}
