use std::{fs::File, path::PathBuf};

use anyhow::{anyhow, Result};
use flate2::read::GzDecoder;
use lapce_plugin::{
    psp_types::{
        lsp_types::{request::Initialize, DocumentFilter, InitializeParams, InitializeResult, Url},
        Request,
    },
    register_plugin, Http, LapcePlugin, VoltEnvironment, PLUGIN_RPC,
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

type LspParams = (Url, Vec<String>, Vec<DocumentFilter>, Option<Value>);

fn calculate_lsp_params(params: Value) -> Result<LspParams> {
    let params = serde_json::from_value::<InitializeParams>(params)?;
    let document_filters = vec![DocumentFilter {
        language: Some("rust".to_string()),
        scheme: None,
        pattern: Some("**/**.rs".to_string()),
    }];

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
        let program = match VoltEnvironment::operating_system().as_deref() {
            Ok("windows") => "where",
            _ => "which",
        };
        return match PLUGIN_RPC
            .execute_process(program.to_string(), vec![server_path.to_string()])
            .map(|r| r.success)
        {
            Ok(true) => Ok((
                Url::parse(&format!("urn:{server_path}"))?,
                Vec::new(),
                document_filters,
                params.initialization_options,
            )),
            Ok(false) => Err(anyhow!(
                "Cannot find the LSP binary at the server path provided. Please check."
            )),
            Err(err) => Err(anyhow!("Unable to execute command because {}", err)),
        };
    }

    let arch = match VoltEnvironment::architecture().as_deref() {
        Ok("x86_64") => "x86_64",
        Ok("aarch64") => "aarch64",
        Ok(o) => return Err(anyhow!("'{}' is not a supported architecture", o)),
        Err(_) => return Err(anyhow!("Unable to determine the CPU architecture in use")),
    };
    let os = match VoltEnvironment::operating_system().as_deref() {
        Ok("linux") => "unknown-linux-gnu",
        Ok("macos") => "apple-darwin",
        Ok("windows") => "pc-windows-msvc",
        Ok(o) => return Err(anyhow!("'{}' is not a supported operating system", o)),
        Err(_) => return Err(anyhow!("Unable to determine the operating system in use")),
    };
    let file_name = format!("rust-analyzer-{arch}-{os}");
    let file_path = PathBuf::from(&file_name);
    let gz_path = PathBuf::from(file_name.clone() + ".gz");
    if !file_path.exists() {
        let result: Result<()> = {
            let mut resp = Http::get(&format!(
                "https://github.com/rust-lang/rust-analyzer/releases/download/2023-01-02/{file_name}.gz"
            ))?;
            let body = resp.body_read_all()?;
            std::fs::write(&gz_path, body)?;
            let mut gz = GzDecoder::new(File::open(&gz_path)?);
            let mut file = File::create(&file_path)?;
            std::io::copy(&mut gz, &mut file)?;
            std::fs::remove_file(&gz_path)?;
            Ok(())
        };
        if let Err(err) = result {
            return Err(anyhow!(
                "can't download rust-analyzer because '{}'. Please use server path in the settings.",
                err.to_string()
            ));
        }
    }

    return Ok((
        Url::parse(&format!("urn:{}", VoltEnvironment::uri()?))?.join(&file_name)?,
        Vec::new(),
        document_filters,
        params.initialization_options,
    ));
}

impl LapcePlugin for State {
    fn handle_request(&mut self, id: u64, method: String, params: Value) {
        #[allow(clippy::single_match)]
        match method.as_str() {
            Initialize::METHOD => match calculate_lsp_params(params.clone()) {
                Ok((uri, args, filters, params)) => {
                    PLUGIN_RPC.start_lsp(uri, args, filters, params).unwrap();
                    PLUGIN_RPC
                        .host_success(id, InitializeResult::default())
                        .unwrap()
                }
                Err(err) => PLUGIN_RPC.host_error(id, err.to_string()).unwrap(),
            },
            o => PLUGIN_RPC
                .host_error(id, format!("Plugin does not understand method '{o}'"))
                .unwrap(),
        };
    }
}
