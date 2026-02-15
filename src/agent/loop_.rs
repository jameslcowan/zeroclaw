use crate::agent::tool_calls::{execute_tool_call, format_tool_result, parse_tool_calls, MAX_TOOL_ITERATIONS};
use crate::config::Config;
use crate::memory::{self, Memory, MemoryCategory};
use crate::observability::{self, Observer, ObserverEvent};
use crate::providers::{self, Provider};
use crate::runtime;
use crate::security::SecurityPolicy;
use crate::tools::{self, Tool};
use crate::util::truncate_with_ellipsis;
use anyhow::Result;
use std::fmt::Write;
use std::sync::Arc;
use std::time::Instant;

/// Build context preamble by searching memory for relevant entries
async fn build_context(mem: &dyn Memory, user_msg: &str) -> String {
    let mut context = String::new();

    // Pull relevant memories for this message
    if let Ok(entries) = mem.recall(user_msg, 5).await {
        if !entries.is_empty() {
            context.push_str("[Memory context]\n");
            for entry in &entries {
                let _ = writeln!(context, "- {}: {}", entry.key, entry.content);
            }
            context.push('\n');
        }
    }

    context
}

/// Handle LLM response with tool calling loop
///
/// This function parses tool calls from the LLM response, executes them,
/// and feeds the results back to the LLM until it provides a final answer.
async fn handle_response_with_tools(
    provider: &dyn Provider,
    system_prompt: &str,
    message: &str,
    model: &str,
    temperature: f64,
    tools: &[Box<dyn Tool>],
) -> Result<String> {
    let mut current_message = message.to_string();
    let mut iteration = 0;
    let mut full_response = String::new();

    loop {
        if iteration >= MAX_TOOL_ITERATIONS {
            tracing::warn!("Tool calling exceeded maximum iterations ({MAX_TOOL_ITERATIONS})");
            break;
        }

        let response = provider
            .chat_with_system(Some(system_prompt), &current_message, model, temperature)
            .await?;

        // Check if response contains tool calls
        let tool_calls = parse_tool_calls(&response);

        if tool_calls.is_empty() {
            // No tool calls, this is the final response
            full_response = response;
            break;
        }

        // Execute tool calls
        println!("\n🔧 Executing {} tool call(s)...", tool_calls.len());
        let mut tool_results = Vec::new();

        for tool_call in &tool_calls {
            println!("  → {} with args: {}", tool_call.name, tool_call.arguments);
            let result = execute_tool_call(tool_call, tools).await;
            let formatted = format_tool_result(tool_call, &result);
            println!("  ← {}", formatted);
            tool_results.push(formatted);
        }

        // Build next message with tool results
        current_message = format!(
            "Previous message: {}\n\nTool execution results:\n{}\n\nPlease continue based on these tool results.",
            message,
            tool_results.join("\n\n")
        );

        iteration += 1;
    }

    Ok(full_response)
}

