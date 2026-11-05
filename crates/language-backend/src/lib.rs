//! Contracts for lowering validated universal IR into a language-specific IR.
//!
//! A language backend owns language syntax and language-level representation,
//! but it must consume only validated universal IR. Framework concerns such as
//! NestJS decorators and project layout belong to a later framework boundary.

use krunt_ir::IRGraph;
use std::error::Error;

/// A pluggable lowering implementation for one target language.
///
/// The associated output keeps each language's IR private to its backend while
/// preserving one stable seam for compiler orchestration. Implementations must
/// be deterministic for equivalent input graphs.
pub trait LanguageBackend {
	type Output;
	type Error: Error;

	/// Stable language-backend identity, such as `typescript`.
	fn backend_id(&self) -> &'static str;

	/// Lower validated universal IR into this backend's language IR.
	fn lower(&self, graph: &IRGraph) -> Result<Self::Output, Self::Error>;
}
