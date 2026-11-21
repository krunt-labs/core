//! Versioned JSON boundary for compiler adapters.
//!
//! This crate owns wire translation only. The compiler pipeline remains the
//! semantic authority, while WASM, service, and CLI hosts can reuse this same
//! request/response mapping.

use krunt_compiler_pipeline::{CompilationError, CompilationTarget, CompilerPipeline};
use krunt_diagnostics::{
	CompilerStage, Diagnostic, DiagnosticCode, Diagnostics, GraphElementId, Severity,
};
use krunt_graph_model::{GraphDocument, GraphId};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;
use std::fmt;

pub const PROTOCOL_VERSION: u32 = 1;

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BridgeError {
	INVALID_JSON(String),
	INVALID_REQUEST(String),
	UNSUPPORTED_PROTOCOL(u32),
	SERIALIZATION(String),
}

impl fmt::Display for BridgeError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(formatter, "{self:?}")
	}
}

impl Error for BridgeError {}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Deserialize)]
enum WireOperation {
	VALIDATE,
	COMPILE,
}

#[derive(Clone, Debug, Deserialize)]
struct WireRequest {
	protocol_version: u32,
	request_id: String,
	operation: WireOperation,
	target: WireTarget,
	document: Value,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct WireTarget {
	language: String,
	framework: String,
}

#[derive(Serialize)]
struct WireResponse {
	protocol_version: u32,
	request_id: String,
	diagnostics: Vec<WireDiagnostic>,
	artifact: Option<WireArtifact>,
}

#[derive(Serialize)]
struct WireElement {
	kind: &'static str,
	id: u64,
}

#[derive(Serialize)]
struct WireRelatedElement {
	element: WireElement,
	role: Option<String>,
}

#[derive(Serialize)]
struct WireFix {
	action: String,
	arguments: Vec<(String, String)>,
}

#[derive(Serialize)]
struct WireHelp {
	summary: String,
	fix: Option<WireFix>,
}

#[derive(Serialize)]
struct WireDiagnostic {
	code: String,
	severity: Severity,
	stage: CompilerStage,
	message: String,
	primary: WireElement,
	related: Vec<WireRelatedElement>,
	help: Option<WireHelp>,
}

#[derive(Serialize)]
struct WireArtifact {
	target: WireTarget,
	files: Vec<WireFile>,
}

#[derive(Serialize)]
struct WireFile {
	path: String,
	content: String,
}

/// Handle one versioned JSON request and return one versioned JSON response.
///
/// Invalid graph documents with a recognizable request identity are returned
/// as structured diagnostics. Invalid envelopes return `BridgeError` because
/// there is no reliable request identity to correlate with a client call.
pub fn handle_json(input: &str) -> Result<String, BridgeError> {
	let request: WireRequest = serde_json::from_str(input)
		.map_err(|error| BridgeError::INVALID_JSON(error.to_string()))?;
	if request.protocol_version != PROTOCOL_VERSION {
		return Err(BridgeError::UNSUPPORTED_PROTOCOL(request.protocol_version));
	}
	if request.request_id.trim().is_empty() {
		return Err(BridgeError::INVALID_REQUEST(
			"request_id must be a non-empty string".to_string(),
		));
	}

	let graph_id = graph_id_from_value(&request.document);
	let document_json = serde_json::to_string(&request.document)
		.map_err(|error| BridgeError::SERIALIZATION(error.to_string()))?;
	let document = match GraphDocument::from_json(&document_json) {
		Ok(document) => document,
		Err(error) => {
			return response_json(WireResponse {
				protocol_version: PROTOCOL_VERSION,
				request_id: request.request_id,
				diagnostics: vec![bridge_diagnostic(
					"GRAPH.INVALID_DOCUMENT",
					Severity::ERROR,
					CompilerStage::STRUCTURAL_VALIDATION,
					format!("graph document is invalid: {error}"),
					graph_id,
				)],
				artifact: None,
			});
		}
	};

	let Some(target) = target_from_wire(&request.target) else {
		return response_json(WireResponse {
			protocol_version: PROTOCOL_VERSION,
			request_id: request.request_id,
			diagnostics: vec![bridge_diagnostic(
				"TARGET.UNSUPPORTED",
				Severity::ERROR,
				CompilerStage::PARSE,
				format!(
					"unsupported compilation target: {}/{}",
					request.target.language, request.target.framework
				),
				graph_id,
			)],
			artifact: None,
		});
	};

	let pipeline = CompilerPipeline::new();
	let response = match request.operation {
		WireOperation::VALIDATE => WireResponse {
			protocol_version: PROTOCOL_VERSION,
			request_id: request.request_id,
			diagnostics: wire_diagnostics(pipeline.validate(&document)),
			artifact: None,
		},
		WireOperation::COMPILE => {
			compile_response(&pipeline, &document, target, request.request_id, graph_id)
		}
	};
	response_json(response)
}

fn target_from_wire(target: &WireTarget) -> Option<CompilationTarget> {
	if target.language == "TYPESCRIPT" && target.framework == "NESTJS" {
		Some(CompilationTarget::TYPESCRIPT_NESTJS)
	} else {
		None
	}
}

fn compile_response(
	pipeline: &CompilerPipeline,
	document: &GraphDocument,
	target: CompilationTarget,
	request_id: String,
	graph_id: u64,
) -> WireResponse {
	match pipeline.compile(document, target) {
		Ok(artifact) => WireResponse {
			protocol_version: PROTOCOL_VERSION,
			request_id,
			diagnostics: Vec::new(),
			artifact: Some(WireArtifact {
				target: WireTarget {
					language: "TYPESCRIPT".to_string(),
					framework: "NESTJS".to_string(),
				},
				files: artifact
					.project()
					.files()
					.iter()
					.map(|(path, content)| WireFile {
						path: path.clone(),
						content: content.clone(),
					})
					.collect(),
			}),
		},
		Err(CompilationError::DIAGNOSTICS(diagnostics)) => WireResponse {
			protocol_version: PROTOCOL_VERSION,
			request_id,
			diagnostics: wire_diagnostics(diagnostics),
			artifact: None,
		},
		Err(CompilationError::TYPESCRIPT_LOWERING(error)) => WireResponse {
			protocol_version: PROTOCOL_VERSION,
			request_id,
			diagnostics: vec![bridge_diagnostic(
				"LOWERING.TARGET_FAILURE",
				Severity::ERROR,
				CompilerStage::LOWERING,
				format!("TypeScript lowering failed: {error}"),
				graph_id,
			)],
			artifact: None,
		},
		Err(CompilationError::NESTJS_GENERATION(error)) => WireResponse {
			protocol_version: PROTOCOL_VERSION,
			request_id,
			diagnostics: vec![bridge_diagnostic(
				"CODE_GENERATION.TARGET_FAILURE",
				Severity::ERROR,
				CompilerStage::CODE_GENERATION,
				format!("NestJS generation failed: {error}"),
				graph_id,
			)],
			artifact: None,
		},
		Err(CompilationError::UNSUPPORTED_TARGET(_)) => WireResponse {
			protocol_version: PROTOCOL_VERSION,
			request_id,
			diagnostics: vec![bridge_diagnostic(
				"TARGET.UNSUPPORTED",
				Severity::ERROR,
				CompilerStage::PARSE,
				"requested compiler target is not registered".to_string(),
				graph_id,
			)],
			artifact: None,
		},
	}
}

fn response_json(response: WireResponse) -> Result<String, BridgeError> {
	serde_json::to_string(&response).map_err(|error| BridgeError::SERIALIZATION(error.to_string()))
}

fn graph_id_from_value(document: &Value) -> u64 {
	document
		.get("metadata")
		.and_then(|metadata| metadata.get("graph_id"))
		.and_then(Value::as_u64)
		.filter(|id| *id > 0)
		.unwrap_or(1)
}

fn wire_diagnostics(mut diagnostics: Diagnostics) -> Vec<WireDiagnostic> {
	diagnostics.sort_canonical();
	diagnostics.as_slice().iter().map(wire_diagnostic).collect()
}

fn wire_diagnostic(diagnostic: &Diagnostic) -> WireDiagnostic {
	WireDiagnostic {
		code: diagnostic.code().as_str().to_string(),
		severity: diagnostic.severity(),
		stage: diagnostic.stage(),
		message: diagnostic.message().to_string(),
		primary: wire_element(diagnostic.primary()),
		related: diagnostic
			.related()
			.iter()
			.map(|related| WireRelatedElement {
				element: wire_element(related.element()),
				role: related.role().map(str::to_string),
			})
			.collect(),
		help: diagnostic.help().map(|help| WireHelp {
			summary: help.summary().to_string(),
			fix: help.fix().map(|fix| WireFix {
				action: fix.action().to_string(),
				arguments: fix.arguments().to_vec(),
			}),
		}),
	}
}

fn wire_element(element: GraphElementId) -> WireElement {
	match element {
		GraphElementId::GRAPH(id) => WireElement {
			kind: "GRAPH",
			id: id.as_u64(),
		},
		GraphElementId::NODE(id) => WireElement {
			kind: "NODE",
			id: id.as_u64(),
		},
		GraphElementId::PORT(id) => WireElement {
			kind: "PORT",
			id: id.as_u64(),
		},
		GraphElementId::EDGE(id) => WireElement {
			kind: "EDGE",
			id: id.as_u64(),
		},
		GraphElementId::GROUP(id) => WireElement {
			kind: "GROUP",
			id: id.as_u64(),
		},
		GraphElementId::SUBGRAPH(id) => WireElement {
			kind: "SUBGRAPH",
			id: id.as_u64(),
		},
	}
}

fn bridge_diagnostic(
	code: &'static str,
	severity: Severity,
	stage: CompilerStage,
	message: String,
	graph_id: u64,
) -> WireDiagnostic {
	let diagnostic = Diagnostic::new(
		DiagnosticCode::new(code).expect("bridge diagnostic codes are non-empty"),
		severity,
		stage,
		message,
		GraphElementId::GRAPH(GraphId::new(graph_id.max(1)).expect("graph ID is positive")),
	)
	.expect("bridge diagnostic messages are non-empty");
	wire_diagnostic(&diagnostic)
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;

	fn document(with_response: bool) -> Value {
		let mut nodes = vec![json!({
			"id": 1,
			"type_id": "backend.http_route",
			"ports": [
				{"id": 11, "direction": "OUTPUT", "kind": "EXECUTION", "type_reference": null},
				{"id": 12, "direction": "OUTPUT", "kind": "DATA", "type_reference": "HttpRequest"}
			],
			"properties": {"method": "GET", "path": "/users"},
			"annotations": []
		})];
		let mut edges = Vec::new();
		if with_response {
			nodes.push(json!({
				"id": 2,
				"type_id": "backend.success_response",
				"ports": [
					{"id": 21, "direction": "INPUT", "kind": "EXECUTION", "type_reference": null},
					{"id": 22, "direction": "INPUT", "kind": "DATA", "type_reference": "ANY"}
				],
				"properties": {"status_code": 200},
				"annotations": []
			}));
			edges = vec![
				json!({"id": 31, "from": 11, "to": 21, "kind": "EXECUTION"}),
				json!({"id": 32, "from": 12, "to": 22, "kind": "DATA"}),
			];
		}
		json!({
			"schema_version": 1,
			"metadata": {
				"graph_id": 1,
				"semantic_version": "backend-v1",
				"compiler_version": "krunt-test",
				"plugins": [{"id": "krunt.backend", "version": "0.1.0"}]
			},
			"nodes": nodes,
			"edges": edges,
			"groups": [],
			"subgraphs": [],
			"extensions": {}
		})
	}

	fn request(operation: &str, document: Value) -> String {
		serde_json::to_string(&json!({
			"protocol_version": 1,
			"request_id": "bridge-test-0001",
			"operation": operation,
			"target": {"language": "TYPESCRIPT", "framework": "NESTJS"},
			"document": document
		}))
		.unwrap()
	}

	#[test]
	fn returns_a_deterministic_artifact_for_valid_compile_requests() {
		let input = request("COMPILE", document(true));
		let first = handle_json(&input).unwrap();
		let second = handle_json(&input).unwrap();
		assert_eq!(first, second);
		let response: Value = serde_json::from_str(&first).unwrap();
		assert_eq!(response["request_id"], "bridge-test-0001");
		assert!(response["artifact"]["files"]
			.as_array()
			.unwrap()
			.iter()
			.any(|file| { file["path"] == "src/app.controller.ts" }));
	}

	#[test]
	fn returns_structured_diagnostics_and_no_artifact_for_invalid_graphs() {
		let response: Value =
			serde_json::from_str(&handle_json(&request("VALIDATE", document(false))).unwrap())
				.unwrap();
		assert_eq!(response["artifact"], Value::Null);
		assert_eq!(response["diagnostics"][0]["severity"], "ERROR");
		assert_eq!(response["diagnostics"][0]["primary"]["kind"], "NODE");
		assert_eq!(response["diagnostics"][0]["primary"]["id"], 1);
	}

	#[test]
	fn returns_parse_diagnostics_for_invalid_graph_documents() {
		let mut invalid = document(true);
		invalid["schema_version"] = json!(99);
		let response: Value =
			serde_json::from_str(&handle_json(&request("VALIDATE", invalid)).unwrap()).unwrap();
		assert_eq!(response["diagnostics"][0]["code"], "GRAPH.INVALID_DOCUMENT");
		assert_eq!(response["diagnostics"][0]["stage"], "STRUCTURAL_VALIDATION");
		assert_eq!(response["artifact"], Value::Null);
	}
}
