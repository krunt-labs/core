//! Stable, source-linked diagnostics shared by Krunt compiler stages.
//!
//! Diagnostics describe compiler findings. They do not render UI, localize
//! messages, or propose editor commands directly.

use krunt_graph_model::{EdgeId, GraphId, GroupId, NodeId, PortId, SubgraphId};
use serde::Serialize;
use std::cmp::Ordering;
use std::fmt;

/// Stable machine-readable diagnostic identity.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct DiagnosticCode(String);

impl DiagnosticCode {
	pub fn new(value: impl Into<String>) -> Result<Self, DiagnosticError> {
		let value = value.into();
		if value.trim().is_empty() {
			return Err(DiagnosticError::EMPTY_CODE);
		}
		Ok(Self(value))
	}

	pub fn as_str(&self) -> &str {
		&self.0
	}
}

/// User-visible importance of a diagnostic.
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum Severity {
	ERROR,
	WARNING,
	INFO,
}

/// Compiler stage that produced a diagnostic.
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum CompilerStage {
	PARSE,
	STRUCTURAL_VALIDATION,
	PLUGIN_RESOLUTION,
	SEMANTIC_ANALYSIS,
	TYPE_ANALYSIS,
	CONTRACT_VALIDATION,
	CAPABILITY_RESOLUTION,
	WHOLE_GRAPH_VALIDATION,
	LOWERING,
	OPTIMIZATION,
	CODE_GENERATION,
}

/// A graph element that can be highlighted or used as a diagnostic anchor.
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", content = "id")]
pub enum GraphElementId {
	GRAPH(GraphId),
	NODE(NodeId),
	PORT(PortId),
	EDGE(EdgeId),
	GROUP(GroupId),
	SUBGRAPH(SubgraphId),
}

impl GraphElementId {
	pub fn node(id: NodeId) -> Self {
		Self::NODE(id)
	}

	pub fn port(id: PortId) -> Self {
		Self::PORT(id)
	}

	pub fn edge(id: EdgeId) -> Self {
		Self::EDGE(id)
	}
}

/// A secondary source element relevant to a diagnostic.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RelatedElement {
	element: GraphElementId,
	role: Option<String>,
}

impl RelatedElement {
	pub fn new(element: GraphElementId) -> Self {
		Self {
			element,
			role: None,
		}
	}

	pub fn with_role(mut self, role: impl Into<String>) -> Result<Self, DiagnosticError> {
		let role = role.into();
		if role.trim().is_empty() {
			return Err(DiagnosticError::EMPTY_RELATED_ROLE);
		}
		self.role = Some(role);
		Ok(self)
	}

	pub fn element(&self) -> GraphElementId {
		self.element
	}

	pub fn role(&self) -> Option<&str> {
		self.role.as_deref()
	}
}

/// Structured help attached to a diagnostic.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DiagnosticHelp {
	summary: String,
	fix: Option<FixProposal>,
}

impl DiagnosticHelp {
	pub fn new(summary: impl Into<String>) -> Result<Self, DiagnosticError> {
		let summary = summary.into();
		if summary.trim().is_empty() {
			return Err(DiagnosticError::EMPTY_HELP_SUMMARY);
		}
		Ok(Self { summary, fix: None })
	}

	pub fn with_fix(mut self, fix: FixProposal) -> Self {
		self.fix = Some(fix);
		self
	}

	pub fn summary(&self) -> &str {
		&self.summary
	}

	pub fn fix(&self) -> Option<&FixProposal> {
		self.fix.as_ref()
	}
}

/// Structured, renderer-neutral fix information.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FixProposal {
	action: String,
	arguments: Vec<(String, String)>,
}

impl FixProposal {
	pub fn new(action: impl Into<String>) -> Result<Self, DiagnosticError> {
		let action = action.into();
		if action.trim().is_empty() {
			return Err(DiagnosticError::EMPTY_FIX_ACTION);
		}
		Ok(Self {
			action,
			arguments: Vec::new(),
		})
	}

	pub fn with_argument(
		mut self,
		key: impl Into<String>,
		value: impl Into<String>,
	) -> Result<Self, DiagnosticError> {
		let key = key.into();
		if key.trim().is_empty() {
			return Err(DiagnosticError::EMPTY_FIX_ARGUMENT_KEY);
		}
		if self.arguments.iter().any(|(existing, _)| existing == &key) {
			return Err(DiagnosticError::DUPLICATE_FIX_ARGUMENT(key));
		}
		self.arguments.push((key, value.into()));
		self.arguments.sort();
		Ok(self)
	}

