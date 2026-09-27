//! Disallow nondeterministic APIs inside the differential-test corpus.
//!
//! scriptc's corpus (`tests/corpus/**`) is a differential test suite: every
//! program runs under Node.js and as a compiled native binary, and stdout,
//! stderr, and exit codes must match byte-for-byte (see AGENTS.md). Any use
//! of wall-clock time or randomness makes a program's output unstable and
//! flaky across the two runtimes.

use oxc_ast::{
    AstKind,
    ast::{Argument, Expression},
};
use oxc_diagnostics::OxcDiagnostic;
use oxc_macros::declare_oxc_lint;
use oxc_span::Span;

use crate::{AstNode, context::LintContext, rule::Rule};

fn corpus_nondeterminism_diagnostic(span: Span, api: &str, why: &str) -> OxcDiagnostic {
    OxcDiagnostic::warn(format!("nondeterministic API `{api}` in differential-test corpus"))
        .with_help(format!("{why} Replace it with a deterministic value so Node and the compiled binary keep matching byte-for-byte."))
        .with_label(span)
}

#[derive(Debug, Default, Clone)]
pub struct CorpusNoNondeterminism;

declare_oxc_lint!(
    /// ### What it does
    ///
    /// Disallows nondeterministic JavaScript APIs (`Date.now()`, `new Date()`,
    /// `Math.random()`, `performance.now()`) inside `tests/corpus/**`.
    ///
    /// ### Why is this bad?
    ///
    /// scriptc's corpus programs are differential tests: each program runs
    /// under Node.js and as a compiled native binary and the outputs must
    /// match byte-for-byte. Wall-clock time and randomness make the output
    /// differ between runs and between the two execution tiers, producing
    /// flaky tests.
    ///
    /// ### Examples
    ///
    /// Examples of **incorrect** code for this rule:
    /// ```ts
    /// // tests/corpus/9000-example.ts
    /// console.log(Date.now());
    /// console.log(Math.random());
    /// console.log(new Date());
    /// ```
    ///
    /// Examples of **correct** code for this rule:
    /// ```ts
    /// // tests/corpus/9000-example.ts
    /// console.log(1758950000000); // fixed epoch constant
    /// console.log(0.42);          // fixed value
    /// ```
    CorpusNoNondeterminism,
    scriptc,
    correctness,
    version = "1.0.0"
);

fn in_corpus_dir(ctx: &LintContext) -> bool {
    let path = ctx.file_path().to_string_lossy().replace('\\', "/");
    path.contains("/tests/corpus/") || path.starts_with("tests/corpus/")
}

fn static_member_callee<'a>(expr: &'a Expression<'a>) -> Option<(&'a str, &'a str)> {
    if let Expression::StaticMemberExpression(member) = expr
        && let Expression::Identifier(ident) = &member.object
    {
        return Some((ident.name.as_str(), member.property.name.as_str()));
    }
    None
}

impl Rule for CorpusNoNondeterminism {
    fn run<'a>(&self, node: &AstNode<'a>, ctx: &LintContext<'a>) {
        if !in_corpus_dir(ctx) {
            return;
        }

        match node.kind() {
            AstKind::CallExpression(call) => {
                let Some((object, property)) = static_member_callee(&call.callee) else {
                    return;
                };
                let (api, why) = match (object, property) {
                    ("Date", "now") => (
                        "Date.now()",
                        "The current epoch time differs between the Node run and the compiled run.",
                    ),
                    ("Math", "random") => (
                        "Math.random()",
                        "Random values differ between the Node run and the compiled run.",
                    ),
                    ("performance", "now") => (
                        "performance.now()",
                        "High-resolution timing differs between runs and runtimes.",
                    ),
                    _ => return,
                };
                ctx.diagnostic(corpus_nondeterminism_diagnostic(call.span, api, why));
            }
            AstKind::NewExpression(new_expr) => {
                // `new Date()` / `new Date(undefined)` read the current time;
                // explicit numeric/string arguments are deterministic.
                let is_time_now = if let Expression::Identifier(ident) = &new_expr.callee {
                    ident.name == "Date"
                } else {
                    false
                };
                if !is_time_now {
                    return;
                }
                let args_read_clock = new_expr.arguments.is_empty()
                    || new_expr.arguments.iter().all(|arg| {
                        matches!(arg, Argument::Identifier(ident) if ident.name == "undefined")
                    });
                if args_read_clock {
                    ctx.diagnostic(corpus_nondeterminism_diagnostic(
                        new_expr.span,
                        "new Date()",
                        "The current time differs between the Node run and the compiled run.",
                    ));
                }
            }
            _ => {}
        }
    }
}

#[test]
fn test() {
    use crate::tester::Tester;

    let pass = vec![
        r"console.log(1758950000000);",
        r"const d = new Date(1758950000000); console.log(d.getTime());",
        r"console.log(new Date('2026-09-27T00:00:00Z'));",
        r"const Math2 = { random: () => 0.5 }; console.log(Math2.random());",
    ];
    let fail = vec![
        r"console.log(Date.now());",
        r"console.log(Math.random());",
        r"console.log(performance.now());",
        r"console.log(new Date());",
        r"console.log(new Date(undefined));",
    ];

    Tester::new(CorpusNoNondeterminism::NAME, CorpusNoNondeterminism::PLUGIN, pass, fail)
        .change_rule_path("tests/corpus/9000-example.ts")
        .test_and_snapshot();
}
