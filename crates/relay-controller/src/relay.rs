use esp_idf_hal::{
    gpio::{Output, PinDriver},
    sys::EspError,
};
use esp_idf_svc::hal::gpio::AnyIOPin;
use log::{debug, info};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

type Driver = PinDriver<'static, AnyIOPin, Output>;

fn pin_driver(pin: AnyIOPin) -> Result<Arc<Mutex<Option<Driver>>>, EspError> {
    let mut driver = PinDriver::output(pin)?;
    driver.set_low()?;
    Ok(Arc::new(Mutex::new(Some(driver))))
}

pub struct RelayController {
    relays: Vec<Arc<Mutex<Option<Driver>>>>,
    states: Arc<Vec<AtomicBool>>,
}

impl RelayController {
    /// Creates a new relay controller with the specified pins
    pub fn new(pins: Vec<AnyIOPin>) -> Result<Self, EspError> {
        let mut relays = Vec::with_capacity(pins.len());
        let mut states = Vec::with_capacity(pins.len());

        for pin in pins {
            relays.push(pin_driver(pin)?);
            states.push(AtomicBool::new(false));
        }

        let controller = Self {
            relays,
            states: Arc::new(states),
        };

        Ok(controller)
    }

    pub fn get_state(&self, relay_id: usize) -> Option<bool> {
        self.states
            .get(relay_id)
            .map(|state| state.load(Ordering::SeqCst))
    }

    pub fn get_all_states(&self) -> Vec<(usize, bool)> {
        self.states
            .iter()
            .enumerate()
            .map(|(idx, state)| (idx, state.load(Ordering::SeqCst)))
            .collect()
    }

    pub fn set_state(&self, relay_id: usize, active: bool) -> Option<bool> {
        let state = self.states.get(relay_id)?;
        let previous_state = state.swap(active, Ordering::SeqCst);

        if previous_state == active {
            return Some(previous_state);
        }

        let relay = self.relays.get(relay_id)?;
        let mut guard = relay.lock().ok()?;

        let result = if let Some(pin) = guard.as_mut() {
            if active {
                pin.set_high()
            } else {
                pin.set_low()
            }
        } else {
            Ok(())
        };

        if let Err(e) = result {
            debug!("Failed to set relay {} state: {:?}", relay_id, e);
            state.store(previous_state, Ordering::SeqCst);
            return None;
        }

        info!(
            "Relay {} set to {}",
            relay_id,
            if active { "ON" } else { "OFF" }
        );
        Some(previous_state)
    }

    pub fn toggle(&self, relay_id: usize) -> Option<bool> {
        let current_state = self.get_state(relay_id)?;
        self.set_state(relay_id, !current_state)
    }
}

impl Clone for RelayController {
    fn clone(&self) -> Self {
        Self {
            relays: self.relays.clone(),
            states: Arc::clone(&self.states),
        }
    }
}
