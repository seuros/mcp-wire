cargo := "cargo"

# Format, lint and test the whole workspace
qa: fmt-check clippy test

fmt-check:
    {{cargo}} fmt --all --check

clippy:
    {{cargo}} clippy --workspace --all-targets -- -D warnings

test:
    {{cargo}} test --workspace

# Check one MCP TLS backend of mcp-guest in isolation; AWS-LC also needs CMake.
qa-tls backend="ring":
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{backend}}" in
        ring|aws-lc) ;;
        *) echo "backend must be ring or aws-lc" >&2; exit 2 ;;
    esac
    features="stdio,tls-{{backend}}"
    {{cargo}} tree --locked -p mcp-guest --no-default-features --features "$features" --edges normal,build,dev --prefix none --format '{p}' |
        bash scripts/check-tls-dependencies.sh "{{backend}}"
    {{cargo}} clippy --locked -p mcp-guest --no-default-features --features "$features" --all-targets -- -D warnings
    {{cargo}} test --locked -p mcp-guest --no-default-features --features "$features"
