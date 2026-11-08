//! DSL-independent compile-time types and assignability for Krunt.
//!
//! This first slice intentionally models opaque named types only. Backend
//! plugins can introduce richer type syntax later without making the graph
//! model or compiler orchestration depend on a particular DSL.

use std::fmt;

/// A validated type name used by the compiler's initial type relation.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct TypeName(String);

impl TypeName {
	pub fn new(value: impl Into<String>) -> Result<Self, TypeError> {
		let value = value.into();
		if value.trim().is_empty() {
			return Err(TypeError::EMPTY_NAME);
		}
		Ok(Self(value))
	}

	pub fn as_str(&self) -> &str {
		&self.0
	}
}

/// A compiler-facing type representation.
#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Type {
	NAMED(TypeName),
	ANY,
}

impl Type {
	pub fn named(value: impl Into<String>) -> Result<Self, TypeError> {
		Ok(Self::NAMED(TypeName::new(value)?))
	}

	/// Parse a graph-facing type reference, including the open response-body
	/// type used by nodes whose payload is supplied by the graph.
	pub fn from_reference(value: impl Into<String>) -> Result<Self, TypeError> {
		let value = value.into();
		if value.eq_ignore_ascii_case("any") {
			Ok(Self::ANY)
		} else {
			Self::named(value)
		}
	}

	pub fn as_name(&self) -> Option<&str> {
		match self {
			Self::NAMED(name) => Some(name.as_str()),
			Self::ANY => None,
		}
	}
}

/// Check whether a produced value can be connected to a consumed value.
pub fn is_assignable(from: &Type, to: &Type) -> bool {
	match (from, to) {
		(_, Type::ANY) => true,
		(Type::NAMED(left), Type::NAMED(right)) => left == right,
		(Type::ANY, Type::NAMED(_)) => false,
	}
}

/// Explain a failed assignability check.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssignabilityError {
	from: Type,
	to: Type,
}

impl AssignabilityError {
	pub fn from(&self) -> &Type {
		&self.from
	}

	pub fn to(&self) -> &Type {
		&self.to
	}
}

pub fn check_assignable(from: &Type, to: &Type) -> Result<(), AssignabilityError> {
	if is_assignable(from, to) {
		Ok(())
	} else {
		Err(AssignabilityError {
			from: from.clone(),
			to: to.clone(),
		})
	}
}

/// Errors raised while constructing the initial type representation.
#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TypeError {
	EMPTY_NAME,
}

impl fmt::Display for TypeError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(formatter, "{self:?}")
	}
}

impl std::error::Error for TypeError {}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn named_types_are_assignable_only_when_their_names_match() {
		let request = Type::named("Request").unwrap();
		let same_request = Type::named("Request").unwrap();
		let response = Type::named("Response").unwrap();

		assert!(is_assignable(&request, &same_request));
		assert!(!is_assignable(&request, &response));
		assert!(check_assignable(&request, &response).is_err());
	}

	#[test]
	fn any_accepts_every_produced_type_but_is_not_a_concrete_output() {
		let request = Type::named("Request").unwrap();

		assert!(is_assignable(&request, &Type::ANY));
		assert!(!is_assignable(&Type::ANY, &request));
	}

	#[test]
	fn empty_type_names_are_rejected() {
		assert_eq!(TypeName::new(" "), Err(TypeError::EMPTY_NAME));
	}

	#[test]
	fn graph_references_parse_any_as_an_open_type() {
		assert_eq!(Type::from_reference("ANY").unwrap(), Type::ANY);
		assert_eq!(
			Type::from_reference("User").unwrap(),
			Type::named("User").unwrap()
		);
	}
}
