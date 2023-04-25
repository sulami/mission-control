use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Config {
    /// Telemetry source
    pub serial: Serial,

    /// Telemetry data to plot
    pub graphs: HashMap<String, Graph>,

    /// Preset commands
    pub commands: Vec<Command>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Serial {
    /// Data input serial port
    pub path: String,

    /// Serial port baud rate
    pub baud: usize,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Graph {
    pub plots: Vec<Plot>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Plot {
    pub name: String,
    pub source_name: String,
    pub color: Color,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub enum Color {
    #[default]
    Red,
    Green,
    Blue,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Command {
    name: String,
    command: String,
}

pub fn load_config(path: &PathBuf) -> Result<Config> {
    toml::from_str(&fs::read_to_string(path).context("unable to read config file")?)
        .context("unable to parse config file")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load() {}
}
