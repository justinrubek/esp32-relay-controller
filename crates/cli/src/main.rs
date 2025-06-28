use std::{error::Error, fs::File, str::FromStr};

use crate::{commands::Commands, config::Config};
use clap::Parser;
use nvs_writer::{Key, Partition};

mod commands;
mod config;
mod error;

fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();

    let args = commands::Args::parse();
    match args.command {
        Commands::Write(write) => {
            let config = Config::load();

            let mut file = File::create(write.output)?;

            let mut partition: Partition = Partition::new();

            let wifi_namespace = Key::from_str("wifi").unwrap();
            partition.add_string_entry(
                &wifi_namespace,
                &Key::from_str("ssid").unwrap(),
                config.wifi_ssid,
            )?;
            partition.add_string_entry(
                &wifi_namespace,
                &Key::from_str("password").unwrap(),
                config.wifi_pass,
            )?;

            let host_namespace = Key::from_str("device").unwrap();
            partition.add_string_entry(
                &host_namespace,
                &Key::from_str("hostname").unwrap(),
                config.hostname.unwrap_or("noname"),
            )?;

            partition.write(&mut file)?;
        }
    }

    Ok(())
}
