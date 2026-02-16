# ZeroClaw — Claude Code Quick Reference

> **For full engineering protocol**, see [AGENTS.md](AGENTS.md)

This file loads automatically in Claude Code sessions for instant context and velocity.

---

## Project DNA

ZeroClaw = **trait-driven autonomous agent runtime** in Rust
- **3.4MB binary, <10ms startup, <5MB RAM**
- **7 swappable subsystems** — all traits, all factories
- **1,017 tests, 22+ providers, 8+ channels**

**Golden rule**: Extension = implement trait + register in factory. Read the trait first.

---

## Architecture at a Glance

| Subsystem | Trait | Factory | Quick Add |
|-----------|-------|---------|-----------|
| AI Models | `Provider` | `src/providers/mod.rs` | Implement `Provider` |
| Channels | `Channel` | `src/channels/mod.rs` | Implement `Channel` |
| Tools | `Tool` | `src/tools/mod.rs` | Implement `Tool` |
| Memory | `Memory` | `src/memory/mod.rs` | Implement `Memory` |
| Observability | `Observer` | `src/observability/mod.rs` | Implement `Observer` |
| Runtime | `RuntimeAdapter` | `src/runtime/mod.rs` | Implement `RuntimeAdapter` |
| Security | `SecurityPolicy` | `src/security/mod.rs` | Policy enforcement |

### Module Map (Mental Model)

```
src/main.rs          → CLI entry, command routing
src/config/          → schema, loading (treat as public API)
src/agent/           → orchestration loop
src/gateway/         → webhook server, pairing, auth
src/security/        → policy, secrets (HIGH BLAST RADIUS)
src/memory/          → SQLite + embeddings, vector search
src/providers/       → LLM providers, resilient wrapper
src/channels/        → Telegram, Discord, Slack, etc.
src/tools/           → shell, file, memory, browser (HIGH BLAST RADIUS)
src/runtime/         → native, docker, wasm adapters (HIGH BLAST RADIUS)
src/peripherals/     → hardware (STM32, RPi GPIO)
```

---

## Validation Commands (Muscle Memory)

### Quick Pre-Commit
```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

### Full Local CI (Recommended)
```bash
./dev/ci.sh all
```

### Build Variants
```bash
cargo build                           # Dev
cargo build --release                 # Release (~3.4MB)
CARGO_BUILD_JOBS=1 cargo build --release  # Low-memory (Raspberry Pi)
```

### Git Workflow
```bash
# Enable pre-push hooks (run once)
git config core.hooksPath .githooks

# Feature branch (never push to main directly)
git checkout -b feat/your-feature

# Conventional commit
git commit -m "feat(providers): add xyz provider"

