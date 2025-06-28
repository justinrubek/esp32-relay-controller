use crate::{
    config::Config,
    error::{Error, Result},
};
use embedded_svc::wifi::{ClientConfiguration, Configuration};
use esp_idf_hal::modem::Modem;
use esp_idf_svc::{
    eventloop::{EspEventLoop, System},
    ipv4::{self, DHCPClientSettings},
    netif::{self, EspNetif},
    nvs::EspDefaultNvsPartition,
    timer::{EspTimerService, Task},
    wifi::{AsyncWifi, EspWifi, WifiDriver},
};
use log::{info, warn};
use std::{net::Ipv4Addr, str::FromStr, sync::Arc, time::Duration};

use tokio::{sync::RwLock, time::sleep};

#[allow(dead_code)]
pub struct WifiState {
    pub mac_address: String,
    pub ssid: String,
    ip_addr: RwLock<Option<Ipv4Addr>>,
}

#[allow(dead_code)]
impl WifiState {
    pub async fn ip_addr(&self) -> Option<Ipv4Addr> {
        *self.ip_addr.read().await
    }
}

pub struct WifiConnection<'a> {
    pub state: Arc<WifiState>,
    wifi: AsyncWifi<EspWifi<'a>>,
}

impl<'a> WifiConnection<'a> {
    /// Initializes the wifi driver.
    pub async fn new(
        modem: Modem,
        event_loop: EspEventLoop<System>,
        timer: EspTimerService<Task>,
        default_partition: Option<EspDefaultNvsPartition>,
        config: &Config,
    ) -> Result<Self> {
        info!("initializing wifi driver");

        let dhcp_settings = match &config.hostname {
            Some(name) => ipv4::DHCPClientSettings {
                hostname: Some(
                    name.as_str()
                        .try_into()
                        .map_err(|_| Error::HostnameTooLong)?,
                ),
            },

            _ => DHCPClientSettings::default(),
        };

        let wifi_driver = WifiDriver::new(modem, event_loop.clone(), default_partition)?;
        let ipv4_config = ipv4::ClientConfiguration::DHCP(dhcp_settings);
        let net_if = EspNetif::new_with_conf(&netif::NetifConfiguration {
            ip_configuration: Some(ipv4::Configuration::Client(ipv4_config)),
            ..netif::NetifConfiguration::wifi_default_client()
        })?;

        let mac = net_if.get_mac()?;
        let mac_address = format!(
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
        );
        let state = Arc::new(WifiState {
            ip_addr: RwLock::new(None),
            mac_address,
            ssid: config.wifi_ssid.to_string(),
        });

        let esp_wifi =
            EspWifi::wrap_all(wifi_driver, net_if, EspNetif::new(netif::NetifStack::Ap)?)?;
        let mut wifi = AsyncWifi::wrap(esp_wifi, event_loop, timer.clone())?;

        info!("setting credentials");
        let client_config = ClientConfiguration {
            ssid: heapless::String::from_str(&config.wifi_ssid).map_err(|_| Error::SsidTooLong)?,
            password: heapless::String::from_str(&config.wifi_pass)
                .map_err(|_| Error::PasswordTooLong)?,
            ..Default::default()
        };
        wifi.set_configuration(&Configuration::Client(client_config))?;

        info!("starting wifi");
        wifi.start().await?;

        info!("wifi initializtion success");
        Ok(Self { state, wifi })
    }

    pub async fn connect(&mut self) -> Result<()> {
        loop {
            info!("Connecting to SSID '{}'...", self.state.ssid);
            if let Err(err) = self.wifi.connect().await {
                warn!("Connection failed: {err:?}");
                self.wifi.disconnect().await?;
                sleep(Duration::from_secs(1)).await;
                continue;
            }

            info!("Acquiring IP address...");
            let timeout = Some(Duration::from_secs(10));
            if let Err(err) = self
                .wifi
                .ip_wait_while(|w| w.is_up().map(|s| !s), timeout)
                .await
            {
                warn!("IP association failed: {err:?}");
                self.wifi.disconnect().await?;
                sleep(Duration::from_secs(1)).await;
                continue;
            }

            let ip_info = self.wifi.wifi().sta_netif().get_ip_info();
            *self.state.ip_addr.write().await = ip_info.ok().map(|i| i.ip);
            info!("Connected to '{}': {ip_info:#?}", self.state.ssid);

            // Wait for Wi-Fi to be down
            self.wifi.wifi_wait(|w| w.is_up(), None).await?;
            warn!("Wi-Fi disconnected.");
        }
    }
}
