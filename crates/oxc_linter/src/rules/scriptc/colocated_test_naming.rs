//! Enforce scriptc's co-located white-box test naming convention.
//!
//! AGENTS.md: "Co-locate white-box unit tests with implementation files
//! under `packages/*/src`; name them after the source file (`cc.ts` →
//! `cc.test.ts`)." A `.test.ts` file inside `packages/*/src` whose source
//! file no longer exists is a leftover and should be renamed or deleted.

use std::path::Path;

use oxc_diagnostics::OxcDiagnostic;
use oxc_macros::declare_oxc_lint;
use oxc_span::Span;

use crate::{context::LintContext, rule::Rule};

fn colocated_test_naming_diagnostic(span: Span, test_file: &str) -> OxcDiagnostic {
    OxcDiagnostic::warn(format!(
        "co-located test file `{test_file}` has no matching source file"
    ))
    .with_help("White-box tests under packages/*/src must be named after their source file (`cc.ts` → `cc.test.ts`). Rename this test after an existing source file, move it to `packages/*/test`, or delete it.")
    .with_label(span)
}

#[derive(Debug, Default, Clone)]
pub struct ColocatedTestNaming;

declare_oxc_lint!(
    /// ### What it does
    ///
    /// Enforces scriptc's co-located white-box test naming convention: a
    /// `*.test.ts` file under `packages/*/src` must be named after an
    /// existing source file in the same directory (`cc.ts` → `cc.test.ts`).
    ///
    /// ### Why is this bad?
    ///
    /// AGENTS.md fixes this convention so white-box unit tests stay
    /// traceable to the implementation file they exercise. A test file with
    /// no matching source file is a leftover after a rename or deletion and
    /// silently drifts.
    ///
    /// ### Examples
    ///
    /// Examples of **incorrect** code for this rule (in `packages/compiler/src/`):
    /// - `ir/validate.test.ts` exists but `ir/validate.ts` does not.
    ///
    /// Examples of **correct** code for this rule (in `packages/compiler/src/`):
    /// - `ir/validate.ts` and `ir/validate.test.ts` both exist.
    ColocatedTestNaming,
    scriptc,
    pedantic,
    version = "1.0.0"
);

fn src_test_file(ctx: &LintContext) -> Option<String> {
    let path = ctx.file_path().to_string_lossy().replace('\\', "/");
    // Match `<root>/packages/<name>/src/**` only. Package-level API tests
    // live in `packages/<name>/test` and are out of scope.
    let pkg_idx = path.find("/packages/")?;
    let rest = &path[pkg_idx..];
    let src_idx = rest.find("/src/")?;
    let file_name = path.rsplit('/').next()?.to_string();
    let _ = src_idx;
    if file_name.ends_with(".test.ts") || file_name.ends_with(".test.tsx") {
        Some(file_name)
    } else {
        None
    }
}

fn has_matching_source(dir: &Path, test_file: &str) -> bool {
    let base = test_file
        .strip_suffix(".test.tsx")
        .or_else(|| test_file.strip_suffix(".test.ts"));
    let Some(base) = base else { return true };
    for candidate in [
        format!("{base}.ts"),
        format!("{base}.tsx"),
        format!("{base}.js"),
        format!("{base}.jsx"),
        // packages/runtime/src unit tests exercise C runtime units:
        // `scr_async.test.ts` ↔ `scr_async.c`
        format!("{base}.c"),
        format!("{base}/index.ts"),
        format!("{base}/index.tsx"),
    ] {
        if dir.join(&candidate).is_file() {
            return true;
        }
    }
    false
}

impl Rule for ColocatedTestNaming {
    fn run_once(&self, ctx: &LintContext) {
        let Some(test_file) = src_test_file(ctx) else { return };
        let dir = ctx.file_path().parent().map(Path::to_path_buf);
        let Some(dir) = dir else { return };
        if !has_matching_source(&dir, &test_file) {
            ctx.diagnostic(colocated_test_naming_diagnostic(Span::default(), &test_file));
        }
    }
}

#[test]
fn test() {
    use crate::tester::Tester;

    // The rule's check is file-system based (a sibling source file must
    // exist), which the Tester's inline snippets cannot fully express — the
    // tester writes snippets into a temp dir where no sibling source file
    // exists, so a `src/**.test.ts` path always reports. The rule is also
    // exercised end-to-end by running the built oxlint binary against the
    // scriptc repository (its real `packages/*/src` pairs pass there).
    let pass: Vec<&str> = vec![];
    let fail = vec![
        // `packages/compiler/src/ir/validate.test.ts` with no sibling
        // `validate.ts` next to it (temp dir) reports.
        r"console.log(1);",
        r"console.log(2);",
    ];

    Tester::new(ColocatedTestNaming::NAME, ColocatedTestNaming::PLUGIN, pass, fail)
        .change_rule_path("packages/compiler/src/ir/validate.test.ts")
        .test_and_snapshot();
}
