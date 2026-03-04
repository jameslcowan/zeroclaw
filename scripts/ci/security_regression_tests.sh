#!/usr/bin/env bash
set -euo pipefail

# Focused security regression suite covering critical auth/policy/secret paths.
# Keep tests narrowly scoped and deterministic so they can run in security CI.
TESTS=(
  run_tool_call_loop_denies_supervised_tools_on_non_cli_channels
  run_tool_call_loop_blocks_tools_excluded_for_channel
  webhook_rejects_public_traffic_without_auth_layers
  metrics_endpoint_rejects_public_clients_when_pairing_is_disabled
  metrics_endpoint_requires_bearer_token_when_pairing_is_enabled
  extract_ws_bearer_token_rejects_empty_tokens
  autonomy_config_serde_defaults_non_cli_excluded_tools
  config_validate_rejects_duplicate_non_cli_excluded_tools
  config_debug_redacts_sensitive_values
  config_save_encrypts_nested_credentials
  replayed_totp_code_is_rejected
  validate_command_execution_rejects_forbidden_paths
  screenshot_path_validation_blocks_escaped_paths
  test_execute_blocked_in_read_only_mode
  key_file_created_on_first_encrypt
  scrub_google_api_key_prefix
  scrub_aws_access_key_prefix
)

resolve_cargo_bin() {
  local requested="${CARGO_BIN:-}"
  local home_fallback="${CARGO_HOME:-$HOME/.cargo}/bin/cargo"

  if [ -n "${requested}" ] && [ -x "${requested}" ]; then
    printf '%s\n' "${requested}"
    return 0
  fi

  if command -v cargo >/dev/null 2>&1; then
    # Keep this as "cargo" so each invocation re-resolves PATH on the runner.
    printf '%s\n' "cargo"
    return 0
  fi

  if [ -x "${home_fallback}" ]; then
    printf '%s\n' "${home_fallback}"
    return 0
  fi

  if [ -n "${requested}" ]; then
    echo "error: CARGO_BIN is set to '${requested}' but is not executable, and no fallback cargo was found." >&2
  else
    echo "error: cargo binary not found in PATH or ${home_fallback}." >&2
  fi
  return 1
}

CARGO_BIN="$(resolve_cargo_bin)"

for test_name in "${TESTS[@]}"; do
  if [ "${CARGO_BIN}" != "cargo" ] && [ ! -x "${CARGO_BIN}" ]; then
    CARGO_BIN="$(resolve_cargo_bin)"
  fi
  echo "==> ${CARGO_BIN} test --locked --lib ${test_name}"
  "${CARGO_BIN}" test --locked --lib "${test_name}" -- --nocapture
done
