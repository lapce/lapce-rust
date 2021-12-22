use lapce_plugin::{register_plugin, start_lsp, LapcePlugin};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Default)]
struct State {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Configuration {
    language_id: String,
    lsp_server: String,
    options: Option<Value>,
}

register_plugin!(State);

impl LapcePlugin for State {
    fn initialize(&mut self, config: serde_json::Value) {
        if let Ok(config) = serde_json::from_value::<Configuration>(config) {
            start_lsp(&config.lsp_server, &config.language_id, config.options);
        }
    }
}
