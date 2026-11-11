//! Initial Krunt Backend DSL semantic plugin.
//!
//! This crate owns backend meaning for the first vertical slice. It describes
//! HTTP Route, Identity Transform, Success Response, Error Response, Try/Catch,
//! and Validation Error nodes, validates their properties and typed data
//! contracts, and lowers them into target-neutral operations with namespaced
//! metadata. It does not emit TypeScript or NestJS code.

use krunt_diagnostics::{CompilerStage, Diagnostic, DiagnosticCode, GraphElementId, Severity};
use krunt_graph_model::{
	EdgeKind, GraphDocument, Node, NodeId, NodeTypeId, PluginReference, PortDirection, PortId,
	PortKind,
};
use krunt_ir::{AsyncKind, OperationId, TypeId};
use krunt_plugin_api::{
	LoweredOperation, LoweredOperationKind, NodeContract, PluginApiError, PluginManifest,
	PluginRegistry, PortContract,
};
use serde::Serialize;
use serde_json::Value;
use std::fmt;

pub const BACKEND_DSL_ID: &str = "krunt.backend-dsl";
pub const BACKEND_DSL_VERSION: &str = "0.1.0";
pub const HTTP_ROUTE_NODE_TYPE: &str = "backend.http_route";
pub const TRANSFORM_NODE_TYPE: &str = "backend.transform";
pub const SUCCESS_RESPONSE_NODE_TYPE: &str = "backend.success_response";
pub const ERROR_RESPONSE_NODE_TYPE: &str = "backend.error_response";
pub const TRY_CATCH_NODE_TYPE: &str = "backend.try_catch";
pub const VALIDATION_ERROR_NODE_TYPE: &str = "backend.validation_error";
pub const HTTP_REQUEST_TYPE: &str = "HttpRequest";
pub const RESPONSE_BODY_TYPE: &str = "ANY";
pub const SUCCESS_RESPONSE_DEFAULT_STATUS_CODE: u16 = 200;
pub const ERROR_RESPONSE_DEFAULT_STATUS_CODE: u16 = 500;
pub const VALIDATION_ERROR_DEFAULT_STATUS_CODE: u16 = 400;
pub const VALIDATION_ERROR_DEFAULT_MESSAGE: &str = "Validation failed";

const HTTP_METHODS: [&str; 7] = ["DELETE", "GET", "HEAD", "OPTIONS", "PATCH", "POST", "PUT"];

/// Serializable property kind exposed to editor and AI clients.
#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub enum BackendPropertyKind {
	STRING,
	INTEGER,
	ENUM { values: Vec<String> },
}

/// Serializable property schema for one Backend DSL node.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BackendPropertyDescriptor {
	key: String,
	kind: BackendPropertyKind,
	required: bool,
	default: Option<Value>,
}

impl BackendPropertyDescriptor {
	pub fn new(
		key: impl Into<String>,
		kind: BackendPropertyKind,
		required: bool,
		default: Option<Value>,
	) -> Self {
		Self {
			key: key.into(),
			kind,
			required,
			default,
		}
	}

	pub fn key(&self) -> &str {
		&self.key
	}

	pub fn kind(&self) -> &BackendPropertyKind {
		&self.kind
	}

	pub fn required(&self) -> bool {
		self.required
	}

	pub fn default(&self) -> Option<&Value> {
		self.default.as_ref()
	}
}

/// Serializable descriptor for a semantic port.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BackendPortDescriptor {
	name: String,
	contract: PortContract,
}

impl BackendPortDescriptor {
	pub fn new(name: impl Into<String>, contract: PortContract) -> Self {
		Self {
			name: name.into(),
			contract,
		}
	}

	pub fn name(&self) -> &str {
		&self.name
	}

	pub fn contract(&self) -> &PortContract {
		&self.contract
	}
}

/// Serializable editor/AI descriptor for one Backend DSL node.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BackendNodeDescriptor {
	type_id: NodeTypeId,
	label: String,
	category: String,
	inputs: Vec<BackendPortDescriptor>,
	outputs: Vec<BackendPortDescriptor>,
	properties: Vec<BackendPropertyDescriptor>,
}

impl BackendNodeDescriptor {
	pub fn type_id(&self) -> &NodeTypeId {
		&self.type_id
	}

	pub fn label(&self) -> &str {
		&self.label
	}

	pub fn category(&self) -> &str {
		&self.category
	}

	pub fn inputs(&self) -> &[BackendPortDescriptor] {
		&self.inputs
	}

	pub fn outputs(&self) -> &[BackendPortDescriptor] {
		&self.outputs
	}

	pub fn properties(&self) -> &[BackendPropertyDescriptor] {
		&self.properties
	}
}

/// The initial Backend DSL plugin implementation.
pub struct BackendDslPlugin {
	manifest: PluginManifest,
	descriptors: Vec<BackendNodeDescriptor>,
}

