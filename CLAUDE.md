# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

kubetui is a Kubernetes TUI (Terminal User Interface) tool for real-time monitoring and exploration of Kubernetes resources. Written in Rust, it provides interactive views for pods, logs, configs, events, network resources, and YAML inspection.

## Build & Development Commands

```bash
# Build
cargo build

# Run all tests
cargo test --all

# Run a single test
cargo test <test_name>

# Run tests in a specific module
cargo test --lib <module_path>

# Format (nightly required for rustfmt.toml options)
cargo +nightly fmt

# Check formatting
cargo +nightly fmt --check

# Lint
cargo clippy
```

Linker: uses `mold` via `clang` on x86_64-linux-gnu (configured in `.cargo/config.toml`).

## Architecture

### Worker Threading Model

The application runs 4 concurrent workers connected via crossbeam bounded channels:

```
UserInput (thread) ──┐
                     ├──> Sender<Message> ──> Render (thread)
Tick (thread) ───────┤                          │
                     │                          │ KubeRequest
KubeWorker ──────────┘                          ▼
  (tokio async)                           KubeController
  ├─ PodPoller                            (dispatches to pollers)
  ├─ ConfigPoller
  ├─ NetworkPoller
  ├─ EventPoller
  └─ ApiPoller
```

- **KubeWorker** (`src/workers/kube/`): Async tokio runtime managing Kubernetes API interactions. `KubeController` (~700 lines) orchestrates feature-specific pollers.
- **Render** (`src/workers/render.rs`): Receives all messages, updates UI state, and renders frames via ratatui.
- **UserInput** (`src/workers/user_input.rs`): Polls terminal events (keyboard, mouse).
- **Tick** (`src/workers/tick.rs`): 200ms periodic refresh signal.

Channel capacities: input=128, kube=256, shutdown=1.

### Message System

Inter-thread communication uses `Message` enum (`src/message.rs`):
- `Message::User(UserEvent)` - keyboard/mouse input
- `Message::Kube(Kube)` - Kubernetes data updates and requests
- `Message::Tick` - UI refresh
- `Message::Error(NotifyError)` - error notifications displayed in UI

Each feature defines its own sub-message enum (e.g., `PodMessage`, `LogMessage`, `ConfigMessage`).

### Worker Traits

Defined in `src/workers/kube/worker.rs`:
- **`Worker`**: One-shot async task, returns `Output` via `JoinHandle`.
- **`InfiniteWorker`**: Long-running async loop (pollers), returns `AbortHandle` for cancellation.

### Feature Module Pattern

Each feature follows a consistent structure under `src/features/`:

```
features/{name}/
├── {name}.rs          # Public API, message types
├── view/              # UI components
└── kube/              # Kubernetes interactions
    ├── poller.rs      # InfiniteWorker impl
    └── ...            # Feature-specific logic
```

Features: pod, config, event, network, api_resources, yaml, get, namespace, context, help.

### UI Layer

Built on **ratatui** (v0.30.0):
- `Window` (`src/ui/window.rs`) manages tabs, each containing widgets.
- Custom widgets in `src/ui/widget/` (Table, Text, Input, SingleSelect, CheckList, etc.).
- Callback system via `define_callback!` macro (`src/ui/callback.rs`).

### Kubernetes Client

`KubeClient` (`src/kube/client.rs`) wraps `kube::Client` behind `KubeClientRequest` trait, enabling mock-based testing. Three request methods: `request_table`, `request`, `request_text`.

### Error Handling

`NotifyError` (`src/error.rs`) with `ErrorSource` enum categorizes errors by feature. Errors are sent as `Message::Error` to the UI for display. Workers continue running after errors.

### Configuration

Loaded via figment (`src/config/`): built-in defaults -> YAML file (`~/.config/kubetui/config.yaml`) -> environment variables (`KUBETUI__` prefix). Configurable: pod columns, theme, log buffer size, highlight rules.

## Testing

- Inline unit tests with `#[cfg(test)]` modules
- `mockall` / `mockall_double` for mocking `KubeClientRequest`
- `rstest` for parameterized tests
- `pretty_assertions` for readable diffs
- `indoc` for multi-line test fixtures
- Integration test infrastructure via KIND cluster (`make create-kind`)

## Key Conventions

- Rust edition 2021, stable toolchain for builds
- `rustfmt.toml`: `force_multiline_blocks = true`, `imports_layout = "HorizontalVertical"`
- `async-trait` for async traits, `enum_dispatch` for zero-cost enum dispatch
- Log queries use a custom nom-based parser supporting regex, jq, and jmespath filters
