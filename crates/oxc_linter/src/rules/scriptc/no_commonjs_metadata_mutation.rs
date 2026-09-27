//! Disallow CommonJS module metadata mutation, which a compiled binary refuses.

use oxc_ast::{
    AstKind,
    ast::{AssignmentTarget, Expression, UnaryExpression},
};
use oxc_diagnostics::OxcDiagnostic;
use oxc_macros::declare_oxc_lint;
use oxc_span::Span;

use crate::{context::LintContext, rule::Rule, AstNode};

fn metadata_write_diagnostic(span: Span, what: &str) -> OxcDiagnostic {
    OxcDiagnostic::warn(format!("writing `{what}` is an explicit scriptc refusal"))
        .with_help("A compiled binary has a fixed module graph; CommonJS module metadata is read-only.")
        .with_label(span)
}

fn extensions_diagnostic(span: Span) -> OxcDiagnostic {
    OxcDiagnostic::warn("`require.extensions` is an explicit scriptc refusal")
        .with_help("A compiled binary embeds its module graph at build time; there is no source loader to extend.")
        .with_label(span)
}

const MODULE_METADATA_FIELDS: &[&str] = &[
    "id",
    "filename",
    "path",
    "paths",
    "loaded",
    "isPreloading",
    "parent",
    "children",
];

/// `module.paths.push/pop/shift/unshift/splice(...)` style mutation calls.
const PATHS_MUTATION_METHODS: &[&str] = &["push", "pop", "shift", "unshift", "splice"];

#[derive(Debug, Default, Clone)]
pub struct NoCommonjsMetadataMutation;

declare_oxc_lint!(
    /// ### What it does
    ///
    /// Disallows writes to CommonJS module metadata (`module.id`,
    /// `module.filename`, `module.path`, `module.paths`, `module.loaded`,
    /// `module.isPreloading`, `module.parent`, `module.children`,
    /// `require.main`), cache deletion/reloading (`delete
    /// require.cache[...]`, `require.cache[k] = ...`), and any use of
    /// `require.extensions`.
    ///
    /// ### Why is this bad?
    ///
    /// A compiled binary has a fixed module graph. scriptc compiles CommonJS
    /// metadata reads natively, but cache deletion/reloading, metadata
    /// writes, `module.paths` mutation, and `require.extensions` are explicit
    /// compile-time refusals.
    ///
    /// ### Examples
    ///
    /// Examples of **incorrect** code for this rule:
    /// ```ts
    /// module.paths.push('./node_modules');
    /// delete require.cache[require.main!.filename];
    /// require.extensions['.ts'] = () => {};
    /// ```
    ///
    /// Examples of **correct** code for this rule:
    /// ```ts
    /// console.log(module.filename); // reads compile natively
    /// ```
    NoCommonjsMetadataMutation,
    scriptc,
    correctness,
    version = "1.0.0"
);

/// `(module|require).field` written as a bare assignment target.
fn target_pair<'a>(
    member: &'a oxc_ast::ast::StaticMemberExpression<'a>,
) -> Option<(&'a str, &'a str)> {
    if let Expression::Identifier(object) = &member.object {
        return Some((object.name.as_str(), member.property.name.as_str()));
    }
    None
}

fn is_require_cache_object(expr: &Expression) -> bool {
    match expr {
        Expression::StaticMemberExpression(member) => {
            matches!(&member.object, Expression::Identifier(id) if id.name == "require")
                && member.property.name == "cache"
        }
        _ => false,
    }
}

fn check_unary_delete(expr: &UnaryExpression, ctx: &LintContext) {
    if !expr.operator.is_delete() {
        return;
    }
    // `delete require.cache[...]` — computed member over require.cache.
    if let Expression::ComputedMemberExpression(member) = &expr.argument
        && is_require_cache_object(&member.object)
    {
        ctx.diagnostic(metadata_write_diagnostic(member.span, "delete require.cache[...]"));
    }
}

impl Rule for NoCommonjsMetadataMutation {
    fn run<'a>(&self, node: &AstNode<'a>, ctx: &LintContext<'a>) {
        match node.kind() {
            AstKind::StaticMemberExpression(member) => {
                // `require.extensions` in any position is refused.
                if let Expression::Identifier(object) = &member.object
                    && object.name == "require"
                    && member.property.name == "extensions"
                {
                    ctx.diagnostic(extensions_diagnostic(member.span));
                }
            }
            AstKind::AssignmentExpression(assign) => match &assign.left {
                AssignmentTarget::StaticMemberExpression(member) => {
                    let Some((object, property)) = target_pair(member) else {
                        return;
                    };
                    if object == "module" && MODULE_METADATA_FIELDS.contains(&property) {
                        ctx.diagnostic(metadata_write_diagnostic(
                            member.span,
                            &format!("module.{property}"),
                        ));
                    } else if object == "require" && (property == "main" || property == "cache") {
                        ctx.diagnostic(metadata_write_diagnostic(
                            member.span,
                            &format!("require.{property}"),
                        ));
                    }
                }
                AssignmentTarget::ComputedMemberExpression(member) => {
                    // `require.cache[k] = ...`
                    if is_require_cache_object(&member.object) {
                        ctx.diagnostic(metadata_write_diagnostic(member.span, "require.cache[k]"));
                    }
                }
                _ => {}
            },
            AstKind::CallExpression(call) => {
                // `module.paths.push(...)` and friends mutate module metadata.
                if let Expression::StaticMemberExpression(callee) = &call.callee
                    && let Expression::StaticMemberExpression(object) = &callee.object
                    && let Some(("module", "paths")) = target_pair(object)
                    && PATHS_MUTATION_METHODS.contains(&callee.property.name.as_str())
                {
                    ctx.diagnostic(metadata_write_diagnostic(call.span, "module.paths"));
                }
            }
            AstKind::UnaryExpression(expr) => check_unary_delete(expr, ctx),
            _ => {}
        }
    }
}

#[test]
fn test() {
    use crate::tester::Tester;

    let pass = vec![
        r"console.log(module.filename, module.id);",
        r"console.log(require.cache[module.filename]);",
        r"console.log(require.main === module);",
        r"delete cache[k];",
    ];
    let fail = vec![
        r"module.paths.push('./x');",
        r"module.loaded = true;",
        r"module.id = 'x';",
        r"require.main = module;",
        r"delete require.cache[k];",
        r"require.cache[k] = undefined;",
        r"require.extensions['.ts'] = f;",
    ];

    Tester::new(NoCommonjsMetadataMutation::NAME, NoCommonjsMetadataMutation::PLUGIN, pass, fail)
        .test_and_snapshot();
}