impl BackendDslPlugin {
	pub fn new() -> Self {
		let route_type = NodeTypeId::new(HTTP_ROUTE_NODE_TYPE).expect("static node type is valid");
		let transform_type =
			NodeTypeId::new(TRANSFORM_NODE_TYPE).expect("static node type is valid");
		let response_type =
			NodeTypeId::new(SUCCESS_RESPONSE_NODE_TYPE).expect("static node type is valid");
		let error_response_type =
			NodeTypeId::new(ERROR_RESPONSE_NODE_TYPE).expect("static node type is valid");
		let try_catch_type =
			NodeTypeId::new(TRY_CATCH_NODE_TYPE).expect("static node type is valid");
		let validation_error_type =
			NodeTypeId::new(VALIDATION_ERROR_NODE_TYPE).expect("static node type is valid");
		let request_type = TypeId::new(HTTP_REQUEST_TYPE).expect("static type is valid");
		let response_body_type = TypeId::new(RESPONSE_BODY_TYPE).expect("static type is valid");
		let manifest = PluginManifest::new(
			PluginReference::new(BACKEND_DSL_ID, BACKEND_DSL_VERSION)
				.expect("static plugin identity is valid"),
			[
				route_type.clone(),
				transform_type.clone(),
				response_type.clone(),
				error_response_type.clone(),
				try_catch_type.clone(),
				validation_error_type.clone(),
			],
		)
		.expect("static plugin manifest is valid");
		let descriptors = vec![
			BackendNodeDescriptor {
				type_id: route_type,
				label: "HTTP Route".to_string(),
				category: "Entry".to_string(),
				inputs: Vec::new(),
				outputs: vec![
					BackendPortDescriptor::new(
						"next",
						PortContract::new(PortKind::EXECUTION, None, true),
					),
					BackendPortDescriptor::new(
						"request",
						PortContract::new(PortKind::DATA, Some(request_type), true),
					),
				],
				properties: vec![
					BackendPropertyDescriptor::new(
						"method",
						BackendPropertyKind::ENUM {
							values: HTTP_METHODS
								.iter()
								.map(|value| (*value).to_string())
								.collect(),
						},
						true,
						Some(Value::String("GET".to_string())),
					),
					BackendPropertyDescriptor::new(
						"path",
						BackendPropertyKind::STRING,
						true,
						Some(Value::String("/".to_string())),
					),
				],
			},
			BackendNodeDescriptor {
				type_id: transform_type,
				label: "Identity Transform".to_string(),
				category: "Data".to_string(),
				inputs: vec![
					BackendPortDescriptor::new(
						"in",
						PortContract::new(PortKind::EXECUTION, None, true),
					),
					BackendPortDescriptor::new(
						"value",
						PortContract::new(PortKind::DATA, None, true),
					),
				],
				outputs: vec![
					BackendPortDescriptor::new(
						"next",
						PortContract::new(PortKind::EXECUTION, None, true),
					),
					BackendPortDescriptor::new(
						"result",
						PortContract::new(PortKind::DATA, None, true),
					),
				],
				properties: Vec::new(),
			},
			BackendNodeDescriptor {
				type_id: response_type,
				label: "Success Response".to_string(),
				category: "Response".to_string(),
				inputs: vec![
					BackendPortDescriptor::new(
						"in",
						PortContract::new(PortKind::EXECUTION, None, true),
					),
					BackendPortDescriptor::new(
						"body",
						PortContract::new(PortKind::DATA, Some(response_body_type.clone()), false),
					),
				],
				outputs: Vec::new(),
				properties: vec![BackendPropertyDescriptor::new(
					"status_code",
					BackendPropertyKind::INTEGER,
					false,
					Some(Value::Number(serde_json::Number::from(
						SUCCESS_RESPONSE_DEFAULT_STATUS_CODE,
					))),
				)],
			},
			BackendNodeDescriptor {
				type_id: error_response_type,
				label: "Error Response".to_string(),
				category: "Response".to_string(),
				inputs: vec![
					BackendPortDescriptor::new(
						"in",
						PortContract::new(PortKind::EXECUTION, None, true),
					),
					BackendPortDescriptor::new(
						"body",
						PortContract::new(PortKind::DATA, Some(response_body_type.clone()), false),
					),
				],
				outputs: Vec::new(),
				properties: vec![BackendPropertyDescriptor::new(
					"status_code",
					BackendPropertyKind::INTEGER,
					false,
					Some(Value::Number(serde_json::Number::from(
						ERROR_RESPONSE_DEFAULT_STATUS_CODE,
					))),
				)],
			},
			BackendNodeDescriptor {
				type_id: try_catch_type,
				label: "Try/Catch".to_string(),
				category: "Error Handling".to_string(),
				inputs: vec![BackendPortDescriptor::new(
					"in",
					PortContract::new(PortKind::EXECUTION, None, true),
				)],
				outputs: vec![
					BackendPortDescriptor::new(
						"try",
						PortContract::new(PortKind::EXECUTION, None, true),
					),
					BackendPortDescriptor::new(
						"catch",
						PortContract::new(PortKind::EXECUTION, None, true),
					),
				],
				properties: Vec::new(),
			},
			BackendNodeDescriptor {
				type_id: validation_error_type,
				label: "Validation Error".to_string(),
				category: "Error Handling".to_string(),
				inputs: vec![
					BackendPortDescriptor::new(
						"in",
						PortContract::new(PortKind::EXECUTION, None, true),
					),
					BackendPortDescriptor::new(
						"body",
						PortContract::new(PortKind::DATA, Some(response_body_type), false),
					),
				],
				outputs: Vec::new(),
				properties: vec![BackendPropertyDescriptor::new(
					"message",
					BackendPropertyKind::STRING,
					false,
					Some(Value::String(VALIDATION_ERROR_DEFAULT_MESSAGE.to_string())),
				)],
			},
		];
		Self {
			manifest,
			descriptors,
		}
	}

	pub fn register(registry: &mut PluginRegistry) -> Result<(), PluginApiError> {
		registry.register(Box::new(Self::new()))
	}

	pub fn descriptors(&self) -> &[BackendNodeDescriptor] {
		&self.descriptors
	}
}

impl Default for BackendDslPlugin {
	fn default() -> Self {
		Self::new()
	}
}

impl krunt_plugin_api::SemanticPlugin for BackendDslPlugin {
	fn manifest(&self) -> &PluginManifest {
		&self.manifest
	}

	fn contract(&self, node: &Node) -> NodeContract {
		match node.type_id().as_str() {
			HTTP_ROUTE_NODE_TYPE => {
				route_contract(TypeId::new(HTTP_REQUEST_TYPE).expect("static type is valid"))
			}
			TRANSFORM_NODE_TYPE => transform_contract(),
			SUCCESS_RESPONSE_NODE_TYPE | ERROR_RESPONSE_NODE_TYPE => response_contract(),
			TRY_CATCH_NODE_TYPE => try_catch_contract(),
			VALIDATION_ERROR_NODE_TYPE => response_contract(),
			_ => NodeContract::new([], [], [], [], [], AsyncKind::SYNC, []),
		}
	}

