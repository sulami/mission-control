use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Default, Deserialize)]
pub struct Config {
    /// Telemetry source
    pub source: Source,

    /// Telemetry data to plot
    pub graphs: HashMap<String, Graph>,

    /// Preset commands
    pub commands: Vec<Command>,
}

#[derive(Deserialize)]
pub enum Source {
    Serial {
        /// Data input serial port
        path: String,

        /// Serial port baud rate
        baud: usize,
    },
}

impl Default for Source {
    fn default() -> Self {
        Self::Serial {
            path: "".into(),
            baud: 9600,
        }
    }
}

#[derive(Default, Deserialize)]
pub struct Graph {
    pub plots: Vec<Plot>,
}

#[derive(Default, Deserialize)]
pub struct Plot {
    pub name: String,
    pub source_name: String,
    pub color: Color,
}

#[derive(Default, Deserialize)]
pub enum Color {
    #[default]
    Red,
    Green,
    Blue,
}

#[derive(Default, Deserialize)]
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
