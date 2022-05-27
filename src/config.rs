use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use log::{debug, trace};
use serde::{Serialize, Deserialize};
use crate::JResult;

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub generator: Generator
}

#[derive(Serialize, Deserialize, Default)]
pub struct Generator {
    /// Mappings from a Java type to a Rust type
    /// E.g. java.lang.String -> ejni::String.
    ///
    /// The Rust type must impl Into<jni::JValue>
    pub mappings: HashMap<String, String>,
}

impl Config {
    /// Create a new Config instance. Read the configuration from `./config.toml`,
    /// creates it if it does not already exist.
    ///
    /// # Errors
    ///
    /// If an IO error occurs, or if (de)serializing fails
    pub fn new() -> JResult<Self> {
        let path = Path::new("./config.toml");
        if !path.exists() {
            debug!("Config file does not exist");
            let this = Self::default();

            trace!("Creating config file");
            let mut f = fs::File::create(path)?;
            trace!("Serializing default config");
            let toml = toml::to_string_pretty(&this)?;

            trace!("Writing default config");
            f.write_all(toml.as_bytes())?;
            return Ok(this);
        }

        debug!("Config file exists");
        trace!("Opening config file");
        let mut f = fs::File::open("./config.toml")?;

        trace!("Reading config file");
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;

        trace!("Deserrializing config");
        let this: Self = toml::from_slice(&buf)?;
        Ok(this)
    }
}