	fn analyze_graph(&self, document: &GraphDocument) -> Vec<Diagnostic> {
		let mut diagnostics = validate_try_catch_branches(document);
		diagnostics.extend(validate_route_response_paths(document));
		diagnostics
	}

	fn analyze_node(&self, node: &Node) -> Vec<Diagnostic> {
		match node.type_id().as_str() {
			HTTP_ROUTE_NODE_TYPE => route_properties(node)
				.err()
				.map(|diagnostic| *diagnostic)
				.into_iter()
				.collect(),
			TRANSFORM_NODE_TYPE => transform_data_types(node)
				.err()
				.map(|diagnostic| *diagnostic)
				.into_iter()
				.collect(),
			SUCCESS_RESPONSE_NODE_TYPE => success_response_status_code(node)
				.err()
				.map(|diagnostic| *diagnostic)
				.into_iter()
				.collect(),
			ERROR_RESPONSE_NODE_TYPE => error_response_status_code(node)
				.err()
				.map(|diagnostic| *diagnostic)
				.into_iter()
				.collect(),
			TRY_CATCH_NODE_TYPE => Vec::new(),
			VALIDATION_ERROR_NODE_TYPE => validation_error_message(node)
				.err()
				.map(|diagnostic| *diagnostic)
				.into_iter()
				.collect(),
			_ => Vec::new(),
		}
	}

