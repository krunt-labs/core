//! Krunt's domain-neutral, versioned graph document.
//!
//! This crate owns structure only: stable IDs, nodes, ports, edges, groups,
//! subgraph boundaries, metadata, extensions, and deterministic serialization.
//! Backend semantics belong to semantic plugins above this layer.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

macro_rules! define_id {
	($name:ident, $kind:literal) => {
		#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
		pub struct $name(u64);

		impl $name {
			pub fn new(value: u64) -> Result<Self, DocumentError> {
				if value == 0 {
					return Err(DocumentError::INVALID_ID { kind: $kind, value });
				}
				Ok(Self(value))
			}

			pub const fn as_u64(self) -> u64 {
				self.0
			}
		}
	};
}

define_id!(GraphId, "graph");
define_id!(NodeId, "node");
define_id!(PortId, "port");
define_id!(EdgeId, "edge");
define_id!(GroupId, "group");
define_id!(SubgraphId, "subgraph");

/// Opaque semantic node-type identity. The graph model stores it but never
/// interprets its backend meaning.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct NodeTypeId(String);

impl NodeTypeId {
	pub fn new(value: impl Into<String>) -> Result<Self, DocumentError> {
		let value = value.into();
		if value.trim().is_empty() {
			return Err(DocumentError::EMPTY_NODE_TYPE);
		}
		Ok(Self(value))
	}

	pub fn as_str(&self) -> &str {
		&self.0
	}
}

/// Opaque type-system identity attached to a data port.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct TypeReference(String);

impl TypeReference {
	pub fn new(value: impl Into<String>) -> Result<Self, DocumentError> {
		let value = value.into();
		if value.trim().is_empty() {
			return Err(DocumentError::EMPTY_TYPE_REFERENCE);
		}
		Ok(Self(value))
	}

	pub fn as_str(&self) -> &str {
		&self.0
	}
}

/// Direction of a semantic port.
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum PortDirection {
	INPUT,
	OUTPUT,
}

/// Structural category of a semantic port.
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum PortKind {
	EXECUTION,
	DATA,
}

/// Structural category of an edge.
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum EdgeKind {
	EXECUTION,
	DATA,
}

/// A plugin identity recorded in the graph document metadata.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct PluginReference {
	id: String,
	version: String,
}

impl PluginReference {
	pub fn new(id: impl Into<String>, version: impl Into<String>) -> Result<Self, DocumentError> {
		let id = id.into();
		let version = version.into();
		if id.trim().is_empty() {
			return Err(DocumentError::EMPTY_PLUGIN_ID);
		}
		if version.trim().is_empty() {
			return Err(DocumentError::EMPTY_PLUGIN_VERSION);
		}
		Ok(Self { id, version })
	}

	pub fn id(&self) -> &str {
		&self.id
	}

	pub fn version(&self) -> &str {
		&self.version
	}
}

/// Version metadata required to reproduce and interpret a graph document.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GraphMetadata {
	graph_id: GraphId,
	semantic_version: String,
	compiler_version: String,
	plugins: Vec<PluginReference>,
}

impl GraphMetadata {
	pub fn new(
		graph_id: GraphId,
		semantic_version: impl Into<String>,
		compiler_version: impl Into<String>,
	) -> Result<Self, DocumentError> {
		let semantic_version = semantic_version.into();
		let compiler_version = compiler_version.into();
		if semantic_version.trim().is_empty() {
			return Err(DocumentError::EMPTY_SEMANTIC_VERSION);
		}
		if compiler_version.trim().is_empty() {
			return Err(DocumentError::EMPTY_COMPILER_VERSION);
		}
		Ok(Self {
			graph_id,
			semantic_version,
			compiler_version,
			plugins: Vec::new(),
		})
	}

	pub fn with_plugin(mut self, plugin: PluginReference) -> Result<Self, DocumentError> {
		if self.plugins.iter().any(|existing| existing.id == plugin.id) {
			return Err(DocumentError::DUPLICATE_PLUGIN(plugin.id));
		}
		self.plugins.push(plugin);
		self.plugins.sort();
		Ok(self)
	}

	pub fn graph_id(&self) -> GraphId {
		self.graph_id
	}

	pub fn semantic_version(&self) -> &str {
		&self.semantic_version
	}

	pub fn compiler_version(&self) -> &str {
		&self.compiler_version
	}

