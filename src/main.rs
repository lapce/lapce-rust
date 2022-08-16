use std::{
    error::Error,
    fs::File,
    io::{self, Read, Write},
    path::PathBuf,
};

use anyhow::Result;
use flate2::read::GzDecoder;
use lapce_plugin::{register_plugin, Http, LapcePlugin, PLUGIN_RPC};
use lsp_types::{
    request::{Initialize, Request},
    InitializeParams,
};
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

fn initialize(params: InitializeParams) -> Result<()> {
    let arch = match std::env::var("ARCH").as_deref() {
        Ok("x86_64") => "x86_64",
        Ok("aarch64") => "aarch64",
        _ => return Ok(()),
    };
    let os = match std::env::var("OS").as_deref() {
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
        PLUGIN_RPC.stderr(&format!("url {url}"));
        let mut resp = Http::get(&url)?;
        PLUGIN_RPC.stderr(&format!("status code {}", resp.status_code));
        let body = resp.body_read_all()?;
        std::fs::write(&gz_path, body)?;
        let mut gz = GzDecoder::new(File::open(&gz_path)?);
        let mut file = File::create(&file_path)?;
        std::io::copy(&mut gz, &mut file)?;
        std::fs::remove_file(&gz_path)?;
    }

    PLUGIN_RPC.start_lsp(file_path, "rust", params.initialization_options);
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

    fn initialize(&mut self, info: serde_json::Value) {
        // let arch = match info.arch.as_str() {
        //     "x86_64" => "x86_64",
        //     "aarch64" => "aarch64",
        //     _ => return,
        // };
        // let os = match info.os.as_str() {
        //     "linux" => "unknown-linux-gnu",
        //     "macos" => "apple-darwin",
        //     "windows" => "pc-windows-msvc",
        //     _ => return,
        // };
        // let file_name = format!("rust-analyzer-{}-{}", arch, os);
        // let lock_file = PathBuf::from("donwload.lock");
        // send_notification(
        //     "lock_file",
        //     &json!({
        //         "path": &lock_file,
        //     }),
        // );
        // if !PathBuf::from(&file_name).exists() {
        //     let url = format!(
        //         "https://github.com/rust-analyzer/rust-analyzer/releases/download/2022-07-18/{}.gz",
        //         file_name
        //     );
        //     let gz_file = PathBuf::from(file_name.clone() + ".gz");

        //     if gz_file.exists() {
        //         std::fs::remove_file(&gz_file);
        //     }

        //     {
        //         // send_notification(
        //         //     "download_file",
        //         //     &json!({
        //         //         "url": url,
        //         //         "path": gz_file,
        //         //     }),
        //         // );
        //         if !gz_file.exists() {
        //             std::fs::remove_file(&lock_file);
        //             return;
        //         }
        //         eprintln!("start to unzip");
        //         let mut gz = GzDecoder::new(File::open(&gz_file).unwrap());
        //         let mut lsp_file = File::create(&file_name).unwrap();
        //         std::io::copy(&mut gz, &mut lsp_file).unwrap();
        //         // send_notification(
        //         //     "make_file_executable",
        //         //     &json!({
        //         //         "path": file_name,
        //         //     }),
        //         // );
        //     }
        //     std::fs::remove_file(gz_file);
        // }
        // std::fs::remove_file(&lock_file);

        // PLUGIN_RPC.start_lsp(PathBuf::from(file_name), "rust", info.configuration.options);
    }
}
