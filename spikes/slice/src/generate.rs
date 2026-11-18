//! Finds the sites of the two operators the slice covers and turns them into mutants: `<`
//! comparisons (relational) and function bodies with a written return type (body replacement).

use std::collections::BTreeMap;

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    ArrowFunctionExpression, BinaryExpression, BinaryOperator, Class, Expression, Function,
    MethodDefinition, TSTypeAnnotation, VariableDeclarator,
};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType};
use oxc_syntax::scope::ScopeFlags;
use vile_core::{
    mutant::{Identity, Mutant, MutantId},
    operator::Operator,
    patch::Patch,
    span::Span,
};

use crate::value;

/// Why a file gives no mutants.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GenerateError {
    #[error("the file isn't TypeScript or JavaScript")]
    UnknownExtension,
    #[error("the file doesn't parse: {0}")]
    Syntax(String),
}

/// A place an operator applies, with one of its replacements.
#[derive(Debug)]
struct Site {
    scope: Vec<String>,
    operator: Operator,
    span: oxc_span::Span,
    replacement: String,
}

/// Walks a file's syntax tree, keeping the names of the functions and classes around each site.
struct Finder<'s> {
    source: &'s str,
    scope: Vec<String>,
    sites: Vec<Site>,
}

/// The `<` between two operands, found in the text between them since Oxc keeps no span for
/// the operator itself.
fn find_operator(source: &str, expression: &BinaryExpression<'_>) -> Option<oxc_span::Span> {
    let between_start = expression.left.span().end;
    let between = oxc_span::Span::new(between_start, expression.right.span().start);
    let offset = between.source_text(source).find('<')?;
    let start = between_start + u32::try_from(offset).ok()?;
    Some(oxc_span::Span::new(start, start + 1))
}

/// Whether two bodies are the same apart from whitespace, so replacing one with the other
/// changes nothing.
fn is_same_body(original: &str, replacement: &str) -> bool {
    let without_whitespace =
        |text: &str| -> String { text.chars().filter(|c| !c.is_whitespace()).collect() };
    without_whitespace(original) == without_whitespace(replacement)
}

impl Finder<'_> {
    fn add(&mut self, operator: Operator, span: oxc_span::Span, replacement: &str) {
        self.sites.push(Site {
            scope: self.scope.clone(),
            operator,
            span,
            replacement: replacement.to_owned(),
        });
    }

    fn add_bodies(
        &mut self,
        return_type: Option<&TSTypeAnnotation<'_>>,
        is_async: bool,
        body: oxc_span::Span,
    ) {
        let Some(return_type) = return_type else {
            return;
        };
        let original = body.source_text(self.source);
        for replacement in value::list_bodies(&return_type.type_annotation, is_async, self.source) {
            if !is_same_body(original, &replacement) {
                self.add(Operator::BodyReplacement, body, &replacement);
            }
        }
    }

    fn walk_named(&mut self, name: Option<&str>, walk: impl FnOnce(&mut Self)) {
        let Some(name) = name else {
            walk(self);
            return;
        };
        self.scope.push(name.to_owned());
        walk(self);
        self.scope.pop();
    }
}

impl<'a> Visit<'a> for Finder<'_> {
    fn visit_binary_expression(&mut self, expression: &BinaryExpression<'a>) {
        if expression.operator == BinaryOperator::LessThan
            && let Some(operator) = find_operator(self.source, expression)
        {
            self.add(Operator::Relational, operator, "<=");
            self.add(Operator::Relational, operator, "!==");
            self.add(Operator::Relational, expression.span, "false");
        }
        walk::walk_binary_expression(self, expression);
    }

    fn visit_function(&mut self, function: &Function<'a>, flags: ScopeFlags) {
        let name = function.id.as_ref().map(|id| id.name.as_str());
        self.walk_named(name, |finder| {
            let is_generator = function.generator;
            if let Some(body) = function.body.as_deref().filter(|_| !is_generator) {
                finder.add_bodies(function.return_type.as_deref(), function.r#async, body.span);
            }
            walk::walk_function(finder, function, flags);
        });
    }

    fn visit_arrow_function_expression(&mut self, function: &ArrowFunctionExpression<'a>) {
        self.add_bodies(
            function.return_type.as_deref(),
            function.r#async,
            function.body.span(),
        );
        walk::walk_arrow_function_expression(self, function);
    }

    fn visit_class(&mut self, class: &Class<'a>) {
        let name = class.id.as_ref().map(|id| id.name.as_str());
        self.walk_named(name, |finder| walk::walk_class(finder, class));
    }

    fn visit_method_definition(&mut self, method: &MethodDefinition<'a>) {
        let key = method.key.static_name();
        let name = key.as_ref().and_then(|key| key.as_str());
        self.walk_named(name, |finder| walk::walk_method_definition(finder, method));
    }

    fn visit_variable_declarator(&mut self, declarator: &VariableDeclarator<'a>) {
        let is_function = matches!(
            declarator.init,
            Some(Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_))
        );
        let identifier = declarator.id.get_identifier_name();
        let name = identifier.filter(|_| is_function).map(|name| name.as_str());
        self.walk_named(name, |finder| {
            walk::walk_variable_declarator(finder, declarator);
        });
    }
}