	pub fn plugins(&self) -> &[PluginReference] {
		&self.plugins
	}

	fn validate(&self) -> Result<(), DocumentError> {
		if self.graph_id.0 == 0 {
			return Err(DocumentError::INVALID_ID {
				kind: "graph",
				value: self.graph_id.0,
			});
		}
		if self.semantic_version.trim().is_empty() {
			return Err(DocumentError::EMPTY_SEMANTIC_VERSION);
		}
		if self.compiler_version.trim().is_empty() {
			return Err(DocumentError::EMPTY_COMPILER_VERSION);
		}
		let mut plugin_ids = BTreeSet::new();
		for plugin in &self.plugins {
			if plugin.id.trim().is_empty() {
				return Err(DocumentError::EMPTY_PLUGIN_ID);
			}
			if plugin.version.trim().is_empty() {
				return Err(DocumentError::EMPTY_PLUGIN_VERSION);
			}
			if !plugin_ids.insert(plugin.id.as_str()) {
				return Err(DocumentError::DUPLICATE_PLUGIN(plugin.id.clone()));
			}
		}
		Ok(())
	}
}

/// One typed or execution connection endpoint.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Port {
	id: PortId,
	direction: PortDirection,
	kind: PortKind,
	type_reference: Option<TypeReference>,
}

impl Port {
	pub fn new(id: PortId, direction: PortDirection, kind: PortKind) -> Self {
		Self {
			id,
			direction,
			kind,
			type_reference: None,
		}
	}

	pub fn with_type_reference(mut self, type_reference: TypeReference) -> Self {
		self.type_reference = Some(type_reference);
		self
	}

	pub fn id(&self) -> PortId {
		self.id
	}

	pub fn direction(&self) -> PortDirection {
		self.direction
	}

	pub fn kind(&self) -> PortKind {
		self.kind
	}

	pub fn type_reference(&self) -> Option<&TypeReference> {
		self.type_reference.as_ref()
	}
}

/// A domain-neutral graph node. Properties are opaque to this crate.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Node {
	id: NodeId,
	type_id: NodeTypeId,
	ports: Vec<Port>,
	properties: BTreeMap<String, Value>,
	annotations: Vec<Annotation>,
}

impl Node {
	pub fn new(id: NodeId, type_id: NodeTypeId) -> Self {
		Self {
			id,
			type_id,
			ports: Vec::new(),
			properties: BTreeMap::new(),
			annotations: Vec::new(),
		}
	}

	pub fn add_port(&mut self, port: Port) -> Result<(), DocumentError> {
		if self.ports.iter().any(|existing| existing.id == port.id) {
			return Err(DocumentError::DUPLICATE_PORT(port.id));
		}
		self.ports.push(port);
		self.ports.sort_by_key(|port| port.id);
		Ok(())
	}

	pub fn with_property(
		mut self,
		key: impl Into<String>,
		value: Value,
	) -> Result<Self, DocumentError> {
		let key = key.into();
		if key.trim().is_empty() {
			return Err(DocumentError::EMPTY_PROPERTY_KEY);
		}
		self.properties.insert(key, value);
		Ok(self)
	}

	pub fn add_annotation(&mut self, annotation: Annotation) -> Result<(), DocumentError> {
		if self
			.annotations
			.iter()
			.any(|existing| existing.key == annotation.key)
		{
			return Err(DocumentError::DUPLICATE_ANNOTATION(annotation.key));
		}
		self.annotations.push(annotation);
		self.annotations
			.sort_by(|left, right| left.key.cmp(&right.key));
		Ok(())
	}

	pub fn id(&self) -> NodeId {
		self.id
	}

	pub fn type_id(&self) -> &NodeTypeId {
		&self.type_id
	}

	pub fn ports(&self) -> &[Port] {
		&self.ports
	}

	pub fn properties(&self) -> &BTreeMap<String, Value> {
		&self.properties
	}

	pub fn annotations(&self) -> &[Annotation] {
		&self.annotations
	}
}

/// Human-readable metadata attached to a graph node.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Annotation {
	key: String,
	value: Value,
}

impl Annotation {
	pub fn new(key: impl Into<String>, value: Value) -> Result<Self, DocumentError> {
		let key = key.into();
		if key.trim().is_empty() {
			return Err(DocumentError::EMPTY_ANNOTATION_KEY);
		}
		Ok(Self { key, value })
	}

