//! Initial Krunt Backend DSL semantic plugin.
//!
//! This crate owns backend meaning for the first vertical slice. It describes
//! HTTP Route, Cron, Branch, Merge, Loop, Subgraph, Entity, Repository, Query,
//! Mutation, HTTP Request, Logging, Storage, Identity Transform, Success Response,
//! Error Response, Try/Catch, and Validation Error nodes, validates their properties
//! and typed data contracts, and lowers them into target-neutral operations with
//! namespaced metadata. It does not emit TypeScript or NestJS code.

use krunt_diagnostics::{CompilerStage, Diagnostic, DiagnosticCode, GraphElementId, Severity};
use krunt_graph_model::{
	EdgeKind, GraphDocument, Node, NodeId, NodeTypeId, PluginReference, PortDirection, PortId,
	PortKind, SubgraphId,
};
use krunt_ir::{AsyncKind, EffectId, OperationId, TypeId};
use krunt_plugin_api::{
	LoweredOperation, LoweredOperationKind, NodeContract, PluginApiError, PluginManifest,
	PluginRegistry, PortContract,
};
use krunt_type_system::Type;
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

pub const BACKEND_DSL_ID: &str = "krunt.backend-dsl";
pub const BACKEND_DSL_VERSION: &str = "0.2.0";
pub const HTTP_ROUTE_NODE_TYPE: &str = "backend.http_route";
pub const HTTP_REQUEST_NODE_TYPE: &str = "backend.http_request";
pub const CRON_NODE_TYPE: &str = "backend.cron";
pub const BRANCH_NODE_TYPE: &str = "backend.branch";
pub const MERGE_NODE_TYPE: &str = "backend.merge";
pub const LOOP_NODE_TYPE: &str = "backend.loop";
pub const SUBGRAPH_NODE_TYPE: &str = "backend.subgraph";
pub const ENTITY_NODE_TYPE: &str = "backend.entity";
pub const REPOSITORY_NODE_TYPE: &str = "backend.repository";
pub const QUERY_NODE_TYPE: &str = "backend.query";
pub const MUTATION_NODE_TYPE: &str = "backend.mutation";
pub const LOGGING_NODE_TYPE: &str = "backend.logging";
pub const STORAGE_NODE_TYPE: &str = "backend.storage";
pub const TRANSFORM_NODE_TYPE: &str = "backend.transform";
pub const SUCCESS_RESPONSE_NODE_TYPE: &str = "backend.success_response";
pub const ERROR_RESPONSE_NODE_TYPE: &str = "backend.error_response";
pub const TRY_CATCH_NODE_TYPE: &str = "backend.try_catch";
pub const VALIDATION_ERROR_NODE_TYPE: &str = "backend.validation_error";
pub const HTTP_REQUEST_TYPE: &str = "HttpRequest";
pub const BOOLEAN_TYPE: &str = "Boolean";
pub const RESPONSE_BODY_TYPE: &str = "ANY";
pub const DATABASE_READ_EFFECT: &str = "database.read";
pub const DATABASE_WRITE_EFFECT: &str = "database.write";
pub const HTTP_REQUEST_EFFECT: &str = "http.request";
pub const LOGGING_EFFECT: &str = "logging.write";
pub const STORAGE_READ_EFFECT: &str = "storage.read";
pub const STORAGE_WRITE_EFFECT: &str = "storage.write";
pub const SUCCESS_RESPONSE_DEFAULT_STATUS_CODE: u16 = 200;
pub const ERROR_RESPONSE_DEFAULT_STATUS_CODE: u16 = 500;
pub const VALIDATION_ERROR_DEFAULT_STATUS_CODE: u16 = 400;
pub const VALIDATION_ERROR_DEFAULT_MESSAGE: &str = "Validation failed";
pub const CRON_SCHEDULE_PROPERTY: &str = "schedule";
pub const SUBGRAPH_ID_PROPERTY: &str = "subgraph_id";
pub const ENTITY_NAME_PROPERTY: &str = "name";
pub const ENTITY_FIELDS_PROPERTY: &str = "fields";
pub const HTTP_ROUTE_REQUEST_ENTITY_PROPERTY: &str = "request_entity";
pub const REPOSITORY_NAME_PROPERTY: &str = "name";
pub const REPOSITORY_ENTITY_PROPERTY: &str = "entity";
pub const QUERY_NAME_PROPERTY: &str = "name";
pub const QUERY_REPOSITORY_PROPERTY: &str = "repository";
pub const MUTATION_NAME_PROPERTY: &str = "name";
pub const MUTATION_REPOSITORY_PROPERTY: &str = "repository";
pub const HTTP_REQUEST_METHOD_PROPERTY: &str = "method";
pub const HTTP_REQUEST_URL_PROPERTY: &str = "url";
pub const LOGGING_LEVEL_PROPERTY: &str = "level";
pub const STORAGE_OPERATION_PROPERTY: &str = "operation";
pub const STORAGE_KEY_PROPERTY: &str = "key";

const HTTP_METHODS: [&str; 7] = ["DELETE", "GET", "HEAD", "OPTIONS", "PATCH", "POST", "PUT"];
const LOG_LEVELS: [&str; 4] = ["DEBUG", "INFO", "WARN", "ERROR"];
const STORAGE_OPERATIONS: [&str; 2] = ["READ", "WRITE"];

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StorageOperation {
	READ,
	WRITE,
}

impl StorageOperation {
	fn as_str(self) -> &'static str {
		match self {
			Self::READ => "READ",
			Self::WRITE => "WRITE",
		}
	}

	fn effect(self) -> EffectId {
		let effect = match self {
			Self::READ => STORAGE_READ_EFFECT,
			Self::WRITE => STORAGE_WRITE_EFFECT,
		};
		EffectId::new(effect).expect("static effect is valid")
	}
}