# Skip hook only if necessary
git push --no-verify
```

---

## Naming Conventions (Copy-Paste)

| Category | Format | Examples |
|----------|--------|----------|
| Modules/files | `snake_case` | `memory_store.rs`, `provider_factory.rs` |
| Types/traits/enums | `PascalCase` | `SecurityPolicy`, `Provider`, `Channel` |
| Functions/variables | `snake_case` | `send_message()`, `config_path` |
| Constants | `SCREAMING_SNAKE_CASE` | `MAX_RETRIES`, `DEFAULT_TIMEOUT` |

**Domain-driven naming** (not implementation detail):
- ✅ `DiscordChannel`, `SecurityPolicy`, `MemoryStore`
- ❌ `Manager`, `Helper`, `Handler`, `Processor`

**Trait implementers** (explicit and predictable):
- ✅ `OpenAIProvider`, `TelegramChannel`, `ShellTool`, `SqliteMemory`
- ❌ `OAI`, `TG`, `Shell`, `SQL`

**Factory keys** (lowercase, user-facing):
- ✅ `"openai"`, `"discord"`, `"shell"`, `"sqlite"`

**Test naming** (behavior-first):
- ✅ `test_openai_provider_returns_streaming_response()`
- ❌ `test_provider()`

---

## Quick Playbooks

### Adding a Provider
1. Read `src/providers/traits.rs`
2. Implement `Provider` in `src/providers/<name>_provider.rs`
3. Register in `src/providers/mod.rs` factory
4. Add config schema entry if needed
5. Test factory wiring + error paths

### Adding a Channel
1. Read `src/channels/traits.rs`
2. Implement `send()`, `listen()`, `health_check()`, `typing`
3. Register in `src/channels/mod.rs` factory
4. Add config to `[channels_config.<name>]`
5. Test auth/allowlist/health

### Adding a Tool
1. Read `src/tools/traits.rs`
2. Implement with strict parameter schema
3. Validate and sanitize all inputs
4. Return structured `ToolResult`; avoid panics
5. **Security check**: Tools execute actions — validate twice

---

## Risk Tiers (Review Depth)

| Tier | Scope | Examples |
|------|-------|----------|
| **Low** | Docs, chore, tests-only | README fixes, typos |
| **Medium** | Most `src/**` behavior | Features, refactor |
| **High** | Security, runtime, gateway, tools, CI | Auth, sandbox, workflows |

**When uncertain, classify as HIGHER risk.**

---

## Security First (High Blast Radius Areas)

Handle with extreme care:
- `src/security/policy.rs` — access control, allowlists
- `src/gateway/` — public-facing HTTP surface
- `src/tools/` — executes arbitrary commands/IO
- `src/runtime/` — sandbox enforcement boundary
- `.github/workflows/` — CI automation

**Security first questions:**
1. Does this broaden permissions? Default to NO.
2. Is there rollback path? If not, split the change.
3. Are secrets/logs safe? Never log tokens/keys.

---

## Anti-Patterns (Avoid)

- ❌ Heavy dependencies for minor convenience
- ❌ Silently weakening security policy
- ❌ Speculative config/feature flags "just in case"
- ❌ Massive formatting changes mixed with functional changes
- ❌ Modifying unrelated modules "while we're here"
- ❌ Bypassing failing checks without explanation
- ❌ Hiding behavior changes in refactor commits
- ❌ Personal/sensitive data in code, tests, or commits

---

## Key Files Reference

| File | Purpose | Read Before... |
|------|---------|----------------|
| [AGENTS.md](AGENTS.md) | Full engineering protocol | Any significant work |
| `CONTRIBUTING.md` | Contribution guide | Opening PR |
| `docs/pr-workflow.md` | PR workflow policy | PR process |
| `docs/reviewer-playbook.md` | Reviewer guide | Reviewing PRs |
| `docs/ci-map.md` | CI ownership and triage | CI/workflow changes |
| `SECURITY.md` | Security disclosure | Security issues |
| `.env.example` | Environment variables | Config changes |
| `Cargo.toml` | Dependencies and features | Adding deps |

---

## Performance Invariants (Non-Negotiable)

- Binary size: ~3.4MB release build
- Startup time: <10ms on 0.8GHz cores
- Memory footprint: <5MB RAM
- Test count: 1,017+ tests

**Before adding dependencies:** Is it worth the binary size cost?

---

## Worktree Workflow (Multi-Track Development)

```bash
# Create isolated worktree
git worktree add ../zeroclaw-wt-provider feat/new-provider

# Work in isolation
cd ../zeroclaw-wt-provider
# ... make changes, validate, commit ...

# Remove when done
git worktree remove ../zeroclaw-wt-provider
```

---

## Identity-Safe Naming (Privacy First)

When identity-like context is unavoidable:
- ✅ `ZeroClawAgent`, `zeroclaw_user`, `zeroclaw_bot`, `zeroclaw_node`
- ❌ Real names, emails, tokens, IDs

**Never commit**: real names, emails, phone numbers, addresses, tokens, keys, credentials, private URLs.

---

## Final Reminder

> **"Extensions are trait implementations. Security is never optional. Performance is a feature."**

When in doubt:
1. Read the trait definition
2. Follow existing patterns
3. Keep changes scoped and reversible
4. Run validation before committing
5. Document security impact

---

**Built in the open. Built for speed. Built to last.** 🦀
