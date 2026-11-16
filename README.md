# mcp-wire

The Model Context Protocol, as two Rust crates that agree on what goes over the wire.

| Crate | What it is |
|---|---|
| [`mcp-wire`](crates/mcp-wire) | The protocol's types: JSON-RPC envelopes, content blocks, capabilities, tool listings, elicitation schemas, log levels. Serde in, serde out, no runtime. |
| [`mcp-guest`](crates/mcp-guest) | An MCP client: stdio or streamable HTTP, tasks, sampling, elicitation. Built on `mcp-wire`. |

Servers come from [`mcp-host`](https://crates.io/crates/mcp-host), which re-exports `mcp-wire`, so a host and a guest compile against one definition of the protocol instead of two that drift.

```toml
[dependencies]
mcp-guest = "0.17"
```

## TLS

`mcp-guest` speaks plain HTTP over the `http` feature. For HTTPS pick exactly one provider: `tls-ring` or `tls-aws-lc`. CI checks each in isolation so one never sneaks into the other's dependency graph.

## Development

```sh
just qa            # fmt, clippy, tests
just qa-tls ring   # one TLS backend of mcp-guest, isolated
```

Rust 1.99 or newer. BSD-3-Clause.