	fn lower_node(&self, node: &Node) -> Result<LoweredOperation, Vec<Diagnostic>> {
		match node.type_id().as_str() {
			HTTP_ROUTE_NODE_TYPE => {
				let properties = route_properties(node).map_err(|error| vec![*error])?;
				Ok(LoweredOperation::new(
					OperationId::new(HTTP_ROUTE_NODE_TYPE).expect("static operation is valid"),
					[],
					[TypeId::new(HTTP_REQUEST_TYPE).expect("static type is valid")],
					AsyncKind::SYNC,
					[],
				)
				.with_attribute("backend.http.method", properties.method)
				.with_attribute("backend.http.path", properties.path))
			}
			TRANSFORM_NODE_TYPE => {
				let (input_type, output_type) =
					transform_data_types(node).map_err(|error| vec![*error])?;
				Ok(LoweredOperation::new(
					OperationId::new(TRANSFORM_NODE_TYPE).expect("static operation is valid"),
					[input_type],
					[output_type],
					AsyncKind::SYNC,
					[],
				)
				.with_attribute("backend.transform.kind", "IDENTITY"))
			}
			SUCCESS_RESPONSE_NODE_TYPE => {
				let status_code =
					success_response_status_code(node).map_err(|error| vec![*error])?;
				Ok(LoweredOperation::new(
					OperationId::new(SUCCESS_RESPONSE_NODE_TYPE)
						.expect("static operation is valid"),
					[TypeId::new(RESPONSE_BODY_TYPE).expect("static type is valid")],
					[],
					AsyncKind::SYNC,
					[],
				)
				.with_return(true)
				.with_attribute("response.kind", "SUCCESS")
				.with_attribute("response.status_code", status_code.to_string()))
			}
			ERROR_RESPONSE_NODE_TYPE => {
				let status_code = error_response_status_code(node).map_err(|error| vec![*error])?;
				Ok(LoweredOperation::new(
					OperationId::new(ERROR_RESPONSE_NODE_TYPE).expect("static operation is valid"),
					[TypeId::new(RESPONSE_BODY_TYPE).expect("static type is valid")],
					[],
					AsyncKind::SYNC,
					[],
				)
				.with_return(true)
				.with_attribute("response.kind", "ERROR")
				.with_attribute("response.status_code", status_code.to_string()))
			}
			TRY_CATCH_NODE_TYPE => Ok(LoweredOperation::new(
				OperationId::new(TRY_CATCH_NODE_TYPE).expect("static operation is valid"),
				[],
				[],
				AsyncKind::SYNC,
				[],
			)
			.with_kind(LoweredOperationKind::BRANCH)
			.with_attribute("error.handling.kind", "TRY_CATCH")
			.with_attribute("error.handling.try_output", "0")
			.with_attribute("error.handling.catch_output", "1")),
			VALIDATION_ERROR_NODE_TYPE => {
				let message = validation_error_message(node).map_err(|error| vec![*error])?;
				Ok(LoweredOperation::new(
					OperationId::new(VALIDATION_ERROR_NODE_TYPE)
						.expect("static operation is valid"),
					[TypeId::new(RESPONSE_BODY_TYPE).expect("static type is valid")],
					[],
					AsyncKind::SYNC,
					[],
				)
				.with_return(true)
				.with_attribute("response.kind", "ERROR")
				.with_attribute(
					"response.status_code",
					VALIDATION_ERROR_DEFAULT_STATUS_CODE.to_string(),
				)
				.with_attribute("error.kind", "VALIDATION")
				.with_attribute("error.message", message))
			}
			_ => unreachable!("registry only resolves Backend DSL node types"),
		}
	}
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RouteProperties {
	method: String,
	path: String,
}

fn route_contract(request_type: TypeId) -> NodeContract {
	NodeContract::new(
		[],
		[
			PortContract::new(PortKind::EXECUTION, None, true),
			PortContract::new(PortKind::DATA, Some(request_type), true),
		],
		[],
		[],
		[],
		AsyncKind::SYNC,
		[],
	)
	.with_entry(true)
}

fn transform_contract() -> NodeContract {
	NodeContract::new(
		[
			PortContract::new(PortKind::EXECUTION, None, true),
			PortContract::new(PortKind::DATA, None, true),
		],
		[
			PortContract::new(PortKind::EXECUTION, None, true),
			PortContract::new(PortKind::DATA, None, true),
		],
		[],
		[],
		[],
		AsyncKind::SYNC,
		[],
	)
}

fn response_contract() -> NodeContract {
	NodeContract::new(
		[
			PortContract::new(PortKind::EXECUTION, None, true),
			PortContract::new(
				PortKind::DATA,
				Some(TypeId::new(RESPONSE_BODY_TYPE).expect("static type is valid")),
				false,
			),
		],
		[],
		[],
		[],
		[],
		AsyncKind::SYNC,
		[],
	)
}

fn try_catch_contract() -> NodeContract {
	NodeContract::new(
		[PortContract::new(PortKind::EXECUTION, None, true)],
		[
			PortContract::new(PortKind::EXECUTION, None, true),
			PortContract::new(PortKind::EXECUTION, None, true),
		],
		[],
		[],
		[],
		AsyncKind::SYNC,
		[],
	)
}

fn validation_error_message(node: &Node) -> Result<String, Box<Diagnostic>> {
	let Some(value) = node.properties().get("message") else {
		return Ok(VALIDATION_ERROR_DEFAULT_MESSAGE.to_string());
	};
	let Some(message) = value.as_str() else {
		return Err(node_diagnostic(
			node,
			"BACKEND_DSL.VALIDATION_ERROR_MESSAGE_TYPE",
			"Validation Error property 'message' must be a string.",
		));
	};
	if message.trim().is_empty() {
		return Err(node_diagnostic(
			node,
			"BACKEND_DSL.VALIDATION_ERROR_MESSAGE_EMPTY",
			"Validation Error property 'message' must not be empty.",
		));
	}
	Ok(message.to_string())
}

fn transform_data_types(node: &Node) -> Result<(TypeId, TypeId), Box<Diagnostic>> {
	let input = node
		.ports()
		.iter()
		.find(|port| port.direction() == PortDirection::INPUT && port.kind() == PortKind::DATA)
		.ok_or_else(|| {
			node_diagnostic(
				node,
				"BACKEND_DSL.TRANSFORM_INPUT_TYPE_MISSING",
				"Identity Transform requires a typed data input.",
			)
		})?;
	let output = node
		.ports()
		.iter()
		.find(|port| port.direction() == PortDirection::OUTPUT && port.kind() == PortKind::DATA)
		.ok_or_else(|| {
			node_diagnostic(
				node,
				"BACKEND_DSL.TRANSFORM_OUTPUT_TYPE_MISSING",
				"Identity Transform requires a typed data output.",
			)
		})?;
	let input_type = input.type_reference().ok_or_else(|| {
		node_diagnostic(
			node,
			"BACKEND_DSL.TRANSFORM_INPUT_TYPE_MISSING",
			"Identity Transform requires a typed data input.",
		)
	})?;
	let output_type = output.type_reference().ok_or_else(|| {
		node_diagnostic(
			node,
			"BACKEND_DSL.TRANSFORM_OUTPUT_TYPE_MISSING",
			"Identity Transform requires a typed data output.",
		)
	})?;
	let input_type = TypeId::new(input_type.as_str()).map_err(|_| {
		node_diagnostic(
			node,
			"BACKEND_DSL.TRANSFORM_INPUT_TYPE_INVALID",
			"Identity Transform input type is invalid.",
		)
	})?;
	let output_type = TypeId::new(output_type.as_str()).map_err(|_| {
		node_diagnostic(
			node,
			"BACKEND_DSL.TRANSFORM_OUTPUT_TYPE_INVALID",
			"Identity Transform output type is invalid.",
		)
	})?;
	if input_type != output_type {
		return Err(node_diagnostic(
			node,
			"BACKEND_DSL.TRANSFORM_TYPE_MISMATCH",
			"Identity Transform input and output types must match.",
		));
	}
	Ok((input_type, output_type))
}

fn success_response_status_code(node: &Node) -> Result<u16, Box<Diagnostic>> {
	response_status_code(
		node,
		SUCCESS_RESPONSE_DEFAULT_STATUS_CODE,
		200,
		299,
		"Success",
		"BACKEND_DSL.SUCCESS_RESPONSE_STATUS_CODE_TYPE",
		"BACKEND_DSL.SUCCESS_RESPONSE_STATUS_CODE_INVALID",
	)
}

fn error_response_status_code(node: &Node) -> Result<u16, Box<Diagnostic>> {
	response_status_code(
		node,
		ERROR_RESPONSE_DEFAULT_STATUS_CODE,
		400,
		599,
		"Error",
		"BACKEND_DSL.ERROR_RESPONSE_STATUS_CODE_TYPE",
		"BACKEND_DSL.ERROR_RESPONSE_STATUS_CODE_INVALID",
	)
}

fn response_status_code(
	node: &Node,
	default: u16,
	minimum: u16,
	maximum: u16,
	response_kind: &str,
	type_code: &str,
	invalid_code: &str,
) -> Result<u16, Box<Diagnostic>> {
	let Some(value) = node.properties().get("status_code") else {
		return Ok(default);
	};
	let Some(status_code) = value.as_u64().and_then(|value| u16::try_from(value).ok()) else {
		return Err(node_diagnostic(
			node,
			type_code,
			format!("{response_kind} Response property 'status_code' must be an integer."),
		));
	};
	if !(minimum..=maximum).contains(&status_code) {
		return Err(node_diagnostic(
			node,
			invalid_code,
			format!(
				"{response_kind} Response status_code must be between {minimum} and {maximum}."
			),
		));
	}
	Ok(status_code)
}

fn validate_route_response_paths(document: &GraphDocument) -> Vec<Diagnostic> {
	let mut port_owners = std::collections::BTreeMap::<PortId, NodeId>::new();
	for node in document.nodes() {
		for port in node.ports() {
			port_owners.insert(port.id(), node.id());
		}
	}

	let mut execution_adjacency = std::collections::BTreeMap::<NodeId, Vec<NodeId>>::new();
	for edge in document
		.edges()
		.iter()
		.filter(|edge| edge.kind() == EdgeKind::EXECUTION)
	{
		let (Some(from), Some(to)) = (port_owners.get(&edge.from()), port_owners.get(&edge.to()))
		else {
			continue;
		};
		execution_adjacency.entry(*from).or_default().push(*to);
	}
	for successors in execution_adjacency.values_mut() {
		successors.sort();
		successors.dedup();
	}

	let terminal_response_nodes: std::collections::BTreeSet<_> = document
		.nodes()
		.iter()
		.filter(|node| {
			matches!(
				node.type_id().as_str(),
				SUCCESS_RESPONSE_NODE_TYPE | ERROR_RESPONSE_NODE_TYPE | VALIDATION_ERROR_NODE_TYPE
			)
		})
		.map(Node::id)
		.collect();

	document
		.nodes()
		.iter()
		.filter(|node| node.type_id().as_str() == HTTP_ROUTE_NODE_TYPE)
		.filter_map(|route| {
			let mut visited = std::collections::BTreeSet::new();
			let mut worklist = vec![route.id()];
			let mut reaches_response = false;
			let mut has_unterminated_path = false;

			while let Some(node_id) = worklist.pop() {
				if !visited.insert(node_id) {
					continue;
				}
				if terminal_response_nodes.contains(&node_id) {
					reaches_response = true;
					continue;
				}
				let successors = execution_adjacency
					.get(&node_id)
					.map(Vec::as_slice)
					.unwrap_or(&[]);
				if successors.is_empty() {
					has_unterminated_path = true;
				}
				worklist.extend(successors.iter().copied());
			}

			if reaches_response && !has_unterminated_path {
				return None;
			}
			Some(node_diagnostic(
				route,
				"BACKEND_DSL.HTTP_ROUTE_MISSING_RESPONSE",
				"Every HTTP Route execution path must reach a Success Response or Error Response.",
			))
		})
		.map(|diagnostic| *diagnostic)
		.collect()
}

fn validate_try_catch_branches(document: &GraphDocument) -> Vec<Diagnostic> {
	document
		.nodes()
		.iter()
		.filter(|node| node.type_id().as_str() == TRY_CATCH_NODE_TYPE)
		.flat_map(|node| {
			node.ports()
				.iter()
				.filter(|port| {
					port.direction() == PortDirection::OUTPUT && port.kind() == PortKind::EXECUTION
				})
				.filter(|port| {
					!document
						.edges()
						.iter()
						.any(|edge| edge.kind() == EdgeKind::EXECUTION && edge.from() == port.id())
				})
				.map(|port| {
					port_diagnostic(
						port,
						"BACKEND_DSL.TRY_CATCH_BRANCH_UNCONNECTED",
						"Try/Catch requires both the try and catch execution branches to be connected.",
					)
				})
		})
		.map(|diagnostic| *diagnostic)
		.collect()
}

fn route_properties(node: &Node) -> Result<RouteProperties, Box<Diagnostic>> {
	let method = match node.properties().get("method") {
		Some(Value::String(value)) => value.to_ascii_uppercase(),
		Some(_) => {
			return Err(node_diagnostic(
				node,
				"BACKEND_DSL.HTTP_ROUTE_METHOD_TYPE",
				"HTTP Route property 'method' must be a string.",
			))
		}
		None => {
			return Err(node_diagnostic(
				node,
				"BACKEND_DSL.HTTP_ROUTE_METHOD_MISSING",
				"HTTP Route requires a 'method' property.",
			))
		}
	};
	if !HTTP_METHODS.contains(&method.as_str()) {
		return Err(node_diagnostic(
			node,
			"BACKEND_DSL.HTTP_ROUTE_METHOD_INVALID",
			format!("HTTP Route method '{method}' is not supported."),
		));
	}

	let path = match node.properties().get("path") {
		Some(Value::String(value)) => value.clone(),
		Some(_) => {
			return Err(node_diagnostic(
				node,
				"BACKEND_DSL.HTTP_ROUTE_PATH_TYPE",
				"HTTP Route property 'path' must be a string.",
			))
		}
		None => {
			return Err(node_diagnostic(
				node,
				"BACKEND_DSL.HTTP_ROUTE_PATH_MISSING",
				"HTTP Route requires a 'path' property.",
			))
		}
	};
	if !path.starts_with('/') || path.chars().any(char::is_whitespace) {
		return Err(node_diagnostic(
			node,
			"BACKEND_DSL.HTTP_ROUTE_PATH_INVALID",
			"HTTP Route path must start with '/' and contain no whitespace.",
		));
	}

	Ok(RouteProperties { method, path })
}

fn node_diagnostic(node: &Node, code: &str, message: impl Into<String>) -> Box<Diagnostic> {
	Box::new(
		Diagnostic::new(
			DiagnosticCode::new(code).expect("static diagnostic code is valid"),
			Severity::ERROR,
			CompilerStage::SEMANTIC_ANALYSIS,
			message,
			GraphElementId::NODE(node.id()),
		)
		.expect("static Backend DSL diagnostic is valid"),
	)
}

fn port_diagnostic(
	port: &krunt_graph_model::Port,
	code: &str,
	message: impl Into<String>,
) -> Box<Diagnostic> {
	Box::new(
		Diagnostic::new(
			DiagnosticCode::new(code).expect("static diagnostic code is valid"),
			Severity::ERROR,
			CompilerStage::SEMANTIC_ANALYSIS,
			message,
			GraphElementId::PORT(port.id()),
		)
		.expect("static Backend DSL diagnostic is valid"),
	)
}

impl fmt::Display for BackendPropertyKind {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(formatter, "{self:?}")
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use krunt_plugin_api::SemanticPlugin;

