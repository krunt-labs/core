//! Initial Krunt Backend DSL semantic plugin.
//!
//! This crate owns backend meaning for the first vertical slice. It describes
//! HTTP Route, Branch, Merge, Loop, Identity Transform, Success Response, Error
//! Response, Try/Catch, and Validation Error nodes, validates their properties
//! and typed data contracts, and lowers them into target-neutral operations with
//! namespaced metadata. It does not emit TypeScript or NestJS code.

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
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

pub const BACKEND_DSL_ID: &str = "krunt.backend-dsl";
pub const BACKEND_DSL_VERSION: &str = "0.1.0";
pub const HTTP_ROUTE_NODE_TYPE: &str = "backend.http_route";
pub const CRON_NODE_TYPE: &str = "backend.cron";
pub const BRANCH_NODE_TYPE: &str = "backend.branch";
pub const MERGE_NODE_TYPE: &str = "backend.merge";
pub const LOOP_NODE_TYPE: &str = "backend.loop";
pub const TRANSFORM_NODE_TYPE: &str = "backend.transform";
pub const SUCCESS_RESPONSE_NODE_TYPE: &str = "backend.success_response";
pub const ERROR_RESPONSE_NODE_TYPE: &str = "backend.error_response";
pub const TRY_CATCH_NODE_TYPE: &str = "backend.try_catch";
pub const VALIDATION_ERROR_NODE_TYPE: &str = "backend.validation_error";
pub const HTTP_REQUEST_TYPE: &str = "HttpRequest";
pub const BOOLEAN_TYPE: &str = "Boolean";
pub const RESPONSE_BODY_TYPE: &str = "ANY";
pub const SUCCESS_RESPONSE_DEFAULT_STATUS_CODE: u16 = 200;
pub const ERROR_RESPONSE_DEFAULT_STATUS_CODE: u16 = 500;
pub const VALIDATION_ERROR_DEFAULT_STATUS_CODE: u16 = 400;
pub const VALIDATION_ERROR_DEFAULT_MESSAGE: &str = "Validation failed";
pub const CRON_SCHEDULE_PROPERTY: &str = "schedule";

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
		let cron_type = NodeTypeId::new(CRON_NODE_TYPE).expect("static node type is valid");
		let branch_type = NodeTypeId::new(BRANCH_NODE_TYPE).expect("static node type is valid");
		let merge_type = NodeTypeId::new(MERGE_NODE_TYPE).expect("static node type is valid");
		let loop_type = NodeTypeId::new(LOOP_NODE_TYPE).expect("static node type is valid");
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
		let boolean_type = TypeId::new(BOOLEAN_TYPE).expect("static type is valid");
		let response_body_type = TypeId::new(RESPONSE_BODY_TYPE).expect("static type is valid");
		let manifest = PluginManifest::new(
			PluginReference::new(BACKEND_DSL_ID, BACKEND_DSL_VERSION)
				.expect("static plugin identity is valid"),
			[
				route_type.clone(),
				cron_type.clone(),
				branch_type.clone(),
				merge_type.clone(),
				loop_type.clone(),
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
				type_id: cron_type,
				label: "Cron".to_string(),
				category: "Entry".to_string(),
				inputs: Vec::new(),
				outputs: vec![BackendPortDescriptor::new(
					"next",
					PortContract::new(PortKind::EXECUTION, None, true),
				)],
				properties: vec![BackendPropertyDescriptor::new(
					CRON_SCHEDULE_PROPERTY,
					BackendPropertyKind::STRING,
					true,
					None,
				)],
			},
			BackendNodeDescriptor {
				type_id: branch_type,
				label: "Branch".to_string(),
				category: "Flow".to_string(),
				inputs: vec![
					BackendPortDescriptor::new(
						"in",
						PortContract::new(PortKind::EXECUTION, None, true),
					),
					BackendPortDescriptor::new(
						"condition",
						PortContract::new(PortKind::DATA, Some(boolean_type.clone()), true),
					),
				],
				outputs: vec![
					BackendPortDescriptor::new(
						"true",
						PortContract::new(PortKind::EXECUTION, None, true),
					),
					BackendPortDescriptor::new(
						"false",
						PortContract::new(PortKind::EXECUTION, None, true),
					),
				],
				properties: Vec::new(),
			},
			BackendNodeDescriptor {
				type_id: merge_type,
				label: "Merge".to_string(),
				category: "Flow".to_string(),
				inputs: vec![
					BackendPortDescriptor::new(
						"left",
						PortContract::new(PortKind::EXECUTION, None, true),
					),
					BackendPortDescriptor::new(
						"right",
						PortContract::new(PortKind::EXECUTION, None, true),
					),
				],
				outputs: vec![BackendPortDescriptor::new(
					"out",
					PortContract::new(PortKind::EXECUTION, None, true),
				)],
				properties: Vec::new(),
			},
			BackendNodeDescriptor {
				type_id: loop_type,
				label: "Loop".to_string(),
				category: "Flow".to_string(),
				inputs: vec![
					BackendPortDescriptor::new(
						"entry",
						PortContract::new(PortKind::EXECUTION, None, true),
					),
					BackendPortDescriptor::new(
						"back",
						PortContract::new(PortKind::EXECUTION, None, true),
					),
					BackendPortDescriptor::new(
						"condition",
						PortContract::new(PortKind::DATA, Some(boolean_type.clone()), true),
					),
				],
				outputs: vec![
					BackendPortDescriptor::new(
						"body",
						PortContract::new(PortKind::EXECUTION, None, true),
					),
					BackendPortDescriptor::new(
						"done",
						PortContract::new(PortKind::EXECUTION, None, true),
					),
				],
				properties: Vec::new(),
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
			CRON_NODE_TYPE => cron_contract(),
			BRANCH_NODE_TYPE => branch_contract(),
			MERGE_NODE_TYPE => merge_contract(),
			LOOP_NODE_TYPE => loop_contract(),
			TRANSFORM_NODE_TYPE => transform_contract(),
			SUCCESS_RESPONSE_NODE_TYPE | ERROR_RESPONSE_NODE_TYPE => response_contract(),
			TRY_CATCH_NODE_TYPE => try_catch_contract(),
			VALIDATION_ERROR_NODE_TYPE => response_contract(),
			_ => NodeContract::new([], [], [], [], [], AsyncKind::SYNC, []),
		}
	}

	fn analyze_graph(&self, document: &GraphDocument) -> Vec<Diagnostic> {
		let mut diagnostics = validate_required_execution_ports(
			document,
			CRON_NODE_TYPE,
			PortDirection::OUTPUT,
			"BACKEND_DSL.CRON_OUTPUT_UNCONNECTED",
			"Cron requires its execution output to be connected.",
		);
		diagnostics.extend(validate_required_execution_ports(
			document,
			BRANCH_NODE_TYPE,
			PortDirection::OUTPUT,
			"BACKEND_DSL.BRANCH_OUTPUT_UNCONNECTED",
			"Branch requires both the true and false execution outputs to be connected.",
		));
		diagnostics.extend(validate_required_execution_ports(
			document,
			MERGE_NODE_TYPE,
			PortDirection::INPUT,
			"BACKEND_DSL.MERGE_INPUT_UNCONNECTED",
			"Merge requires both execution inputs to be connected.",
		));
		diagnostics.extend(validate_required_execution_ports(
			document,
			LOOP_NODE_TYPE,
			PortDirection::INPUT,
			"BACKEND_DSL.LOOP_INPUT_UNCONNECTED",
			"Loop requires its entry and back execution inputs to be connected.",
		));
		diagnostics.extend(validate_required_execution_ports(
			document,
			LOOP_NODE_TYPE,
			PortDirection::OUTPUT,
			"BACKEND_DSL.LOOP_OUTPUT_UNCONNECTED",
			"Loop requires its body and done execution outputs to be connected.",
		));
		diagnostics.extend(validate_loop_back_edges(document));
		diagnostics.extend(validate_try_catch_branches(document));
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
			CRON_NODE_TYPE => cron_schedule(node)
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
			CRON_NODE_TYPE => {
				let schedule = cron_schedule(node).map_err(|error| vec![*error])?;
				Ok(LoweredOperation::new(
					OperationId::new(CRON_NODE_TYPE).expect("static operation is valid"),
					[],
					[],
					AsyncKind::SYNC,
					[],
				)
				.with_attribute("backend.cron.schedule", schedule))
			}
			BRANCH_NODE_TYPE => Ok(LoweredOperation::new(
				OperationId::new(BRANCH_NODE_TYPE).expect("static operation is valid"),
				[TypeId::new(BOOLEAN_TYPE).expect("static type is valid")],
				[],
				AsyncKind::SYNC,
				[],
			)
			.with_kind(LoweredOperationKind::BRANCH)
			.with_attribute("control.kind", "BRANCH")),
			MERGE_NODE_TYPE => Ok(LoweredOperation::new(
				OperationId::new(MERGE_NODE_TYPE).expect("static operation is valid"),
				[],
				[],
				AsyncKind::SYNC,
				[],
			)
			.with_kind(LoweredOperationKind::MERGE)
			.with_attribute("control.kind", "MERGE")),
			LOOP_NODE_TYPE => Ok(LoweredOperation::new(
				OperationId::new(LOOP_NODE_TYPE).expect("static operation is valid"),
				[TypeId::new(BOOLEAN_TYPE).expect("static type is valid")],
				[],
				AsyncKind::SYNC,
				[],
			)
			.with_kind(LoweredOperationKind::LOOP)
			.with_attribute("control.kind", "LOOP")),
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
			.with_attribute("error.handling.kind", "TRY_CATCH")),
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