/// The site as a mutant, numbered among the sites in the same scope with the same text.
fn build_mutant(
    file: &str,
    source: &str,
    site: Site,
    occurrences: &mut BTreeMap<(Vec<String>, &'static str, String, String), u32>,
) -> Mutant {
    let original = site.span.source_text(source).to_owned();
    let key = (
        site.scope.clone(),
        // `Operator` isn't `Ord`, so the key holds its name.
        site.operator.name(),
        original.clone(),
        site.replacement.clone(),
    );
    let occurrence = occurrences.entry(key).or_insert(0);
    let id = MutantId::new(&Identity {
        file,
        scope: &site.scope,
        original: &original,
        operator: site.operator,
        replacement: &site.replacement,
        occurrence: *occurrence,
    });
    *occurrence += 1;
    let span = Span::new(site.span.start, site.span.end).expect("Oxc's spans aren't reversed");
    Mutant {
        id,
        file: file.to_owned(),
        operator: site.operator,
        original,
        patch: Patch {
            span,
            replacement: site.replacement,
        },
        range: span
            .locate(source)
            .expect("Oxc's spans fall on character boundaries"),
    }
}

/// Every mutant of one file, in source order. `file` is its path from the project's root.
///
/// # Errors
///
/// [`GenerateError`] when the file isn't TypeScript or JavaScript, or doesn't parse.
pub fn generate(file: &str, source: &str) -> Result<Vec<Mutant>, GenerateError> {
    let source_type = SourceType::from_path(file).map_err(|_| GenerateError::UnknownExtension)?;
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, source_type).parse();
    if let Some(error) = parsed.diagnostics.errors().next() {
        return Err(GenerateError::Syntax(error.to_string()));
    }
    let mut finder = Finder {
        source,
        scope: Vec::new(),
        sites: Vec::new(),
    };
    finder.visit_program(&parsed.program);
    let mut sites = finder.sites;
    sites.sort_by_key(|site| (site.span.start, site.span.end));
    let mut occurrences = BTreeMap::new();
    Ok(sites
        .into_iter()
        .map(|site| build_mutant(file, source, site, &mut occurrences))
        .collect())
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeSet, fs, path::Path};

    use super::generate;

    /// The (original, replacement) pairs of the mutants, in order.
    fn list_changes(source: &str) -> Vec<(String, String)> {
        generate("src/example.ts", source)
            .expect("the test source parses")
            .into_iter()
            .map(|mutant| (mutant.original, mutant.patch.replacement))
            .collect()
    }

    fn pairs(changes: &[(&str, &str)]) -> Vec<(String, String)> {
        changes
            .iter()
            .map(|(original, replacement)| ((*original).to_owned(), (*replacement).to_owned()))
            .collect()
    }

    #[test]
    fn finds_the_sufficient_set_for_less_than() {
        assert_eq!(
            list_changes("const isSmall = a < b;"),
            pairs(&[("a < b", "false"), ("<", "<="), ("<", "!==")])
        );
    }

    #[test]
    fn leaves_other_comparisons_and_type_arguments_alone() {
        assert_eq!(
            list_changes("const a = b <= c; const d: Array<number> = [];"),
            pairs(&[])
        );
    }

    #[test]
    fn finds_the_operator_past_a_comment_without_one() {
        let changes = list_changes("const isSmall = a /* left */ < b;");
        assert_eq!(changes[1], ("<".to_owned(), "<=".to_owned()));
    }

    #[test]
    fn replaces_bodies_by_their_written_return_type() {
        let body = "{\n  return input.length;\n}";
        let source = format!("function count(input: string): number {body}");
        assert_eq!(
            list_changes(&source),
            pairs(&[
                (body, "{ return 0; }"),
                (body, "{ return 1; }"),
                (body, "{ return -1; }")
            ])
        );
    }

    #[test]
    fn gives_undefined_first_for_an_optional_value() {
        let changes = list_changes("const find = (): string | undefined => names[0];");
        let replacements: Vec<&str> = changes.iter().map(|(_, r)| r.as_str()).collect();
        assert_eq!(
            replacements,
            [
                "{ return undefined; }",
                "{ return \"\"; }",
                "{ return \"xyzzy\"; }"
            ]
        );
    }

    #[test]
    fn unwraps_the_promise_of_an_async_function() {
        let changes = list_changes("async function save(): Promise<void> { await write(); }");
        assert_eq!(changes[0].1, "{}");
        assert_eq!(
            list_changes("function later(): Promise<number> { return load(); }"),
            pairs(&[])
        );
    }

    #[test]
    fn skips_types_it_cant_read_and_bodies_that_already_are_the_value() {
        assert_eq!(
            list_changes("function cents(): Cents { return 1 as Cents; }"),
            pairs(&[])
        );
        assert_eq!(list_changes("function untyped() { return 1; }"), pairs(&[]));
        assert_eq!(
            list_changes("function no(): boolean { return false; }"),
            pairs(&[("{ return false; }", "{ return true; }")])
        );
    }

    #[test]
    fn names_the_scope_and_counts_repeats() {
        let source = "class Slot { isFree(a: number) { return a < 1 || a < 1; } }";
        let mutants = generate("src/slot.ts", source).expect("the test source parses");
        let ids: Vec<String> = mutants.iter().map(|m| m.id.to_string()).collect();
        assert!(ids[0].starts_with("src/slot.ts:Slot.isFree:relational:"));
        assert!(ids[0].ends_with(":0"));
        assert!(ids[3].ends_with(":1"));
    }

    #[test]
    fn puts_a_body_in_its_own_function_scope() {
        let mutants = generate("src/count.ts", "function count(): number { return 2; }")
            .expect("the test source parses");
        assert!(
            mutants[0]
                .id
                .to_string()
                .starts_with("src/count.ts:count:body-replacement:")
        );
    }

    #[test]
    fn refuses_a_file_that_doesnt_parse() {
        assert!(generate("src/broken.ts", "const = ;").is_err());
    }

    /// What tells two mutants apart in a fixture's expected.json: file, line, operator, original
    /// (empty for body replacement) and replacement.
    type Key = (String, u32, String, String, String);

    fn read_expected(fixture: &Path) -> (Vec<String>, BTreeSet<Key>) {
        let text = fs::read_to_string(fixture.join("expected.json"))
            .expect("the fixture has expectations");
        let expected: serde_json::Value =
            serde_json::from_str(&text).expect("expected.json is JSON");
        let operators = expected["operators"]
            .as_array()
            .expect("expected.json has the list")
            .iter()
            .map(|operator| operator.as_str().expect("operators are names").to_owned())
            .collect();
        let field = |mutant: &serde_json::Value, name: &str| {
            mutant[name].as_str().unwrap_or_default().to_owned()
        };
        let keys = expected["mutants"]
            .as_array()
            .expect("expected.json has the list")
            .iter()
            .map(|mutant| {
                let line = u32::try_from(mutant["line"].as_u64().expect("lines are numbers"))
                    .expect("lines fit u32");
                let operator = field(mutant, "operator");
                let original = field(mutant, "original");
                (
                    field(mutant, "file"),
                    line,
                    operator,
                    original,
                    field(mutant, "replacement"),
                )
            })
            .collect();
        (operators, keys)
    }

    fn generate_fixture(fixture: &Path, operators: &[String]) -> BTreeSet<Key> {
        let mut keys = BTreeSet::new();
        for entry in fs::read_dir(fixture.join("src")).expect("the fixture has sources") {
            let path = entry.expect("the entry reads").path();
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .expect("names are UTF-8")
                .to_owned();
            if name.ends_with(".test.ts") {
                continue;
            }
            let file = format!("src/{name}");
            let source = fs::read_to_string(&path).expect("the source reads");
            for mutant in generate(&file, &source).expect("the fixture parses") {
                let operator = mutant.operator.name().to_owned();
                if !operators.contains(&operator) {
                    continue;
                }
                let original = if operator == "body-replacement" {
                    String::new()
                } else {
                    mutant.original
                };
                let line = mutant.range.start.line;
                keys.insert((
                    file.clone(),
                    line,
                    operator,
                    original,
                    mutant.patch.replacement,
                ));
            }
        }
        keys
    }

    /// Whether the slice makes this mutant: a `<` site, or a body whose written return type
    /// says its values (the fixture's `Cents` needs tsrs).
    fn is_covered((file, _, operator, original, replacement): &Key) -> bool {
        match operator.as_str() {
            "relational" => original == "<" || (replacement == "false" && original.contains(" < ")),
            "body-replacement" => file != "src/cents.ts",
            _ => false,
        }
    }

    #[test]
    fn makes_the_fixtures_mutants_for_the_operators_it_covers() {
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
        for fixture in ["operators", "body-replacement"] {
            let fixture = fixtures.join(fixture);
            let (operators, expected) = read_expected(&fixture);
            let expected: BTreeSet<Key> = expected.into_iter().filter(is_covered).collect();
            let generated = generate_fixture(&fixture, &operators);
            let generated: BTreeSet<Key> = generated.into_iter().filter(is_covered).collect();
            assert!(!expected.is_empty());
            assert_eq!(generated, expected, "in {}", fixture.display());
        }
    }
}
