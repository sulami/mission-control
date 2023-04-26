use crate::config::Color;

pub const RED: egui::Color32 = egui::Color32::from_rgb(231, 111, 81);
pub const ORANGE: egui::Color32 = egui::Color32::from_rgb(244, 162, 97);
pub const YELLOW: egui::Color32 = egui::Color32::from_rgb(233, 196, 106);
pub const GREEN: egui::Color32 = egui::Color32::from_rgb(42, 157, 143);
pub const BLUE: egui::Color32 = egui::Color32::from_rgb(69, 123, 157);

pub fn egui_color(color: Color) -> egui::Color32 {
    match color {
        Color::Red => RED,
        Color::Orange => ORANGE,
        Color::Yellow => YELLOW,
        Color::Green => GREEN,
        Color::Blue => BLUE,
    }
}