fn cron_contract() -> NodeContract {
	NodeContract::new(
		[],
		[PortContract::new(PortKind::EXECUTION, None, true)],
		[],
		[],
		[],
		AsyncKind::SYNC,
		[],
	)
	.with_entry(true)
}

fn branch_contract() -> NodeContract {
	NodeContract::new(
		[
			PortContract::new(PortKind::EXECUTION, None, true),
			PortContract::new(
				PortKind::DATA,
				Some(TypeId::new(BOOLEAN_TYPE).expect("static type is valid")),
				true,
			),
		],
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

fn merge_contract() -> NodeContract {
	NodeContract::new(
		[
			PortContract::new(PortKind::EXECUTION, None, true),
			PortContract::new(PortKind::EXECUTION, None, true),
		],
		[PortContract::new(PortKind::EXECUTION, None, true)],
		[],
		[],
		[],
		AsyncKind::SYNC,
		[],
	)
}

fn loop_contract() -> NodeContract {
	NodeContract::new(
		[
			PortContract::new(PortKind::EXECUTION, None, true),
			PortContract::new(PortKind::EXECUTION, None, true),
			PortContract::new(
				PortKind::DATA,
				Some(TypeId::new(BOOLEAN_TYPE).expect("static type is valid")),
				true,
			),
		],
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
	.with_loop(true)
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
				"Every HTTP Route execution path must reach a Success Response, Error Response, or Validation Error.",
			))
		})
		.map(|diagnostic| *diagnostic)
		.collect()
}