#[allow(clippy::too_many_lines)]
pub async fn run(
    config: Config,
    message: Option<String>,
    provider_override: Option<String>,
    model_override: Option<String>,
    temperature: f64,
) -> Result<()> {
    // ── Wire up agnostic subsystems ──────────────────────────────
    let observer: Arc<dyn Observer> =
        Arc::from(observability::create_observer(&config.observability));
    let _runtime = runtime::create_runtime(&config.runtime)?;
    let security = Arc::new(SecurityPolicy::from_config(
        &config.autonomy,
        &config.workspace_dir,
    ));

    // ── Memory (the brain) ────────────────────────────────────────
    let mem: Arc<dyn Memory> = Arc::from(memory::create_memory(
        &config.memory,
        &config.workspace_dir,
        config.api_key.as_deref(),
    )?);
    tracing::info!(backend = mem.name(), "Memory initialized");

    // ── Tools (including memory tools) ────────────────────────────
    let composio_key = if config.composio.enabled {
        config.composio.api_key.as_deref()
    } else {
        None
    };
    let tools = tools::all_tools(&security, mem.clone(), composio_key, &config.browser);

    // ── Resolve provider ─────────────────────────────────────────
    let provider_name = provider_override
        .as_deref()
        .or(config.default_provider.as_deref())
        .unwrap_or("openrouter");

    let model_name = model_override
        .as_deref()
        .or(config.default_model.as_deref())
        .unwrap_or("anthropic/claude-sonnet-4-20250514");

    let provider: Box<dyn Provider> = providers::create_resilient_provider(
        provider_name,
        config.api_key.as_deref(),
        &config.reliability,
    )?;

    observer.record_event(&ObserverEvent::AgentStart {
        provider: provider_name.to_string(),
        model: model_name.to_string(),
    });

    // ── Build system prompt from workspace MD files (OpenClaw framework) ──
    let skills = crate::skills::load_skills(&config.workspace_dir);
    let mut tool_descs: Vec<(&str, &str)> = vec![
        (
            "shell",
            "Execute terminal commands. Use when: running local checks, build/test commands, diagnostics. Don't use when: a safer dedicated tool exists, or command is destructive without approval.",
        ),
        (
            "file_read",
            "Read file contents. Use when: inspecting project files, configs, logs. Don't use when: a targeted search is enough.",
        ),
        (
            "file_write",
            "Write file contents. Use when: applying focused edits, scaffolding files, updating docs/code. Don't use when: side effects are unclear or file ownership is uncertain.",
        ),
        (
            "memory_store",
            "Save to memory. Use when: preserving durable preferences, decisions, key context. Don't use when: information is transient/noisy/sensitive without need.",
        ),
        (
            "memory_recall",
            "Search memory. Use when: retrieving prior decisions, user preferences, historical context. Don't use when: answer is already in current context.",
        ),
        (
            "memory_forget",
            "Delete a memory entry. Use when: memory is incorrect/stale or explicitly requested for removal. Don't use when: impact is uncertain.",
        ),
    ];
    if config.browser.enabled {
        tool_descs.push((
            "browser_open",
            "Open approved HTTPS URLs in Brave Browser (allowlist-only, no scraping)",
        ));
    }
    if config.composio.enabled {
        tool_descs.push((
            "composio",
            "Execute actions on 1000+ apps via Composio (Gmail, Notion, GitHub, Slack, etc.). Use action='list' to discover, 'execute' to run, 'connect' to OAuth.",
        ));
    }
    let system_prompt = crate::channels::build_system_prompt(
        &config.workspace_dir,
        model_name,
        &tool_descs,
        &skills,
    );

    // ── Execute ──────────────────────────────────────────────────
    let start = Instant::now();

    if let Some(msg) = message {
        // Auto-save user message to memory
        if config.memory.auto_save {
            let _ = mem
                .store("user_msg", &msg, MemoryCategory::Conversation)
                .await;
        }

        // Inject memory context into user message
        let context = build_context(mem.as_ref(), &msg).await;
        let enriched = if context.is_empty() {
            msg.clone()
        } else {
            format!("{context}{msg}")
        };

        let response = handle_response_with_tools(
            provider.as_ref(),
            &system_prompt,
            &enriched,
            model_name,
            temperature,
            &tools,
        )
        .await?;
        println!("{response}");

        // Auto-save assistant response to daily log
        if config.memory.auto_save {
            let summary = truncate_with_ellipsis(&response, 100);
            let _ = mem
                .store("assistant_resp", &summary, MemoryCategory::Daily)
                .await;
        }
    } else {
        println!("🦀 ZeroClaw Interactive Mode");
        println!("Type /quit to exit.\n");

        let (tx, mut rx) = tokio::sync::mpsc::channel(32);
        let cli = crate::channels::CliChannel::new();

        // Spawn listener
        let listen_handle = tokio::spawn(async move {
            let _ = crate::channels::Channel::listen(&cli, tx).await;
        });

        while let Some(msg) = rx.recv().await {
            // Auto-save conversation turns
            if config.memory.auto_save {
                let _ = mem
                    .store("user_msg", &msg.content, MemoryCategory::Conversation)
                    .await;
            }

            // Inject memory context into user message
            let context = build_context(mem.as_ref(), &msg.content).await;
            let enriched = if context.is_empty() {
                msg.content.clone()
            } else {
                format!("{context}{}", msg.content)
            };

            let response = handle_response_with_tools(
                provider.as_ref(),
                &system_prompt,
                &enriched,
                model_name,
                temperature,
                &tools,
            )
            .await?;
            println!("\n{response}\n");

            if config.memory.auto_save {
                let summary = truncate_with_ellipsis(&response, 100);
                let _ = mem
                    .store("assistant_resp", &summary, MemoryCategory::Daily)
                    .await;
            }
        }

        listen_handle.abort();
    }

    let duration = start.elapsed();
    observer.record_event(&ObserverEvent::AgentEnd {
        duration,
        tokens_used: None,
    });

    Ok(())
}
