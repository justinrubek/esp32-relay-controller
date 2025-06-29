use crate::{
    config::Config, error::Result, ota::OtaHandler, relay::RelayController, server::run_server,
};
use esp_idf_hal::prelude::Peripherals;
use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs, timer::EspTaskTimerService};
use log::{error, info};
use plan9::Plan9Connection;
use std::sync::Arc;
use wifi::WifiConnection;

mod config;
mod error;
mod ota;
mod plan9;
mod relay;
mod server;
mod wifi;

fn main() {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let result = esp_idf_svc::io::vfs::MountedEventfs::mount(10);
    match result {
        Ok(_) => info!("EventFD initialized successfully"),
        Err(e) => {
            error!("Failed to initialize EventFD: {:?}", e);
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
    }

    std::thread::Builder::new()
        .stack_size(60000)
        .spawn(runtime)
        .unwrap()
        .join()
        .unwrap()
        .unwrap();
}

fn runtime() -> Result<()> {
    info!("initializing tokio runtime");

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| display_error("tokio runtime", e))?;

    match rt.block_on(async { async_main().await }) {
        Ok(()) => info!("main() finished"),
        Err(e) => error!("main() failed: {e:?}"),
    }

    info!("rebooting");
    esp_idf_hal::reset::restart();
}

async fn async_main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let event_loop = EspSystemEventLoop::take()?;
    let timer = EspTaskTimerService::new()?;
    let peripherals = Peripherals::take()?;
    let nvs_default_partition = nvs::EspDefaultNvsPartition::take()?;

    info!("loading configuration from nvs");
    let config = Config::load(nvs_default_partition.clone())?;

    let relay_controller = RelayController::new(vec![
        peripherals.pins.gpio12.into(),
        peripherals.pins.gpio13.into(),
    ])?;
    let relay_controller = Arc::new(relay_controller);

    info!("iniializing networking");
    // initialize network before starting the server
    let mut wifi_connection = WifiConnection::new(
        peripherals.modem,
        event_loop,
        timer.clone(),
        Some(nvs_default_partition.clone()),
        &config,
    )
    .await?;

    let mut ota_handler = OtaHandler::new(
        "nas:4501".into(),
        "/esp32/relay-controller".into(),
        timer.clone(),
    )
    .await?;

    tokio::try_join!(
        run_server(wifi_connection.state.clone(), relay_controller),
        wifi_connection.connect(),
        ota_handler.run(),
    )?;

    Ok(())
}

fn display_error<T: std::fmt::Debug>(ctx: &str, err: T) -> T {
    log::error!("{ctx}: {err:?}");
    std::thread::sleep(std::time::Duration::from_secs(8));
    err
}