	fn route(properties: &[(&str, Value)]) -> Node {
		let mut node = Node::new(
			krunt_graph_model::NodeId::new(1).unwrap(),
			NodeTypeId::new(HTTP_ROUTE_NODE_TYPE).unwrap(),
		);
		for (key, value) in properties {
			node = node.with_property(*key, value.clone()).unwrap();
		}
		node
	}

	#[test]
	fn exposes_route_and_response_descriptors() {
		let plugin = BackendDslPlugin::new();
		assert_eq!(plugin.descriptors().len(), 6);
		assert_eq!(plugin.descriptors()[0].label(), "HTTP Route");
		assert_eq!(plugin.descriptors()[0].properties().len(), 2);
		assert_eq!(plugin.descriptors()[1].label(), "Identity Transform");
		assert_eq!(plugin.descriptors()[1].inputs().len(), 2);
		assert_eq!(plugin.descriptors()[1].outputs().len(), 2);
		let response = plugin
			.descriptors()
			.iter()
			.find(|descriptor| descriptor.type_id().as_str() == SUCCESS_RESPONSE_NODE_TYPE)
			.expect("response descriptor should be registered");
		assert_eq!(response.label(), "Success Response");
		assert_eq!(response.inputs().len(), 2);
		assert!(!response.inputs()[1].contract().required());
		assert_eq!(response.properties().len(), 1);
		assert_eq!(response.properties()[0].key(), "status_code");
		assert_eq!(
			response.properties()[0].kind(),
			&BackendPropertyKind::INTEGER
		);
		assert!(!response.properties()[0].required());
		assert_eq!(
			response.properties()[0].default(),
			Some(&Value::Number(serde_json::Number::from(
				SUCCESS_RESPONSE_DEFAULT_STATUS_CODE,
			)))
		);
		assert_eq!(
			response.inputs()[1].contract().type_id().unwrap().as_str(),
			RESPONSE_BODY_TYPE
		);
		let error_response = plugin
			.descriptors()
			.iter()
			.find(|descriptor| descriptor.type_id().as_str() == ERROR_RESPONSE_NODE_TYPE)
			.expect("error response descriptor should be registered");
		assert_eq!(error_response.label(), "Error Response");
		assert_eq!(error_response.inputs().len(), 2);
		assert!(!error_response.inputs()[1].contract().required());
		assert_eq!(error_response.properties().len(), 1);
		assert_eq!(
			error_response.properties()[0].default(),
			Some(&Value::Number(serde_json::Number::from(
				ERROR_RESPONSE_DEFAULT_STATUS_CODE,
			)))
		);
		let try_catch = plugin
			.descriptors()
			.iter()
			.find(|descriptor| descriptor.type_id().as_str() == TRY_CATCH_NODE_TYPE)
			.expect("try/catch descriptor should be registered");
		assert_eq!(try_catch.label(), "Try/Catch");
		assert_eq!(try_catch.category(), "Error Handling");
		assert_eq!(try_catch.inputs().len(), 1);
		assert_eq!(try_catch.outputs().len(), 2);
		assert!(try_catch.properties().is_empty());
		let validation_error = plugin
			.descriptors()
			.iter()
			.find(|descriptor| descriptor.type_id().as_str() == VALIDATION_ERROR_NODE_TYPE)
			.expect("validation error descriptor should be registered");
		assert_eq!(validation_error.label(), "Validation Error");
		assert_eq!(validation_error.category(), "Error Handling");
		assert_eq!(validation_error.inputs().len(), 2);
		assert!(!validation_error.inputs()[1].contract().required());
		assert_eq!(validation_error.properties().len(), 1);
		assert_eq!(validation_error.properties()[0].key(), "message");
		assert_eq!(
			validation_error.properties()[0].default(),
			Some(&Value::String(VALIDATION_ERROR_DEFAULT_MESSAGE.to_string()))
		);
	}

