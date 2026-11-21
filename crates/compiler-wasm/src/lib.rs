//! Browser-facing adapter for the shared Krunt compiler JSON bridge.

use krunt_compiler_bridge::handle_json;
use wasm_bindgen::prelude::*;

/// Compile or validate one versioned JSON request.
///
/// Successful calls return the versioned compiler response JSON. Envelope
/// failures return a small JSON error object so JavaScript callers can surface
/// a transport failure without mistaking it for compiler diagnostics.
#[wasm_bindgen]
pub fn compile_json(request_json: String) -> String {
	match handle_json(&request_json) {
		Ok(response) => response,
		Err(error) => serde_json::json!({
			"error": error.to_string(),
		})
		.to_string(),
	}
}

#[cfg(test)]
mod tests {
	use super::compile_json;

	#[test]
	fn returns_a_json_error_for_an_invalid_envelope() {
		let response = compile_json("not-json".to_string());
		assert!(response.contains("error"));
	}
}
