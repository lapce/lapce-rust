use std::{
    fs::File,
    path::PathBuf
};

use flate2::read::GzDecoder;
use lapce_plugin::{register_plugin, send_notification, start_lsp, LapcePlugin};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Default)]
struct State {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    arch: String,
    os: String,
    configuration: Configuration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Configuration {
    language_id: String,
    options: Option<Value>,
}

register_plugin!(State);

impl LapcePlugin for State {
    fn initialize(&mut self, info: serde_json::Value) {
        let info = serde_json::from_value::<PluginInfo>(info).unwrap();
        let arch = match info.arch.as_str() {
            "x86_64" => "x86_64",
            "aarch64" => "aarch64",
            _ => return,
        };
        let os = match info.os.as_str() {
            "linux" => "unknown-linux-gnu",
            "macos" => "apple-darwin",
            "windows" => "pc-windows-msvc",
            _ => return,
        };
        let file_name = format!("rust-analyzer-{}-{}", arch, os);
        let lock_file = PathBuf::from("download.lock");
        send_notification(
            "lock_file",
            &json!({
                "path": &lock_file,
            }),
        );
        if !PathBuf::from(&file_name).exists() {
            let url = format!(
                "https://github.com/rust-analyzer/rust-analyzer/releases/download/2022-07-25/{}.gz",
                file_name
            );
            let gz_file = PathBuf::from(file_name.clone() + ".gz");

            if gz_file.exists() {
                std::fs::remove_file(&gz_file).ok();
            }

            {
                send_notification(
                    "download_file",
                    &json!({
                        "url": url,
                        "path": gz_file,
                    }),
                );
                if !gz_file.exists() {
                    std::fs::remove_file(&lock_file).ok();
                    return;
                }
                eprintln!("start to unzip");
                let mut gz = GzDecoder::new(File::open(&gz_file).unwrap());
                let mut lsp_file = File::create(&file_name).unwrap();
                std::io::copy(&mut gz, &mut lsp_file).unwrap();
                send_notification(
                    "make_file_executable",
                    &json!({
                        "path": file_name,
                    }),
                );
            }
            std::fs::remove_file(gz_file).ok();
        }
        std::fs::remove_file(&lock_file).ok();

        start_lsp(&file_name, "rust", info.configuration.options);
    }
}
