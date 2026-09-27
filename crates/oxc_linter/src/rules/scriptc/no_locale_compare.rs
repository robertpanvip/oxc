//! Warn on `localeCompare`, which compares code units in scriptc, not ICU collation.

use oxc_ast::{AstKind, ast::Expression};
use oxc_diagnostics::OxcDiagnostic;
use oxc_macros::declare_oxc_lint;
use oxc_span::Span;

use crate::{context::LintContext, rule::Rule, AstNode};

fn locale_compare_diagnostic(span: Span) -> OxcDiagnostic {
    OxcDiagnostic::warn("`localeCompare` compares code units in scriptc, not ICU collation")
        .with_help("This is a documented divergence from Node: results can differ from V8 for locale-sensitive ordering. Sort with a plain comparator or normalize first if byte-stable order is what you need.")
        .with_label(span)
}

#[derive(Debug, Default, Clone)]
pub struct NoLocaleCompare;

declare_oxc_lint!(
    /// ### What it does
    ///
    /// Warns on `str.localeCompare(other)` calls.
    ///
    /// ### Why is this bad?
    ///
    /// scriptc's `localeCompare` compares code units, not ICU collation — a
    /// documented, pinned divergence from Node. Programs that rely on
    /// locale-aware ordering will diverge from Node at runtime.
    ///
    /// ### Examples
    ///
    /// Examples of **incorrect** code for this rule:
    /// ```ts
    /// const sorted = names.sort((a, b) => a.localeCompare(b));
    /// ```
    ///
    /// Examples of **correct** code for this rule:
    /// ```ts
    /// const sorted = names.sort(); // code-unit order, byte-stable
    /// ```
    NoLocaleCompare,
    scriptc,
    pedantic,
    version = "1.0.0"
);

impl Rule for NoLocaleCompare {
    fn run<'a>(&self, node: &AstNode<'a>, ctx: &LintContext<'a>) {
        let AstKind::CallExpression(call) = node.kind() else {
            return;
        };
        let Expression::StaticMemberExpression(member) = &call.callee else {
            return;
        };
        if member.property.name == "localeCompare" {
            ctx.diagnostic(locale_compare_diagnostic(member.property.span));
        }
    }
}

#[test]
fn test() {
    use crate::tester::Tester;

    let pass = vec![
        r"const sorted = names.sort();",
        r"const cmp = a < b;",
    ];
    let fail = vec![r"const cmp = a.localeCompare(b);"];

    Tester::new(NoLocaleCompare::NAME, NoLocaleCompare::PLUGIN, pass, fail).test_and_snapshot();
}
