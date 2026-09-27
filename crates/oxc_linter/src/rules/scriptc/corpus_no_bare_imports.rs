//! Disallow bare (package) imports inside the differential-test corpus.
//!
//! scriptc's flat corpus (`tests/corpus/*.ts`) must stay dependency-free:
//! corpus programs run under Node.js and as a compiled native binary in a
//! sandbox with no `node_modules`. Node built-ins (with or without the
//! `node:` prefix), relative/self imports, and `#` subpath imports are fine.
//! Corpus cases that genuinely need packages live in nested directories
//! carrying their own `package.json` (or an installed `node_modules`
//! fixture) — those directories are exempt.

use std::path::Path;

use oxc_ast::AstKind;
use oxc_ast::ast::{Argument, Expression};
use oxc_diagnostics::OxcDiagnostic;
use oxc_macros::declare_oxc_lint;
use oxc_span::Span;

use crate::{context::LintContext, rule::Rule};

fn corpus_bare_import_diagnostic(span: Span, source: &str) -> OxcDiagnostic {
    OxcDiagnostic::warn(format!(
        "bare package import `{source}` in differential-test corpus"
    ))
    .with_help("Flat corpus programs must be dependency-free: use `node:` built-ins, relative imports, or literals only. Package-dependent cases belong in a nested corpus directory with its own package.json, or suppress this diagnostic for a deliberate failure case.")
    .with_label(span)
}

#[derive(Debug, Default, Clone)]
pub struct CorpusNoBareImports;

declare_oxc_lint!(
    /// ### What it does
    ///
    /// Disallows bare npm package specifiers (`import x from 'pkg'`) in flat
    /// corpus programs under `tests/corpus/**`. Node built-ins (with or
    /// without the `node:` prefix), relative imports, and `#` subpath
    /// imports are allowed. Nested corpus directories that carry their own
    /// `package.json` or an installed `node_modules` fixture are exempt.
    ///
    /// ### Why is this bad?
    ///
    /// Flat corpus programs run under Node.js and as compiled native
    /// binaries in an environment without `node_modules`. A bare import
    /// makes the differential pair non-runnable and the test flaky.
    ///
    /// ### Examples
    ///
    /// Examples of **incorrect** code for this rule:
    /// ```ts
    /// // tests/corpus/9000-example.ts
    /// import { red } from 'colorette';
    /// ```
    ///
    /// Examples of **correct** code for this rule:
    /// ```ts
    /// // tests/corpus/9000-example.ts
    /// import { readFileSync } from 'node:fs';
    /// import { helper } from './helper.ts';
    /// ```
    CorpusNoBareImports,
    scriptc,
    restriction,
    version = "1.0.0"
);

fn in_corpus_dir(ctx: &LintContext) -> bool {
    let path = ctx.file_path().to_string_lossy().replace('\\', "/");
    path.contains("/tests/corpus/") || path.starts_with("tests/corpus/")
}

/// Node.js built-in module names, accepted with or without the `node:`
/// prefix (corpus predates the prefix convention and uses both forms).
const NODE_BUILTINS: &[&str] = &[
    "assert", "async_hooks", "buffer", "child_process", "cluster", "console", "constants",
    "crypto", "dgram", "diagnostics_channel", "dns", "domain", "events", "fs", "http", "http2",
    "https", "inspector", "module", "net", "os", "path", "perf_hooks", "process", "punycode",
    "querystring", "readline", "repl", "stream", "string_decoder", "sys", "timers", "tls",
    "trace_events", "tty", "url", "util", "v8", "vm", "wasi", "worker_threads", "zlib",
];

fn is_bare_specifier(source: &str) -> bool {
    if source.is_empty()
        || source.starts_with('.')
        || source.starts_with('/')
        || source.starts_with('#')
    {
        return false;
    }
    // Node built-ins, with or without the `node:` prefix; subpath specifiers
    // like `stream/promises` count as built-in when the first segment is.
    let stripped = source.strip_prefix("node:").unwrap_or(source);
    let first = stripped.split('/').next().unwrap_or(stripped);
    !NODE_BUILTINS.contains(&first)
}

/// Walk from the file's directory up to (excluding) the corpus root; any
/// level carrying its own `package.json` (a package-dependent nested case)
/// or `node_modules` (a fixture-installed case) exempts the file.
fn has_package_context(file: &Path) -> bool {
    let mut dir = file.parent();
    while let Some(d) = dir {
        if d.file_name().and_then(|n| n.to_str()) == Some("corpus") {
            return false;
        }
        if d.join("package.json").is_file() || d.join("node_modules").is_dir() {
            return true;
        }
        dir = d.parent();
    }
    false
}

fn check_source(source: &str, span: Span, ctx: &LintContext) {
    if is_bare_specifier(source) {
        ctx.diagnostic(corpus_bare_import_diagnostic(span, source));
    }
}

fn require_source<'a>(call: &'a oxc_ast::ast::CallExpression<'a>) -> Option<&'a str> {
    if let Expression::Identifier(ident) = &call.callee
        && ident.name == "require"
        && call.arguments.len() == 1
        && let Some(Argument::StringLiteral(lit)) = call.arguments.first()
    {
        return Some(lit.value.as_str());
    }
    None
}

impl Rule for CorpusNoBareImports {
    fn run_once(&self, ctx: &LintContext) {
        if !in_corpus_dir(ctx) || has_package_context(ctx.file_path()) {
            return;
        }

        for node in ctx.semantic().nodes() {
            match node.kind() {
                AstKind::ImportDeclaration(decl) => {
                    check_source(decl.source.value.as_str(), decl.span, ctx);
                }
                AstKind::ExportAllDeclaration(decl) => {
                    check_source(decl.source.value.as_str(), decl.span, ctx);
                }
                AstKind::CallExpression(call) => {
                    if let Some(source) = require_source(call) {
                        check_source(source, call.span, ctx);
                    }
                }
                _ => {}
            }
        }
    }
}

#[test]
fn test() {
    use crate::tester::Tester;

    let pass = vec![
        r"import { readFileSync } from 'node:fs';",
        r"import { readFileSync } from 'fs';",
        r"import { promises } from 'stream/promises';",
        r"import { helper } from './helper.ts';",
        r"import { up } from '../util.ts';",
        r"const fs = require('node:fs');",
        r"const util = require('./util.cjs');",
        r"console.log(1);",
    ];
    let fail = vec![
        r"import { red } from 'colorette';",
        r"import * as zod from 'zod';",
        r"const _ = require('lodash');",
        r"export * from 'everything';",
    ];

    Tester::new(CorpusNoBareImports::NAME, CorpusNoBareImports::PLUGIN, pass, fail)
        .change_rule_path("tests/corpus/9000-example.ts")
        .test_and_snapshot();
}
