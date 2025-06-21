#[derive(Debug)]
pub struct Config {
    pub hostname: Option<&'static str>,
    pub wifi_ssid: &'static str,
    pub wifi_pass: &'static str,
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            hostname: option_env!("HOSTNAME"),
            wifi_ssid: env!("WIFI_SSID"),
            wifi_pass: env!("WIFI_PASS"),
        })
    }
}
