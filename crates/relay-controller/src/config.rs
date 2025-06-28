use crate::error::{Error, Result};
use esp_idf_svc::nvs::{EspDefaultNvsPartition, EspNvs};

#[derive(Debug)]
pub struct Config {
    pub hostname: Option<String>,
    pub wifi_ssid: String,
    pub wifi_pass: String,
}

impl Config {
    pub fn load(partition: EspDefaultNvsPartition) -> Result<Self> {
        let wifi_namespace = EspNvs::new(partition.clone(), "wifi", false)?;
        let ssid_key = "ssid";
        let password_key = "password";
        let device_namespace = EspNvs::new(partition, "device", false)?;
        let hostname_key = "hostname";

        let mut buf = [0; 100];
        let wifi_ssid = {
            match wifi_namespace
                .get_str(ssid_key, &mut buf)?
                .map(String::from)
            {
                Some(v) => v,
                None => return Err(Error::MissingConfig("wifi.ssid".into())),
            }
        };
        let wifi_pass = {
            match wifi_namespace
                .get_str(password_key, &mut buf)?
                .map(String::from)
            {
                Some(v) => v,
                None => return Err(Error::MissingConfig("wifi.password".into())),
            }
        };
        let hostname = {
            device_namespace
                .get_str(hostname_key, &mut buf)?
                .map(String::from)
        };

        Ok(Self {
            hostname,
            wifi_ssid,
            wifi_pass,
        })
    }
}
