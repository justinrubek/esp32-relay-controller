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

/// Controls relay states through GPIO pins
pub struct RelayController {
    relay0: Arc<Mutex<Option<Driver>>>,
    relay1: Arc<Mutex<Option<Driver>>>,
    states: Arc<[AtomicBool; 2]>,
}

impl RelayController {
    /// Creates a new relay controller with the specified pins
    pub fn new(pin0: AnyIOPin, pin1: AnyIOPin) -> Result<Self, EspError> {
        // Create pin drivers
        let mut relay0 = PinDriver::output(pin0)?;
        let mut relay1 = PinDriver::output(pin1)?;

        // Initialize as off
        relay0.set_low()?;
        relay1.set_low()?;

        // To make them 'static, we need to leak them
        let relay0: Driver = unsafe { std::mem::transmute(relay0) };
        let relay1: Driver = unsafe { std::mem::transmute(relay1) };

        let controller = Self {
            relay0: Arc::new(Mutex::new(Some(relay0))),
            relay1: Arc::new(Mutex::new(Some(relay1))),
            states: Arc::new([AtomicBool::new(false), AtomicBool::new(false)]),
        };

        info!("Relay controller initialized with 2 relays");
        Ok(controller)
    }

    /// Get the state of a specific relay
    pub fn get_state(&self, relay_id: usize) -> Option<bool> {
        if relay_id < 2 {
            Some(self.states[relay_id].load(Ordering::SeqCst))
        } else {
            None
        }
    }

    /// Get states of all relays
    pub fn get_all_states(&self) -> Vec<(usize, bool)> {
        vec![
            (0, self.states[0].load(Ordering::SeqCst)),
            (1, self.states[1].load(Ordering::SeqCst)),
        ]
    }

    pub fn set_state(&self, relay_id: usize, active: bool) -> Option<bool> {
        if relay_id >= 2 {
            return None;
        }

        let previous_state = self.states[relay_id].swap(active, Ordering::SeqCst);

        if previous_state == active {
            return Some(previous_state);
        }

        let result = match relay_id {
            0 => {
                let mut guard = self.relay0.lock().ok()?;
                if let Some(pin) = guard.as_mut() {
                    if active {
                        pin.set_high()
                    } else {
                        pin.set_low()
                    }
                } else {
                    Ok(())
                }
            }
            1 => {
                let mut guard = self.relay1.lock().ok()?;
                if let Some(pin) = guard.as_mut() {
                    if active {
                        pin.set_high()
                    } else {
                        pin.set_low()
                    }
                } else {
                    Ok(())
                }
            }
            _ => unreachable!(),
        };

        if let Err(e) = result {
            debug!("Failed to set relay {} state: {:?}", relay_id, e);
            self.states[relay_id].store(previous_state, Ordering::SeqCst);
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
            relay0: Arc::clone(&self.relay0),
            relay1: Arc::clone(&self.relay1),
            states: Arc::clone(&self.states),
        }
    }
}
