//! Disallow fenced `Date` usage: the multi-argument constructor and setters.

use oxc_ast::{AstKind, ast::Expression};
use oxc_diagnostics::OxcDiagnostic;
use oxc_macros::declare_oxc_lint;
use oxc_span::Span;

use crate::{context::LintContext, rule::Rule, AstNode};

fn date_ctor_diagnostic(span: Span, argc: usize) -> OxcDiagnostic {
    OxcDiagnostic::warn(format!("the {argc}-argument `new Date(y, m, ...)` constructor is fenced in scriptc"))
        .with_help("Use `new Date(number)` or `new Date(dateString)` (date-only or explicit-Z/offset forms), or the `Date.parse(dateString)` form.")
        .with_label(span)
}

fn date_setter_diagnostic(span: Span, setter: &str) -> OxcDiagnostic {
    OxcDiagnostic::warn(format!("`Date` setters (`{setter}`) are fenced in scriptc"))
        .with_help("Construct a new `Date` value from an epoch number or a supported date string instead of mutating an existing one.")
        .with_label(span)
}

const DATE_SETTERS: &[&str] = &[
    "setFullYear",
    "setMonth",
    "setDate",
    "setHours",
    "setMinutes",
    "setSeconds",
    "setMilliseconds",
    "setTime",
    "setYear",
    "setUTCFullYear",
    "setUTCMonth",
    "setUTCDate",
    "setUTCHours",
    "setUTCMinutes",
    "setUTCSeconds",
    "setUTCMilliseconds",
];

#[derive(Debug, Default, Clone)]
pub struct NoDateFencedUsage;

declare_oxc_lint!(
    /// ### What it does
    ///
    /// Disallows the fenced parts of the `Date` surface: the year/month field
    /// constructor (`new Date(y, m, d, ...)`) and every `Date` setter
    /// (`setFullYear`, `setHours`, `setTime`, ...).
    ///
    /// ### Why is this bad?
    ///
    /// scriptc's `Date` supports zero/one-argument construction, storage and
    /// passing, `getTime`/`valueOf`/`toISOString`, the calendar getters, and
    /// `getTimezoneOffset`. The field constructor and setters are fenced and
    /// reach SC2020 at compile time. The check is call-shape based (method
    /// name), so it needs no type information.
    ///
    /// ### Examples
    ///
    /// Examples of **incorrect** code for this rule:
    /// ```ts
    /// const d = new Date(2026, 8, 27);
    /// d.setFullYear(2030);
    /// ```
    ///
    /// Examples of **correct** code for this rule:
    /// ```ts
    /// const d = new Date('2026-09-27T00:00:00Z');
    /// const later = new Date(d.getTime() + 86_400_000);
    /// ```
    NoDateFencedUsage,
    scriptc,
    correctness,
    version = "1.0.0"
);

impl Rule for NoDateFencedUsage {
    fn run<'a>(&self, node: &AstNode<'a>, ctx: &LintContext<'a>) {
        match node.kind() {
            AstKind::NewExpression(new_expr) => {
                if matches!(&new_expr.callee, Expression::Identifier(ident) if ident.name == "Date")
                    && new_expr.arguments.len() >= 2
                {
                    ctx.diagnostic(date_ctor_diagnostic(
                        new_expr.span,
                        new_expr.arguments.len(),
                    ));
                }
            }
            AstKind::CallExpression(call) => {
                // Flag any call to a Date-setter-named method. The names are
                // specific enough that a non-Date object carrying one is
                // vanishingly rare in practice.
                if let Expression::StaticMemberExpression(member) = &call.callee
                    && DATE_SETTERS.contains(&member.property.name.as_str())
                {
                    ctx.diagnostic(date_setter_diagnostic(call.span, &member.property.name));
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
        r"const d = new Date(1758950000000);",
        r"const d = new Date('2026-09-27T00:00:00Z');",
        r"const d = new Date();",
        r"console.log(new Date(1758950000000).getTime());",
    ];
    let fail = vec![
        r"const d = new Date(2026, 8, 27);",
        r"new Date(2026, 8, 27, 10, 30);",
        r"const d = new Date(1758950000000); d.setFullYear(2030);",
        r"const d = new Date(1758950000000); d.setTime(1);",
        r"const d = new Date(1758950000000); d.setUTCHours(3);",
    ];

    Tester::new(NoDateFencedUsage::NAME, NoDateFencedUsage::PLUGIN, pass, fail).test_and_snapshot();
}