	pub fn key(&self) -> &str {
		&self.key
	}

	pub fn value(&self) -> &Value {
		&self.value
	}
}

/// A directed relationship between two semantic ports.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Edge {
	id: EdgeId,
	from: PortId,
	to: PortId,
	kind: EdgeKind,
}

impl Edge {
	pub fn new(id: EdgeId, from: PortId, to: PortId, kind: EdgeKind) -> Self {
		Self { id, from, to, kind }
	}

	pub fn id(&self) -> EdgeId {
		self.id
	}

	pub fn from(&self) -> PortId {
		self.from
	}

	pub fn to(&self) -> PortId {
		self.to
	}

	pub fn kind(&self) -> EdgeKind {
		self.kind
	}
}

/// Presentation-only membership. Groups never alter graph execution.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Group {
	id: GroupId,
	nodes: Vec<NodeId>,
}

impl Group {
	pub fn new(
		id: GroupId,
		nodes: impl IntoIterator<Item = NodeId>,
	) -> Result<Self, DocumentError> {
		let mut nodes: Vec<_> = nodes.into_iter().collect();
		nodes.sort();
		if nodes.windows(2).any(|pair| pair[0] == pair[1]) {
			return Err(DocumentError::DUPLICATE_GROUP_MEMBER(id));
		}
		Ok(Self { id, nodes })
	}

	pub fn id(&self) -> GroupId {
		self.id
	}

	pub fn nodes(&self) -> &[NodeId] {
		&self.nodes
	}
}

/// A subgraph's explicit input or output boundary port.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SubgraphPort {
	id: PortId,
	direction: PortDirection,
	kind: PortKind,
	type_reference: Option<TypeReference>,
}

impl SubgraphPort {
	pub fn new(id: PortId, direction: PortDirection, kind: PortKind) -> Self {
		Self {
			id,
			direction,
			kind,
			type_reference: None,
		}
	}

	pub fn with_type_reference(mut self, type_reference: TypeReference) -> Self {
		self.type_reference = Some(type_reference);
		self
	}

	pub fn id(&self) -> PortId {
		self.id
	}

	pub fn direction(&self) -> PortDirection {
		self.direction
	}

	pub fn kind(&self) -> PortKind {
		self.kind
	}
}

/// Reusable graph composition with explicit boundary ports.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Subgraph {
	id: SubgraphId,
	nodes: Vec<NodeId>,
	ports: Vec<SubgraphPort>,
}

impl Subgraph {
	pub fn new(
		id: SubgraphId,
		nodes: impl IntoIterator<Item = NodeId>,
		ports: impl IntoIterator<Item = SubgraphPort>,
	) -> Result<Self, DocumentError> {
		let mut nodes: Vec<_> = nodes.into_iter().collect();
		nodes.sort();
		if nodes.windows(2).any(|pair| pair[0] == pair[1]) {
			return Err(DocumentError::DUPLICATE_SUBGRAPH_NODE(id));
		}

		let mut ports: Vec<_> = ports.into_iter().collect();
		ports.sort_by_key(|port| port.id);
		if ports.windows(2).any(|pair| pair[0].id == pair[1].id) {
			return Err(DocumentError::DUPLICATE_PORT(ports[1].id));
		}
		Ok(Self { id, nodes, ports })
	}

	pub fn id(&self) -> SubgraphId {
		self.id
	}

	pub fn nodes(&self) -> &[NodeId] {
		&self.nodes
	}

	pub fn ports(&self) -> &[SubgraphPort] {
		&self.ports
	}
}

/// The immutable, validated canonical source document.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GraphDocument {
	schema_version: u32,
	metadata: GraphMetadata,
	nodes: Vec<Node>,
	edges: Vec<Edge>,
	groups: Vec<Group>,
	subgraphs: Vec<Subgraph>,
	extensions: BTreeMap<String, Value>,
}

impl GraphDocument {
	pub const CURRENT_SCHEMA_VERSION: u32 = 1;

	pub fn schema_version(&self) -> u32 {
		self.schema_version
	}

	pub fn metadata(&self) -> &GraphMetadata {
		&self.metadata
	}

	pub fn nodes(&self) -> &[Node] {
		&self.nodes
	}

