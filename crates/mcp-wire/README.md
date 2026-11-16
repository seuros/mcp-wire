# mcp-wire

The wire types of the Model Context Protocol: JSON-RPC envelopes, content
blocks, capabilities, tool listings and their naming and schema rules,
elicitation schemas and log levels.

Serde in, serde out, no runtime. [`mcp-host`](https://crates.io/crates/mcp-host)
(servers) re-exports it and [`mcp-guest`](https://crates.io/crates/mcp-guest)
(clients) builds on it, so both sides share one definition of the protocol.

```toml
[dependencies]
mcp-wire = "0.1"
```

Requires Rust 1.99 or newer. BSD-3-Clause.
