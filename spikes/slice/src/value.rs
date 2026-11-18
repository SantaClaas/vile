//! The bodies that replace a function's body, chosen from its written return type alone.
//!
//! Phase 2 asks tsrs for the type instead, which also covers inferred return types and aliases.

use oxc_ast::ast::{TSType, TSTypeName, TSTypeOperatorOperator};

/// What a function could return instead of what it computes.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Value {
    /// The function returns nothing, so its body becomes `{}`.
    Nothing,
    /// An expression the body returns, such as `0` or `""`.
    Expression(String),
}

fn expressions(values: &[&str]) -> Vec<Value> {
    values
        .iter()
        .map(|value| Value::Expression((*value).to_owned()))
        .collect()
}

/// Whether a type reference names an array type, as in `Array<T>`.
fn is_array_reference(name: &TSTypeName<'_>) -> bool {
    let TSTypeName::IdentifierReference(identifier) = name else {
        return false;
    };
    matches!(identifier.name.as_str(), "Array" | "ReadonlyArray")
}

/// The type a `Promise<T>` resolves to, or `None` when the type isn't a promise.
fn find_promised<'t, 'a>(type_annotation: &'t TSType<'a>) -> Option<&'t TSType<'a>> {
    let TSType::TSTypeReference(reference) = type_annotation else {
        return None;
    };
    let TSTypeName::IdentifierReference(identifier) = &reference.type_name else {
        return None;
    };
    if identifier.name.as_str() != "Promise" {
        return None;
    }
    reference.type_arguments.as_ref()?.params.first()
}

/// The values of a type, in the order the plan lists them, or none when the written type alone
/// doesn't say (a type alias, a brand, an object type).
fn list_values(type_annotation: &TSType<'_>, source: &str) -> Vec<Value> {
    match type_annotation {
        TSType::TSVoidKeyword(_) => vec![Value::Nothing],
        TSType::TSTypePredicate(predicate) if predicate.asserts => vec![Value::Nothing],
        TSType::TSBooleanKeyword(_) | TSType::TSTypePredicate(_) => expressions(&["true", "false"]),
        TSType::TSNumberKeyword(_) => expressions(&["0", "1", "-1"]),
        TSType::TSStringKeyword(_) => expressions(&["\"\"", "\"xyzzy\""]),
        TSType::TSUndefinedKeyword(_) => expressions(&["undefined"]),
        TSType::TSNullKeyword(_) => expressions(&["null"]),
        TSType::TSArrayType(_) => expressions(&["[]"]),
        TSType::TSTypeOperatorType(operator)
            if operator.operator == TSTypeOperatorOperator::Readonly =>
        {
            list_values(&operator.type_annotation, source)
        }
        TSType::TSTypeReference(reference) if is_array_reference(&reference.type_name) => {
            expressions(&["[]"])
        }
        TSType::TSLiteralType(literal) => expressions(&[literal.span.source_text(source)]),
        TSType::TSParenthesizedType(parenthesized) => {
            list_values(&parenthesized.type_annotation, source)
        }
        TSType::TSUnionType(union) => list_union(&union.types, source),
        _ => Vec::new(),
    }
}

/// Each member's values, `undefined` first, without repeats. A `void` member counts as
/// `undefined`, which is what a caller sees.
fn list_union(types: &[TSType<'_>], source: &str) -> Vec<Value> {
    let mut values: Vec<Value> = Vec::new();
    let undefined_first = types
        .iter()
        .filter(|member| matches!(member, TSType::TSUndefinedKeyword(_)))
        .chain(
            types
                .iter()
                .filter(|member| !matches!(member, TSType::TSUndefinedKeyword(_))),
        );
    for member in undefined_first {
        for value in list_values(member, source) {
            let value = match value {
                Value::Nothing => Value::Expression("undefined".to_owned()),
                expression @ Value::Expression(_) => expression,
            };
            if !values.contains(&value) {
                values.push(value);
            }
        }
    }
    values
}

/// The bodies that replace a function's body, braces included, for its written return type.
///
/// An `async` function's `Promise<T>` gives the values of `T`; a promise from a function that
/// isn't `async` gives none, since its values would need wrapping.
#[must_use]
pub fn list_bodies(return_type: &TSType<'_>, is_async: bool, source: &str) -> Vec<String> {
    let promised = find_promised(return_type);
    let values = match (is_async, promised) {
        (true, Some(promised)) => list_values(promised, source),
        (false, Some(_)) => Vec::new(),
        (_, None) => list_values(return_type, source),
    };
    values
        .into_iter()
        .map(|value| match value {
            Value::Nothing => "{}".to_owned(),
            Value::Expression(expression) => format!("{{ return {expression}; }}"),
        })
        .collect()
}
