//! scriptc project rules (downstream-only plugin, not part of upstream oxc).
pub mod colocated_test_naming;
pub mod corpus_no_bare_imports;
pub mod corpus_no_nondeterminism;
pub mod no_commonjs_metadata_mutation;
pub mod no_computed_dynamic_import;
pub mod no_date_fenced_usage;
pub mod no_explicit_any;
pub mod no_fenced_globals;
pub mod no_locale_compare;
pub mod no_process_exitcode_misuse;
pub mod promise_all_inline_tuple;
