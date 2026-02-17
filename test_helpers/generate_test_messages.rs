//! Test message generator for Telegram integration testing.
//! Generates messages of various lengths for testing message splitting.
//!
//! Usage:
//!   cargo run --bin generate_test_messages -- [type]
//!
//! Available types:
//!   short      - Short message (< 100 chars)
//!   medium     - Medium message (~1000 chars)
//!   long       - Long message (~5000 chars, requires splitting)
//!   exact      - Exactly 4096 chars
//!   over       - Just over 4096 chars
//!   multi      - Very long (3+ chunks)
//!   newline    - Many newlines (tests line splitting)
//!   word       - Clear word boundaries
//!   all        - Show info for all types

use std::env;

const TELEGRAM_LIMIT: usize = 4096;

fn generate_short_message() -> String {
    "Hello! This is a short test message.".to_string()
}

fn generate_medium_message() -> String {
    "This is a medium-length test message. ".repeat(25)
}

fn generate_long_message() -> String {
    "This is a very long test message that will be split into multiple chunks. ".repeat(70)
}

fn generate_exact_limit_message() -> String {
    "x".repeat(TELEGRAM_LIMIT)
}

fn generate_over_limit_message() -> String {
    "x".repeat(4200)
}

fn generate_multi_chunk_message() -> String {
    "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(250)
}

fn generate_newline_message() -> String {
    "Line of text\n".repeat(400)
}

fn generate_word_boundary_message() -> String {
    "word ".repeat(1000)
}

fn print_message_info(message: &str, name: &str) {
    println!("\n{}", "=".repeat(60));
    println!("{}", name);
    println!("{}", "=".repeat(60));
    println!("Length: {} characters", message.len());
    println!(
        "Will split: {}",
        if message.len() > TELEGRAM_LIMIT { "Yes" } else { "No" }
    );
    if message.len() > TELEGRAM_LIMIT {
        let chunks = (message.len() + TELEGRAM_LIMIT - 1) / TELEGRAM_LIMIT;
        println!("Estimated chunks: {}", chunks);
    }
    println!("{}", "=".repeat(60));
    let preview = if message.len() > 200 {
        format!("{}...", &message[..200])
    } else {
        message.to_string()
    };
    println!("{}", preview);
    println!("{}\n", "=".repeat(60));
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let test_type = if args.len() > 1 {
        args[1].to_lowercase()
    } else {
        println!("Usage: generate_test_messages [type]");
        println!("\nAvailable types:");
        println!("  short      - Short message (< 100 chars)");
        println!("  medium     - Medium message (~1000 chars)");
        println!("  long       - Long message (~5000 chars, requires splitting)");
        println!("  exact      - Exactly 4096 chars");
        println!("  over       - Just over 4096 chars");
        println!("  multi      - Very long (3+ chunks)");
        println!("  newline    - Many newlines (tests line splitting)");
        println!("  word       - Clear word boundaries");
        println!("  all        - Show info for all types");
        println!("\nExample:");
        println!("  generate_test_messages long");
        std::process::exit(1);
    };

    let messages: Vec<(&str, String)> = vec![
        ("Short Message", generate_short_message()),
        ("Medium Message", generate_medium_message()),
        ("Long Message", generate_long_message()),
        ("Exact Limit (4096)", generate_exact_limit_message()),
        ("Just Over Limit", generate_over_limit_message()),
        ("Multi-Chunk Message", generate_multi_chunk_message()),
        ("Newline Test", generate_newline_message()),
        ("Word Boundary Test", generate_word_boundary_message()),
    ];

    let types_map: Vec<&str> = vec![
        "short", "medium", "long", "exact", "over", "multi", "newline", "word",
    ];

    if test_type == "all" {
        for (name, msg) in &messages {
            print_message_info(msg, name);
        }
    } else if let Some(idx) = types_map.iter().position(|&t| t == test_type) {
        let (_, msg) = &messages[idx];
        println!("{}", msg);
    } else {
        eprintln!("Error: Unknown type '{}'", test_type);
        eprintln!("Run without arguments to see available types.");
        std::process::exit(1);
    }
}
