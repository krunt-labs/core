//! Initial Krunt Backend DSL semantic plugin.
//!
//! This crate owns backend meaning for the first vertical slice. It describes
//! HTTP Route and Success Response nodes, validates their properties, and
//! lowers them into target-neutral operations with namespaced metadata. It does
//! not emit TypeScript or NestJS code.

use krunt_diagnostics::{CompilerStage, Diagnostic, DiagnosticCode, GraphElementId, Severity};
use krunt_graph_model::{
	EdgeKind, GraphDocument, Node, NodeId, NodeTypeId, PluginReference, PortId, PortKind,
};
use krunt_ir::{AsyncKind, OperationId, TypeId};
use krunt_plugin_api::{
	LoweredOperation, NodeContract, PluginApiError, PluginManifest, PluginRegistry, PortContract,
};
use serde::Serialize;
use serde_json::Value;
use std::fmt;

pub const BACKEND_DSL_ID: &str = "krunt.backend-dsl";
pub const BACKEND_DSL_VERSION: &str = "0.1.0";
pub const HTTP_ROUTE_NODE_TYPE: &str = "backend.http_route";
pub const SUCCESS_RESPONSE_NODE_TYPE: &str = "backend.success_response";
pub const HTTP_REQUEST_TYPE: &str = "HttpRequest";
pub const RESPONSE_BODY_TYPE: &str = "ANY";

const HTTP_METHODS: [&str; 7] = ["DELETE", "GET", "HEAD", "OPTIONS", "PATCH", "POST", "PUT"];

/// Serializable property kind exposed to editor and AI clients.
#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub enum BackendPropertyKind {
	STRING,
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
		let response_type =
			NodeTypeId::new(SUCCESS_RESPONSE_NODE_TYPE).expect("static node type is valid");
		let request_type = TypeId::new(HTTP_REQUEST_TYPE).expect("static type is valid");
		let response_body_type = TypeId::new(RESPONSE_BODY_TYPE).expect("static type is valid");
		let manifest = PluginManifest::new(
			PluginReference::new(BACKEND_DSL_ID, BACKEND_DSL_VERSION)
				.expect("static plugin identity is valid"),
			[route_type.clone(), response_type.clone()],
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
						PortContract::new(PortKind::DATA, Some(response_body_type), false),
					),
				],
				outputs: Vec::new(),
				properties: Vec::new(),
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
			SUCCESS_RESPONSE_NODE_TYPE => response_contract(),
			_ => NodeContract::new([], [], [], [], [], AsyncKind::SYNC, []),
		}
	}

	fn analyze_graph(&self, document: &GraphDocument) -> Vec<Diagnostic> {
		validate_route_response_paths(document)
	}

	fn analyze_node(&self, node: &Node) -> Vec<Diagnostic> {
		if node.type_id().as_str() == HTTP_ROUTE_NODE_TYPE {
			return route_properties(node)
				.err()
				.map(|diagnostic| *diagnostic)
				.into_iter()
				.collect();
		}
		Vec::new()
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
			SUCCESS_RESPONSE_NODE_TYPE => Ok(LoweredOperation::new(
				OperationId::new(SUCCESS_RESPONSE_NODE_TYPE).expect("static operation is valid"),
				[TypeId::new(RESPONSE_BODY_TYPE).expect("static type is valid")],
				[],
				AsyncKind::SYNC,
				[],
			)
			.with_return(true)
			.with_attribute("response.status_code", "200")),
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

	let response_nodes: std::collections::BTreeSet<_> = document
		.nodes()
		.iter()
		.filter(|node| node.type_id().as_str() == SUCCESS_RESPONSE_NODE_TYPE)
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
				if response_nodes.contains(&node_id) {
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
				"Every HTTP Route execution path must reach a Success Response.",
			))
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
		assert_eq!(plugin.descriptors().len(), 2);
		assert_eq!(plugin.descriptors()[0].label(), "HTTP Route");
		assert_eq!(plugin.descriptors()[0].properties().len(), 2);
		assert_eq!(plugin.descriptors()[1].label(), "Success Response");
		assert_eq!(plugin.descriptors()[1].inputs().len(), 2);
		assert!(!plugin.descriptors()[1].inputs()[1].contract().required());
		assert_eq!(
			plugin.descriptors()[1].inputs()[1]
				.contract()
				.type_id()
				.unwrap()
				.as_str(),
			RESPONSE_BODY_TYPE
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
}
