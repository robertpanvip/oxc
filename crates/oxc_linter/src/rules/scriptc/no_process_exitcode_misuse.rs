//! Enforce scriptc's fenced `process.exitCode` surface: statement-position numeric writes only.

use oxc_ast::{AstKind, ast::{Expression, StaticMemberExpression}};
use oxc_diagnostics::OxcDiagnostic;
use oxc_macros::declare_oxc_lint;
use oxc_span::{GetSpan, Span};

use crate::{context::LintContext, rule::Rule, AstNode};

fn exitcode_diagnostic(span: Span, why: &str) -> OxcDiagnostic {
    OxcDiagnostic::warn("`process.exitCode` outside a statement-position numeric write is fenced")
        .with_help(format!("{why}"))
        .with_label(span)
}

#[derive(Debug, Default, Clone)]
pub struct NoProcessExitcodeMisuse;

declare_oxc_lint!(
    /// ### What it does
    ///
    /// Restricts `process.exitCode` to the surface scriptc lowers: numeric
    /// writes in statement position (`process.exitCode = 2;`) set the
    /// implicit exit status. Reads, resets, and assignments used as values
    /// are flagged.
    ///
    /// ### Why is this bad?
    ///
    /// scriptc fences reading or resetting `process.exitCode` and
    /// assignments used as values; only the statement-position numeric write
    /// form compiles (and `process.exit()` with no argument then uses that
    /// status).
    ///
    /// ### Examples
    ///
    /// Examples of **incorrect** code for this rule:
    /// ```ts
    /// if (process.exitCode === 1) {} // read
    /// const code = (process.exitCode = 2); // assignment used as a value
    /// ```
    ///
    /// Examples of **correct** code for this rule:
    /// ```ts
    /// process.exitCode = 2; // statement-position numeric write
    /// process.exit();       // uses that status
    /// ```
    NoProcessExitcodeMisuse,
    scriptc,
    correctness,
    version = "1.0.0"
);

fn is_process_exitcode(member: &StaticMemberExpression) -> bool {
    matches!(&member.object, Expression::Identifier(ident) if ident.name == "process")
        && member.property.name == "exitCode"
}

impl Rule for NoProcessExitcodeMisuse {
    fn run<'a>(&self, node: &AstNode<'a>, ctx: &LintContext<'a>) {
        let AstKind::StaticMemberExpression(member) = node.kind() else {
            return;
        };
        if !is_process_exitcode(member) {
            return;
        }

        let parent = ctx.nodes().parent_node(node.id());

        // Assignment target: allowed only when the whole assignment is an
        // expression statement (a statement-position write).
        if let AstKind::AssignmentExpression(assign) = parent.kind()
            && assign.left.span() == member.span
        {
            let in_statement = matches!(
                ctx.nodes().parent_node(parent.id()).kind(),
                AstKind::ExpressionStatement(_)
            );
            if !in_statement {
                ctx.diagnostic(exitcode_diagnostic(
                    member.span,
                    "Assignments used as values are fenced; write it as a bare statement.",
                ));
            }
            return;
        }

        ctx.diagnostic(exitcode_diagnostic(
            member.span,
            "Reading or resetting `process.exitCode` is fenced; only numeric statement-position writes compile.",
        ));
    }
}

#[test]
fn test() {
    use crate::tester::Tester;

    let pass = vec![
        r"process.exitCode = 2;",
        r"process.exitCode = 0;",
        r"other.exitCode = 2;",
        r"console.log(process.argv);",
    ];
    let fail = vec![
        r"if (process.exitCode === 1) {}",
        r"const code = process.exitCode;",
        r"const code = (process.exitCode = 2);",
        r"foo(process.exitCode);",
    ];

    Tester::new(NoProcessExitcodeMisuse::NAME, NoProcessExitcodeMisuse::PLUGIN, pass, fail)
        .test_and_snapshot();
}
