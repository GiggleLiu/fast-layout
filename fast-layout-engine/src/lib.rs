//! Graph coordinates in pure Rust. The optional WASM entry point uses CBOR.
#[cfg(target_arch = "wasm32")]
use wasm_minimal_protocol::*;
#[cfg(target_arch = "wasm32")]
initiate_protocol!();

mod barnes_hut;
mod buchheim;
mod eigen;
mod graph;
mod schema;
mod shell;
mod spectral;
mod spring;
mod stress;
pub use schema::{Algorithm, Request, Response};

/// Compute positions in node-index order without a serialization round trip.
pub fn compute(request: &Request) -> Result<Response, String> {
    request.validate()?;
    let response = match request.algorithm {
        Algorithm::Stress => stress::run(request),
        Algorithm::Spring => spring::run(request),
        Algorithm::Spectral => spectral::run(request),
        Algorithm::Shell => shell::run(request),
        Algorithm::Buchheim => buchheim::run(request),
    }?;
    if response.positions.iter().flatten().any(|x| !x.is_finite())
        || response.objective.is_some_and(|x| !x.is_finite())
    {
        return Err(
            "fast-layout-engine: numerical overflow; reduce coordinate or weight magnitudes".into(),
        );
    }
    Ok(response)
}

#[cfg_attr(target_arch = "wasm32", wasm_func)]
pub fn version() -> Vec<u8> {
    format!("fast-layout-engine {}", env!("CARGO_PKG_VERSION")).into_bytes()
}

#[cfg_attr(target_arch = "wasm32", wasm_func)]
pub fn layout(input: &[u8]) -> Result<Vec<u8>, String> {
    if input.len() > 32 * 1024 * 1024 {
        return Err("fast-layout-engine: CBOR request exceeds 32 MiB".into());
    }
    let mut reader = input;
    let request: Request = ciborium::from_reader(&mut reader)
        .map_err(|e| format!("fast-layout-engine: invalid request: {e}"))?;
    if !reader.is_empty() {
        return Err("fast-layout-engine: trailing bytes after CBOR request".into());
    }
    let response = compute(&request)?;
    let mut output = Vec::new();
    ciborium::into_writer(&response, &mut output)
        .map_err(|e| format!("fast-layout-engine: cannot encode positions: {e}"))?;
    Ok(output)
}
