use serde::Deserialize;
use std::collections::HashMap; // New import

// New Enum for Button Modes
#[derive(Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ButtonMode {
    Trigger, // 1 on press, 0 on release
    Toggle,  // Switch between 1 and 0 on press, ignores release
}

impl Default for ButtonMode {
    fn default() -> Self {
        ButtonMode::Trigger
    }
}

// New Struct for Button-specific Settings
#[derive(Deserialize, Debug, Clone)]
pub(crate) struct ButtonConfig {
    #[serde(default)]
    pub mode: ButtonMode,
}

#[derive(Deserialize, Debug)]
pub(crate) struct Settings {
    #[serde(default)]
    pub notemaps: Vec<u8>,
    #[serde(default)]
    pub client_name: String,
    #[serde(default)]
    pub port_name: String,
    #[serde(default)]
    pub osc_ip: String, // New field for OSC IP
    #[serde(default)]
    pub osc_port: u16,  // New field for OSC Port
    #[serde(default)]
    pub button_configs: HashMap<String, ButtonConfig>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            notemaps: vec![
                49, 27, 31, 57, 48, 47, 43, 59, 36, 38, 46, 51, 36, 38, 42, 44,
            ],
            client_name: "Maschine Mikro MK3".to_string(),
            port_name: "Maschine Mikro MK3 MIDI".to_string(),
            osc_ip: "127.0.0.1".to_string(), // Default to localhost
            osc_port: 57120, // Default to a common OSC port
            button_configs: HashMap::new(), // Default to empty map
        }
    }
}

impl Settings {
    pub(crate) fn validate(&self) -> Result<(), String> {
        // todo: is there a better way to do it that doesn't bring too many new useless dependencies?

        let padcnt = self.notemaps.len();
        if padcnt != 16 {
            return Err(format!("The should be 16 pads exactly (found {padcnt})"));
        }

        if self.notemaps.iter().any(|x| *x >= 128) {
            return Err("MIDI notes should be 0 to 127".to_string());
        }

        if self.client_name.is_empty() {
            return Err("Client name must not be empty".to_string());
        }

        if self.port_name.is_empty() {
            return Err("Port name must not be empty".to_string());
        }

        Ok(())
    }
}