fn validate_try_catch_branches(document: &GraphDocument) -> Vec<Diagnostic> {
	validate_required_execution_ports(
		document,
		TRY_CATCH_NODE_TYPE,
		PortDirection::OUTPUT,
		"BACKEND_DSL.TRY_CATCH_BRANCH_UNCONNECTED",
		"Try/Catch requires both the try and catch execution branches to be connected.",
	)
}

fn validate_required_execution_ports(
	document: &GraphDocument,
	node_type: &str,
	direction: PortDirection,
	code: &str,
	message: &str,
) -> Vec<Diagnostic> {
	document
		.nodes()
		.iter()
		.filter(|node| node.type_id().as_str() == node_type)
		.flat_map(|node| {
			node.ports()
				.iter()
				.filter(|port| port.direction() == direction && port.kind() == PortKind::EXECUTION)
				.filter(|port| {
					let connected = document.edges().iter().any(|edge| {
						edge.kind() == EdgeKind::EXECUTION
							&& match direction {
								PortDirection::INPUT => edge.to() == port.id(),
								PortDirection::OUTPUT => edge.from() == port.id(),
							}
					});
					!connected
				})
				.map(|port| port_diagnostic(port, code, message))
		})
		.map(|diagnostic| *diagnostic)
		.collect()
}

