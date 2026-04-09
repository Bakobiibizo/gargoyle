use gargoyle_lib::agents::memory_agent::{MemoryAction, MemoryAgent, MemoryAgentRequest};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

fn setup_memory_db() -> Arc<Mutex<Connection>> {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(include_str!("../migrations/008_memory_tables.sql"))
        .unwrap();
    Arc::new(Mutex::new(conn))
}

#[test]
fn test_full_memory_flow() {
    let conn = setup_memory_db();
    let agent = MemoryAgent::new(conn.clone());

    // 1. Start a conversation
    let start = agent.handle(MemoryAgentRequest {
        action: MemoryAction::StartConversation,
    });
    assert!(start.success, "Failed to start conversation");
    let conv_id = start.conversation_id.unwrap();
    println!("✓ Started conversation: {}", conv_id);

    // 2. Record messages
    let msg1 = agent.handle(MemoryAgentRequest {
        action: MemoryAction::RecordMessage {
            conversation_id: conv_id.clone(),
            role: "user".to_string(),
            content: "I prefer using Rust for systems programming".to_string(),
        },
    });
    assert!(msg1.success, "Failed to record user message");
    println!("✓ Recorded user message");

    let msg2 = agent.handle(MemoryAgentRequest {
        action: MemoryAction::RecordMessage {
            conversation_id: conv_id.clone(),
            role: "assistant".to_string(),
            content: "Noted! Rust is great for performance-critical applications.".to_string(),
        },
    });
    assert!(msg2.success, "Failed to record assistant message");
    println!("✓ Recorded assistant message");

    // 3. Create an observation (STM)
    let obs = agent.handle(MemoryAgentRequest {
        action: MemoryAction::CreateObservation {
            content: "User prefers Rust for systems programming".to_string(),
            conversation_id: Some(conv_id.clone()),
        },
    });
    assert!(obs.success, "Failed to create observation");
    let stm_id = obs.memory_id.unwrap();
    println!("✓ Created STM observation: {}", stm_id);

    // 4. Search memories
    let search = agent.handle(MemoryAgentRequest {
        action: MemoryAction::SearchMemories {
            query: "Rust".to_string(),
            limit: Some(10),
        },
    });
    assert!(search.success, "Search failed");
    let memories = search.memories.unwrap();
    assert!(!memories.is_empty(), "No memories found for 'Rust'");
    println!("✓ Found {} memories for 'Rust'", memories.len());

    // 5. Promote to long-term memory
    let promote = agent.handle(MemoryAgentRequest {
        action: MemoryAction::PromoteToLongTerm {
            stm_id: stm_id.clone(),
            memory_type: "preference".to_string(),
            category: Some("programming".to_string()),
        },
    });
    assert!(promote.success, "Failed to promote to LTM");
    println!("✓ Promoted STM to LTM: {}", promote.memory_id.unwrap());

    // 6. End conversation
    let end = agent.handle(MemoryAgentRequest {
        action: MemoryAction::EndConversation {
            conversation_id: conv_id.clone(),
            summary: Some("Discussion about Rust preferences".to_string()),
        },
    });
    assert!(end.success, "Failed to end conversation");
    println!("✓ Ended conversation");

    // 7. Get context
    let ctx = agent.handle(MemoryAgentRequest {
        action: MemoryAction::GetContext {
            conversation_id: None,
            query: Some("programming language".to_string()),
        },
    });
    assert!(ctx.success, "Failed to get context");
    println!("✓ Retrieved context:\n{}", ctx.context.unwrap_or_default());

    // Verify data in DB
    let db = conn.lock().unwrap();
    let conv_count: i64 = db
        .query_row("SELECT COUNT(*) FROM conversations", [], |r| r.get(0))
        .unwrap();
    let seg_count: i64 = db
        .query_row("SELECT COUNT(*) FROM conversation_segments", [], |r| r.get(0))
        .unwrap();
    let stm_count: i64 = db
        .query_row("SELECT COUNT(*) FROM short_term_memories", [], |r| r.get(0))
        .unwrap();
    let ltm_count: i64 = db
        .query_row("SELECT COUNT(*) FROM long_term_memories", [], |r| r.get(0))
        .unwrap();

    println!("\n=== Database State ===");
    println!("Conversations: {}", conv_count);
    println!("Segments: {}", seg_count);
    println!("Short-term memories: {}", stm_count);
    println!("Long-term memories: {}", ltm_count);

    assert_eq!(conv_count, 1);
    assert_eq!(seg_count, 2);
    assert!(stm_count >= 1);
    assert!(ltm_count >= 1);

    println!("\n✅ All memory storage verified!");
}