	#[test]
	fn lowers_an_identity_transform_with_preserved_data_types() {
		let mut node = Node::new(
			NodeId::new(2).unwrap(),
			NodeTypeId::new(TRANSFORM_NODE_TYPE).unwrap(),
		);
		node.add_port(krunt_graph_model::Port::new(
			krunt_graph_model::PortId::new(21).unwrap(),
			PortDirection::INPUT,
			PortKind::EXECUTION,
		))
		.unwrap();
		node.add_port(
			krunt_graph_model::Port::new(
				krunt_graph_model::PortId::new(22).unwrap(),
				PortDirection::INPUT,
				PortKind::DATA,
			)
			.with_type_reference(krunt_graph_model::TypeReference::new(HTTP_REQUEST_TYPE).unwrap()),
		)
		.unwrap();
		node.add_port(krunt_graph_model::Port::new(
			krunt_graph_model::PortId::new(23).unwrap(),
			PortDirection::OUTPUT,
			PortKind::EXECUTION,
		))
		.unwrap();
		node.add_port(
			krunt_graph_model::Port::new(
				krunt_graph_model::PortId::new(24).unwrap(),
				PortDirection::OUTPUT,
				PortKind::DATA,
			)
			.with_type_reference(krunt_graph_model::TypeReference::new(HTTP_REQUEST_TYPE).unwrap()),
		)
		.unwrap();

		let lowered = BackendDslPlugin::new().lower_node(&node).unwrap();

		assert_eq!(
			lowered.input_types(),
			&[TypeId::new(HTTP_REQUEST_TYPE).unwrap()]
		);
		assert_eq!(
			lowered.output_types(),
			&[TypeId::new(HTTP_REQUEST_TYPE).unwrap()]
		);
		assert_eq!(lowered.attributes()["backend.transform.kind"], "IDENTITY");
	}

