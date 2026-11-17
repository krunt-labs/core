//! DSL-independent compile-time types and assignability for Krunt.
//!
//! The type system owns reusable primitive and named type semantics. Backend
//! plugins can compose these types into domain schemas without making the
//! graph model or compiler orchestration depend on a particular DSL.

use std::collections::BTreeMap;
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
	STRING,
	BOOLEAN,
	INTEGER,
	FLOAT,
	NUMBER,
	NULL,
	VOID,
	ARRAY(Box<Type>),
	RECORD(BTreeMap<String, RecordField>),
	NAMED(TypeName),
	ANY,
}

/// One field in a structural record type.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RecordField {
	type_value: Type,
	required: bool,
}

impl RecordField {
	pub fn new(type_value: Type, required: bool) -> Self {
		Self {
			type_value,
			required,
		}
	}

	pub fn type_value(&self) -> &Type {
		&self.type_value
	}

	pub fn required(&self) -> bool {
		self.required
	}
}

impl Type {
	pub fn named(value: impl Into<String>) -> Result<Self, TypeError> {
		Ok(Self::NAMED(TypeName::new(value)?))
	}

	/// Parse a graph-facing type reference, including the open response-body
	/// type used by nodes whose payload is supplied by the graph.
	pub fn from_reference(value: impl Into<String>) -> Result<Self, TypeError> {
		let value = value.into();
		if let Some(open) = value.find('<') {
			if !value[..open].eq_ignore_ascii_case("Array") {
				return Self::named(value);
			}
			let inner = &value[open + 1..];
			let Some(inner) = inner.strip_suffix('>') else {
				return Err(TypeError::INVALID_REFERENCE);
			};
			if inner.trim().is_empty() {
				return Err(TypeError::INVALID_REFERENCE);
			}
			return Ok(Self::ARRAY(Box::new(Self::from_reference(inner.trim())?)));
		}
		match value.to_ascii_lowercase().as_str() {
			"string" => Ok(Self::STRING),
			"boolean" => Ok(Self::BOOLEAN),
			"integer" => Ok(Self::INTEGER),
			"float" => Ok(Self::FLOAT),
			"number" => Ok(Self::NUMBER),
			"null" => Ok(Self::NULL),
			"void" => Ok(Self::VOID),
			"any" => Ok(Self::ANY),
			_ => Self::named(value),
		}
	}

	pub fn array(element: Type) -> Self {
		Self::ARRAY(Box::new(element))
	}

	pub fn record(fields: BTreeMap<String, RecordField>) -> Self {
		Self::RECORD(fields)
	}

	pub fn array_element(&self) -> Option<&Type> {
		match self {
			Self::ARRAY(element) => Some(element),
			_ => None,
		}
	}

	/// Whether this type is a concrete scalar value suitable for a flat schema.
	pub fn is_scalar(&self) -> bool {
		matches!(
			self,
			Self::STRING | Self::BOOLEAN | Self::INTEGER | Self::FLOAT | Self::NUMBER | Self::NULL
		)
	}

	pub fn as_name(&self) -> Option<&str> {
		match self {
			Self::NAMED(name) => Some(name.as_str()),
			Self::STRING
			| Self::BOOLEAN
			| Self::INTEGER
			| Self::FLOAT
			| Self::NUMBER
			| Self::NULL
			| Self::VOID
			| Self::ARRAY(_)
			| Self::RECORD(_)
			| Self::ANY => None,
		}
	}
}

/// Check whether a produced value can be connected to a consumed value.
pub fn is_assignable(from: &Type, to: &Type) -> bool {
	match (from, to) {
		(_, Type::ANY) => true,
		(Type::INTEGER, Type::NUMBER) | (Type::FLOAT, Type::NUMBER) => true,
		(Type::ARRAY(left), Type::ARRAY(right)) => is_assignable(left, right),
		(Type::RECORD(left), Type::RECORD(right)) => right.iter().all(|(name, target)| {
			let Some(source) = left.get(name) else {
				return !target.required();
			};
			(!target.required() || source.required())
				&& is_assignable(source.type_value(), target.type_value())
		}),
		(left, right) if left == right => true,
		(Type::NAMED(left), Type::NAMED(right)) => left == right,
		(Type::ANY, Type::NAMED(_)) => false,
		(Type::ANY, _) => false,
		_ => false,
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
	INVALID_REFERENCE,
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

	#[test]
	fn graph_references_parse_primitive_types_case_insensitively() {
		assert_eq!(Type::from_reference("String").unwrap(), Type::STRING);
		assert_eq!(Type::from_reference("boolean").unwrap(), Type::BOOLEAN);
		assert_eq!(Type::from_reference("INTEGER").unwrap(), Type::INTEGER);
		assert_eq!(Type::from_reference("Float").unwrap(), Type::FLOAT);
		assert_eq!(Type::from_reference("Number").unwrap(), Type::NUMBER);
		assert_eq!(Type::from_reference("Null").unwrap(), Type::NULL);
		assert_eq!(Type::from_reference("Void").unwrap(), Type::VOID);
	}

	#[test]
	fn numeric_primitives_widen_to_number() {
		assert!(is_assignable(&Type::INTEGER, &Type::NUMBER));
		assert!(is_assignable(&Type::FLOAT, &Type::NUMBER));
		assert!(!is_assignable(&Type::NUMBER, &Type::INTEGER));
		assert!(!is_assignable(&Type::ANY, &Type::NUMBER));
	}

	#[test]
	fn array_references_and_assignability_are_structural() {
		let strings = Type::from_reference("Array<String>").unwrap();
		let values = Type::array(Type::ANY);

		assert_eq!(strings, Type::array(Type::STRING));
		assert_eq!(
			Type::from_reference("ARRAY<Boolean>").unwrap(),
			Type::array(Type::BOOLEAN)
		);
		assert!(is_assignable(&strings, &values));
		assert!(!is_assignable(&values, &strings));
		assert_eq!(
			Type::from_reference("Array<>"),
			Err(TypeError::INVALID_REFERENCE)
		);
	}

	#[test]
	fn records_respect_required_fields_and_nested_assignability() {
		let mut source_fields = BTreeMap::new();
		source_fields.insert("id".to_string(), RecordField::new(Type::INTEGER, true));
		source_fields.insert(
			"nickname".to_string(),
			RecordField::new(Type::STRING, false),
		);
		let source = Type::record(source_fields);

		let mut target_fields = BTreeMap::new();
		target_fields.insert("id".to_string(), RecordField::new(Type::NUMBER, true));
		target_fields.insert("avatar".to_string(), RecordField::new(Type::STRING, false));
		let target = Type::record(target_fields);

		assert!(is_assignable(&source, &target));

		let mut required_target_fields = BTreeMap::new();
		required_target_fields.insert("nickname".to_string(), RecordField::new(Type::STRING, true));
		assert!(!is_assignable(
			&source,
			&Type::record(required_target_fields)
		));
	}
}
