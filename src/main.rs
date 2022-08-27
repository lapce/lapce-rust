use std::{fs::File, path::PathBuf};

use anyhow::Result;
use flate2::read::GzDecoder;
use lapce_plugin::{
    psp_types::{
        lsp_types::{request::Initialize, InitializeParams, Url},
        Request,
    },
    register_plugin, Http, LapcePlugin, PLUGIN_RPC,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

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

fn initialize(params: InitializeParams) -> Result<()> {
    if let Some(options) = params.initialization_options.as_ref() {
        if let Some(configuration) = options.get("configuration") {
            if let Some(server_path) = configuration.get("serverPath") {
                if let Some(server_path) = server_path.as_str() {
                    if !server_path.is_empty() {
                        PLUGIN_RPC.start_lsp(
                            Url::parse(&format!("urn:{}", server_path))?,
                            "rust",
                            params.initialization_options,
                        );
                        return Ok(());
                    }
                }
            }
        }
    }
    let arch = match std::env::var("VOLT_ARCH").as_deref() {
        Ok("x86_64") => "x86_64",
        Ok("aarch64") => "aarch64",
        _ => return Ok(()),
    };
    let os = match std::env::var("VOLT_OS").as_deref() {
        Ok("linux") => "unknown-linux-gnu",
        Ok("macos") => "apple-darwin",
        Ok("windows") => "pc-windows-msvc",
        _ => return Ok(()),
    };
    let file_name = format!("rust-analyzer-{}-{}", arch, os);
    let file_path = PathBuf::from(&file_name);
    let gz_path = PathBuf::from(file_name.clone() + ".gz");
    if !file_path.exists() {
        let url = format!(
            "https://github.com/rust-analyzer/rust-analyzer/releases/download/2022-07-18/{}.gz",
            file_name
        );
        let mut resp = Http::get(&url)?;
        let body = resp.body_read_all()?;
        std::fs::write(&gz_path, body)?;
        let mut gz = GzDecoder::new(File::open(&gz_path)?);
        let mut file = File::create(&file_path)?;
        std::io::copy(&mut gz, &mut file)?;
        std::fs::remove_file(&gz_path)?;
    }

    let volt_uri = std::env::var("VOLT_URI")?;
    let server_path = Url::parse(&volt_uri)?.join(&file_name)?;
    PLUGIN_RPC.start_lsp(server_path, "rust", params.initialization_options);
    Ok(())
}

impl LapcePlugin for State {
    fn handle_request(&mut self, id: u64, method: String, params: Value) {
        match method.as_str() {
            Initialize::METHOD => {
                let params: InitializeParams = serde_json::from_value(params).unwrap();
                let _ = initialize(params);
            }
            _ => {}
        }
    }
}