	pub fn edges(&self) -> &[Edge] {
		&self.edges
	}

	pub fn groups(&self) -> &[Group] {
		&self.groups
	}

	pub fn subgraphs(&self) -> &[Subgraph] {
		&self.subgraphs
	}

	pub fn extensions(&self) -> &BTreeMap<String, Value> {
		&self.extensions
	}

	pub fn to_json(&self) -> Result<String, serde_json::Error> {
		serde_json::to_string(self)
	}

	pub fn from_json(input: &str) -> Result<Self, DocumentError> {
		let wire: GraphDocumentWire = serde_json::from_str(input)
			.map_err(|error| DocumentError::INVALID_JSON(error.to_string()))?;
		let mut builder = GraphBuilder::with_schema_version(wire.schema_version, wire.metadata);
		for node in wire.nodes {
			builder.add_node(node)?;
		}
		for edge in wire.edges {
			builder.add_edge(edge)?;
		}
		for group in wire.groups {
			builder.add_group(group)?;
		}
		for subgraph in wire.subgraphs {
			builder.add_subgraph(subgraph)?;
		}
		for (namespace, value) in wire.extensions {
			builder.add_extension(namespace, value)?;
		}
		builder.build()
	}
}

#[derive(Deserialize)]
struct GraphDocumentWire {
	schema_version: u32,
	metadata: GraphMetadata,
	#[serde(default)]
	nodes: Vec<Node>,
	#[serde(default)]
	edges: Vec<Edge>,
	#[serde(default)]
	groups: Vec<Group>,
	#[serde(default)]
	subgraphs: Vec<Subgraph>,
	#[serde(default)]
	extensions: BTreeMap<String, Value>,
}

/// Mutable validation boundary for creating a graph document.
#[derive(Default)]
pub struct GraphBuilder {
	schema_version: u32,
	metadata: Option<GraphMetadata>,
	nodes: BTreeMap<NodeId, Node>,
	edges: BTreeMap<EdgeId, Edge>,
	groups: BTreeMap<GroupId, Group>,
	subgraphs: BTreeMap<SubgraphId, Subgraph>,
	extensions: BTreeMap<String, Value>,
}

impl GraphBuilder {
	pub fn new(metadata: GraphMetadata) -> Self {
		Self {
			schema_version: GraphDocument::CURRENT_SCHEMA_VERSION,
			metadata: Some(metadata),
			..Self::default()
		}
	}

	fn with_schema_version(schema_version: u32, metadata: GraphMetadata) -> Self {
		Self {
			schema_version,
			metadata: Some(metadata),
			..Self::default()
		}
	}

	pub fn add_node(&mut self, node: Node) -> Result<(), DocumentError> {
		if self.nodes.contains_key(&node.id) {
			return Err(DocumentError::DUPLICATE_NODE(node.id));
		}
		validate_node(&node)?;
		for port in &node.ports {
			if self.port_exists(port.id) {
				return Err(DocumentError::DUPLICATE_PORT(port.id));
			}
		}
		self.nodes.insert(node.id, node);
		Ok(())
	}

	pub fn add_edge(&mut self, edge: Edge) -> Result<(), DocumentError> {
		if self.edges.contains_key(&edge.id) {
			return Err(DocumentError::DUPLICATE_EDGE(edge.id));
		}
		self.edges.insert(edge.id, edge);
		Ok(())
	}

	pub fn add_group(&mut self, group: Group) -> Result<(), DocumentError> {
		if self.groups.contains_key(&group.id) {
			return Err(DocumentError::DUPLICATE_GROUP(group.id));
		}
		validate_group(&group)?;
		self.groups.insert(group.id, group);
		Ok(())
	}

	pub fn add_subgraph(&mut self, subgraph: Subgraph) -> Result<(), DocumentError> {
		if self.subgraphs.contains_key(&subgraph.id) {
			return Err(DocumentError::DUPLICATE_SUBGRAPH(subgraph.id));
		}
		validate_subgraph(&subgraph)?;
		for port in &subgraph.ports {
			if self.port_exists(port.id) {
				return Err(DocumentError::DUPLICATE_PORT(port.id));
			}
		}
		self.subgraphs.insert(subgraph.id, subgraph);
		Ok(())
	}