fn validate_loop_back_edges(document: &GraphDocument) -> Vec<Diagnostic> {
	let mut port_owners = BTreeMap::<PortId, NodeId>::new();
	for node in document.nodes() {
		for port in node.ports() {
			port_owners.insert(port.id(), node.id());
		}
	}

	let mut outgoing = BTreeMap::<PortId, Vec<(NodeId, PortId)>>::new();
	for edge in document
		.edges()
		.iter()
		.filter(|edge| edge.kind() == EdgeKind::EXECUTION)
	{
		let Some(target_node) = port_owners.get(&edge.to()).copied() else {
			continue;
		};
		outgoing
			.entry(edge.from())
			.or_default()
			.push((target_node, edge.to()));
	}

	document
		.nodes()
		.iter()
		.filter(|node| node.type_id().as_str() == LOOP_NODE_TYPE)
		.filter_map(|loop_node| {
			let execution_inputs: Vec<_> = loop_node
				.ports()
				.iter()
				.filter(|port| {
					port.direction() == PortDirection::INPUT && port.kind() == PortKind::EXECUTION
				})
				.collect();
			let body_output = loop_node.ports().iter().find(|port| {
				port.direction() == PortDirection::OUTPUT && port.kind() == PortKind::EXECUTION
			})?;
			let back_input = *execution_inputs.get(1)?;

			let mut worklist = VecDeque::new();
			let mut visited = BTreeSet::new();
			for (target_node, target_port) in outgoing
				.get(&body_output.id())
				.into_iter()
				.flatten()
				.copied()
			{
				worklist.push_back((target_node, target_port));
			}

			let mut reaches_back_input = false;
			while let Some((node_id, target_port)) = worklist.pop_front() {
				if !visited.insert((node_id, target_port)) {
					continue;
				}
				if node_id == loop_node.id() {
					if target_port == back_input.id() {
						reaches_back_input = true;
						break;
					}
					continue;
				}

				let Some(node) = document.nodes().iter().find(|node| node.id() == node_id) else {
					continue;
				};
				for output in node.ports().iter().filter(|port| {
					port.direction() == PortDirection::OUTPUT && port.kind() == PortKind::EXECUTION
				}) {
					for successor in outgoing.get(&output.id()).into_iter().flatten() {
						worklist.push_back(*successor);
					}
				}
			}

			if reaches_back_input {
				None
			} else {
				Some(port_diagnostic(
					body_output,
					"BACKEND_DSL.LOOP_BODY_MISSING_BACK_EDGE",
					"Loop body execution must return to the loop's back input.",
				))
			}
		})
		.map(|diagnostic| *diagnostic)
		.collect()
}

