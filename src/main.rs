use std::{fs::File, path::PathBuf};

use anyhow::Result;
use flate2::read::GzDecoder;
use lapce_plugin::{
    psp_types::{
        lsp_types::{request::Initialize, DocumentFilter, InitializeParams, MessageType, Url},
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
    let server_path = params
        .initialization_options
        .as_ref()
        .and_then(|options| options.get("serverPath"))
        .and_then(|server_path| server_path.as_str())
        .and_then(|server_path| {
            if !server_path.is_empty() {
                Some(server_path)
            } else {
                None
            }
        });

    if let Some(server_path) = server_path {
        let program = match std::env::var("VOLT_OS").as_deref() {
            Ok("windows") => "where",
            _ => "which",
        };
        let exits = PLUGIN_RPC
            .execute_process(program.to_string(), vec![server_path.to_string()])
            .map(|r| r.success)
            .unwrap_or(false);
        if !exits {
            PLUGIN_RPC.window_show_message(
                MessageType::ERROR,
                format!("server path {server_path} couldn't be found, please check"),
            );
            return Ok(());
        }
        PLUGIN_RPC.start_lsp(
            Url::parse(&format!("urn:{}", server_path))?,
            Vec::new(),
            vec![DocumentFilter {
                language: Some("rust".to_string()),
                scheme: None,
                pattern: None,
            }],
            params.initialization_options,
        );
        return Ok(());
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
        let result: Result<()> = {
            let url = format!(
                "https://github.com/rust-lang/rust-analyzer/releases/download/2023-02-13/{}.gz",
                file_name
            );
            let mut resp = Http::get(&url)?;
            let body = resp.body_read_all()?;
            std::fs::write(&gz_path, body)?;
            let mut gz = GzDecoder::new(File::open(&gz_path)?);
            let mut file = File::create(&file_path)?;
            std::io::copy(&mut gz, &mut file)?;
            std::fs::remove_file(&gz_path)?;
            Ok(())
        };
        if result.is_err() {
            PLUGIN_RPC.window_show_message(
                MessageType::ERROR,
                format!("can't download rust-analyzer, please use server path in the settings."),
            );
            return Ok(());
        }
    }

    let volt_uri = std::env::var("VOLT_URI")?;
    let server_path = Url::parse(&volt_uri)?.join(&file_name)?;
    PLUGIN_RPC.start_lsp(
        server_path,
        Vec::new(),
        vec![DocumentFilter {
            language: Some("rust".to_string()),
            scheme: None,
            pattern: None,
        }],
        params.initialization_options,
    );
    Ok(())
}

impl LapcePlugin for State {
    fn handle_request(&mut self, _id: u64, method: String, params: Value) {
        #[allow(clippy::single_match)]
        match method.as_str() {
            Initialize::METHOD => {
                let params: InitializeParams = serde_json::from_value(params).unwrap();
                if let Err(e) = initialize(params) {
                    PLUGIN_RPC.stderr(&format!("plugin returned with error: {e}"))
                }
            }
            _ => {}
        }
    }
}