	pub fn add_extension(
		&mut self,
		namespace: impl Into<String>,
		value: Value,
	) -> Result<(), DocumentError> {
		let namespace = namespace.into();
		if namespace.trim().is_empty() {
			return Err(DocumentError::EMPTY_EXTENSION_NAMESPACE);
		}
		if self.extensions.contains_key(&namespace) {
			return Err(DocumentError::DUPLICATE_EXTENSION(namespace));
		}
		self.extensions.insert(namespace, value);
		Ok(())
	}

	pub fn build(self) -> Result<GraphDocument, DocumentError> {
		if self.schema_version != GraphDocument::CURRENT_SCHEMA_VERSION {
			return Err(DocumentError::UNSUPPORTED_SCHEMA_VERSION {
				found: self.schema_version,
				current: GraphDocument::CURRENT_SCHEMA_VERSION,
			});
		}
		let metadata = self.metadata.ok_or(DocumentError::MISSING_METADATA)?;
		metadata.validate()?;

		let mut ports = BTreeMap::new();
		for node in self.nodes.values() {
			validate_node(node)?;
			for port in &node.ports {
				register_port(&mut ports, port.id, port.direction, port.kind)?;
			}
		}
		for subgraph in self.subgraphs.values() {
			validate_subgraph(subgraph)?;
			for port in &subgraph.ports {
				register_port(&mut ports, port.id, port.direction, port.kind)?;
			}
			for node_id in &subgraph.nodes {
				if !self.nodes.contains_key(node_id) {
					return Err(DocumentError::MISSING_SUBGRAPH_NODE {
						subgraph: subgraph.id,
						node: *node_id,
					});
				}
			}
		}

		for group in self.groups.values() {
			validate_group(group)?;
			for node_id in &group.nodes {
				if !self.nodes.contains_key(node_id) {
					return Err(DocumentError::MISSING_GROUP_NODE {
						group: group.id,
						node: *node_id,
					});
				}
			}
		}

		for edge in self.edges.values() {
			if edge.id.0 == 0 {
				return Err(DocumentError::INVALID_ID {
					kind: "edge",
					value: edge.id.0,
				});
			}
			let from = ports
				.get(&edge.from)
				.ok_or(DocumentError::MISSING_PORT(edge.from))?;
			let to = ports
				.get(&edge.to)
				.ok_or(DocumentError::MISSING_PORT(edge.to))?;
			if from.0 != PortDirection::OUTPUT || to.0 != PortDirection::INPUT {
				return Err(DocumentError::INVALID_EDGE_DIRECTION(edge.id));
			}
			let expected_kind = match edge.kind {
				EdgeKind::EXECUTION => PortKind::EXECUTION,
				EdgeKind::DATA => PortKind::DATA,
			};
			if from.1 != expected_kind || to.1 != expected_kind {
				return Err(DocumentError::EDGE_KIND_MISMATCH {
					edge: edge.id,
					expected: expected_kind,
					from: from.1,
					to: to.1,
				});
			}
		}

		Ok(GraphDocument {
			schema_version: self.schema_version,
			metadata,
			nodes: self.nodes.into_values().collect(),
			edges: self.edges.into_values().collect(),
			groups: self.groups.into_values().collect(),
			subgraphs: self.subgraphs.into_values().collect(),
			extensions: self.extensions,
		})
	}

	fn port_exists(&self, id: PortId) -> bool {
		self.nodes
			.values()
			.any(|node| node.ports.iter().any(|port| port.id == id))
			|| self
				.subgraphs
				.values()
				.any(|subgraph| subgraph.ports.iter().any(|port| port.id == id))
	}
}

fn validate_node(node: &Node) -> Result<(), DocumentError> {
	if node.id.0 == 0 {
		return Err(DocumentError::INVALID_ID {
			kind: "node",
			value: node.id.0,
		});
	}
	if node.type_id.0.trim().is_empty() {
		return Err(DocumentError::EMPTY_NODE_TYPE);
	}
	let mut port_ids = BTreeSet::new();
	for port in &node.ports {
		if port.id.0 == 0 {
			return Err(DocumentError::INVALID_ID {
				kind: "port",
				value: port.id.0,
			});
		}
		if port
			.type_reference
			.as_ref()
			.is_some_and(|reference| reference.0.trim().is_empty())
		{
			return Err(DocumentError::EMPTY_TYPE_REFERENCE);
		}
		if !port_ids.insert(port.id) {
			return Err(DocumentError::DUPLICATE_PORT(port.id));
		}
	}
	let mut annotation_keys = BTreeSet::new();
	for annotation in &node.annotations {
		if annotation.key.trim().is_empty() {
			return Err(DocumentError::EMPTY_ANNOTATION_KEY);
		}
		if !annotation_keys.insert(annotation.key.as_str()) {
			return Err(DocumentError::DUPLICATE_ANNOTATION(annotation.key.clone()));
		}
	}
	for key in node.properties.keys() {
		if key.trim().is_empty() {
			return Err(DocumentError::EMPTY_PROPERTY_KEY);
		}
	}
	Ok(())
}

