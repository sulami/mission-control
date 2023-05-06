use anyhow::Result;
use eframe::egui;
use egui::widgets::plot::LinkedCursorsGroup;
use time::{Duration, OffsetDateTime};
use tokio::sync::broadcast::{Receiver, Sender};

mod color;
mod graph;

use crate::{config::Config, Command, Message};
use color::*;
use graph::Graph;

// TODO: Add a pause button for graph updates. I guess just drop
// telemetry on the floor.

pub fn run(cfg: Config, rx: Receiver<Message>, tx: Sender<Message>) -> Result<()> {
    let native_options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1024., 768.)),
        // maximized: true,
        ..Default::default()
    };
    eframe::run_native(
        "Mission Control",
        native_options,
        Box::new(|cc| Box::new(App::new(cc, cfg, rx, tx))),
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
    rx: Receiver<Message>,
    tx: Sender<Message>,
}

impl App {
    fn new(
        _cc: &eframe::CreationContext<'_>,
        cfg: Config,
        rx: Receiver<Message>,
        tx: Sender<Message>,
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
            rx,
            tx,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let now = OffsetDateTime::now_local().unwrap();

        while let Ok(msg) = self.rx.try_recv() {
            if let Message::Telemetry(frame) = msg {
                for graph in self.graphs.iter_mut() {
                    graph.add_data(&frame);
                }
                self.last_data = now;
            }
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

                ui.separator();
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
                ui.separator();
                ui.label(format!("GCT: {:.0}", now - self.start_time));
                if data_stale {
                    ui.separator();
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
                send_command(Command::SendCommand(self.input_text.clone()), &self.tx);
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
                                    send_command(
                                        Command::SendCommand(command.command.clone()),
                                        &self.tx,
                                    );
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
                                send_command(Command::Export, &self.tx);
                            };

                            ui.add_space(20.);

                            if ui.button("Reset").clicked() {
                                for graph in &mut self.graphs {
                                    send_command(Command::Reset, &self.tx);
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
            egui::ScrollArea::new([true, true])
                .auto_shrink([false, false])
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

        ctx.request_repaint_after(std::time::Duration::from_millis(20));
    }
}

fn send_command(cmd: Command, bus: &Sender<Message>) {
    if let Err(e) = bus.send(Message::Command(cmd)) {
        eprintln!("[SYSTEM] Error sending command: {}", e);
    }
}