	pub fn action(&self) -> &str {
		&self.action
	}

	pub fn arguments(&self) -> &[(String, String)] {
		&self.arguments
	}
}

/// One compiler finding with stable source traceability.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Diagnostic {
	code: DiagnosticCode,
	severity: Severity,
	stage: CompilerStage,
	message: String,
	primary: GraphElementId,
	related: Vec<RelatedElement>,
	help: Option<DiagnosticHelp>,
}

impl Diagnostic {
	pub fn new(
		code: DiagnosticCode,
		severity: Severity,
		stage: CompilerStage,
		message: impl Into<String>,
		primary: GraphElementId,
	) -> Result<Self, DiagnosticError> {
		let message = message.into();
		if message.trim().is_empty() {
			return Err(DiagnosticError::EMPTY_MESSAGE);
		}
		Ok(Self {
			code,
			severity,
			stage,
			message,
			primary,
			related: Vec::new(),
			help: None,
		})
	}

	pub fn with_related(mut self, related: impl IntoIterator<Item = RelatedElement>) -> Self {
		self.related.extend(related);
		self.related.sort_by(|left, right| {
			left.element
				.cmp(&right.element)
				.then_with(|| left.role.cmp(&right.role))
		});
		self.related
			.dedup_by(|left, right| left.element == right.element && left.role == right.role);
		self
	}

	pub fn with_help(mut self, help: DiagnosticHelp) -> Self {
		self.help = Some(help);
		self
	}

	pub fn code(&self) -> &DiagnosticCode {
		&self.code
	}

	pub fn severity(&self) -> Severity {
		self.severity
	}

	pub fn stage(&self) -> CompilerStage {
		self.stage
	}

	pub fn message(&self) -> &str {
		&self.message
	}

	pub fn primary(&self) -> GraphElementId {
		self.primary
	}

	pub fn related(&self) -> &[RelatedElement] {
		&self.related
	}

	pub fn help(&self) -> Option<&DiagnosticHelp> {
		self.help.as_ref()
	}
}

/// Deterministically ordered compiler diagnostics.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct Diagnostics {
	items: Vec<Diagnostic>,
}

impl Diagnostics {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn push(&mut self, diagnostic: Diagnostic) {
		self.items.push(diagnostic);
	}

	pub fn extend(&mut self, diagnostics: impl IntoIterator<Item = Diagnostic>) {
		self.items.extend(diagnostics);
	}

	pub fn sort_canonical(&mut self) {
		self.items.sort_by(compare_diagnostics);
	}

	pub fn has_errors(&self) -> bool {
		self.items
			.iter()
			.any(|item| item.severity == Severity::ERROR)
	}

	pub fn is_empty(&self) -> bool {
		self.items.is_empty()
	}

	pub fn as_slice(&self) -> &[Diagnostic] {
		&self.items
	}

	pub fn into_vec(self) -> Vec<Diagnostic> {
		self.items
	}
}

fn compare_diagnostics(left: &Diagnostic, right: &Diagnostic) -> Ordering {
	left.stage
		.cmp(&right.stage)
		.then_with(|| left.severity.cmp(&right.severity))
		.then_with(|| left.code.cmp(&right.code))
		.then_with(|| left.primary.cmp(&right.primary))
		.then_with(|| left.message.cmp(&right.message))
}

/// Errors raised while constructing a diagnostic.
#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DiagnosticError {
	EMPTY_CODE,
	EMPTY_MESSAGE,
	EMPTY_RELATED_ROLE,
	EMPTY_HELP_SUMMARY,
	EMPTY_FIX_ACTION,
	EMPTY_FIX_ARGUMENT_KEY,
	DUPLICATE_FIX_ARGUMENT(String),
}

impl fmt::Display for DiagnosticError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(formatter, "{self:?}")
	}
}

impl std::error::Error for DiagnosticError {}

#[cfg(test)]
mod tests {
	use super::*;

	fn code(value: &str) -> DiagnosticCode {
		DiagnosticCode::new(value).unwrap()
	}