fn validate_group(group: &Group) -> Result<(), DocumentError> {
	if group.id.0 == 0 {
		return Err(DocumentError::INVALID_ID {
			kind: "group",
			value: group.id.0,
		});
	}
	if group.nodes.windows(2).any(|pair| pair[0] == pair[1]) {
		return Err(DocumentError::DUPLICATE_GROUP_MEMBER(group.id));
	}
	if group.nodes.iter().any(|node| node.0 == 0) {
		return Err(DocumentError::INVALID_ID {
			kind: "node",
			value: 0,
		});
	}
	Ok(())
}

fn validate_subgraph(subgraph: &Subgraph) -> Result<(), DocumentError> {
	if subgraph.id.0 == 0 {
		return Err(DocumentError::INVALID_ID {
			kind: "subgraph",
			value: subgraph.id.0,
		});
	}
	if subgraph.nodes.windows(2).any(|pair| pair[0] == pair[1]) {
		return Err(DocumentError::DUPLICATE_SUBGRAPH_NODE(subgraph.id));
	}
	if subgraph.nodes.iter().any(|node| node.0 == 0) {
		return Err(DocumentError::INVALID_ID {
			kind: "node",
			value: 0,
		});
	}
	if subgraph
		.ports
		.windows(2)
		.any(|pair| pair[0].id == pair[1].id)
	{
		return Err(DocumentError::DUPLICATE_PORT(subgraph.ports[1].id));
	}
	for port in &subgraph.ports {
		if port.id.0 == 0 {
			return Err(DocumentError::INVALID_ID {
				kind: "port",
				value: port.id.0,
			});
		}
		if port
			.type_reference
			.as_ref()
			.is_some_and(|reference| reference.0.trim().is_empty())
		{
			return Err(DocumentError::EMPTY_TYPE_REFERENCE);
		}
	}
	Ok(())
}

fn register_port(
	ports: &mut BTreeMap<PortId, (PortDirection, PortKind)>,
	id: PortId,
	direction: PortDirection,
	kind: PortKind,
) -> Result<(), DocumentError> {
	if id.0 == 0 {
		return Err(DocumentError::INVALID_ID {
			kind: "port",
			value: id.0,
		});
	}
	if ports.insert(id, (direction, kind)).is_some() {
		return Err(DocumentError::DUPLICATE_PORT(id));
	}
	Ok(())
}

/// Errors produced while creating or parsing a graph document.
#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DocumentError {
	INVALID_ID {
		kind: &'static str,
		value: u64,
	},
	EMPTY_NODE_TYPE,
	EMPTY_TYPE_REFERENCE,
	EMPTY_PLUGIN_ID,
	EMPTY_PLUGIN_VERSION,
	EMPTY_SEMANTIC_VERSION,
	EMPTY_COMPILER_VERSION,
	EMPTY_PROPERTY_KEY,
	EMPTY_ANNOTATION_KEY,
	EMPTY_EXTENSION_NAMESPACE,
	DUPLICATE_PLUGIN(String),
	DUPLICATE_NODE(NodeId),
	DUPLICATE_PORT(PortId),
	DUPLICATE_EDGE(EdgeId),
	DUPLICATE_GROUP(GroupId),
	DUPLICATE_GROUP_MEMBER(GroupId),
	DUPLICATE_SUBGRAPH(SubgraphId),
	DUPLICATE_SUBGRAPH_NODE(SubgraphId),
	DUPLICATE_ANNOTATION(String),
	DUPLICATE_EXTENSION(String),
	MISSING_METADATA,
	MISSING_NODE(NodeId),
	MISSING_PORT(PortId),
	MISSING_GROUP_NODE {
		group: GroupId,
		node: NodeId,
	},
	MISSING_SUBGRAPH_NODE {
		subgraph: SubgraphId,
		node: NodeId,
	},
	INVALID_EDGE_DIRECTION(EdgeId),
	EDGE_KIND_MISMATCH {
		edge: EdgeId,
		expected: PortKind,
		from: PortKind,
		to: PortKind,
	},
	UNSUPPORTED_SCHEMA_VERSION {
		found: u32,
		current: u32,
	},
	INVALID_JSON(String),
}

