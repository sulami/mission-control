use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Config {
    /// Number of seconds of data to display in graphs
    pub window_size: f32,

    /// Number of seconds after which data is considered stale
    pub data_timeout: f32,

    /// Telemetry source
    pub serial: Serial,

    /// Telemetry data to plot
    pub graphs: Vec<Graph>,

    /// Preset commands
    pub commands: Vec<Command>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Serial {
    /// Data input serial port
    pub path: String,

    /// Serial port baud rate
    pub baud: u32,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Graph {
    pub name: String,
    pub plots: Vec<Plot>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Plot {
    pub name: String,
    pub source_name: String,
    pub color: Color,
}

#[derive(Copy, Clone, Debug, Default, Deserialize)]
pub enum Color {
    #[default]
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Command {
    pub name: String,
    pub command: String,
    pub color: Color,
}

pub fn load_config(path: &PathBuf) -> Result<Config> {
    toml::from_str(&fs::read_to_string(path).context("unable to read config file")?)
        .context("unable to parse config file")
}
