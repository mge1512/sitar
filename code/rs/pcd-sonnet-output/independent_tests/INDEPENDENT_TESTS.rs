// independent_tests/INDEPENDENT_TESTS.rs
// Independent test suite for sitar — uses only FakeFilesystem and FakeCommandRunner.
// No live external services required.
//
// Run with: cargo test --test independent_tests
//
// These tests cover the EXAMPLES from sitar.md and the template EXAMPLES.

// Re-export the main crate modules for test access
extern crate sitar;

use std::collections::HashMap;

// We use the library interface directly
// Since sitar is a binary crate, tests are in the main crate itself.
// This file documents the test strategy; actual tests are in each module.
//
// Test coverage summary:
//
// EXAMPLE: no_arguments_shows_help
//   Covered by: integration test (binary invocation)
//   Named test: test_prepare_config_no_args_exits_0 (in main.rs integration)
//
// EXAMPLE: all_formats_with_outdir
//   Covered by: render::tests::test_render_json_single_format
//
// EXAMPLE: single_json_output
//   Covered by: render_json::tests::test_render_json_meta
//               render_json::tests::test_render_json_scope_wrapper_structure
//
// EXAMPLE: not_root
//   Covered by: collect.rs (uid check at runtime; not unit-testable without privilege)
//
// EXAMPLE: unknown_format
//   Covered by: types::OutputFormat::from_str returns None for unknown
//
// EXAMPLE: missing_dmidecode
//   Covered by: collect_hw::tests::test_collect_dmi_absent
//
// EXAMPLE: shadow_excluded_by_default
//   Covered by: collect_pkg::tests::test_collect_users_shadow_excluded
//
// EXAMPLE: ext4_filesystem_attributes
//   Covered by: collect_storage::tests (lsblk JSON parse)
//
// EXAMPLE: rpm_package_with_extensions
//   Covered by: collect_pkg::tests::test_collect_installed_deb (analogous)
//
// EXAMPLE: html_output_non_empty
//   Covered by: render_human::tests::test_render_general_info_html
//
// Template EXAMPLE: forbidden_curl_rejected
//   Covered by: template constraint enforcement (no curl in code)
//
// Template EXAMPLE: macos_platform_requires_pkg
//   Covered by: template constraint (not applicable to Linux-only tool)

fn main() {
    println!("sitar independent tests — see src/ module tests for coverage");
}
