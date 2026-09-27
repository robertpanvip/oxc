//! Disallow computed `import()` specifiers, which stay fenced in static builds.

use oxc_ast::{AstKind, ast::Expression};
use oxc_diagnostics::OxcDiagnostic;
use oxc_macros::declare_oxc_lint;
use oxc_span::Span;

use crate::{context::LintContext, rule::Rule, AstNode};

fn computed_import_diagnostic(span: Span) -> OxcDiagnostic {
    OxcDiagnostic::warn("computed `import()` specifiers stay fenced in scriptc")
        .with_help("Literal `import()` of compiled ESM/TypeScript modules and supported Node builtins compiles without the dynamic engine; a computed specifier requires `--dynamic` (or lands on an explicit fence).")
        .with_label(span)
}

#[derive(Debug, Default, Clone)]
pub struct NoComputedDynamicImport;

declare_oxc_lint!(
    /// ### What it does
    ///
    /// Disallows dynamic `import()` with a computed (non-literal) specifier.
    /// Only string literals and expression-less template literals pass.
    ///
    /// ### Why is this bad?
    ///
    /// scriptc compiles literal `import()` of compiled ESM/TypeScript modules
    /// and supported Node builtins without the dynamic engine, but computed
    /// specifiers remain explicitly fenced — they require the `--dynamic`
    /// engine. Flagging them keeps programs on the static path.
    ///
    /// ### Examples
    ///
    /// Examples of **incorrect** code for this rule:
    /// ```ts
    /// const name = './worker';
    /// await import(name);
    /// await import(`./workers/${id}.ts`);
    /// ```
    ///
    /// Examples of **correct** code for this rule:
    /// ```ts
    /// await import('./worker.ts');
    /// ```
    NoComputedDynamicImport,
    scriptc,
    restriction,
    version = "1.0.0"
);

impl Rule for NoComputedDynamicImport {
    fn run<'a>(&self, node: &AstNode<'a>, ctx: &LintContext<'a>) {
        let AstKind::ImportExpression(import) = node.kind() else {
            return;
        };
        let is_literal = match &import.source {
            Expression::StringLiteral(_) => true,
            Expression::TemplateLiteral(tpl) => tpl.expressions.is_empty(),
            _ => false,
        };
        if !is_literal {
            ctx.diagnostic(computed_import_diagnostic(import.span));
        }
    }
}

#[test]
fn test() {
    use crate::tester::Tester;

    let pass = vec![
        r"await import('./worker.ts');",
        r"await import('node:fs');",
        r"await import(`./worker.ts`);",
    ];
    let fail = vec![
        r"const name = './worker'; await import(name);",
        r"await import(`./workers/${id}.ts`);",
        r"await import('./a' + suffix);",
    ];

    Tester::new(NoComputedDynamicImport::NAME, NoComputedDynamicImport::PLUGIN, pass, fail)
        .test_and_snapshot();
}