impl fmt::Display for DocumentError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(formatter, "{self:?}")
	}
}

impl std::error::Error for DocumentError {}

#[cfg(test)]
mod tests {
	use super::*;

	fn metadata() -> GraphMetadata {
		GraphMetadata::new(GraphId::new(1).unwrap(), "backend-v1", "krunt-dev")
			.unwrap()
			.with_plugin(PluginReference::new("backend-dsl", "0.1.0").unwrap())
			.unwrap()
	}

	fn node(id: u64, type_id: &str, ports: &[(u64, PortDirection, PortKind)]) -> Node {
		let mut node = Node::new(NodeId::new(id).unwrap(), NodeTypeId::new(type_id).unwrap());
		for (port_id, direction, kind) in ports {
			node.add_port(Port::new(PortId::new(*port_id).unwrap(), *direction, *kind))
				.unwrap();
		}
		node
	}

	fn route_graph(reverse_insertion: bool) -> GraphDocument {
		let route = node(
			1,
			"backend.route",
			&[(11, PortDirection::OUTPUT, PortKind::EXECUTION)],
		);
		let response = node(
			2,
			"backend.response",
			&[(12, PortDirection::INPUT, PortKind::EXECUTION)],
		);
		let edge = Edge::new(
			EdgeId::new(20).unwrap(),
			PortId::new(11).unwrap(),
			PortId::new(12).unwrap(),
			EdgeKind::EXECUTION,
		);
		let mut builder = GraphBuilder::new(metadata());
		if reverse_insertion {
			builder.add_node(response).unwrap();
			builder.add_node(route).unwrap();
		} else {
			builder.add_node(route).unwrap();
			builder.add_node(response).unwrap();
		}
		builder.add_edge(edge).unwrap();
		builder
			.add_extension("com.example.editor", serde_json::json!({"theme": "dark"}))
			.unwrap();
		builder.build().unwrap()
	}

	#[test]
	fn builds_a_structural_graph_with_explicit_port_and_edge_kinds() {
		let graph = route_graph(false);

		assert_eq!(
			graph.schema_version(),
			GraphDocument::CURRENT_SCHEMA_VERSION
		);
		assert_eq!(graph.nodes().len(), 2);
		assert_eq!(graph.edges()[0].kind(), EdgeKind::EXECUTION);
		assert_eq!(
			graph.nodes()[0].ports()[0].direction(),
			PortDirection::OUTPUT
		);
	}

	#[test]
	fn canonical_json_is_independent_of_insertion_order() {
		let first = route_graph(false).to_json().unwrap();
		let second = route_graph(true).to_json().unwrap();

		assert_eq!(first, second);
	}

	#[test]
	fn json_round_trip_revalidates_and_preserves_the_document() {
		let graph = route_graph(false);
		let json = graph.to_json().unwrap();
		let restored = GraphDocument::from_json(&json).unwrap();

		assert_eq!(restored, graph);
	}

	#[test]
	fn rejects_wrong_edge_direction() {
		let mut builder = GraphBuilder::new(metadata());
		builder
			.add_node(node(
				1,
				"backend.route",
				&[(11, PortDirection::INPUT, PortKind::EXECUTION)],
			))
			.unwrap();
		builder
			.add_node(node(
				2,
				"backend.response",
				&[(12, PortDirection::OUTPUT, PortKind::EXECUTION)],
			))
			.unwrap();
		builder
			.add_edge(Edge::new(
				EdgeId::new(20).unwrap(),
				PortId::new(11).unwrap(),
				PortId::new(12).unwrap(),
				EdgeKind::EXECUTION,
			))
			.unwrap();

		assert_eq!(
			builder.build(),
			Err(DocumentError::INVALID_EDGE_DIRECTION(
				EdgeId::new(20).unwrap()
			))
		);
	}

