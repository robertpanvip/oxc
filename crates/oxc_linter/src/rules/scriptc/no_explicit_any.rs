//! Disallow explicit `any`, which is a compile error in static scriptc builds.

use oxc_ast::AstKind;
use oxc_diagnostics::OxcDiagnostic;
use oxc_macros::declare_oxc_lint;
use oxc_span::Span;

use crate::{context::LintContext, rule::Rule, AstNode};

fn any_diagnostic(span: Span) -> OxcDiagnostic {
    OxcDiagnostic::warn("explicit `any` is a compile error in static scriptc builds (SC2011)")
        .with_help("Use `unknown` with a checked cast (`JSON.parse(s) as Config`), or opt into the dynamic engine with `--dynamic`.")
        .with_label(span)
}

#[derive(Debug, Default, Clone)]
pub struct NoExplicitAny;

declare_oxc_lint!(
    /// ### What it does
    ///
    /// Disallows explicit `any` type annotations (`: any`, `as any`, `<any>`).
    ///
    /// ### Why is this bad?
    ///
    /// scriptc rejects `any` without `--dynamic` with SC2011. The static tier
    /// needs a concrete shape for every value; `unknown` plus a checked cast
    /// keeps the program compilable while preserving the dynamic boundary's
    /// "lying cast throws" safety. This rule flags `any` early so programs
    /// stay on the default static path.
    ///
    /// ### Examples
    ///
    /// Examples of **incorrect** code for this rule:
    /// ```ts
    /// const config: any = JSON.parse(text);
    /// const value = data as any;
    /// function f(x: any) {}
    /// ```
    ///
    /// Examples of **correct** code for this rule:
    /// ```ts
    /// const config = JSON.parse(text) as Config; // checked cast, throws on mismatch
    /// function f(x: unknown) {}
    /// ```
    NoExplicitAny,
    scriptc,
    restriction,
    version = "1.0.0"
);

impl Rule for NoExplicitAny {
    fn run<'a>(&self, node: &AstNode<'a>, ctx: &LintContext<'a>) {
        if let AstKind::TSAnyKeyword(keyword) = node.kind() {
            ctx.diagnostic(any_diagnostic(keyword.span));
        }
    }
}

#[test]
fn test() {
    use crate::tester::Tester;

    let pass = vec![
        r"const x: unknown = JSON.parse(s);",
        r"const y = data as Config;",
        r"function f(x: string | number) {}",
    ];
    let fail = vec![
        r"const x: any = 1;",
        r"const y = v as any;",
        r"const z = <any>v;",
        r"function f(x: any) {}",
    ];

    Tester::new(NoExplicitAny::NAME, NoExplicitAny::PLUGIN, pass, fail).test_and_snapshot();
}