	#[test]
	fn rejects_identity_transforms_with_mismatched_data_types() {
		let mut node = Node::new(
			NodeId::new(2).unwrap(),
			NodeTypeId::new(TRANSFORM_NODE_TYPE).unwrap(),
		);
		node.add_port(
			krunt_graph_model::Port::new(
				krunt_graph_model::PortId::new(21).unwrap(),
				PortDirection::INPUT,
				PortKind::DATA,
			)
			.with_type_reference(krunt_graph_model::TypeReference::new("Request").unwrap()),
		)
		.unwrap();
		node.add_port(
			krunt_graph_model::Port::new(
				krunt_graph_model::PortId::new(22).unwrap(),
				PortDirection::OUTPUT,
				PortKind::DATA,
			)
			.with_type_reference(krunt_graph_model::TypeReference::new("Response").unwrap()),
		)
		.unwrap();

		let diagnostics = BackendDslPlugin::new().analyze_node(&node);

		assert_eq!(diagnostics.len(), 1);
		assert_eq!(
			diagnostics[0].code().as_str(),
			"BACKEND_DSL.TRANSFORM_TYPE_MISMATCH"
		);
	}

	#[test]
	fn lowers_success_response_with_an_optional_body_input() {
		let plugin = BackendDslPlugin::new();
		let node = Node::new(
			NodeId::new(2).unwrap(),
			NodeTypeId::new(SUCCESS_RESPONSE_NODE_TYPE).unwrap(),
		);
		let lowered = plugin.lower_node(&node).unwrap();

		assert_eq!(
			lowered.input_types(),
			&[TypeId::new(RESPONSE_BODY_TYPE).unwrap()]
		);
		assert!(lowered.returns());
		assert_eq!(lowered.attributes()["response.kind"], "SUCCESS");
		assert_eq!(lowered.attributes()["response.status_code"], "200");
	}

	#[test]
	fn lowers_success_response_with_a_custom_status_code() {
		let node = Node::new(
			NodeId::new(2).unwrap(),
			NodeTypeId::new(SUCCESS_RESPONSE_NODE_TYPE).unwrap(),
		)
		.with_property("status_code", Value::Number(serde_json::Number::from(201)))
		.unwrap();
		let lowered = BackendDslPlugin::new().lower_node(&node).unwrap();

		assert_eq!(lowered.attributes()["response.status_code"], "201");
	}

	#[test]
	fn lowers_error_response_with_a_default_status_code() {
		let node = Node::new(
			NodeId::new(2).unwrap(),
			NodeTypeId::new(ERROR_RESPONSE_NODE_TYPE).unwrap(),
		);
		let lowered = BackendDslPlugin::new().lower_node(&node).unwrap();

		assert!(lowered.returns());
		assert_eq!(lowered.attributes()["response.kind"], "ERROR");
		assert_eq!(lowered.attributes()["response.status_code"], "500");
	}

	#[test]
	fn rejects_invalid_error_response_status_codes() {
		let node = Node::new(
			NodeId::new(2).unwrap(),
			NodeTypeId::new(ERROR_RESPONSE_NODE_TYPE).unwrap(),
		)
		.with_property("status_code", Value::Number(serde_json::Number::from(201)))
		.unwrap();
		let diagnostics = BackendDslPlugin::new().analyze_node(&node);

		assert_eq!(diagnostics.len(), 1);
		assert_eq!(
			diagnostics[0].code().as_str(),
			"BACKEND_DSL.ERROR_RESPONSE_STATUS_CODE_INVALID"
		);
	}

	#[test]
	fn lowers_try_catch_with_target_neutral_error_handling_metadata() {
		let node = Node::new(
			NodeId::new(2).unwrap(),
			NodeTypeId::new(TRY_CATCH_NODE_TYPE).unwrap(),
		);
		let lowered = BackendDslPlugin::new().lower_node(&node).unwrap();

		assert_eq!(lowered.operation().as_str(), TRY_CATCH_NODE_TYPE);
		assert_eq!(lowered.kind(), LoweredOperationKind::BRANCH);
		assert_eq!(lowered.attributes()["error.handling.kind"], "TRY_CATCH");
		assert!(!lowered.returns());
	}

	#[test]
	fn lowers_validation_error_with_default_response_metadata() {
		let node = Node::new(
			NodeId::new(2).unwrap(),
			NodeTypeId::new(VALIDATION_ERROR_NODE_TYPE).unwrap(),
		);
		let lowered = BackendDslPlugin::new().lower_node(&node).unwrap();

		assert!(lowered.returns());
		assert_eq!(lowered.attributes()["response.kind"], "ERROR");
		assert_eq!(
			lowered.attributes()["response.status_code"],
			VALIDATION_ERROR_DEFAULT_STATUS_CODE.to_string()
		);
		assert_eq!(
			lowered.attributes()["error.message"],
			VALIDATION_ERROR_DEFAULT_MESSAGE
		);
	}

	#[test]
	fn lowers_validation_error_with_a_custom_message() {
		let node = Node::new(
			NodeId::new(2).unwrap(),
			NodeTypeId::new(VALIDATION_ERROR_NODE_TYPE).unwrap(),
		)
		.with_property("message", Value::String("email is invalid".to_string()))
		.unwrap();
		let lowered = BackendDslPlugin::new().lower_node(&node).unwrap();

		assert_eq!(lowered.attributes()["error.message"], "email is invalid");
	}

	#[test]
	fn rejects_invalid_validation_error_messages() {
		let plugin = BackendDslPlugin::new();
		let wrong_type = Node::new(
			NodeId::new(2).unwrap(),
			NodeTypeId::new(VALIDATION_ERROR_NODE_TYPE).unwrap(),
		)
		.with_property("message", Value::Bool(true))
		.unwrap();
		let empty = Node::new(
			NodeId::new(3).unwrap(),
			NodeTypeId::new(VALIDATION_ERROR_NODE_TYPE).unwrap(),
		)
		.with_property("message", Value::String("  ".to_string()))
		.unwrap();

		assert_eq!(
			plugin.analyze_node(&wrong_type)[0].code().as_str(),
			"BACKEND_DSL.VALIDATION_ERROR_MESSAGE_TYPE"
		);
		assert_eq!(
			plugin.analyze_node(&empty)[0].code().as_str(),
			"BACKEND_DSL.VALIDATION_ERROR_MESSAGE_EMPTY"
		);
	}