	#[test]
	fn rejects_mixed_edge_kinds() {
		let mut builder = GraphBuilder::new(metadata());
		builder
			.add_node(node(
				1,
				"backend.route",
				&[(11, PortDirection::OUTPUT, PortKind::DATA)],
			))
			.unwrap();
		builder
			.add_node(node(
				2,
				"backend.response",
				&[(12, PortDirection::INPUT, PortKind::DATA)],
			))
			.unwrap();
		builder
			.add_edge(Edge::new(
				EdgeId::new(20).unwrap(),
				PortId::new(11).unwrap(),
				PortId::new(12).unwrap(),
				EdgeKind::EXECUTION,
			))
			.unwrap();

		assert!(matches!(
			builder.build(),
			Err(DocumentError::EDGE_KIND_MISMATCH { .. })
		));
	}

	#[test]
	fn rejects_duplicate_port_ids_across_nodes() {
		let mut builder = GraphBuilder::new(metadata());
		builder
			.add_node(node(
				1,
				"backend.route",
				&[(11, PortDirection::OUTPUT, PortKind::EXECUTION)],
			))
			.unwrap();

		assert_eq!(
			builder.add_node(node(
				2,
				"backend.response",
				&[(11, PortDirection::INPUT, PortKind::EXECUTION)],
			)),
			Err(DocumentError::DUPLICATE_PORT(PortId::new(11).unwrap()))
		);
	}

	#[test]
	fn validates_groups_and_subgraph_boundaries_without_giving_them_execution_semantics() {
		let mut builder = GraphBuilder::new(metadata());
		builder
			.add_node(node(
				1,
				"backend.route",
				&[(11, PortDirection::OUTPUT, PortKind::EXECUTION)],
			))
			.unwrap();
		builder
			.add_group(Group::new(GroupId::new(30).unwrap(), [NodeId::new(1).unwrap()]).unwrap())
			.unwrap();
		builder
			.add_subgraph(
				Subgraph::new(
					SubgraphId::new(40).unwrap(),
					[NodeId::new(1).unwrap()],
					[SubgraphPort::new(
						PortId::new(41).unwrap(),
						PortDirection::OUTPUT,
						PortKind::EXECUTION,
					)],
				)
				.unwrap(),
			)
			.unwrap();

		let graph = builder.build().unwrap();
		assert_eq!(graph.groups().len(), 1);
		assert_eq!(graph.subgraphs()[0].ports().len(), 1);
	}

	#[test]
	fn duplicate_extensions_do_not_replace_the_original_value() {
		let mut builder = GraphBuilder::new(metadata());
		builder
			.add_extension("com.example.editor", serde_json::json!({"theme": "dark"}))
			.unwrap();

		assert_eq!(
			builder.add_extension("com.example.editor", serde_json::json!({"theme": "light"})),
			Err(DocumentError::DUPLICATE_EXTENSION(
				"com.example.editor".to_owned()
			))
		);
		assert_eq!(
			builder.build().unwrap().extensions()["com.example.editor"]["theme"],
			"dark"
		);
	}

	#[test]
	fn rejects_future_schema_versions() {
		let json = serde_json::json!({
			"schema_version": 99,
			"metadata": {
				"graph_id": 1,
				"semantic_version": "backend-v1",
				"compiler_version": "krunt-dev",
				"plugins": []
			},
			"nodes": [],
			"edges": [],
			"groups": [],
			"subgraphs": [],
			"extensions": {}
		})
		.to_string();

		assert_eq!(
			GraphDocument::from_json(&json),
			Err(DocumentError::UNSUPPORTED_SCHEMA_VERSION {
				found: 99,
				current: GraphDocument::CURRENT_SCHEMA_VERSION
			})
		);
	}

	#[test]
	fn rejects_zero_ids_from_json_instead_of_bypassing_constructors() {
		let json = serde_json::json!({
			"schema_version": 1,
			"metadata": {
				"graph_id": 0,
				"semantic_version": "backend-v1",
				"compiler_version": "krunt-dev",
				"plugins": []
			},
			"nodes": [],
			"edges": [],
			"groups": [],
			"subgraphs": [],
			"extensions": {}
		})
		.to_string();

		assert_eq!(
			GraphDocument::from_json(&json),
			Err(DocumentError::INVALID_ID {
				kind: "graph",
				value: 0
			})
		);
	}
}
