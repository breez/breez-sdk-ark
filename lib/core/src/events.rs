use crate::models::Payment;
use std::collections::HashMap;
use std::sync::Mutex;

/// Enum representing different SDK events
#[derive(Clone, Debug)]
pub enum SdkEvent {
    /// Wallet has been synced with the network
    Synced {},

    /// Sucesfull Payment
    PaymentSucceeded {
        /// The payment details
        payment: Payment,
    },

    /// A pending payment
    PaymentPending {
        /// The payment details
        payment: Payment,
    },
}

/// Trait for event listeners
pub trait EventListener: Send + Sync {
    /// Called when an event occurs
    fn on_event(&self, event: &SdkEvent);
}

/// Event emitter for SDK events
pub struct EventEmitter {
    listeners: Mutex<HashMap<String, Box<dyn EventListener>>>,
}

impl EventEmitter {
    /// Creates a new event emitter
    pub fn new() -> Self {
        Self {
            listeners: Mutex::new(HashMap::new()),
        }
    }

    /// Adds a listener to the event emitter
    ///
    /// # Arguments
    ///
    /// * `listener` - The listener to add
    ///
    /// # Returns
    ///
    /// A unique ID for the listener
    pub fn add_listener(&self, listener: Box<dyn EventListener>) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        let mut listeners = self.listeners.lock().unwrap();
        listeners.insert(id.clone(), listener);
        id
    }

    /// Removes a listener from the event emitter
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the listener to remove
    ///
    /// # Returns
    ///
    /// `true` if the listener was found and removed, `false` otherwise
    pub fn remove_listener(&self, id: &str) -> bool {
        let mut listeners = self.listeners.lock().unwrap();
        listeners.remove(id).is_some()
    }

    /// Emits an event to all listeners
    ///
    /// # Arguments
    ///
    /// * `event` - The event to emit
    pub fn emit(&self, event: &SdkEvent) {
        let listeners = self.listeners.lock().unwrap();
        for listener in listeners.values() {
            listener.on_event(event);
        }
    }
}