	#[test]
	fn rejects_invalid_success_response_status_codes() {
		let node = Node::new(
			NodeId::new(2).unwrap(),
			NodeTypeId::new(SUCCESS_RESPONSE_NODE_TYPE).unwrap(),
		)
		.with_property("status_code", Value::Number(serde_json::Number::from(404)))
		.unwrap();
		let diagnostics = BackendDslPlugin::new().analyze_node(&node);

		assert_eq!(diagnostics.len(), 1);
		assert_eq!(
			diagnostics[0].code().as_str(),
			"BACKEND_DSL.SUCCESS_RESPONSE_STATUS_CODE_INVALID"
		);
	}

	#[test]
	fn lowers_a_valid_route_with_target_neutral_http_metadata() {
		let plugin = BackendDslPlugin::new();
		let node = route(&[
			("method", Value::String("get".to_string())),
			("path", Value::String("/users".to_string())),
		]);
		let lowered = plugin.lower_node(&node).unwrap();

		assert_eq!(lowered.operation().as_str(), HTTP_ROUTE_NODE_TYPE);
		assert_eq!(lowered.attributes()["backend.http.method"], "GET");
		assert_eq!(lowered.attributes()["backend.http.path"], "/users");
	}

	#[test]
	fn invalid_route_properties_return_stable_diagnostics() {
		let plugin = BackendDslPlugin::new();
		let diagnostics =
			plugin.analyze_node(&route(&[("path", Value::String("users".to_string()))]));

		assert_eq!(diagnostics.len(), 1);
		assert_eq!(
			diagnostics[0].code().as_str(),
			"BACKEND_DSL.HTTP_ROUTE_METHOD_MISSING"
		);
	}

	#[test]
	fn route_without_a_response_returns_a_graph_diagnostic() {
		let metadata = krunt_graph_model::GraphMetadata::new(
			krunt_graph_model::GraphId::new(1).unwrap(),
			"backend-v1",
			"krunt-test",
		)
		.unwrap();
		let mut builder = krunt_graph_model::GraphBuilder::new(metadata);
		builder
			.add_node(route(&[
				("method", Value::String("GET".to_string())),
				("path", Value::String("/users".to_string())),
			]))
			.unwrap();
		let document = builder.build().unwrap();
		let diagnostics = BackendDslPlugin::new().analyze_graph(&document);

		assert_eq!(diagnostics.len(), 1);
		assert_eq!(
			diagnostics[0].code().as_str(),
			"BACKEND_DSL.HTTP_ROUTE_MISSING_RESPONSE"
		);
	}

	#[test]
	fn rejects_a_try_catch_with_an_unconnected_branch() {
		let metadata = krunt_graph_model::GraphMetadata::new(
			krunt_graph_model::GraphId::new(2).unwrap(),
			"backend-v1",
			"krunt-test",
		)
		.unwrap();
		let mut builder = krunt_graph_model::GraphBuilder::new(metadata);

		let mut route = route(&[
			("method", Value::String("GET".to_string())),
			("path", Value::String("/users".to_string())),
		]);
		route
			.add_port(krunt_graph_model::Port::new(
				krunt_graph_model::PortId::new(11).unwrap(),
				PortDirection::OUTPUT,
				PortKind::EXECUTION,
			))
			.unwrap();
		let mut try_catch = Node::new(
			NodeId::new(2).unwrap(),
			NodeTypeId::new(TRY_CATCH_NODE_TYPE).unwrap(),
		);
		try_catch
			.add_port(krunt_graph_model::Port::new(
				krunt_graph_model::PortId::new(21).unwrap(),
				PortDirection::INPUT,
				PortKind::EXECUTION,
			))
			.unwrap();
		try_catch
			.add_port(krunt_graph_model::Port::new(
				krunt_graph_model::PortId::new(22).unwrap(),
				PortDirection::OUTPUT,
				PortKind::EXECUTION,
			))
			.unwrap();
		try_catch
			.add_port(krunt_graph_model::Port::new(
				krunt_graph_model::PortId::new(23).unwrap(),
				PortDirection::OUTPUT,
				PortKind::EXECUTION,
			))
			.unwrap();
		let mut response = Node::new(
			NodeId::new(3).unwrap(),
			NodeTypeId::new(SUCCESS_RESPONSE_NODE_TYPE).unwrap(),
		);
		response
			.add_port(krunt_graph_model::Port::new(
				krunt_graph_model::PortId::new(31).unwrap(),
				PortDirection::INPUT,
				PortKind::EXECUTION,
			))
			.unwrap();

		builder.add_node(route).unwrap();
		builder.add_node(try_catch).unwrap();
		builder.add_node(response).unwrap();
		builder
			.add_edge(krunt_graph_model::Edge::new(
				krunt_graph_model::EdgeId::new(41).unwrap(),
				krunt_graph_model::PortId::new(11).unwrap(),
				krunt_graph_model::PortId::new(21).unwrap(),
				EdgeKind::EXECUTION,
			))
			.unwrap();
		builder
			.add_edge(krunt_graph_model::Edge::new(
				krunt_graph_model::EdgeId::new(42).unwrap(),
				krunt_graph_model::PortId::new(22).unwrap(),
				krunt_graph_model::PortId::new(31).unwrap(),
				EdgeKind::EXECUTION,
			))
			.unwrap();
		let diagnostics = BackendDslPlugin::new().analyze_graph(&builder.build().unwrap());

		assert_eq!(diagnostics.len(), 1);
		assert_eq!(
			diagnostics[0].code().as_str(),
			"BACKEND_DSL.TRY_CATCH_BRANCH_UNCONNECTED"
		);
		assert_eq!(
			diagnostics[0].primary(),
			GraphElementId::PORT(krunt_graph_model::PortId::new(23).unwrap())
		);
	}
}