fn cron_schedule(node: &Node) -> Result<String, Box<Diagnostic>> {
	let Some(value) = node.properties().get(CRON_SCHEDULE_PROPERTY) else {
		return Err(node_diagnostic(
			node,
			"BACKEND_DSL.CRON_SCHEDULE_MISSING",
			"Cron requires a 'schedule' property.",
		));
	};
	let Some(schedule) = value.as_str() else {
		return Err(node_diagnostic(
			node,
			"BACKEND_DSL.CRON_SCHEDULE_TYPE",
			"Cron property 'schedule' must be a string.",
		));
	};
	let schedule = schedule.split_whitespace().collect::<Vec<_>>().join(" ");
	let fields: Vec<_> = schedule.split(' ').collect();
	let valid = fields.len() == 5
		&& fields.iter().all(|field| {
			!field.is_empty()
				&& field.chars().all(|character| {
					character.is_ascii_digit() || matches!(character, '*' | '/' | ',' | '-')
				})
		});
	if !valid {
		return Err(node_diagnostic(
			node,
			"BACKEND_DSL.CRON_SCHEDULE_INVALID",
			"Cron schedule must contain five fields using digits, '*', '/', ',', and '-'.",
		));
	}
	Ok(schedule)
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

	fn loop_document(with_back_edge: bool) -> GraphDocument {
		let metadata = krunt_graph_model::GraphMetadata::new(
			krunt_graph_model::GraphId::new(if with_back_edge { 10 } else { 11 }).unwrap(),
			"backend-v1",
			"krunt-test",
		)
		.unwrap();
		let mut source = Node::new(
			NodeId::new(1).unwrap(),
			NodeTypeId::new("backend.test_source").unwrap(),
		);
		source
			.add_port(krunt_graph_model::Port::new(
				PortId::new(11).unwrap(),
				PortDirection::OUTPUT,
				PortKind::EXECUTION,
			))
			.unwrap();
		source
			.add_port(krunt_graph_model::Port::new(
				PortId::new(13).unwrap(),
				PortDirection::OUTPUT,
				PortKind::EXECUTION,
			))
			.unwrap();
		source
			.add_port(
				krunt_graph_model::Port::new(
					PortId::new(12).unwrap(),
					PortDirection::OUTPUT,
					PortKind::DATA,
				)
				.with_type_reference(krunt_graph_model::TypeReference::new(BOOLEAN_TYPE).unwrap()),
			)
			.unwrap();

		let mut loop_node = Node::new(
			NodeId::new(2).unwrap(),
			NodeTypeId::new(LOOP_NODE_TYPE).unwrap(),
		);
		for (port_id, direction, kind) in [
			(21, PortDirection::INPUT, PortKind::EXECUTION),
			(22, PortDirection::INPUT, PortKind::EXECUTION),
			(24, PortDirection::OUTPUT, PortKind::EXECUTION),
			(25, PortDirection::OUTPUT, PortKind::EXECUTION),
		] {
			loop_node
				.add_port(krunt_graph_model::Port::new(
					PortId::new(port_id).unwrap(),
					direction,
					kind,
				))
				.unwrap();
		}
		loop_node
			.add_port(
				krunt_graph_model::Port::new(
					PortId::new(23).unwrap(),
					PortDirection::INPUT,
					PortKind::DATA,
				)
				.with_type_reference(krunt_graph_model::TypeReference::new(BOOLEAN_TYPE).unwrap()),
			)
			.unwrap();

		let mut body = Node::new(
			NodeId::new(3).unwrap(),
			NodeTypeId::new("backend.test_body").unwrap(),
		);
		body.add_port(krunt_graph_model::Port::new(
			PortId::new(31).unwrap(),
			PortDirection::INPUT,
			PortKind::EXECUTION,
		))
		.unwrap();
		body.add_port(krunt_graph_model::Port::new(
			PortId::new(32).unwrap(),
			PortDirection::OUTPUT,
			PortKind::EXECUTION,
		))
		.unwrap();

		let mut sink = Node::new(
			NodeId::new(4).unwrap(),
			NodeTypeId::new("backend.test_sink").unwrap(),
		);
		for port_id in [41, 42] {
			sink.add_port(krunt_graph_model::Port::new(
				PortId::new(port_id).unwrap(),
				PortDirection::INPUT,
				PortKind::EXECUTION,
			))
			.unwrap();
		}

		let mut builder = krunt_graph_model::GraphBuilder::new(metadata);
		for node in [source, loop_node, body, sink] {
			builder.add_node(node).unwrap();
		}
		let edges = [
			(51, 11, 21, EdgeKind::EXECUTION),
			(52, 12, 23, EdgeKind::DATA),
			(53, 24, 31, EdgeKind::EXECUTION),
			(54, 25, 41, EdgeKind::EXECUTION),
		];
		for (edge_id, from, to, kind) in edges {
			builder
				.add_edge(krunt_graph_model::Edge::new(
					krunt_graph_model::EdgeId::new(edge_id).unwrap(),
					PortId::new(from).unwrap(),
					PortId::new(to).unwrap(),
					kind,
				))
				.unwrap();
		}
		if with_back_edge {
			builder
				.add_edge(krunt_graph_model::Edge::new(
					krunt_graph_model::EdgeId::new(55).unwrap(),
					PortId::new(32).unwrap(),
					PortId::new(22).unwrap(),
					EdgeKind::EXECUTION,
				))
				.unwrap();
		} else {
			builder
				.add_edge(krunt_graph_model::Edge::new(
					krunt_graph_model::EdgeId::new(56).unwrap(),
					PortId::new(13).unwrap(),
					PortId::new(22).unwrap(),
					EdgeKind::EXECUTION,
				))
				.unwrap();
			builder
				.add_edge(krunt_graph_model::Edge::new(
					krunt_graph_model::EdgeId::new(57).unwrap(),
					PortId::new(32).unwrap(),
					PortId::new(42).unwrap(),
					EdgeKind::EXECUTION,
				))
				.unwrap();
		}
		builder.build().unwrap()
	}

	#[test]
	fn exposes_route_and_response_descriptors() {
		let plugin = BackendDslPlugin::new();
		assert_eq!(plugin.descriptors().len(), 10);
		assert_eq!(plugin.descriptors()[0].label(), "HTTP Route");
		assert_eq!(plugin.descriptors()[0].properties().len(), 2);
		let cron = plugin
			.descriptors()
			.iter()
			.find(|descriptor| descriptor.type_id().as_str() == CRON_NODE_TYPE)
			.expect("cron descriptor should be registered");
		assert_eq!(cron.label(), "Cron");
		assert_eq!(cron.category(), "Entry");
		assert!(cron.inputs().is_empty());
		assert_eq!(cron.outputs().len(), 1);
		assert_eq!(cron.properties().len(), 1);
		assert_eq!(cron.properties()[0].key(), CRON_SCHEDULE_PROPERTY);
		assert_eq!(cron.properties()[0].kind(), &BackendPropertyKind::STRING);
		assert!(cron.properties()[0].required());
		assert!(cron.properties()[0].default().is_none());
		let branch = plugin
			.descriptors()
			.iter()
			.find(|descriptor| descriptor.type_id().as_str() == BRANCH_NODE_TYPE)
			.expect("branch descriptor should be registered");
		assert_eq!(branch.label(), "Branch");
		assert_eq!(branch.category(), "Flow");
		assert_eq!(branch.inputs().len(), 2);
		assert_eq!(
			branch.inputs()[1].contract().type_id().unwrap().as_str(),
			BOOLEAN_TYPE
		);
		assert_eq!(branch.outputs().len(), 2);
		let merge = plugin
			.descriptors()
			.iter()
			.find(|descriptor| descriptor.type_id().as_str() == MERGE_NODE_TYPE)
			.expect("merge descriptor should be registered");
		assert_eq!(merge.label(), "Merge");
		assert_eq!(merge.category(), "Flow");
		assert_eq!(merge.inputs().len(), 2);
		assert_eq!(merge.outputs().len(), 1);
		let loop_descriptor = plugin
			.descriptors()
			.iter()
			.find(|descriptor| descriptor.type_id().as_str() == LOOP_NODE_TYPE)
			.expect("loop descriptor should be registered");
		assert_eq!(loop_descriptor.label(), "Loop");
		assert_eq!(loop_descriptor.category(), "Flow");
		assert_eq!(loop_descriptor.inputs().len(), 3);
		assert_eq!(
			loop_descriptor.inputs()[2]
				.contract()
				.type_id()
				.unwrap()
				.as_str(),
			BOOLEAN_TYPE
		);
		assert_eq!(loop_descriptor.outputs().len(), 2);
		assert!(loop_descriptor.properties().is_empty());
		let transform = plugin
			.descriptors()
			.iter()
			.find(|descriptor| descriptor.type_id().as_str() == TRANSFORM_NODE_TYPE)
			.expect("transform descriptor should be registered");
		assert_eq!(transform.label(), "Identity Transform");
		assert_eq!(transform.inputs().len(), 2);
		assert_eq!(transform.outputs().len(), 2);
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
	fn lowers_branch_with_a_boolean_condition_and_explicit_control_kind() {
		let node = Node::new(
			NodeId::new(2).unwrap(),
			NodeTypeId::new(BRANCH_NODE_TYPE).unwrap(),
		);
		let lowered = BackendDslPlugin::new().lower_node(&node).unwrap();

		assert_eq!(lowered.kind(), LoweredOperationKind::BRANCH);
		assert_eq!(lowered.input_types(), &[TypeId::new(BOOLEAN_TYPE).unwrap()]);
		assert!(lowered.output_types().is_empty());
		assert_eq!(lowered.attributes()["control.kind"], "BRANCH");
	}

	#[test]
	fn lowers_merge_with_explicit_control_kind() {
		let node = Node::new(
			NodeId::new(2).unwrap(),
			NodeTypeId::new(MERGE_NODE_TYPE).unwrap(),
		);
		let lowered = BackendDslPlugin::new().lower_node(&node).unwrap();

		assert_eq!(lowered.kind(), LoweredOperationKind::MERGE);
		assert!(lowered.input_types().is_empty());
		assert!(lowered.output_types().is_empty());
		assert_eq!(lowered.attributes()["control.kind"], "MERGE");
	}

	#[test]
	fn lowers_loop_with_a_boolean_condition_and_structured_control_kind() {
		let node = Node::new(
			NodeId::new(2).unwrap(),
			NodeTypeId::new(LOOP_NODE_TYPE).unwrap(),
		);
		let contract = BackendDslPlugin::new().contract(&node);
		let lowered = BackendDslPlugin::new().lower_node(&node).unwrap();

		assert!(contract.is_loop());
		assert_eq!(lowered.kind(), LoweredOperationKind::LOOP);
		assert_eq!(lowered.input_types(), &[TypeId::new(BOOLEAN_TYPE).unwrap()]);
		assert!(lowered.output_types().is_empty());
		assert_eq!(lowered.attributes()["control.kind"], "LOOP");
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
	fn lowers_a_valid_cron_with_target_neutral_schedule_metadata() {
		let plugin = BackendDslPlugin::new();
		let node = Node::new(
			NodeId::new(2).unwrap(),
			NodeTypeId::new(CRON_NODE_TYPE).unwrap(),
		)
		.with_property(
			CRON_SCHEDULE_PROPERTY,
			Value::String("*/5 * * * *".to_string()),
		)
		.unwrap();
		let contract = plugin.contract(&node);
		let lowered = plugin.lower_node(&node).unwrap();

		assert!(contract.is_entry());
		assert_eq!(lowered.operation().as_str(), CRON_NODE_TYPE);
		assert!(lowered.input_types().is_empty());
		assert!(lowered.output_types().is_empty());
		assert_eq!(lowered.attributes()["backend.cron.schedule"], "*/5 * * * *");
	}

	#[test]
	fn rejects_invalid_cron_schedules() {
		let plugin = BackendDslPlugin::new();
		let missing = Node::new(
			NodeId::new(2).unwrap(),
			NodeTypeId::new(CRON_NODE_TYPE).unwrap(),
		);
		let wrong_type = missing
			.clone()
			.with_property(CRON_SCHEDULE_PROPERTY, Value::Bool(true))
			.unwrap();
		let invalid = missing
			.clone()
			.with_property(
				CRON_SCHEDULE_PROPERTY,
				Value::String("every-five-minutes".to_string()),
			)
			.unwrap();

		assert_eq!(
			plugin.analyze_node(&missing)[0].code().as_str(),
			"BACKEND_DSL.CRON_SCHEDULE_MISSING"
		);
		assert_eq!(
			plugin.analyze_node(&wrong_type)[0].code().as_str(),
			"BACKEND_DSL.CRON_SCHEDULE_TYPE"
		);
		assert_eq!(
			plugin.analyze_node(&invalid)[0].code().as_str(),
			"BACKEND_DSL.CRON_SCHEDULE_INVALID"
		);
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
	fn rejects_a_cron_with_an_unconnected_execution_output() {
		let metadata = krunt_graph_model::GraphMetadata::new(
			krunt_graph_model::GraphId::new(5).unwrap(),
			"backend-v1",
			"krunt-test",
		)
		.unwrap();
		let mut cron = Node::new(
			NodeId::new(2).unwrap(),
			NodeTypeId::new(CRON_NODE_TYPE).unwrap(),
		)
		.with_property(
			CRON_SCHEDULE_PROPERTY,
			Value::String("0 * * * *".to_string()),
		)
		.unwrap();
		cron.add_port(krunt_graph_model::Port::new(
			PortId::new(21).unwrap(),
			PortDirection::OUTPUT,
			PortKind::EXECUTION,
		))
		.unwrap();
		let mut builder = krunt_graph_model::GraphBuilder::new(metadata);
		builder.add_node(cron).unwrap();
		let diagnostics = BackendDslPlugin::new().analyze_graph(&builder.build().unwrap());

		assert_eq!(diagnostics.len(), 1);
		assert_eq!(
			diagnostics[0].code().as_str(),
			"BACKEND_DSL.CRON_OUTPUT_UNCONNECTED"
		);
		assert_eq!(
			diagnostics[0].primary(),
			GraphElementId::PORT(PortId::new(21).unwrap())
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

	#[test]
	fn rejects_a_branch_with_unconnected_outputs() {
		let metadata = krunt_graph_model::GraphMetadata::new(
			krunt_graph_model::GraphId::new(3).unwrap(),
			"backend-v1",
			"krunt-test",
		)
		.unwrap();
		let mut branch = Node::new(
			NodeId::new(2).unwrap(),
			NodeTypeId::new(BRANCH_NODE_TYPE).unwrap(),
		);
		branch
			.add_port(krunt_graph_model::Port::new(
				PortId::new(21).unwrap(),
				PortDirection::INPUT,
				PortKind::EXECUTION,
			))
			.unwrap();
		branch
			.add_port(
				krunt_graph_model::Port::new(
					PortId::new(22).unwrap(),
					PortDirection::INPUT,
					PortKind::DATA,
				)
				.with_type_reference(krunt_graph_model::TypeReference::new(BOOLEAN_TYPE).unwrap()),
			)
			.unwrap();
		branch
			.add_port(krunt_graph_model::Port::new(
				PortId::new(23).unwrap(),
				PortDirection::OUTPUT,
				PortKind::EXECUTION,
			))
			.unwrap();
		branch
			.add_port(krunt_graph_model::Port::new(
				PortId::new(24).unwrap(),
				PortDirection::OUTPUT,
				PortKind::EXECUTION,
			))
			.unwrap();
		let mut builder = krunt_graph_model::GraphBuilder::new(metadata);
		builder.add_node(branch).unwrap();
		let diagnostics = BackendDslPlugin::new().analyze_graph(&builder.build().unwrap());

		assert_eq!(diagnostics.len(), 2);
		assert!(diagnostics.iter().all(|diagnostic| {
			diagnostic.code().as_str() == "BACKEND_DSL.BRANCH_OUTPUT_UNCONNECTED"
		}));
	}

	#[test]
	fn rejects_a_merge_with_unconnected_inputs() {
		let metadata = krunt_graph_model::GraphMetadata::new(
			krunt_graph_model::GraphId::new(4).unwrap(),
			"backend-v1",
			"krunt-test",
		)
		.unwrap();
		let mut merge = Node::new(
			NodeId::new(2).unwrap(),
			NodeTypeId::new(MERGE_NODE_TYPE).unwrap(),
		);
		merge
			.add_port(krunt_graph_model::Port::new(
				PortId::new(21).unwrap(),
				PortDirection::INPUT,
				PortKind::EXECUTION,
			))
			.unwrap();
		merge
			.add_port(krunt_graph_model::Port::new(
				PortId::new(22).unwrap(),
				PortDirection::INPUT,
				PortKind::EXECUTION,
			))
			.unwrap();
		merge
			.add_port(krunt_graph_model::Port::new(
				PortId::new(23).unwrap(),
				PortDirection::OUTPUT,
				PortKind::EXECUTION,
			))
			.unwrap();
		let mut builder = krunt_graph_model::GraphBuilder::new(metadata);
		builder.add_node(merge).unwrap();
		let diagnostics = BackendDslPlugin::new().analyze_graph(&builder.build().unwrap());

		assert_eq!(diagnostics.len(), 2);
		assert!(diagnostics.iter().all(|diagnostic| {
			diagnostic.code().as_str() == "BACKEND_DSL.MERGE_INPUT_UNCONNECTED"
		}));
	}

	#[test]
	fn accepts_a_loop_with_a_body_back_edge_and_done_exit() {
		let diagnostics = BackendDslPlugin::new().analyze_graph(&loop_document(true));

		assert!(diagnostics.is_empty());
	}

	#[test]
	fn rejects_a_loop_whose_body_does_not_return_to_the_back_input() {
		let diagnostics = BackendDslPlugin::new().analyze_graph(&loop_document(false));

		assert_eq!(diagnostics.len(), 1);
		assert_eq!(
			diagnostics[0].code().as_str(),
			"BACKEND_DSL.LOOP_BODY_MISSING_BACK_EDGE"
		);
		assert_eq!(
			diagnostics[0].primary(),
			GraphElementId::PORT(PortId::new(24).unwrap())
		);
	}
}
