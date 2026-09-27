//! Disallow fenced globals and console methods in scriptc programs (SC2020).

use oxc_ast::{
    AstKind,
    ast::{Expression, StaticMemberExpression},
};
use oxc_diagnostics::OxcDiagnostic;
use oxc_macros::declare_oxc_lint;
use oxc_span::Span;

use crate::{context::LintContext, rule::Rule, AstNode};

fn fenced_global_diagnostic(span: Span, name: &str, why: &str) -> OxcDiagnostic {
    OxcDiagnostic::warn(format!("`{name}` is outside scriptc's lowered static surface (SC2020)"))
        .with_help(format!("{why}"))
        .with_label(span)
}

const FENCED_GLOBALS: &[(&str, &str)] = &[
    ("Symbol", "Symbol is not lowered in static builds; use string or number keys and plain string constants instead."),
    ("SharedArrayBuffer", "SharedArrayBuffer is not supported by the scriptc runtime."),
    ("WeakRef", "WeakRef is not supported; memory is reference-counted with deterministic cycle collection."),
    ("FinalizationRegistry", "FinalizationRegistry is not supported; memory is reference-counted, not garbage-collected concurrently."),
    ("BigInt64Array", "BigInt crossing typed arrays is fenced; BigInt64Array/BigUint64Array do not compile."),
    ("BigUint64Array", "BigInt crossing typed arrays is fenced; BigInt64Array/BigUint64Array do not compile."),
    ("globalThis", "`globalThis` as a value is fenced in static builds; the dynamic tier exposes it through the island host object."),
];

const FENCED_CONSOLE: &[(&str, &str)] = &[
    ("table", "console.table has no lowering; it lands on SC2020."),
    ("time", "console.time has no lowering; it lands on SC2020."),
    ("timeEnd", "console.timeEnd has no lowering; it lands on SC2020."),
    ("timeLog", "console.timeLog has no lowering; it lands on SC2020."),
];

#[derive(Debug, Default, Clone)]
pub struct NoFencedGlobals;

declare_oxc_lint!(
    /// ### What it does
    ///
    /// Disallows identifiers and console methods that scriptc's static tier
    /// explicitly refuses: `Symbol`, `globalThis` (as a value),
    /// `SharedArrayBuffer`, `WeakRef`, `FinalizationRegistry`,
    /// `BigInt64Array`/`BigUint64Array`, and
    /// `console.table`/`time`/`timeEnd`/`timeLog`.
    ///
    /// ### Why is this bad?
    ///
    /// The type checker sees the full standard library, but only the lowered
    /// surface compiles; reaching declared-but-unlowered surface is SC2020.
    /// This rule surfaces those refusals at lint time with the documented
    /// reason, instead of at compile time.
    ///
    /// ### Examples
    ///
    /// Examples of **incorrect** code for this rule:
    /// ```ts
    /// const k = Symbol("id");
    /// const host = globalThis;
    /// console.table(rows);
    /// ```
    ///
    /// Examples of **correct** code for this rule:
    /// ```ts
    /// const k = "id";
    /// console.log(rows);
    /// ```
    NoFencedGlobals,
    scriptc,
    restriction,
    version = "1.0.0"
);

fn static_member<'a>(expr: &'a Expression<'a>) -> Option<&'a StaticMemberExpression<'a>> {
    match expr {
        Expression::StaticMemberExpression(member) => Some(member),
        _ => None,
    }
}

impl Rule for NoFencedGlobals {
    fn run<'a>(&self, node: &AstNode<'a>, ctx: &LintContext<'a>) {
        match node.kind() {
            AstKind::IdentifierReference(ident) => {
                if let Some((name, why)) = FENCED_GLOBALS
                    .iter()
                    .find(|(name, _)| ident.name.as_str() == *name)
                {
                    ctx.diagnostic(fenced_global_diagnostic(ident.span, name, why));
                }
            }
            AstKind::CallExpression(call) => {
                let Some(member) = static_member(&call.callee) else {
                    return;
                };
                let Expression::Identifier(object) = &member.object else {
                    return;
                };
                if object.name == "console"
                    && let Some((name, why)) = FENCED_CONSOLE
                        .iter()
                        .find(|(name, _)| member.property.name.as_str() == *name)
                {
                    ctx.diagnostic(fenced_global_diagnostic(call.span, &format!("console.{name}"), why));
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
        r"const k = 'id';",
        r"console.log(rows);",
        r"const Symbol2 = 1; console.log(Symbol2);",
        r"type T = string;",
    ];
    let fail = vec![
        r"const k = Symbol('id');",
        r"const host = globalThis;",
        r"console.table(rows);",
        r"console.time('t');",
        r"new WeakRef(x);",
    ];

    Tester::new(NoFencedGlobals::NAME, NoFencedGlobals::PLUGIN, pass, fail).test_and_snapshot();
}
