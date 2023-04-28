use anyhow::Result;
use eframe::egui;
use egui::widgets::plot::LinkedCursorsGroup;
use time::{Duration, OffsetDateTime};
use tokio::sync::broadcast::{Receiver, Sender};

mod color;
mod graph;

use crate::config::Config;
use crate::telemetry::Frame;
use crate::Command;
use color::*;
use graph::Graph;

pub fn run(
    cfg: Config,
    telemetry_bus: Receiver<Frame>,
    command_bus: Sender<Command>,
) -> Result<()> {
    let native_options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1024., 768.)),
        // maximized: true,
        ..Default::default()
    };
    eframe::run_native(
        "Mission Control",
        native_options,
        Box::new(|cc| Box::new(App::new(cc, cfg, telemetry_bus, command_bus))),
    )
    .map_err(|e| anyhow::anyhow!("Failed to run gui: {}", e))?;
    Ok(())
}

struct App {
    start_time: OffsetDateTime,
    last_data: OffsetDateTime,
    config: Config,
    graphs: Vec<Graph>,
    input_text: String,
    telemetry_bus: Receiver<Frame>,
    command_bus: Sender<Command>,
}

impl App {
    fn new(
        _cc: &eframe::CreationContext<'_>,
        cfg: Config,
        telemetry_bus: Receiver<Frame>,
        command_bus: Sender<Command>,
    ) -> Self {
        let cursor_group = LinkedCursorsGroup::new(true, false);
        let now = OffsetDateTime::now_local().expect("failed to get local time");
        Self {
            start_time: now,
            last_data: now,
            config: cfg.clone(),
            graphs: cfg
                .graphs
                .iter()
                .map(|(name, g)| {
                    Graph::new(
                        name,
                        &g.plots
                            .iter()
                            .map(|p| (p.name.clone(), p.source_name.clone(), p.color))
                            .collect::<Vec<_>>(),
                        Duration::seconds_f32(cfg.window_size),
                        cursor_group.clone(),
                    )
                })
                .collect(),
            input_text: String::new(),
            telemetry_bus,
            command_bus,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let now = OffsetDateTime::now_local().unwrap();

        while let Ok(frame) = self.telemetry_bus.try_recv() {
            for graph in self.graphs.iter_mut() {
                graph.add_data(&frame);
            }
            self.last_data = now;
        }

        egui::containers::TopBottomPanel::top("Status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let data_age = now - self.last_data;
                let data_stale = data_age > Duration::seconds_f32(self.config.data_timeout);
                let status = if data_stale { "DATA STALE" } else { "GO" };
                ui.label("System status:");
                ui.label(
                    egui::RichText::new(status)
                        .background_color(if data_stale {
                            egui::Color32::from_rgb(231, 111, 81)
                        } else {
                            egui::Color32::from_rgb(42, 157, 143)
                        })
                        .color(egui::Color32::BLACK)
                        .strong(),
                );

                ui.label(format!(
                    "CLT: {}",
                    now.format(
                        &time::format_description::parse(
                            "[year]-[month]-[day] [hour]:[minute]:[second] [offset_hour \
         sign:mandatory][offset_minute]"
                        )
                        .unwrap()
                    )
                    .unwrap()
                ));
                ui.label(format!("GCT: {:.0}", now - self.start_time));
                if data_stale {
                    ui.label(format!("LDT: {:.2}", data_age));
                }
            });
        });

        egui::containers::TopBottomPanel::bottom("Input").show(ctx, |ui| {
            let response = ui.add(
                egui::widgets::TextEdit::singleline(&mut self.input_text)
                    .desired_width(ui.available_width()),
            );
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.command_bus
                    .send(Command::SendCommand(self.input_text.clone()));
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
                            for command in &self.config.commands {
                                let button = egui::Button::new(
                                    egui::RichText::new(&command.name)
                                        .color(egui::Color32::BLACK)
                                        .strong(),
                                )
                                .fill(egui_color(command.color));
                                if ui.add(button).clicked() {
                                    self.command_bus
                                        .send(Command::SendCommand(command.command.clone()));
                                };
                            }
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
                            if ui.button("Save to disk").clicked() {
                                self.command_bus.send(Command::Export);
                            };

                            ui.add_space(20.);

                            if ui.button("Reset").clicked() {
                                for graph in &mut self.graphs {
                                    self.command_bus.send(Command::Reset);
                                    graph.reset();
                                }
                            };
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("Quit")
                                            .color(egui::Color32::BLACK)
                                            .strong(),
                                    )
                                    .fill(RED),
                                )
                                .clicked()
                            {
                                std::process::exit(0);
                            };
                        });
                    });
            });

        egui::containers::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Telemetry");
            // TODO expand the scroll area to fill the available space
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

        ctx.request_repaint_after(std::time::Duration::from_millis(10));
    }
}
