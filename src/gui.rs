use eframe::egui;
use egui::widgets::plot::LinkedCursorsGroup;
use time::OffsetDateTime;

mod graph;

use crate::config::Config;
use graph::Graph;

pub fn run(cfg: Config) {
    let native_options = eframe::NativeOptions::default();
    let _ = eframe::run_native(
        "Mission Control",
        native_options,
        Box::new(|cc| Box::new(App::new(cc, cfg))),
    );
}

struct App {
    start_time: OffsetDateTime,
    graphs: Vec<Graph>,
    input_text: String,
}

impl App {
    fn new(_cc: &eframe::CreationContext<'_>, cfg: Config) -> Self {
        let cursor_group = LinkedCursorsGroup::new(true, false);
        Self {
            start_time: OffsetDateTime::now_local().expect("failed to get local time"),
            graphs: cfg
                .graphs
                .iter()
                .map(|(name, g)| {
                    Graph::new(
                        name,
                        &g.plots
                            .iter()
                            .map(|p| p.name.as_str())
                            .collect::<Vec<&str>>(),
                        cursor_group.clone(),
                    )
                })
                .collect(),
            input_text: String::new(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::containers::TopBottomPanel::top("Status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!(
                    "MCT: {:?}",
                    OffsetDateTime::now_local().unwrap() - self.start_time
                ));
                ui.label("System Status: Connected");
                ui.label("Mission Status: Go");
            });
        });

        egui::containers::TopBottomPanel::bottom("Input").show(ctx, |ui| {
            let response = ui.add(
                egui::widgets::TextEdit::singleline(&mut self.input_text)
                    .desired_width(ui.available_width()),
            );
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.input_text.clear();
                response.request_focus();
            }
        });

        egui::containers::SidePanel::left("Commands")
            .resizable(false)
            .show(ctx, |ui| {
                egui::ScrollArea::new([true, true])
                    .hscroll(false)
                    .show(ui, |ui| {
                        ui.set_width(140.);
                        ui.heading("Commands");
                        ui.set_width(120.);
                        ui.vertical(|ui| {
                            if ui.button("Power on check").clicked() {};
                        });
                    });
            });

        egui::containers::SidePanel::right("System")
            .resizable(false)
            .show(ctx, |ui| {
                egui::ScrollArea::new([true, true])
                    .hscroll(false)
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.set_width(140.);
                            ui.heading("System");
                            if ui.button("Save to disk").clicked() {};

                            ui.add_space(20.);

                            if ui.button("Reset").clicked() {
                                for graph in &mut self.graphs {
                                    graph.reset();
                                }
                            };
                            if ui.button("Quit").clicked() {
                                std::process::exit(0);
                            };
                        });
                    });
            });

        egui::containers::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Telemetry");
            egui::ScrollArea::new([true, true])
                .hscroll(false)
                .show(ui, |ui| {
                    ui.with_layout(
                        egui::Layout::left_to_right(egui::Align::TOP).with_main_wrap(true),
                        |ui| {
                            for graph in &self.graphs {
                                graph.draw(ui);
                            }
                        },
                    )
                })
        });
    }
}
