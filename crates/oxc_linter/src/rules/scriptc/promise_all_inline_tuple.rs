//! Warn on inline `Promise.all([...])` array literals, which infer a tuple.

use oxc_ast::{
    AstKind,
    ast::{Argument, Expression},
};
use oxc_diagnostics::OxcDiagnostic;
use oxc_macros::declare_oxc_lint;
use oxc_span::Span;

use crate::{context::LintContext, rule::Rule, AstNode};

fn tuple_inference_diagnostic(span: Span) -> OxcDiagnostic {
    OxcDiagnostic::warn("inline `Promise.all([...])` infers a tuple type; tuple edges are fenced")
        .with_help("Type the array first: `const jobs: Promise<number>[] = [...]; await Promise.all(jobs);` — see the scriptc limitations page on tuple inference.")
        .with_label(span)
}

#[derive(Debug, Default, Clone)]
pub struct PromiseAllInlineTuple;

declare_oxc_lint!(
    /// ### What it does
    ///
    /// Warns when `Promise.all` is called with an inline array literal.
    ///
    /// ### Why is this bad?
    ///
    /// `Promise.all([work(1), work(2)])` infers a **tuple** type, and tuple
    /// edges (like `.join` on a tuple) are fenced in scriptc. The documented
    /// rewrite types the array first so it is inferred as `Promise<T>[]`.
    ///
    /// ### Examples
    ///
    /// Examples of **incorrect** code for this rule:
    /// ```ts
    /// const results = await Promise.all([work(1), work(2)]);
    /// ```
    ///
    /// Examples of **correct** code for this rule:
    /// ```ts
    /// const jobs: Promise<number>[] = [work(1), work(2)];
    /// const results = await Promise.all(jobs); // number[] — compiles
    /// ```
    PromiseAllInlineTuple,
    scriptc,
    pedantic,
    version = "1.0.0"
);

impl Rule for PromiseAllInlineTuple {
    fn run<'a>(&self, node: &AstNode<'a>, ctx: &LintContext<'a>) {
        let AstKind::CallExpression(call) = node.kind() else {
            return;
        };
        let Expression::StaticMemberExpression(member) = &call.callee else {
            return;
        };
        let Expression::Identifier(object) = &member.object else {
            return;
        };
        if object.name != "Promise" || member.property.name != "all" {
            return;
        }
        if let Some(Argument::ArrayExpression(_)) = call.arguments.first() {
            ctx.diagnostic(tuple_inference_diagnostic(call.span));
        }
    }
}

#[test]
fn test() {
    use crate::tester::Tester;

    let pass = vec![
        r"const jobs: Promise<number>[] = [work(1), work(2)]; const r = await Promise.all(jobs);",
        r"const r = await Promise.all(fetchAll());",
        r"const r = await other.all([1, 2]);",
    ];
    let fail = vec![
        r"const r = await Promise.all([work(1), work(2)]);",
        r"const r = Promise.all([a, b, c]);",
    ];

    Tester::new(PromiseAllInlineTuple::NAME, PromiseAllInlineTuple::PLUGIN, pass, fail)
        .test_and_snapshot();
}