/// Serializable property kind exposed to editor and AI clients.
#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub enum BackendPropertyKind {
	STRING,
	INTEGER,
	OBJECT,
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
		let http_request_type =
			NodeTypeId::new(HTTP_REQUEST_NODE_TYPE).expect("static node type is valid");
		let cron_type = NodeTypeId::new(CRON_NODE_TYPE).expect("static node type is valid");
		let branch_type = NodeTypeId::new(BRANCH_NODE_TYPE).expect("static node type is valid");
		let merge_type = NodeTypeId::new(MERGE_NODE_TYPE).expect("static node type is valid");
		let loop_type = NodeTypeId::new(LOOP_NODE_TYPE).expect("static node type is valid");
		let subgraph_type = NodeTypeId::new(SUBGRAPH_NODE_TYPE).expect("static node type is valid");
		let entity_type = NodeTypeId::new(ENTITY_NODE_TYPE).expect("static node type is valid");
		let repository_type =
			NodeTypeId::new(REPOSITORY_NODE_TYPE).expect("static node type is valid");
		let query_type = NodeTypeId::new(QUERY_NODE_TYPE).expect("static node type is valid");
		let mutation_type = NodeTypeId::new(MUTATION_NODE_TYPE).expect("static node type is valid");
		let logging_type = NodeTypeId::new(LOGGING_NODE_TYPE).expect("static node type is valid");
		let storage_type = NodeTypeId::new(STORAGE_NODE_TYPE).expect("static node type is valid");
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
				http_request_type.clone(),
				cron_type.clone(),
				branch_type.clone(),
				merge_type.clone(),
				loop_type.clone(),
				subgraph_type.clone(),
				entity_type.clone(),
				repository_type.clone(),
				query_type.clone(),
				mutation_type.clone(),
				logging_type.clone(),
				storage_type.clone(),
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
					BackendPropertyDescriptor::new(
						HTTP_ROUTE_REQUEST_ENTITY_PROPERTY,
						BackendPropertyKind::STRING,
						false,
						None,
					),
				],
			},
			BackendNodeDescriptor {
				type_id: http_request_type,
				label: "HTTP Request".to_string(),
				category: "Effects".to_string(),
				inputs: vec![
					BackendPortDescriptor::new(
						"in",
						PortContract::new(PortKind::EXECUTION, None, true),
					),
					BackendPortDescriptor::new(
						"request",
						PortContract::new(PortKind::DATA, Some(response_body_type.clone()), false),
					),
				],
				outputs: vec![
					BackendPortDescriptor::new(
						"next",
						PortContract::new(PortKind::EXECUTION, None, true),
					),
					BackendPortDescriptor::new(
						"response",
						PortContract::new(PortKind::DATA, Some(response_body_type.clone()), true),
					),
				],
				properties: vec![
					BackendPropertyDescriptor::new(
						HTTP_REQUEST_METHOD_PROPERTY,
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
						HTTP_REQUEST_URL_PROPERTY,
						BackendPropertyKind::STRING,
						true,
						None,
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
				type_id: logging_type,
				label: "Logging".to_string(),
				category: "Effects".to_string(),
				inputs: vec![
					BackendPortDescriptor::new(
						"in",
						PortContract::new(PortKind::EXECUTION, None, true),
					),
					BackendPortDescriptor::new(
						"message",
						PortContract::new(PortKind::DATA, Some(response_body_type.clone()), true),
					),
				],
				outputs: vec![BackendPortDescriptor::new(
					"next",
					PortContract::new(PortKind::EXECUTION, None, true),
				)],
				properties: vec![BackendPropertyDescriptor::new(
					LOGGING_LEVEL_PROPERTY,
					BackendPropertyKind::ENUM {
						values: LOG_LEVELS
							.iter()
							.map(|value| (*value).to_string())
							.collect(),
					},
					false,
					Some(Value::String("INFO".to_string())),
				)],
			},
			BackendNodeDescriptor {
				type_id: storage_type,
				label: "Storage".to_string(),
				category: "Effects".to_string(),
				inputs: vec![
					BackendPortDescriptor::new(
						"in",
						PortContract::new(PortKind::EXECUTION, None, true),
					),
					BackendPortDescriptor::new(
						"value",
						PortContract::new(PortKind::DATA, Some(response_body_type.clone()), false),
					),
				],
				outputs: vec![
					BackendPortDescriptor::new(
						"next",
						PortContract::new(PortKind::EXECUTION, None, true),
					),
					BackendPortDescriptor::new(
						"result",
						PortContract::new(PortKind::DATA, Some(response_body_type.clone()), false),
					),
				],
				properties: vec![
					BackendPropertyDescriptor::new(
						STORAGE_OPERATION_PROPERTY,
						BackendPropertyKind::ENUM {
							values: STORAGE_OPERATIONS
								.iter()
								.map(|value| (*value).to_string())
								.collect(),
						},
						true,
						Some(Value::String("READ".to_string())),
					),
					BackendPropertyDescriptor::new(
						STORAGE_KEY_PROPERTY,
						BackendPropertyKind::STRING,
						true,
						None,
					),
				],
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
				type_id: subgraph_type,
				label: "Subgraph".to_string(),
				category: "Flow".to_string(),
				inputs: vec![BackendPortDescriptor::new(
					"in",
					PortContract::new(PortKind::EXECUTION, None, true),
				)],
				outputs: vec![BackendPortDescriptor::new(
					"out",
					PortContract::new(PortKind::EXECUTION, None, true),
				)],
				properties: vec![BackendPropertyDescriptor::new(
					SUBGRAPH_ID_PROPERTY,
					BackendPropertyKind::INTEGER,
					true,
					None,
				)],
			},
			BackendNodeDescriptor {
				type_id: entity_type,
				label: "Entity".to_string(),
				category: "Persistence".to_string(),
				inputs: Vec::new(),
				outputs: Vec::new(),
				properties: vec![
					BackendPropertyDescriptor::new(
						ENTITY_NAME_PROPERTY,
						BackendPropertyKind::STRING,
						true,
						None,
					),
					BackendPropertyDescriptor::new(
						ENTITY_FIELDS_PROPERTY,
						BackendPropertyKind::OBJECT,
						false,
						Some(Value::Object(serde_json::Map::new())),
					),
				],
			},
			BackendNodeDescriptor {
				type_id: repository_type,
				label: "Repository".to_string(),
				category: "Persistence".to_string(),
				inputs: Vec::new(),
				outputs: Vec::new(),
				properties: vec![
					BackendPropertyDescriptor::new(
						REPOSITORY_NAME_PROPERTY,
						BackendPropertyKind::STRING,
						true,
						None,
					),
					BackendPropertyDescriptor::new(
						REPOSITORY_ENTITY_PROPERTY,
						BackendPropertyKind::STRING,
						true,
						None,
					),
				],
			},
			BackendNodeDescriptor {
				type_id: query_type,
				label: "Query".to_string(),
				category: "Persistence".to_string(),
				inputs: vec![BackendPortDescriptor::new(
					"in",
					PortContract::new(PortKind::EXECUTION, None, true),
				)],
				outputs: vec![
					BackendPortDescriptor::new(
						"next",
						PortContract::new(PortKind::EXECUTION, None, true),
					),
					BackendPortDescriptor::new(
						"result",
						PortContract::new(PortKind::DATA, Some(response_body_type.clone()), true),
					),
				],
				properties: vec![
					BackendPropertyDescriptor::new(
						QUERY_NAME_PROPERTY,
						BackendPropertyKind::STRING,
						true,
						None,
					),
					BackendPropertyDescriptor::new(
						QUERY_REPOSITORY_PROPERTY,
						BackendPropertyKind::STRING,
						true,
						None,
					),
				],
			},
			BackendNodeDescriptor {
				type_id: mutation_type,
				label: "Mutation".to_string(),
				category: "Persistence".to_string(),
				inputs: vec![
					BackendPortDescriptor::new(
						"in",
						PortContract::new(PortKind::EXECUTION, None, true),
					),
					BackendPortDescriptor::new(
						"value",
						PortContract::new(PortKind::DATA, Some(response_body_type.clone()), true),
					),
				],
				outputs: vec![
					BackendPortDescriptor::new(
						"next",
						PortContract::new(PortKind::EXECUTION, None, true),
					),
					BackendPortDescriptor::new(
						"result",
						PortContract::new(PortKind::DATA, Some(response_body_type.clone()), true),
					),
				],
				properties: vec![
					BackendPropertyDescriptor::new(
						MUTATION_NAME_PROPERTY,
						BackendPropertyKind::STRING,
						true,
						None,
					),
					BackendPropertyDescriptor::new(
						MUTATION_REPOSITORY_PROPERTY,
						BackendPropertyKind::STRING,
						true,
						None,
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
			HTTP_REQUEST_NODE_TYPE => http_request_contract(),
			CRON_NODE_TYPE => cron_contract(),
			BRANCH_NODE_TYPE => branch_contract(),
			MERGE_NODE_TYPE => merge_contract(),
			LOOP_NODE_TYPE => loop_contract(),
			SUBGRAPH_NODE_TYPE => subgraph_contract(),
			ENTITY_NODE_TYPE => entity_contract(),
			REPOSITORY_NODE_TYPE => repository_contract(),
			QUERY_NODE_TYPE => query_contract(),
			MUTATION_NODE_TYPE => mutation_contract(),
			LOGGING_NODE_TYPE => logging_contract(),
			STORAGE_NODE_TYPE => storage_contract(node),
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
			HTTP_REQUEST_NODE_TYPE,
			PortDirection::OUTPUT,
			"BACKEND_DSL.HTTP_REQUEST_OUTPUT_UNCONNECTED",
			"HTTP Request requires its execution output to be connected.",
		));
		diagnostics.extend(validate_required_execution_ports(
			document,
			LOGGING_NODE_TYPE,
			PortDirection::OUTPUT,
			"BACKEND_DSL.LOGGING_OUTPUT_UNCONNECTED",
			"Logging requires its execution output to be connected.",
		));
		diagnostics.extend(validate_required_execution_ports(
			document,
			STORAGE_NODE_TYPE,
			PortDirection::OUTPUT,
			"BACKEND_DSL.STORAGE_OUTPUT_UNCONNECTED",
			"Storage requires its execution output to be connected.",
		));
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
		diagnostics.extend(validate_required_execution_ports(
			document,
			SUBGRAPH_NODE_TYPE,
			PortDirection::OUTPUT,
			"BACKEND_DSL.SUBGRAPH_OUTPUT_UNCONNECTED",
			"Subgraph requires its execution output to be connected.",
		));
		diagnostics.extend(validate_required_execution_ports(
			document,
			QUERY_NODE_TYPE,
			PortDirection::OUTPUT,
			"BACKEND_DSL.QUERY_OUTPUT_UNCONNECTED",
			"Query requires its execution output to be connected.",
		));
		diagnostics.extend(validate_required_execution_ports(
			document,
			MUTATION_NODE_TYPE,
			PortDirection::OUTPUT,
			"BACKEND_DSL.MUTATION_OUTPUT_UNCONNECTED",
			"Mutation requires its execution output to be connected.",
		));
		diagnostics.extend(validate_subgraph_references(document));
		diagnostics.extend(validate_entity_names(document));
		diagnostics.extend(validate_route_request_entities(document));
		diagnostics.extend(validate_repository_references(document));
		diagnostics.extend(validate_query_references(document));
		diagnostics.extend(validate_mutation_references(document));
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
			HTTP_REQUEST_NODE_TYPE => http_request_properties(node)
				.err()
				.map(|diagnostic| *diagnostic)
				.into_iter()
				.collect(),
			CRON_NODE_TYPE => cron_schedule(node)
				.err()
				.map(|diagnostic| *diagnostic)
				.into_iter()
				.collect(),
			SUBGRAPH_NODE_TYPE => subgraph_reference(node)
				.err()
				.map(|diagnostic| *diagnostic)
				.into_iter()
				.collect(),
			ENTITY_NODE_TYPE => entity_properties(node)
				.err()
				.map(|diagnostic| *diagnostic)
				.into_iter()
				.collect(),
			REPOSITORY_NODE_TYPE => repository_properties(node)
				.err()
				.map(|diagnostic| *diagnostic)
				.into_iter()
				.collect(),
			QUERY_NODE_TYPE => query_properties(node)
				.err()
				.map(|diagnostic| *diagnostic)
				.into_iter()
				.collect(),
			MUTATION_NODE_TYPE => mutation_properties(node)
				.err()
				.map(|diagnostic| *diagnostic)
				.into_iter()
				.collect(),
			LOGGING_NODE_TYPE => logging_level(node)
				.err()
				.map(|diagnostic| *diagnostic)
				.into_iter()
				.collect(),
			STORAGE_NODE_TYPE => storage_properties(node)
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
				let mut lowered = LoweredOperation::new(
					OperationId::new(HTTP_ROUTE_NODE_TYPE).expect("static operation is valid"),
					[],
					[TypeId::new(HTTP_REQUEST_TYPE).expect("static type is valid")],
					AsyncKind::SYNC,
					[],
				)
				.with_attribute("backend.http.method", properties.method)
				.with_attribute("backend.http.path", properties.path);
				if let Some(request_entity) = properties.request_entity {
					lowered = lowered.with_attribute("backend.http.request_entity", request_entity);
				}
				Ok(lowered)
			}
			HTTP_REQUEST_NODE_TYPE => {
				let properties = http_request_properties(node).map_err(|error| vec![*error])?;
				Ok(LoweredOperation::new(
					OperationId::new(HTTP_REQUEST_NODE_TYPE).expect("static operation is valid"),
					[TypeId::new(RESPONSE_BODY_TYPE).expect("static type is valid")],
					[TypeId::new(RESPONSE_BODY_TYPE).expect("static type is valid")],
					AsyncKind::ASYNC,
					[EffectId::new(HTTP_REQUEST_EFFECT).expect("static effect is valid")],
				)
				.with_attribute("backend.effect.kind", "HTTP_REQUEST")
				.with_attribute("backend.http_request.method", properties.method)
				.with_attribute("backend.http_request.url", properties.url))
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
			SUBGRAPH_NODE_TYPE => {
				let subgraph_id = subgraph_reference(node).map_err(|error| vec![*error])?;
				Ok(LoweredOperation::new(
					OperationId::new(SUBGRAPH_NODE_TYPE).expect("static operation is valid"),
					[],
					[],
					AsyncKind::SYNC,
					[],
				)
				.with_kind(LoweredOperationKind::SUBGRAPH)
				.with_attribute("control.kind", "SUBGRAPH")
				.with_attribute("backend.subgraph.id", subgraph_id.as_u64().to_string()))
			}
			ENTITY_NODE_TYPE => {
				let properties = entity_properties(node).map_err(|error| vec![*error])?;
				let mut lowered = LoweredOperation::new(
					OperationId::new(ENTITY_NODE_TYPE).expect("static operation is valid"),
					[],
					[],
					AsyncKind::SYNC,
					[],
				)
				.with_kind(LoweredOperationKind::DECLARATION)
				.with_attribute("backend.entity.name", properties.name);
				if !properties.fields.is_empty() {
					let fields = serde_json::to_string(&properties.fields)
						.expect("validated entity fields should serialize");
					lowered = lowered.with_attribute("backend.entity.fields", fields);
				}
				Ok(lowered)
			}
			REPOSITORY_NODE_TYPE => {
				let properties = repository_properties(node).map_err(|error| vec![*error])?;
				Ok(LoweredOperation::new(
					OperationId::new(REPOSITORY_NODE_TYPE).expect("static operation is valid"),
					[],
					[],
					AsyncKind::SYNC,
					[],
				)
				.with_kind(LoweredOperationKind::DECLARATION)
				.with_attribute("backend.repository.name", properties.name)
				.with_attribute("backend.repository.entity", properties.entity))
			}
			QUERY_NODE_TYPE => {
				let properties = query_properties(node).map_err(|error| vec![*error])?;
				Ok(LoweredOperation::new(
					OperationId::new(QUERY_NODE_TYPE).expect("static operation is valid"),
					[],
					[TypeId::new(RESPONSE_BODY_TYPE).expect("static type is valid")],
					AsyncKind::ASYNC,
					[EffectId::new(DATABASE_READ_EFFECT).expect("static effect is valid")],
				)
				.with_attribute("backend.persistence.kind", "QUERY")
				.with_attribute("backend.query.name", properties.name)
				.with_attribute("backend.query.repository", properties.repository))
			}
			MUTATION_NODE_TYPE => {
				let properties = mutation_properties(node).map_err(|error| vec![*error])?;
				Ok(LoweredOperation::new(
					OperationId::new(MUTATION_NODE_TYPE).expect("static operation is valid"),
					[TypeId::new(RESPONSE_BODY_TYPE).expect("static type is valid")],
					[TypeId::new(RESPONSE_BODY_TYPE).expect("static type is valid")],
					AsyncKind::ASYNC,
					[EffectId::new(DATABASE_WRITE_EFFECT).expect("static effect is valid")],
				)
				.with_attribute("backend.persistence.kind", "MUTATION")
				.with_attribute("backend.mutation.name", properties.name)
				.with_attribute("backend.mutation.repository", properties.repository))
			}
			LOGGING_NODE_TYPE => {
				let level = logging_level(node).map_err(|error| vec![*error])?;
				Ok(LoweredOperation::new(
					OperationId::new(LOGGING_NODE_TYPE).expect("static operation is valid"),
					[TypeId::new(RESPONSE_BODY_TYPE).expect("static type is valid")],
					[],
					AsyncKind::SYNC,
					[EffectId::new(LOGGING_EFFECT).expect("static effect is valid")],
				)
				.with_attribute("backend.effect.kind", "LOGGING")
				.with_attribute("backend.logging.level", level))
			}
			STORAGE_NODE_TYPE => {
				let properties = storage_properties(node).map_err(|error| vec![*error])?;
				Ok(LoweredOperation::new(
					OperationId::new(STORAGE_NODE_TYPE).expect("static operation is valid"),
					[TypeId::new(RESPONSE_BODY_TYPE).expect("static type is valid")],
					[TypeId::new(RESPONSE_BODY_TYPE).expect("static type is valid")],
					AsyncKind::ASYNC,
					[properties.operation.effect()],
				)
				.with_attribute("backend.effect.kind", "STORAGE")
				.with_attribute("backend.storage.operation", properties.operation.as_str())
				.with_attribute("backend.storage.key", properties.key))
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
	request_entity: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct EntityProperties {
	name: String,
	fields: BTreeMap<String, EntityFieldProperties>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct EntityFieldProperties {
	#[serde(rename = "type")]
	type_name: String,
	required: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct HttpRequestProperties {
	method: String,
	url: String,
}

fn http_request_properties(node: &Node) -> Result<HttpRequestProperties, Box<Diagnostic>> {
	let method = enum_property(
		node,
		"HTTP Request",
		HTTP_REQUEST_METHOD_PROPERTY,
		&HTTP_METHODS,
		"BACKEND_DSL.HTTP_REQUEST_METHOD_MISSING",
		"BACKEND_DSL.HTTP_REQUEST_METHOD_TYPE",
		"BACKEND_DSL.HTTP_REQUEST_METHOD_INVALID",
	)?;
	let url = non_empty_string_property(
		node,
		"HTTP Request",
		HTTP_REQUEST_URL_PROPERTY,
		"BACKEND_DSL.HTTP_REQUEST_URL_MISSING",
		"BACKEND_DSL.HTTP_REQUEST_URL_TYPE",
		"BACKEND_DSL.HTTP_REQUEST_URL_INVALID",
	)?;
	Ok(HttpRequestProperties { method, url })
}

fn logging_level(node: &Node) -> Result<String, Box<Diagnostic>> {
	let Some(value) = node.properties().get(LOGGING_LEVEL_PROPERTY) else {
		return Ok("INFO".to_string());
	};
	let Some(level) = value.as_str() else {
		return Err(node_diagnostic(
			node,
			"BACKEND_DSL.LOGGING_LEVEL_TYPE",
			"Logging property 'level' must be one of DEBUG, INFO, WARN, or ERROR.",
		));
	};
	if !LOG_LEVELS.contains(&level) {
		return Err(node_diagnostic(
			node,
			"BACKEND_DSL.LOGGING_LEVEL_INVALID",
			"Logging property 'level' must be one of DEBUG, INFO, WARN, or ERROR.",
		));
	}
	Ok(level.to_string())
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StorageProperties {
	operation: StorageOperation,
	key: String,
}

fn storage_properties(node: &Node) -> Result<StorageProperties, Box<Diagnostic>> {
	let operation = enum_property(
		node,
		"Storage",
		STORAGE_OPERATION_PROPERTY,
		&STORAGE_OPERATIONS,
		"BACKEND_DSL.STORAGE_OPERATION_MISSING",
		"BACKEND_DSL.STORAGE_OPERATION_TYPE",
		"BACKEND_DSL.STORAGE_OPERATION_INVALID",
	)?;
	let key = non_empty_string_property(
		node,
		"Storage",
		STORAGE_KEY_PROPERTY,
		"BACKEND_DSL.STORAGE_KEY_MISSING",
		"BACKEND_DSL.STORAGE_KEY_TYPE",
		"BACKEND_DSL.STORAGE_KEY_INVALID",
	)?;
	let operation = match operation.as_str() {
		"READ" => StorageOperation::READ,
		"WRITE" => StorageOperation::WRITE,
		_ => unreachable!("enum_property validates storage operations"),
	};
	Ok(StorageProperties { operation, key })
}

fn enum_property(
	node: &Node,
	owner: &str,
	key: &str,
	values: &[&str],
	missing_code: &str,
	type_code: &str,
	invalid_code: &str,
) -> Result<String, Box<Diagnostic>> {
	let Some(value) = node.properties().get(key) else {
		return Err(node_diagnostic(
			node,
			missing_code,
			format!("{owner} property '{key}' is required."),
		));
	};
	let Some(value) = value.as_str() else {
		return Err(node_diagnostic(
			node,
			type_code,
			format!(
				"{owner} property '{key}' must be one of {}.",
				values.join(", ")
			),
		));
	};
	if !values.contains(&value) {
		return Err(node_diagnostic(
			node,
			invalid_code,
			format!(
				"{owner} property '{key}' must be one of {}.",
				values.join(", ")
			),
		));
	}
	Ok(value.to_string())
}

fn non_empty_string_property(
	node: &Node,
	owner: &str,
	key: &str,
	missing_code: &str,
	type_code: &str,
	invalid_code: &str,
) -> Result<String, Box<Diagnostic>> {
	let Some(value) = node.properties().get(key) else {
		return Err(node_diagnostic(
			node,
			missing_code,
			format!("{owner} property '{key}' is required."),
		));
	};
	let Some(value) = value.as_str() else {
		return Err(node_diagnostic(
			node,
			type_code,
			format!("{owner} property '{key}' must be a non-empty string."),
		));
	};
	if value.trim().is_empty() || value.chars().any(char::is_whitespace) {
		return Err(node_diagnostic(
			node,
			invalid_code,
			format!("{owner} property '{key}' must be a non-empty string without whitespace."),
		));
	}
	Ok(value.to_string())
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

fn http_request_contract() -> NodeContract {
	NodeContract::new(
		[
			PortContract::new(PortKind::EXECUTION, None, true),
			PortContract::new(
				PortKind::DATA,
				Some(TypeId::new(RESPONSE_BODY_TYPE).expect("static type is valid")),
				false,
			),
		],
		[
			PortContract::new(PortKind::EXECUTION, None, true),
			PortContract::new(
				PortKind::DATA,
				Some(TypeId::new(RESPONSE_BODY_TYPE).expect("static type is valid")),
				true,
			),
		],
		[],
		[],
		[],
		AsyncKind::ASYNC,
		[EffectId::new(HTTP_REQUEST_EFFECT).expect("static effect is valid")],
	)
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

fn subgraph_contract() -> NodeContract {
	NodeContract::new(
		[PortContract::new(PortKind::EXECUTION, None, true)],
		[PortContract::new(PortKind::EXECUTION, None, true)],
		[],
		[],
		[],
		AsyncKind::SYNC,
		[],
	)
}

fn entity_contract() -> NodeContract {
	NodeContract::new([], [], [], [], [], AsyncKind::SYNC, [])
}

fn repository_contract() -> NodeContract {
	NodeContract::new([], [], [], [], [], AsyncKind::SYNC, [])
}

fn query_contract() -> NodeContract {
	NodeContract::new(
		[PortContract::new(PortKind::EXECUTION, None, true)],
		[
			PortContract::new(PortKind::EXECUTION, None, true),
			PortContract::new(
				PortKind::DATA,
				Some(TypeId::new(RESPONSE_BODY_TYPE).expect("static type is valid")),
				true,
			),
		],
		[],
		[],
		[],
		AsyncKind::ASYNC,
		[EffectId::new(DATABASE_READ_EFFECT).expect("static effect is valid")],
	)
}

fn mutation_contract() -> NodeContract {
	NodeContract::new(
		[
			PortContract::new(PortKind::EXECUTION, None, true),
			PortContract::new(
				PortKind::DATA,
				Some(TypeId::new(RESPONSE_BODY_TYPE).expect("static type is valid")),
				true,
			),
		],
		[
			PortContract::new(PortKind::EXECUTION, None, true),
			PortContract::new(
				PortKind::DATA,
				Some(TypeId::new(RESPONSE_BODY_TYPE).expect("static type is valid")),
				true,
			),
		],
		[],
		[],
		[],
		AsyncKind::ASYNC,
		[EffectId::new(DATABASE_WRITE_EFFECT).expect("static effect is valid")],
	)
}

fn logging_contract() -> NodeContract {
	NodeContract::new(
		[
			PortContract::new(PortKind::EXECUTION, None, true),
			PortContract::new(
				PortKind::DATA,
				Some(TypeId::new(RESPONSE_BODY_TYPE).expect("static type is valid")),
				true,
			),
		],
		[PortContract::new(PortKind::EXECUTION, None, true)],
		[],
		[],
		[],
		AsyncKind::SYNC,
		[EffectId::new(LOGGING_EFFECT).expect("static effect is valid")],
	)
}

fn storage_contract(node: &Node) -> NodeContract {
	let operation = storage_properties(node)
		.map(|properties| properties.operation)
		.unwrap_or(StorageOperation::READ);
	NodeContract::new(
		[
			PortContract::new(PortKind::EXECUTION, None, true),
			PortContract::new(
				PortKind::DATA,
				Some(TypeId::new(RESPONSE_BODY_TYPE).expect("static type is valid")),
				operation == StorageOperation::WRITE,
			),
		],
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
		AsyncKind::ASYNC,
		[operation.effect()],
	)
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

fn validate_subgraph_references(document: &GraphDocument) -> Vec<Diagnostic> {
	let subgraphs: BTreeMap<SubgraphId, _> = document
		.subgraphs()
		.iter()
		.map(|subgraph| (subgraph.id(), subgraph))
		.collect();
	let mut dependencies = BTreeMap::<SubgraphId, BTreeSet<SubgraphId>>::new();
	let mut invocations = Vec::new();
	let mut diagnostics = Vec::new();

	for node in document
		.nodes()
		.iter()
		.filter(|node| node.type_id().as_str() == SUBGRAPH_NODE_TYPE)
	{
		let Ok(target) = subgraph_reference(node) else {
			continue;
		};
		let Some(target_subgraph) = subgraphs.get(&target) else {
			diagnostics.push(*node_diagnostic(
				node,
				"BACKEND_DSL.SUBGRAPH_NOT_FOUND",
				format!(
					"Subgraph {target_id} does not exist.",
					target_id = target.as_u64()
				),
			));
			continue;
		};
		if target_subgraph.ports().is_empty() {
			diagnostics.push(*node_diagnostic(
				node,
				"BACKEND_DSL.SUBGRAPH_BOUNDARY_MISSING",
				format!(
					"Subgraph {target_id} must declare at least one boundary port.",
					target_id = target.as_u64()
				),
			));
		}

		for source_subgraph in document
			.subgraphs()
			.iter()
			.filter(|subgraph| subgraph.nodes().contains(&node.id()))
		{
			dependencies
				.entry(source_subgraph.id())
				.or_default()
				.insert(target);
			invocations.push((node.id(), source_subgraph.id(), target));
		}
	}

	for (node_id, source, target) in invocations {
		if subgraph_reaches(target, source, &dependencies) {
			diagnostics.push(
				Diagnostic::new(
					DiagnosticCode::new("BACKEND_DSL.SUBGRAPH_RECURSION")
						.expect("static diagnostic code is valid"),
					Severity::ERROR,
					CompilerStage::SEMANTIC_ANALYSIS,
					"Recursive subgraph invocation is not supported.",
					GraphElementId::NODE(node_id),
				)
				.expect("static Backend DSL diagnostic is valid"),
			);
		}
	}

	diagnostics
}

fn subgraph_reaches(
	start: SubgraphId,
	goal: SubgraphId,
	dependencies: &BTreeMap<SubgraphId, BTreeSet<SubgraphId>>,
) -> bool {
	let mut visited = BTreeSet::new();
	let mut worklist = VecDeque::from([start]);
	while let Some(current) = worklist.pop_front() {
		if !visited.insert(current) {
			continue;
		}
		if current == goal {
			return true;
		}
		if let Some(next) = dependencies.get(&current) {
			worklist.extend(next.iter().copied());
		}
	}
	false
}

fn validate_entity_names(document: &GraphDocument) -> Vec<Diagnostic> {
	let mut names = BTreeMap::<String, NodeId>::new();
	let mut diagnostics = Vec::new();
	for node in document
		.nodes()
		.iter()
		.filter(|node| node.type_id().as_str() == ENTITY_NODE_TYPE)
	{
		let Ok(name) = entity_name(node) else {
			continue;
		};
		if names.insert(name.clone(), node.id()).is_some() {
			diagnostics.push(*node_diagnostic(
				node,
				"BACKEND_DSL.ENTITY_NAME_DUPLICATE",
				format!("Entity name '{name}' is already declared."),
			));
		}
	}
	diagnostics
}

fn validate_route_request_entities(document: &GraphDocument) -> Vec<Diagnostic> {
	let entity_names: BTreeSet<_> = document
		.nodes()
		.iter()
		.filter(|node| node.type_id().as_str() == ENTITY_NODE_TYPE)
		.filter_map(|node| entity_name(node).ok())
		.collect();

	document
		.nodes()
		.iter()
		.filter(|node| node.type_id().as_str() == HTTP_ROUTE_NODE_TYPE)
		.filter_map(|route| {
			let request_entity = route_properties(route).ok()?.request_entity?;
			if entity_names.contains(&request_entity) {
				return None;
			}
			Some(*node_diagnostic(
				route,
				"BACKEND_DSL.HTTP_ROUTE_REQUEST_ENTITY_NOT_FOUND",
				format!(
					"HTTP Route references request entity '{request_entity}', but that entity is not declared."
				),
			))
		})
		.collect()
}

fn validate_repository_references(document: &GraphDocument) -> Vec<Diagnostic> {
	let entity_names: BTreeSet<_> = document
		.nodes()
		.iter()
		.filter(|node| node.type_id().as_str() == ENTITY_NODE_TYPE)
		.filter_map(|node| entity_name(node).ok())
		.collect();
	let mut repository_names = BTreeMap::<String, NodeId>::new();
	let mut diagnostics = Vec::new();

	for node in document
		.nodes()
		.iter()
		.filter(|node| node.type_id().as_str() == REPOSITORY_NODE_TYPE)
	{
		let Ok(properties) = repository_properties(node) else {
			continue;
		};
		if !entity_names.contains(&properties.entity) {
			diagnostics.push(*node_diagnostic(
				node,
				"BACKEND_DSL.REPOSITORY_ENTITY_NOT_FOUND",
				format!(
					"Repository references entity '{}', but that entity is not declared.",
					properties.entity
				),
			));
		}
		if repository_names
			.insert(properties.name.clone(), node.id())
			.is_some()
		{
			diagnostics.push(*node_diagnostic(
				node,
				"BACKEND_DSL.REPOSITORY_NAME_DUPLICATE",
				format!("Repository name '{}' is already declared.", properties.name),
			));
		}
	}

	diagnostics
}

fn validate_query_references(document: &GraphDocument) -> Vec<Diagnostic> {
	let repository_names: BTreeSet<_> = document
		.nodes()
		.iter()
		.filter(|node| node.type_id().as_str() == REPOSITORY_NODE_TYPE)
		.filter_map(|node| {
			repository_properties(node)
				.ok()
				.map(|properties| properties.name)
		})
		.collect();
	let mut query_names = BTreeMap::<String, NodeId>::new();
	let mut diagnostics = Vec::new();

	for node in document
		.nodes()
		.iter()
		.filter(|node| node.type_id().as_str() == QUERY_NODE_TYPE)
	{
		let Ok(properties) = query_properties(node) else {
			continue;
		};
		if !repository_names.contains(&properties.repository) {
			diagnostics.push(*node_diagnostic(
				node,
				"BACKEND_DSL.QUERY_REPOSITORY_NOT_FOUND",
				format!(
					"Query references repository '{}', but that repository is not declared.",
					properties.repository
				),
			));
		}
		if query_names
			.insert(properties.name.clone(), node.id())
			.is_some()
		{
			diagnostics.push(*node_diagnostic(
				node,
				"BACKEND_DSL.QUERY_NAME_DUPLICATE",
				format!("Query name '{}' is already declared.", properties.name),
			));
		}
	}

	diagnostics
}

fn validate_mutation_references(document: &GraphDocument) -> Vec<Diagnostic> {
	let repository_names: BTreeSet<_> = document
		.nodes()
		.iter()
		.filter(|node| node.type_id().as_str() == REPOSITORY_NODE_TYPE)
		.filter_map(|node| {
			repository_properties(node)
				.ok()
				.map(|properties| properties.name)
		})
		.collect();
	let mut mutation_names = BTreeMap::<String, NodeId>::new();
	let mut diagnostics = Vec::new();

	for node in document
		.nodes()
		.iter()
		.filter(|node| node.type_id().as_str() == MUTATION_NODE_TYPE)
	{
		let Ok(properties) = mutation_properties(node) else {
			continue;
		};
		if !repository_names.contains(&properties.repository) {
			diagnostics.push(*node_diagnostic(
				node,
				"BACKEND_DSL.MUTATION_REPOSITORY_NOT_FOUND",
				format!(
					"Mutation references repository '{}', but that repository is not declared.",
					properties.repository
				),
			));
		}
		if mutation_names
			.insert(properties.name.clone(), node.id())
			.is_some()
		{
			diagnostics.push(*node_diagnostic(
				node,
				"BACKEND_DSL.MUTATION_NAME_DUPLICATE",
				format!("Mutation name '{}' is already declared.", properties.name),
			));
		}
	}

	diagnostics
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

fn subgraph_reference(node: &Node) -> Result<SubgraphId, Box<Diagnostic>> {
	let Some(value) = node.properties().get(SUBGRAPH_ID_PROPERTY) else {
		return Err(node_diagnostic(
			node,
			"BACKEND_DSL.SUBGRAPH_ID_MISSING",
			"Subgraph property 'subgraph_id' is required.",
		));
	};
	let Some(value) = value.as_u64() else {
		return Err(node_diagnostic(
			node,
			"BACKEND_DSL.SUBGRAPH_ID_TYPE",
			"Subgraph property 'subgraph_id' must be a positive integer.",
		));
	};
	SubgraphId::new(value).map_err(|_| {
		node_diagnostic(
			node,
			"BACKEND_DSL.SUBGRAPH_ID_INVALID",
			"Subgraph property 'subgraph_id' must be a positive integer.",
		)
	})
}

fn entity_name(node: &Node) -> Result<String, Box<Diagnostic>> {
	let Some(value) = node.properties().get(ENTITY_NAME_PROPERTY) else {
		return Err(node_diagnostic(
			node,
			"BACKEND_DSL.ENTITY_NAME_MISSING",
			"Entity property 'name' is required.",
		));
	};
	let Some(name) = value.as_str() else {
		return Err(node_diagnostic(
			node,
			"BACKEND_DSL.ENTITY_NAME_TYPE",
			"Entity property 'name' must be a non-empty string.",
		));
	};
	if name.trim().is_empty() || name.chars().any(char::is_whitespace) {
		return Err(node_diagnostic(
			node,
			"BACKEND_DSL.ENTITY_NAME_INVALID",
			"Entity property 'name' must be a non-empty string without whitespace.",
		));
	}
	Ok(name.to_string())
}

fn entity_properties(node: &Node) -> Result<EntityProperties, Box<Diagnostic>> {
	let name = entity_name(node)?;
	let fields = match node.properties().get(ENTITY_FIELDS_PROPERTY) {
		None => BTreeMap::new(),
		Some(Value::Object(fields)) => {
			let mut validated = BTreeMap::new();
			for (field_name, field_schema) in fields {
				if !is_identifier(field_name) {
					return Err(node_diagnostic(
						node,
						"BACKEND_DSL.ENTITY_FIELD_NAME_INVALID",
						format!("Entity field name '{field_name}' must be a valid identifier."),
					));
				}
				let (field_type, required) = match field_schema {
					Value::String(field_type) => (field_type.clone(), true),
					Value::Object(schema) => {
						let Some(field_type) = schema.get("type").and_then(Value::as_str) else {
							return Err(node_diagnostic(
								node,
								"BACKEND_DSL.ENTITY_FIELD_TYPE",
								format!(
									"Entity field '{field_name}' schema must contain a string 'type'."
								),
							));
						};
						let required = match schema.get("required") {
							None => true,
							Some(Value::Bool(required)) => *required,
							Some(_) => {
								return Err(node_diagnostic(
									node,
									"BACKEND_DSL.ENTITY_FIELD_REQUIRED_TYPE",
									format!(
										"Entity field '{field_name}' property 'required' must be a boolean."
									),
								));
							}
						};
						(field_type.to_string(), required)
					}
					_ => {
						return Err(node_diagnostic(
							node,
							"BACKEND_DSL.ENTITY_FIELD_TYPE",
							format!(
								"Entity field '{field_name}' must be a type string or a schema object."
							),
						));
					}
				};
				let parsed_type = Type::from_reference(field_type.as_str()).map_err(|_| {
					node_diagnostic(
						node,
						"BACKEND_DSL.ENTITY_FIELD_TYPE_INVALID",
						format!("Entity field '{field_name}' has an invalid type reference."),
					)
				})?;
				if !parsed_type.is_scalar() && parsed_type != Type::ANY {
					return Err(node_diagnostic(
						node,
						"BACKEND_DSL.ENTITY_FIELD_TYPE_UNSUPPORTED",
						format!(
							"Entity field '{field_name}' must use String, Boolean, Integer, Float, Number, Null, or Any."
						),
					));
				}
				validated.insert(
					field_name.clone(),
					EntityFieldProperties {
						type_name: field_type,
						required,
					},
				);
			}
			validated
		}
		Some(_) => {
			return Err(node_diagnostic(
				node,
				"BACKEND_DSL.ENTITY_FIELDS_TYPE",
				"Entity property 'fields' must be an object mapping field names to type references.",
			));
		}
	};
	Ok(EntityProperties { name, fields })
}

fn is_identifier(value: &str) -> bool {
	let mut characters = value.chars();
	let Some(first) = characters.next() else {
		return false;
	};
	(first.is_ascii_alphabetic() || first == '_' || first == '$')
		&& characters.all(|character| {
			character.is_ascii_alphanumeric() || character == '_' || character == '$'
		})
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RepositoryProperties {
	name: String,
	entity: String,
}

fn repository_properties(node: &Node) -> Result<RepositoryProperties, Box<Diagnostic>> {
	let name = identifier_property(
		node,
		"Repository",
		REPOSITORY_NAME_PROPERTY,
		"name",
		"BACKEND_DSL.REPOSITORY_NAME_MISSING",
		"BACKEND_DSL.REPOSITORY_NAME_TYPE",
		"BACKEND_DSL.REPOSITORY_NAME_INVALID",
	)?;
	let entity = identifier_property(
		node,
		"Repository",
		REPOSITORY_ENTITY_PROPERTY,
		"entity",
		"BACKEND_DSL.REPOSITORY_ENTITY_MISSING",
		"BACKEND_DSL.REPOSITORY_ENTITY_TYPE",
		"BACKEND_DSL.REPOSITORY_ENTITY_INVALID",
	)?;
	Ok(RepositoryProperties { name, entity })
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct QueryProperties {
	name: String,
	repository: String,
}

fn query_properties(node: &Node) -> Result<QueryProperties, Box<Diagnostic>> {
	let name = identifier_property(
		node,
		"Query",
		QUERY_NAME_PROPERTY,
		"name",
		"BACKEND_DSL.QUERY_NAME_MISSING",
		"BACKEND_DSL.QUERY_NAME_TYPE",
		"BACKEND_DSL.QUERY_NAME_INVALID",
	)?;
	let repository = identifier_property(
		node,
		"Query",
		QUERY_REPOSITORY_PROPERTY,
		"repository",
		"BACKEND_DSL.QUERY_REPOSITORY_MISSING",
		"BACKEND_DSL.QUERY_REPOSITORY_TYPE",
		"BACKEND_DSL.QUERY_REPOSITORY_INVALID",
	)?;
	Ok(QueryProperties { name, repository })
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct MutationProperties {
	name: String,
	repository: String,
}

fn mutation_properties(node: &Node) -> Result<MutationProperties, Box<Diagnostic>> {
	let name = identifier_property(
		node,
		"Mutation",
		MUTATION_NAME_PROPERTY,
		"name",
		"BACKEND_DSL.MUTATION_NAME_MISSING",
		"BACKEND_DSL.MUTATION_NAME_TYPE",
		"BACKEND_DSL.MUTATION_NAME_INVALID",
	)?;
	let repository = identifier_property(
		node,
		"Mutation",
		MUTATION_REPOSITORY_PROPERTY,
		"repository",
		"BACKEND_DSL.MUTATION_REPOSITORY_MISSING",
		"BACKEND_DSL.MUTATION_REPOSITORY_TYPE",
		"BACKEND_DSL.MUTATION_REPOSITORY_INVALID",
	)?;
	Ok(MutationProperties { name, repository })
}

fn identifier_property(
	node: &Node,
	owner: &str,
	key: &str,
	label: &str,
	missing_code: &str,
	type_code: &str,
	invalid_code: &str,
) -> Result<String, Box<Diagnostic>> {
	let Some(value) = node.properties().get(key) else {
		return Err(node_diagnostic(
			node,
			missing_code,
			format!("{owner} property '{label}' is required."),
		));
	};
	let Some(value) = value.as_str() else {
		return Err(node_diagnostic(
			node,
			type_code,
			format!("{owner} property '{label}' must be a non-empty string."),
		));
	};
	if value.trim().is_empty() || value.chars().any(char::is_whitespace) {
		return Err(node_diagnostic(
			node,
			invalid_code,
			format!("{owner} property '{label}' must be a non-empty string without whitespace."),
		));
	}
	Ok(value.to_string())
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

	let request_entity = optional_identifier_property(
		node,
		"HTTP Route",
		HTTP_ROUTE_REQUEST_ENTITY_PROPERTY,
		"BACKEND_DSL.HTTP_ROUTE_REQUEST_ENTITY_TYPE",
		"BACKEND_DSL.HTTP_ROUTE_REQUEST_ENTITY_INVALID",
	)?;

	Ok(RouteProperties {
		method,
		path,
		request_entity,
	})
}

fn optional_identifier_property(
	node: &Node,
	owner: &str,
	key: &str,
	type_code: &str,
	invalid_code: &str,
) -> Result<Option<String>, Box<Diagnostic>> {
	let Some(value) = node.properties().get(key) else {
		return Ok(None);
	};
	let Some(value) = value.as_str() else {
		return Err(node_diagnostic(
			node,
			type_code,
			format!("{owner} property '{key}' must be a non-empty string."),
		));
	};
	if value.trim().is_empty() || value.chars().any(char::is_whitespace) {
		return Err(node_diagnostic(
			node,
			invalid_code,
			format!("{owner} property '{key}' must be a non-empty string without whitespace."),
		));
	}
	Ok(Some(value.to_string()))
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
		assert_eq!(plugin.descriptors().len(), 18);
		assert_eq!(plugin.descriptors()[0].label(), "HTTP Route");
		assert_eq!(plugin.descriptors()[0].properties().len(), 3);
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
		let http_request = plugin
			.descriptors()
			.iter()
			.find(|descriptor| descriptor.type_id().as_str() == HTTP_REQUEST_NODE_TYPE)
			.expect("HTTP Request descriptor should be registered");
		assert_eq!(http_request.label(), "HTTP Request");
		assert_eq!(http_request.category(), "Effects");
		assert_eq!(http_request.inputs().len(), 2);
		assert!(!http_request.inputs()[1].contract().required());
		assert_eq!(http_request.outputs().len(), 2);
		assert_eq!(http_request.properties().len(), 2);
		let logging = plugin
			.descriptors()
			.iter()
			.find(|descriptor| descriptor.type_id().as_str() == LOGGING_NODE_TYPE)
			.expect("Logging descriptor should be registered");
		assert_eq!(logging.label(), "Logging");
		assert_eq!(logging.category(), "Effects");
		assert_eq!(logging.inputs().len(), 2);
		assert_eq!(logging.outputs().len(), 1);
		assert_eq!(logging.properties().len(), 1);
		assert_eq!(logging.properties()[0].key(), LOGGING_LEVEL_PROPERTY);
		let storage = plugin
			.descriptors()
			.iter()
			.find(|descriptor| descriptor.type_id().as_str() == STORAGE_NODE_TYPE)
			.expect("Storage descriptor should be registered");
		assert_eq!(storage.label(), "Storage");
		assert_eq!(storage.category(), "Effects");
		assert_eq!(storage.inputs().len(), 2);
		assert!(!storage.inputs()[1].contract().required());
		assert_eq!(storage.outputs().len(), 2);
		assert_eq!(storage.properties().len(), 2);
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
	fn exposes_and_lowers_subgraph_invocations() {
		let plugin = BackendDslPlugin::new();
		let descriptor = plugin
			.descriptors()
			.iter()
			.find(|descriptor| descriptor.type_id().as_str() == SUBGRAPH_NODE_TYPE)
			.expect("subgraph descriptor should be registered");
		assert_eq!(descriptor.label(), "Subgraph");
		assert_eq!(descriptor.category(), "Flow");
		assert_eq!(descriptor.inputs().len(), 1);
		assert_eq!(descriptor.outputs().len(), 1);
		assert_eq!(descriptor.properties().len(), 1);
		assert_eq!(descriptor.properties()[0].key(), SUBGRAPH_ID_PROPERTY);
		assert_eq!(
			descriptor.properties()[0].kind(),
			&BackendPropertyKind::INTEGER
		);
		assert!(descriptor.properties()[0].required());

		let node = Node::new(
			NodeId::new(2).unwrap(),
			NodeTypeId::new(SUBGRAPH_NODE_TYPE).unwrap(),
		)
		.with_property(
			SUBGRAPH_ID_PROPERTY,
			Value::Number(serde_json::Number::from(40)),
		)
		.unwrap();
		let lowered = plugin.lower_node(&node).unwrap();
		assert_eq!(lowered.kind(), LoweredOperationKind::SUBGRAPH);
		assert_eq!(lowered.attributes()["control.kind"], "SUBGRAPH");
		assert_eq!(lowered.attributes()["backend.subgraph.id"], "40");
	}

	#[test]
	fn rejects_invalid_subgraph_references() {
		let plugin = BackendDslPlugin::new();
		let missing = Node::new(
			NodeId::new(2).unwrap(),
			NodeTypeId::new(SUBGRAPH_NODE_TYPE).unwrap(),
		);
		let wrong_type = missing
			.clone()
			.with_property(SUBGRAPH_ID_PROPERTY, Value::String("40".to_string()))
			.unwrap();
		let zero = missing
			.clone()
			.with_property(
				SUBGRAPH_ID_PROPERTY,
				Value::Number(serde_json::Number::from(0)),
			)
			.unwrap();

		assert_eq!(
			plugin.analyze_node(&missing)[0].code().as_str(),
			"BACKEND_DSL.SUBGRAPH_ID_MISSING"
		);
		assert_eq!(
			plugin.analyze_node(&wrong_type)[0].code().as_str(),
			"BACKEND_DSL.SUBGRAPH_ID_TYPE"
		);
		assert_eq!(
			plugin.analyze_node(&zero)[0].code().as_str(),
			"BACKEND_DSL.SUBGRAPH_ID_INVALID"
		);
	}

	#[test]
	fn rejects_unknown_and_recursive_subgraph_references() {
		let metadata = krunt_graph_model::GraphMetadata::new(
			krunt_graph_model::GraphId::new(20).unwrap(),
			"backend-v1",
			"krunt-test",
		)
		.unwrap();
		let mut unknown = Node::new(
			NodeId::new(2).unwrap(),
			NodeTypeId::new(SUBGRAPH_NODE_TYPE).unwrap(),
		)
		.with_property(
			SUBGRAPH_ID_PROPERTY,
			Value::Number(serde_json::Number::from(999)),
		)
		.unwrap();
		unknown
			.add_port(krunt_graph_model::Port::new(
				PortId::new(21).unwrap(),
				PortDirection::OUTPUT,
				PortKind::EXECUTION,
			))
			.unwrap();
		let mut builder = krunt_graph_model::GraphBuilder::new(metadata.clone());
		builder.add_node(unknown).unwrap();
		let diagnostics = BackendDslPlugin::new().analyze_graph(&builder.build().unwrap());
		assert!(diagnostics
			.iter()
			.any(|diagnostic| diagnostic.code().as_str() == "BACKEND_DSL.SUBGRAPH_NOT_FOUND"));

		let mut recursive = Node::new(
			NodeId::new(3).unwrap(),
			NodeTypeId::new(SUBGRAPH_NODE_TYPE).unwrap(),
		)
		.with_property(
			SUBGRAPH_ID_PROPERTY,
			Value::Number(serde_json::Number::from(40)),
		)
		.unwrap();
		recursive
			.add_port(krunt_graph_model::Port::new(
				PortId::new(31).unwrap(),
				PortDirection::OUTPUT,
				PortKind::EXECUTION,
			))
			.unwrap();
		let mut recursive_builder = krunt_graph_model::GraphBuilder::new(metadata);
		recursive_builder.add_node(recursive).unwrap();
		recursive_builder
			.add_subgraph(
				krunt_graph_model::Subgraph::new(
					SubgraphId::new(40).unwrap(),
					[NodeId::new(3).unwrap()],
					[krunt_graph_model::SubgraphPort::new(
						PortId::new(41).unwrap(),
						PortDirection::INPUT,
						PortKind::EXECUTION,
					)],
				)
				.unwrap(),
			)
			.unwrap();
		let diagnostics =
			BackendDslPlugin::new().analyze_graph(&recursive_builder.build().unwrap());
		assert!(diagnostics
			.iter()
			.any(|diagnostic| diagnostic.code().as_str() == "BACKEND_DSL.SUBGRAPH_RECURSION"));
	}

	#[test]
	fn exposes_and_lowers_entities_as_named_declarations() {
		let plugin = BackendDslPlugin::new();
		let descriptor = plugin
			.descriptors()
			.iter()
			.find(|descriptor| descriptor.type_id().as_str() == ENTITY_NODE_TYPE)
			.expect("entity descriptor should be registered");
		assert_eq!(descriptor.label(), "Entity");
		assert_eq!(descriptor.category(), "Persistence");
		assert!(descriptor.inputs().is_empty());
		assert!(descriptor.outputs().is_empty());
		assert_eq!(descriptor.properties().len(), 2);
		assert_eq!(descriptor.properties()[0].key(), ENTITY_NAME_PROPERTY);
		assert_eq!(
			descriptor.properties()[0].kind(),
			&BackendPropertyKind::STRING
		);
		assert!(descriptor.properties()[0].required());
		assert_eq!(descriptor.properties()[1].key(), ENTITY_FIELDS_PROPERTY);
		assert_eq!(
			descriptor.properties()[1].kind(),
			&BackendPropertyKind::OBJECT
		);
		assert!(!descriptor.properties()[1].required());

		let node = Node::new(
			NodeId::new(2).unwrap(),
			NodeTypeId::new(ENTITY_NODE_TYPE).unwrap(),
		)
		.with_property(ENTITY_NAME_PROPERTY, Value::String("User".to_string()))
		.unwrap();
		let lowered = plugin.lower_node(&node).unwrap();
		assert_eq!(lowered.operation().as_str(), ENTITY_NODE_TYPE);
		assert_eq!(lowered.kind(), LoweredOperationKind::DECLARATION);
		assert_eq!(lowered.attributes()["backend.entity.name"], "User");
		assert!(!lowered.attributes().contains_key("backend.entity.fields"));
		assert!(lowered.input_types().is_empty());
		assert!(lowered.output_types().is_empty());
	}

	#[test]
	fn lowers_entity_fields_as_deterministic_scalar_metadata() {
		let plugin = BackendDslPlugin::new();
		let node = Node::new(
			NodeId::new(2).unwrap(),
			NodeTypeId::new(ENTITY_NODE_TYPE).unwrap(),
		)
		.with_property(ENTITY_NAME_PROPERTY, Value::String("User".to_string()))
		.unwrap()
		.with_property(
			ENTITY_FIELDS_PROPERTY,
			serde_json::json!({
				"email": {"type": "String", "required": false},
				"id": {"type": "Integer", "required": true}
			}),
		)
		.unwrap();
		let lowered = plugin.lower_node(&node).unwrap();

		assert_eq!(
			lowered.attributes()["backend.entity.fields"],
			"{\"email\":{\"type\":\"String\",\"required\":false},\"id\":{\"type\":\"Integer\",\"required\":true}}"
		);
	}

	#[test]
	fn rejects_invalid_entity_field_schemas() {
		let plugin = BackendDslPlugin::new();
		let base = Node::new(
			NodeId::new(2).unwrap(),
			NodeTypeId::new(ENTITY_NODE_TYPE).unwrap(),
		)
		.with_property(ENTITY_NAME_PROPERTY, Value::String("User".to_string()))
		.unwrap();
		let wrong_shape = base
			.clone()
			.with_property(ENTITY_FIELDS_PROPERTY, Value::String("id".to_string()))
			.unwrap();
		let wrong_type = base
			.clone()
			.with_property(ENTITY_FIELDS_PROPERTY, serde_json::json!({"id": 1}))
			.unwrap();
		let wrong_required = base
			.clone()
			.with_property(
				ENTITY_FIELDS_PROPERTY,
				serde_json::json!({"id": {"type": "Integer", "required": "yes"}}),
			)
			.unwrap();
		let unsupported = base
			.with_property(
				ENTITY_FIELDS_PROPERTY,
				serde_json::json!({"profile": "UserProfile"}),
			)
			.unwrap();

		assert_eq!(
			plugin.analyze_node(&wrong_shape)[0].code().as_str(),
			"BACKEND_DSL.ENTITY_FIELDS_TYPE"
		);
		assert_eq!(
			plugin.analyze_node(&wrong_type)[0].code().as_str(),
			"BACKEND_DSL.ENTITY_FIELD_TYPE"
		);
		assert_eq!(
			plugin.analyze_node(&wrong_required)[0].code().as_str(),
			"BACKEND_DSL.ENTITY_FIELD_REQUIRED_TYPE"
		);
		assert_eq!(
			plugin.analyze_node(&unsupported)[0].code().as_str(),
			"BACKEND_DSL.ENTITY_FIELD_TYPE_UNSUPPORTED"
		);
	}

	#[test]
	fn rejects_invalid_and_duplicate_entity_names() {
		let plugin = BackendDslPlugin::new();
		let missing = Node::new(
			NodeId::new(2).unwrap(),
			NodeTypeId::new(ENTITY_NODE_TYPE).unwrap(),
		);
		let wrong_type = missing
			.clone()
			.with_property(
				ENTITY_NAME_PROPERTY,
				Value::Number(serde_json::Number::from(1)),
			)
			.unwrap();
		let invalid = missing
			.clone()
			.with_property(ENTITY_NAME_PROPERTY, Value::String("User Name".to_string()))
			.unwrap();
		assert_eq!(
			plugin.analyze_node(&missing)[0].code().as_str(),
			"BACKEND_DSL.ENTITY_NAME_MISSING"
		);
		assert_eq!(
			plugin.analyze_node(&wrong_type)[0].code().as_str(),
			"BACKEND_DSL.ENTITY_NAME_TYPE"
		);
		assert_eq!(
			plugin.analyze_node(&invalid)[0].code().as_str(),
			"BACKEND_DSL.ENTITY_NAME_INVALID"
		);

		let metadata = krunt_graph_model::GraphMetadata::new(
			krunt_graph_model::GraphId::new(30).unwrap(),
			"backend-v1",
			"krunt-test",
		)
		.unwrap();
		let duplicate = Node::new(
			NodeId::new(3).unwrap(),
			NodeTypeId::new(ENTITY_NODE_TYPE).unwrap(),
		)
		.with_property(ENTITY_NAME_PROPERTY, Value::String("User".to_string()))
		.unwrap();
		let original = Node::new(
			NodeId::new(4).unwrap(),
			NodeTypeId::new(ENTITY_NODE_TYPE).unwrap(),
		)
		.with_property(ENTITY_NAME_PROPERTY, Value::String("User".to_string()))
		.unwrap();
		let mut builder = krunt_graph_model::GraphBuilder::new(metadata);
		builder.add_node(duplicate).unwrap();
		builder.add_node(original).unwrap();
		let diagnostics = BackendDslPlugin::new().analyze_graph(&builder.build().unwrap());
		assert!(diagnostics
			.iter()
			.any(|diagnostic| diagnostic.code().as_str() == "BACKEND_DSL.ENTITY_NAME_DUPLICATE"));
	}

	#[test]
	fn exposes_and_lowers_repositories_bound_to_entities() {
		let plugin = BackendDslPlugin::new();
		let descriptor = plugin
			.descriptors()
			.iter()
			.find(|descriptor| descriptor.type_id().as_str() == REPOSITORY_NODE_TYPE)
			.expect("repository descriptor should be registered");
		assert_eq!(descriptor.label(), "Repository");
		assert_eq!(descriptor.category(), "Persistence");
		assert!(descriptor.inputs().is_empty());
		assert!(descriptor.outputs().is_empty());
		assert_eq!(descriptor.properties().len(), 2);
		assert_eq!(descriptor.properties()[0].key(), REPOSITORY_NAME_PROPERTY);
		assert_eq!(descriptor.properties()[1].key(), REPOSITORY_ENTITY_PROPERTY);
		assert!(descriptor
			.properties()
			.iter()
			.all(|property| property.required()));

		let node = Node::new(
			NodeId::new(2).unwrap(),
			NodeTypeId::new(REPOSITORY_NODE_TYPE).unwrap(),
		)
		.with_property(
			REPOSITORY_NAME_PROPERTY,
			Value::String("UserRepository".to_string()),
		)
		.unwrap()
		.with_property(
			REPOSITORY_ENTITY_PROPERTY,
			Value::String("User".to_string()),
		)
		.unwrap();
		let lowered = plugin.lower_node(&node).unwrap();
		assert_eq!(lowered.operation().as_str(), REPOSITORY_NODE_TYPE);
		assert_eq!(lowered.kind(), LoweredOperationKind::DECLARATION);
		assert_eq!(
			lowered.attributes()["backend.repository.name"],
			"UserRepository"
		);
		assert_eq!(lowered.attributes()["backend.repository.entity"], "User");
	}

	#[test]
	fn rejects_invalid_repository_properties_and_references() {
		let plugin = BackendDslPlugin::new();
		let missing = Node::new(
			NodeId::new(2).unwrap(),
			NodeTypeId::new(REPOSITORY_NODE_TYPE).unwrap(),
		);
		let wrong_name_type = missing
			.clone()
			.with_property(REPOSITORY_NAME_PROPERTY, Value::Bool(true))
			.unwrap();
		let missing_entity = missing
			.clone()
			.with_property(
				REPOSITORY_NAME_PROPERTY,
				Value::String("UserRepository".to_string()),
			)
			.unwrap();
		let invalid_entity = missing_entity
			.clone()
			.with_property(
				REPOSITORY_ENTITY_PROPERTY,
				Value::String("User Entity".to_string()),
			)
			.unwrap();

		assert_eq!(
			plugin.analyze_node(&missing)[0].code().as_str(),
			"BACKEND_DSL.REPOSITORY_NAME_MISSING"
		);
		assert_eq!(
			plugin.analyze_node(&wrong_name_type)[0].code().as_str(),
			"BACKEND_DSL.REPOSITORY_NAME_TYPE"
		);
		assert_eq!(
			plugin.analyze_node(&missing_entity)[0].code().as_str(),
			"BACKEND_DSL.REPOSITORY_ENTITY_MISSING"
		);
		assert_eq!(
			plugin.analyze_node(&invalid_entity)[0].code().as_str(),
			"BACKEND_DSL.REPOSITORY_ENTITY_INVALID"
		);

		let metadata = krunt_graph_model::GraphMetadata::new(
			krunt_graph_model::GraphId::new(31).unwrap(),
			"backend-v1",
			"krunt-test",
		)
		.unwrap();
		let entity = Node::new(
			NodeId::new(3).unwrap(),
			NodeTypeId::new(ENTITY_NODE_TYPE).unwrap(),
		)
		.with_property(ENTITY_NAME_PROPERTY, Value::String("User".to_string()))
		.unwrap();
		let repository = Node::new(
			NodeId::new(4).unwrap(),
			NodeTypeId::new(REPOSITORY_NODE_TYPE).unwrap(),
		)
		.with_property(
			REPOSITORY_NAME_PROPERTY,
			Value::String("UserRepository".to_string()),
		)
		.unwrap()
		.with_property(
			REPOSITORY_ENTITY_PROPERTY,
			Value::String("User".to_string()),
		)
		.unwrap();
		let duplicate = Node::new(
			NodeId::new(5).unwrap(),
			NodeTypeId::new(REPOSITORY_NODE_TYPE).unwrap(),
		)
		.with_property(
			REPOSITORY_NAME_PROPERTY,
			Value::String("UserRepository".to_string()),
		)
		.unwrap()
		.with_property(
			REPOSITORY_ENTITY_PROPERTY,
			Value::String("Missing".to_string()),
		)
		.unwrap();
		let mut builder = krunt_graph_model::GraphBuilder::new(metadata);
		for node in [entity, repository, duplicate] {
			builder.add_node(node).unwrap();
		}
		let diagnostics = BackendDslPlugin::new().analyze_graph(&builder.build().unwrap());
		assert!(diagnostics.iter().any(|diagnostic| {
			diagnostic.code().as_str() == "BACKEND_DSL.REPOSITORY_ENTITY_NOT_FOUND"
		}));
		assert!(diagnostics.iter().any(|diagnostic| {
			diagnostic.code().as_str() == "BACKEND_DSL.REPOSITORY_NAME_DUPLICATE"
		}));
	}

	#[test]
	fn exposes_and_lowers_queries_bound_to_repositories() {
		let plugin = BackendDslPlugin::new();
		let descriptor = plugin
			.descriptors()
			.iter()
			.find(|descriptor| descriptor.type_id().as_str() == QUERY_NODE_TYPE)
			.expect("query descriptor should be registered");
		assert_eq!(descriptor.label(), "Query");
		assert_eq!(descriptor.category(), "Persistence");
		assert_eq!(descriptor.inputs().len(), 1);
		assert_eq!(descriptor.outputs().len(), 2);
		assert_eq!(descriptor.inputs()[0].name(), "in");
		assert_eq!(descriptor.outputs()[0].name(), "next");
		assert_eq!(descriptor.outputs()[1].name(), "result");
		assert_eq!(descriptor.properties().len(), 2);
		assert_eq!(descriptor.properties()[0].key(), QUERY_NAME_PROPERTY);
		assert_eq!(descriptor.properties()[1].key(), QUERY_REPOSITORY_PROPERTY);
		assert!(descriptor
			.properties()
			.iter()
			.all(|property| property.required()));

		let node = Node::new(
			NodeId::new(2).unwrap(),
			NodeTypeId::new(QUERY_NODE_TYPE).unwrap(),
		)
		.with_property(QUERY_NAME_PROPERTY, Value::String("ListUsers".to_string()))
		.unwrap()
		.with_property(
			QUERY_REPOSITORY_PROPERTY,
			Value::String("UserRepository".to_string()),
		)
		.unwrap();
		let lowered = plugin.lower_node(&node).unwrap();
		assert_eq!(lowered.operation().as_str(), QUERY_NODE_TYPE);
		assert_eq!(lowered.input_types().len(), 0);
		assert_eq!(lowered.output_types().len(), 1);
		assert_eq!(lowered.async_kind(), AsyncKind::ASYNC);
		assert_eq!(
			lowered.effects(),
			&[EffectId::new(DATABASE_READ_EFFECT).unwrap()]
		);
		assert_eq!(
			plugin.contract(&node).effects(),
			&[EffectId::new(DATABASE_READ_EFFECT).unwrap()]
		);
		assert_eq!(lowered.attributes()["backend.query.name"], "ListUsers");
		assert_eq!(
			lowered.attributes()["backend.query.repository"],
			"UserRepository"
		);
	}

	#[test]
	fn rejects_invalid_query_properties_and_references() {
		let plugin = BackendDslPlugin::new();
		let missing = Node::new(
			NodeId::new(2).unwrap(),
			NodeTypeId::new(QUERY_NODE_TYPE).unwrap(),
		);
		let wrong_name_type = missing
			.clone()
			.with_property(QUERY_NAME_PROPERTY, Value::Bool(true))
			.unwrap();
		let missing_repository = missing
			.clone()
			.with_property(QUERY_NAME_PROPERTY, Value::String("ListUsers".to_string()))
			.unwrap();
		let invalid_repository = missing_repository
			.clone()
			.with_property(
				QUERY_REPOSITORY_PROPERTY,
				Value::String("User Repository".to_string()),
			)
			.unwrap();

		assert_eq!(
			plugin.analyze_node(&missing)[0].code().as_str(),
			"BACKEND_DSL.QUERY_NAME_MISSING"
		);
		assert_eq!(
			plugin.analyze_node(&wrong_name_type)[0].code().as_str(),
			"BACKEND_DSL.QUERY_NAME_TYPE"
		);
		assert_eq!(
			plugin.analyze_node(&missing_repository)[0].code().as_str(),
			"BACKEND_DSL.QUERY_REPOSITORY_MISSING"
		);
		assert_eq!(
			plugin.analyze_node(&invalid_repository)[0].code().as_str(),
			"BACKEND_DSL.QUERY_REPOSITORY_INVALID"
		);

		let metadata = krunt_graph_model::GraphMetadata::new(
			krunt_graph_model::GraphId::new(32).unwrap(),
			"backend-v1",
			"krunt-test",
		)
		.unwrap();
		let repository = Node::new(
			NodeId::new(3).unwrap(),
			NodeTypeId::new(REPOSITORY_NODE_TYPE).unwrap(),
		)
		.with_property(
			REPOSITORY_NAME_PROPERTY,
			Value::String("UserRepository".to_string()),
		)
		.unwrap()
		.with_property(
			REPOSITORY_ENTITY_PROPERTY,
			Value::String("User".to_string()),
		)
		.unwrap();
		let query = Node::new(
			NodeId::new(4).unwrap(),
			NodeTypeId::new(QUERY_NODE_TYPE).unwrap(),
		)
		.with_property(QUERY_NAME_PROPERTY, Value::String("ListUsers".to_string()))
		.unwrap()
		.with_property(
			QUERY_REPOSITORY_PROPERTY,
			Value::String("MissingRepository".to_string()),
		)
		.unwrap();
		let duplicate = Node::new(
			NodeId::new(5).unwrap(),
			NodeTypeId::new(QUERY_NODE_TYPE).unwrap(),
		)
		.with_property(QUERY_NAME_PROPERTY, Value::String("ListUsers".to_string()))
		.unwrap()
		.with_property(
			QUERY_REPOSITORY_PROPERTY,
			Value::String("UserRepository".to_string()),
		)
		.unwrap();
		let mut builder = krunt_graph_model::GraphBuilder::new(metadata);
		for node in [repository, query, duplicate] {
			builder.add_node(node).unwrap();
		}
		let diagnostics = BackendDslPlugin::new().analyze_graph(&builder.build().unwrap());
		assert!(diagnostics.iter().any(|diagnostic| {
			diagnostic.code().as_str() == "BACKEND_DSL.QUERY_REPOSITORY_NOT_FOUND"
		}));
		assert!(diagnostics.iter().any(|diagnostic| {
			diagnostic.code().as_str() == "BACKEND_DSL.QUERY_NAME_DUPLICATE"
		}));
	}

	#[test]
	fn exposes_and_lowers_mutations_bound_to_repositories() {
		let plugin = BackendDslPlugin::new();
		let descriptor = plugin
			.descriptors()
			.iter()
			.find(|descriptor| descriptor.type_id().as_str() == MUTATION_NODE_TYPE)
			.expect("mutation descriptor should be registered");
		assert_eq!(descriptor.label(), "Mutation");
		assert_eq!(descriptor.category(), "Persistence");
		assert_eq!(descriptor.inputs().len(), 2);
		assert_eq!(descriptor.outputs().len(), 2);
		assert_eq!(descriptor.inputs()[0].name(), "in");
		assert_eq!(descriptor.inputs()[1].name(), "value");
		assert_eq!(descriptor.outputs()[0].name(), "next");
		assert_eq!(descriptor.outputs()[1].name(), "result");
		assert_eq!(descriptor.properties().len(), 2);
		assert_eq!(descriptor.properties()[0].key(), MUTATION_NAME_PROPERTY);
		assert_eq!(
			descriptor.properties()[1].key(),
			MUTATION_REPOSITORY_PROPERTY
		);
		assert!(descriptor
			.properties()
			.iter()
			.all(|property| property.required()));

		let node = Node::new(
			NodeId::new(2).unwrap(),
			NodeTypeId::new(MUTATION_NODE_TYPE).unwrap(),
		)
		.with_property(
			MUTATION_NAME_PROPERTY,
			Value::String("CreateUser".to_string()),
		)
		.unwrap()
		.with_property(
			MUTATION_REPOSITORY_PROPERTY,
			Value::String("UserRepository".to_string()),
		)
		.unwrap();
		let lowered = plugin.lower_node(&node).unwrap();
		assert_eq!(lowered.operation().as_str(), MUTATION_NODE_TYPE);
		assert_eq!(lowered.input_types().len(), 1);
		assert_eq!(lowered.output_types().len(), 1);
		assert_eq!(lowered.async_kind(), AsyncKind::ASYNC);
		assert_eq!(
			lowered.effects(),
			&[EffectId::new(DATABASE_WRITE_EFFECT).unwrap()]
		);
		assert_eq!(
			plugin.contract(&node).effects(),
			&[EffectId::new(DATABASE_WRITE_EFFECT).unwrap()]
		);
		assert_eq!(lowered.attributes()["backend.mutation.name"], "CreateUser");
		assert_eq!(
			lowered.attributes()["backend.mutation.repository"],
			"UserRepository"
		);
	}

	#[test]
	fn rejects_invalid_mutation_properties_and_references() {
		let plugin = BackendDslPlugin::new();
		let missing = Node::new(
			NodeId::new(2).unwrap(),
			NodeTypeId::new(MUTATION_NODE_TYPE).unwrap(),
		);
		let wrong_name_type = missing
			.clone()
			.with_property(MUTATION_NAME_PROPERTY, Value::Bool(true))
			.unwrap();
		let missing_repository = missing
			.clone()
			.with_property(
				MUTATION_NAME_PROPERTY,
				Value::String("CreateUser".to_string()),
			)
			.unwrap();
		let invalid_repository = missing_repository
			.clone()
			.with_property(
				MUTATION_REPOSITORY_PROPERTY,
				Value::String("User Repository".to_string()),
			)
			.unwrap();

		assert_eq!(
			plugin.analyze_node(&missing)[0].code().as_str(),
			"BACKEND_DSL.MUTATION_NAME_MISSING"
		);
		assert_eq!(
			plugin.analyze_node(&wrong_name_type)[0].code().as_str(),
			"BACKEND_DSL.MUTATION_NAME_TYPE"
		);
		assert_eq!(
			plugin.analyze_node(&missing_repository)[0].code().as_str(),
			"BACKEND_DSL.MUTATION_REPOSITORY_MISSING"
		);
		assert_eq!(
			plugin.analyze_node(&invalid_repository)[0].code().as_str(),
			"BACKEND_DSL.MUTATION_REPOSITORY_INVALID"
		);

		let metadata = krunt_graph_model::GraphMetadata::new(
			krunt_graph_model::GraphId::new(33).unwrap(),
			"backend-v1",
			"krunt-test",
		)
		.unwrap();
		let repository = Node::new(
			NodeId::new(3).unwrap(),
			NodeTypeId::new(REPOSITORY_NODE_TYPE).unwrap(),
		)
		.with_property(
			REPOSITORY_NAME_PROPERTY,
			Value::String("UserRepository".to_string()),
		)
		.unwrap()
		.with_property(
			REPOSITORY_ENTITY_PROPERTY,
			Value::String("User".to_string()),
		)
		.unwrap();
		let mutation = Node::new(
			NodeId::new(4).unwrap(),
			NodeTypeId::new(MUTATION_NODE_TYPE).unwrap(),
		)
		.with_property(
			MUTATION_NAME_PROPERTY,
			Value::String("CreateUser".to_string()),
		)
		.unwrap()
		.with_property(
			MUTATION_REPOSITORY_PROPERTY,
			Value::String("MissingRepository".to_string()),
		)
		.unwrap();
		let duplicate = Node::new(
			NodeId::new(5).unwrap(),
			NodeTypeId::new(MUTATION_NODE_TYPE).unwrap(),
		)
		.with_property(
			MUTATION_NAME_PROPERTY,
			Value::String("CreateUser".to_string()),
		)
		.unwrap()
		.with_property(
			MUTATION_REPOSITORY_PROPERTY,
			Value::String("UserRepository".to_string()),
		)
		.unwrap();
		let mut builder = krunt_graph_model::GraphBuilder::new(metadata);
		for node in [repository, mutation, duplicate] {
			builder.add_node(node).unwrap();
		}
		let diagnostics = BackendDslPlugin::new().analyze_graph(&builder.build().unwrap());
		assert!(diagnostics.iter().any(|diagnostic| {
			diagnostic.code().as_str() == "BACKEND_DSL.MUTATION_REPOSITORY_NOT_FOUND"
		}));
		assert!(diagnostics.iter().any(|diagnostic| {
			diagnostic.code().as_str() == "BACKEND_DSL.MUTATION_NAME_DUPLICATE"
		}));
	}

	#[test]
	fn exposes_and_lowers_effect_nodes_with_explicit_effects() {
		let plugin = BackendDslPlugin::new();

		let http_request = Node::new(
			NodeId::new(6).unwrap(),
			NodeTypeId::new(HTTP_REQUEST_NODE_TYPE).unwrap(),
		)
		.with_property(
			HTTP_REQUEST_METHOD_PROPERTY,
			Value::String("POST".to_string()),
		)
		.unwrap()
		.with_property(
			HTTP_REQUEST_URL_PROPERTY,
			Value::String("https://example.test/users".to_string()),
		)
		.unwrap();
		let lowered_http_request = plugin.lower_node(&http_request).unwrap();
		assert_eq!(
			lowered_http_request.operation().as_str(),
			HTTP_REQUEST_NODE_TYPE
		);
		assert_eq!(lowered_http_request.input_types().len(), 1);
		assert_eq!(lowered_http_request.output_types().len(), 1);
		assert_eq!(lowered_http_request.async_kind(), AsyncKind::ASYNC);
		assert_eq!(
			lowered_http_request.effects(),
			&[EffectId::new(HTTP_REQUEST_EFFECT).unwrap()]
		);
		assert_eq!(
			lowered_http_request.attributes()["backend.http_request.method"],
			"POST"
		);
		assert_eq!(
			lowered_http_request.attributes()["backend.http_request.url"],
			"https://example.test/users"
		);

		let logging = Node::new(
			NodeId::new(7).unwrap(),
			NodeTypeId::new(LOGGING_NODE_TYPE).unwrap(),
		)
		.with_property(LOGGING_LEVEL_PROPERTY, Value::String("WARN".to_string()))
		.unwrap();
		let lowered_logging = plugin.lower_node(&logging).unwrap();
		assert_eq!(lowered_logging.operation().as_str(), LOGGING_NODE_TYPE);
		assert_eq!(lowered_logging.input_types().len(), 1);
		assert!(lowered_logging.output_types().is_empty());
		assert_eq!(lowered_logging.async_kind(), AsyncKind::SYNC);
		assert_eq!(
			lowered_logging.effects(),
			&[EffectId::new(LOGGING_EFFECT).unwrap()]
		);
		assert_eq!(
			lowered_logging.attributes()["backend.logging.level"],
			"WARN"
		);

		let storage_read = Node::new(
			NodeId::new(8).unwrap(),
			NodeTypeId::new(STORAGE_NODE_TYPE).unwrap(),
		)
		.with_property(
			STORAGE_OPERATION_PROPERTY,
			Value::String("READ".to_string()),
		)
		.unwrap()
		.with_property(STORAGE_KEY_PROPERTY, Value::String("users:all".to_string()))
		.unwrap();
		let lowered_storage_read = plugin.lower_node(&storage_read).unwrap();
		assert_eq!(lowered_storage_read.operation().as_str(), STORAGE_NODE_TYPE);
		assert_eq!(lowered_storage_read.input_types().len(), 1);
		assert_eq!(lowered_storage_read.output_types().len(), 1);
		assert_eq!(lowered_storage_read.async_kind(), AsyncKind::ASYNC);
		assert_eq!(
			lowered_storage_read.effects(),
			&[EffectId::new(STORAGE_READ_EFFECT).unwrap()]
		);
		assert_eq!(
			lowered_storage_read.attributes()["backend.storage.operation"],
			"READ"
		);
		assert!(!plugin.contract(&storage_read).inputs()[1].required());

		let storage_write = storage_read
			.clone()
			.with_property(
				STORAGE_OPERATION_PROPERTY,
				Value::String("WRITE".to_string()),
			)
			.unwrap();
		let lowered_storage_write = plugin.lower_node(&storage_write).unwrap();
		assert_eq!(
			lowered_storage_write.effects(),
			&[EffectId::new(STORAGE_WRITE_EFFECT).unwrap()]
		);
		assert!(plugin.contract(&storage_write).inputs()[1].required());
	}

	#[test]
	fn rejects_invalid_effect_properties_and_unconnected_outputs() {
		let plugin = BackendDslPlugin::new();
		let http_request = Node::new(
			NodeId::new(6).unwrap(),
			NodeTypeId::new(HTTP_REQUEST_NODE_TYPE).unwrap(),
		);
		let invalid_method = http_request
			.clone()
			.with_property(
				HTTP_REQUEST_METHOD_PROPERTY,
				Value::String("INVALID".to_string()),
			)
			.unwrap()
			.with_property(
				HTTP_REQUEST_URL_PROPERTY,
				Value::String("https://example.test".to_string()),
			)
			.unwrap();
		assert_eq!(
			plugin.analyze_node(&http_request)[0].code().as_str(),
			"BACKEND_DSL.HTTP_REQUEST_METHOD_MISSING"
		);
		assert_eq!(
			plugin.analyze_node(&invalid_method)[0].code().as_str(),
			"BACKEND_DSL.HTTP_REQUEST_METHOD_INVALID"
		);

		let invalid_logging = Node::new(
			NodeId::new(7).unwrap(),
			NodeTypeId::new(LOGGING_NODE_TYPE).unwrap(),
		)
		.with_property(LOGGING_LEVEL_PROPERTY, Value::String("TRACE".to_string()))
		.unwrap();
		assert_eq!(
			plugin.analyze_node(&invalid_logging)[0].code().as_str(),
			"BACKEND_DSL.LOGGING_LEVEL_INVALID"
		);

		let invalid_storage = Node::new(
			NodeId::new(8).unwrap(),
			NodeTypeId::new(STORAGE_NODE_TYPE).unwrap(),
		)
		.with_property(
			STORAGE_OPERATION_PROPERTY,
			Value::String("REMOVE".to_string()),
		)
		.unwrap()
		.with_property(STORAGE_KEY_PROPERTY, Value::String("users".to_string()))
		.unwrap();
		assert_eq!(
			plugin.analyze_node(&invalid_storage)[0].code().as_str(),
			"BACKEND_DSL.STORAGE_OPERATION_INVALID"
		);

		let metadata = krunt_graph_model::GraphMetadata::new(
			krunt_graph_model::GraphId::new(34).unwrap(),
			"backend-v1",
			"krunt-test",
		)
		.unwrap();
		let mut http_request = invalid_method
			.clone()
			.with_property(
				HTTP_REQUEST_METHOD_PROPERTY,
				Value::String("GET".to_string()),
			)
			.unwrap();
		http_request
			.add_port(krunt_graph_model::Port::new(
				PortId::new(61).unwrap(),
				PortDirection::OUTPUT,
				PortKind::EXECUTION,
			))
			.unwrap();
		let mut logging = Node::new(
			NodeId::new(7).unwrap(),
			NodeTypeId::new(LOGGING_NODE_TYPE).unwrap(),
		);
		logging
			.add_port(krunt_graph_model::Port::new(
				PortId::new(71).unwrap(),
				PortDirection::OUTPUT,
				PortKind::EXECUTION,
			))
			.unwrap();
		let mut storage = Node::new(
			NodeId::new(8).unwrap(),
			NodeTypeId::new(STORAGE_NODE_TYPE).unwrap(),
		)
		.with_property(
			STORAGE_OPERATION_PROPERTY,
			Value::String("READ".to_string()),
		)
		.unwrap()
		.with_property(STORAGE_KEY_PROPERTY, Value::String("users".to_string()))
		.unwrap();
		storage
			.add_port(krunt_graph_model::Port::new(
				PortId::new(81).unwrap(),
				PortDirection::OUTPUT,
				PortKind::EXECUTION,
			))
			.unwrap();
		let mut builder = krunt_graph_model::GraphBuilder::new(metadata);
		for node in [http_request, logging, storage] {
			builder.add_node(node).unwrap();
		}
		let diagnostics = plugin.analyze_graph(&builder.build().unwrap());
		assert!(diagnostics.iter().any(|diagnostic| {
			diagnostic.code().as_str() == "BACKEND_DSL.HTTP_REQUEST_OUTPUT_UNCONNECTED"
		}));
		assert!(diagnostics.iter().any(|diagnostic| {
			diagnostic.code().as_str() == "BACKEND_DSL.LOGGING_OUTPUT_UNCONNECTED"
		}));
		assert!(diagnostics.iter().any(|diagnostic| {
			diagnostic.code().as_str() == "BACKEND_DSL.STORAGE_OUTPUT_UNCONNECTED"
		}));
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
	fn lowers_a_route_request_entity_binding() {
		let plugin = BackendDslPlugin::new();
		let node = route(&[
			("method", Value::String("POST".to_string())),
			("path", Value::String("/users".to_string())),
			(
				HTTP_ROUTE_REQUEST_ENTITY_PROPERTY,
				Value::String("User".to_string()),
			),
		]);
		let lowered = plugin.lower_node(&node).unwrap();

		assert_eq!(lowered.attributes()["backend.http.request_entity"], "User");
	}

	#[test]
	fn rejects_a_route_with_an_undeclared_request_entity() {
		let metadata = krunt_graph_model::GraphMetadata::new(
			krunt_graph_model::GraphId::new(2).unwrap(),
			"backend-v1",
			"krunt-test",
		)
		.unwrap();
		let mut builder = krunt_graph_model::GraphBuilder::new(metadata);
		builder
			.add_node(route(&[
				("method", Value::String("POST".to_string())),
				("path", Value::String("/users".to_string())),
				(
					HTTP_ROUTE_REQUEST_ENTITY_PROPERTY,
					Value::String("Missing".to_string()),
				),
			]))
			.unwrap();
		let diagnostics = BackendDslPlugin::new().analyze_graph(&builder.build().unwrap());

		assert!(diagnostics.iter().any(|diagnostic| {
			diagnostic.code().as_str() == "BACKEND_DSL.HTTP_ROUTE_REQUEST_ENTITY_NOT_FOUND"
		}));
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