	fn diagnostic(
		code_value: &str,
		severity: Severity,
		stage: CompilerStage,
		primary: GraphElementId,
	) -> Diagnostic {
		Diagnostic::new(code(code_value), severity, stage, code_value, primary).unwrap()
	}

	#[test]
	fn diagnostics_keep_structured_source_traceability_and_help() {
		let fix = FixProposal::new("connect_ports")
			.unwrap()
			.with_argument("from", "11")
			.unwrap()
			.with_argument("to", "12")
			.unwrap();
		let help = DiagnosticHelp::new("Connect an output port to an input port.")
			.unwrap()
			.with_fix(fix);
		let diagnostic = diagnostic(
			"GRAPH.INVALID_EDGE_DIRECTION",
			Severity::ERROR,
			CompilerStage::STRUCTURAL_VALIDATION,
			GraphElementId::edge(EdgeId::new(20).unwrap()),
		)
		.with_related([
			RelatedElement::new(GraphElementId::port(PortId::new(12).unwrap()))
				.with_role("target")
				.unwrap(),
		]);
		let diagnostic = diagnostic.with_help(help);

		assert_eq!(
			diagnostic.primary(),
			GraphElementId::EDGE(EdgeId::new(20).unwrap())
		);
		assert_eq!(diagnostic.related()[0].role(), Some("target"));
		assert_eq!(
			diagnostic.help().unwrap().fix().unwrap().action(),
			"connect_ports"
		);
	}

	#[test]
	fn related_elements_are_sorted_and_deduplicated() {
		let diagnostic = diagnostic(
			"GRAPH.TEST",
			Severity::INFO,
			CompilerStage::PARSE,
			GraphElementId::node(NodeId::new(1).unwrap()),
		)
		.with_related([
			RelatedElement::new(GraphElementId::port(PortId::new(12).unwrap())),
			RelatedElement::new(GraphElementId::port(PortId::new(11).unwrap())),
			RelatedElement::new(GraphElementId::port(PortId::new(12).unwrap())),
		]);

		assert_eq!(diagnostic.related().len(), 2);
		assert_eq!(
			diagnostic.related()[0].element(),
			GraphElementId::PORT(PortId::new(11).unwrap())
		);
	}

	#[test]
	fn diagnostics_have_canonical_order_and_error_detection() {
		let mut first = Diagnostics::new();
		first.push(diagnostic(
			"TYPE.MISMATCH",
			Severity::ERROR,
			CompilerStage::TYPE_ANALYSIS,
			GraphElementId::node(NodeId::new(2).unwrap()),
		));
		first.push(diagnostic(
			"GRAPH.MISSING_PORT",
			Severity::ERROR,
			CompilerStage::STRUCTURAL_VALIDATION,
			GraphElementId::node(NodeId::new(1).unwrap()),
		));

		let mut second = Diagnostics::new();
		second.push(diagnostic(
			"GRAPH.MISSING_PORT",
			Severity::ERROR,
			CompilerStage::STRUCTURAL_VALIDATION,
			GraphElementId::node(NodeId::new(1).unwrap()),
		));
		second.push(diagnostic(
			"TYPE.MISMATCH",
			Severity::ERROR,
			CompilerStage::TYPE_ANALYSIS,
			GraphElementId::node(NodeId::new(2).unwrap()),
		));

		first.sort_canonical();
		second.sort_canonical();
		assert_eq!(first, second);
		assert!(first.has_errors());
	}

	#[test]
	fn serialized_diagnostics_are_stable_after_canonical_sorting() {
		let mut diagnostics = Diagnostics::new();
		diagnostics.push(diagnostic(
			"B.CODE",
			Severity::WARNING,
			CompilerStage::SEMANTIC_ANALYSIS,
			GraphElementId::node(NodeId::new(2).unwrap()),
		));
		diagnostics.push(diagnostic(
			"A.CODE",
			Severity::WARNING,
			CompilerStage::SEMANTIC_ANALYSIS,
			GraphElementId::node(NodeId::new(1).unwrap()),
		));
		diagnostics.sort_canonical();
		let first = serde_json::to_string(&diagnostics).unwrap();
		diagnostics.sort_canonical();
		let second = serde_json::to_string(&diagnostics).unwrap();

		assert_eq!(first, second);
	}
}
