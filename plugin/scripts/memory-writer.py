"""
Gargoyle memory writer hook for Claude Code.

Runs on Stop: reads the last assistant message and recent transcript,
asks erasmus LLM to extract noteworthy observations, and writes them
as typed entities to gargoyle SQLite.

Input (stdin JSON):
  {"session_id": "...", "transcript_path": "...", "last_assistant_message": "...", ...}

Output: none (Stop hooks don't inject context)
"""

import json
import os
import sqlite3
import sys
import uuid
from datetime import datetime, timezone

# --- Configuration (from plugin userConfig env vars or fallbacks) ---
DB_PATH = os.environ.get("CLAUDE_PLUGIN_OPTION_DB_PATH", "D:/apps/gargoyle/src-tauri/gargoyle.db")
ERASMUS_TEXT_URL = os.environ.get("CLAUDE_PLUGIN_OPTION_ERASMUS_TEXT_URL", "https://text-erasmus.ngrok.dev/v1/chat/completions")
SEEN_DIR = os.path.join(os.environ.get("TEMP", "/tmp"), "gargoyle-memory-hook")
TRANSCRIPT_TAIL = 6  # Number of recent messages to include as context

# Entity type menu for the LLM — only types present in gargoyle's hardcoded defaults
ENTITY_TYPE_MENU = """
Available entity types (pick the best fit for each observation):

- note: Quick observations, preferences, corrections, patterns discovered.
  Fields: context (string), tags (string array)

- idea: Concepts, approaches, or possibilities worth exploring later.
  Fields: stage (enum: seed|exploring|validated|parked), tags (string array), potential_value (string)

- task: Action items, TODOs, follow-ups that came up.
  Fields: assignee (string), due_date (YYYY-MM-DD), effort (enum: XS|S|M|L|XL), tags (string array)

- person: People mentioned with meaningful context about who they are.
  Fields: full_name (string), email (string), role (string), company (string), relationship (enum: colleague|manager|report|client|vendor|partner|other), notes (string)

- commitment: Deadlines, promises, scheduled deliverables.
  Fields: commitment_type (enum: deadline|meeting|delivery|review|milestone), due_date (YYYY-MM-DD), due_time (string), stakeholders (string array), deliverable (string)

- project: New projects or significant updates to known projects.
  Fields: owner (string), start_date (YYYY-MM-DD), target_end_date (YYYY-MM-DD), priority (enum: low|medium|high|critical), tags (string array)
"""

EXTRACTION_PROMPT = f"""You are a memory extraction agent. Given a conversation excerpt, extract 0-3 noteworthy observations worth remembering for future conversations.

{ENTITY_TYPE_MENU}

Rules:
- Only extract things that would be USEFUL in a FUTURE conversation
- Skip routine code output, debugging steps, generic acknowledgments
- DO save: decisions made, user corrections/preferences, discoveries, project context, people, deadlines
- Each observation needs: entity_type, title (short), body_md (the observation), and canonical_fields matching the type schema
- canonical_fields must ONLY contain fields listed above for that type. Omit fields you don't have data for.
- Return a JSON array of objects. Return [] if nothing is worth saving.

Example output:
[
  {{
    "entity_type": "note",
    "title": "User prefers direct SQLite writes over MCP",
    "body_md": "When gargoyle MCP create_entity panics due to runtime-in-runtime error, user approved direct SQLite inserts as workaround.",
    "canonical_fields": {{"context": "gargoyle plugin development", "tags": ["gargoyle", "sqlite", "workaround"]}}
  }},
  {{
    "entity_type": "task",
    "title": "Fix gargoyle MCP indexer panic",
    "body_md": "The indexer at src-tauri/src/services/indexer.rs:125 calls block_on inside tokio runtime, panicking on create_entity.",
    "canonical_fields": {{"tags": ["gargoyle", "bug", "mcp"], "effort": "M"}}
  }}
]"""


# --- Helpers ---

def _seen_path(session_id: str) -> str:
    os.makedirs(SEEN_DIR, exist_ok=True)
    safe_id = "".join(c if c.isalnum() or c in "-_" else "_" for c in session_id)
    return os.path.join(SEEN_DIR, f"writer-{safe_id}.json")


def load_written(session_id: str) -> set[str]:
    """Load titles already written in this session to avoid duplicates."""
    path = _seen_path(session_id)
    try:
        with open(path, "r") as f:
            return set(json.load(f))
    except (FileNotFoundError, json.JSONDecodeError):
        return set()


def save_written(session_id: str, written: set[str]) -> None:
    path = _seen_path(session_id)
    with open(path, "w") as f:
        json.dump(list(written), f)


def read_transcript_tail(transcript_path: str, n: int) -> list[dict]:
    """Read the last n messages from the JSONL transcript."""
    if not transcript_path or not os.path.exists(transcript_path):
        return []
    try:
        messages = []
        with open(transcript_path, "r", encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    messages.append(json.loads(line))
                except json.JSONDecodeError:
                    continue
        return messages[-n:] if len(messages) > n else messages
    except Exception:
        return []


def format_transcript_context(messages: list[dict], last_message: str) -> str:
    """Format recent transcript messages into a readable context string."""
    parts = []
    for msg in messages:
        role = msg.get("role", "unknown")
        content = ""
        if isinstance(msg.get("content"), str):
            content = msg["content"]
        elif isinstance(msg.get("content"), list):
            # Extract text blocks from content array
            for block in msg["content"]:
                if isinstance(block, dict) and block.get("type") == "text":
                    content += block.get("text", "")
        if content:
            # Truncate long messages
            if len(content) > 500:
                content = content[:497] + "..."
            parts.append(f"[{role}]: {content}")

    if last_message and not any(last_message[:50] in p for p in parts):
        if len(last_message) > 500:
            last_message = last_message[:497] + "..."
        parts.append(f"[assistant]: {last_message}")

    return "\n\n".join(parts)


def extract_observations(conversation_context: str) -> list[dict]:
    """Call erasmus endpoint to extract noteworthy observations."""
    try:
        import httpx
        resp = httpx.post(
            ERASMUS_TEXT_URL,
            json={
                "model": "qwen2.5:7b-instruct",
                "messages": [
                    {"role": "system", "content": EXTRACTION_PROMPT},
                    {"role": "user", "content": conversation_context},
                ],
                "max_tokens": 1000,
                "temperature": 0.2,
            },
            timeout=6.0,
        )
        if resp.status_code != 200:
            return []

        content = resp.json()["choices"][0]["message"]["content"].strip()

        # Find the JSON array in the response
        start = content.find("[")
        end = content.rfind("]")
        if start == -1 or end == -1:
            return []

        observations = json.loads(content[start:end + 1])
        if not isinstance(observations, list):
            return []

        return observations

    except Exception:
        return []


VALID_TYPES = {"note", "idea", "task", "person", "commitment", "project"}

# Canonical field schemas per type (for validation)
TYPE_FIELDS = {
    "note": {"context", "tags"},
    "idea": {"stage", "tags", "potential_value"},
    "task": {"assignee", "due_date", "effort", "tags"},
    "person": {"full_name", "email", "role", "company", "relationship", "notes"},
    "commitment": {"commitment_type", "due_date", "due_time", "stakeholders", "deliverable", "recurrence"},
    "project": {"owner", "start_date", "target_end_date", "actual_end_date", "priority", "tags"},
}


def validate_and_clean(obs: dict) -> dict | None:
    """Validate and clean an observation before writing."""
    etype = obs.get("entity_type", "")
    if etype not in VALID_TYPES:
        return None

    title = obs.get("title", "").strip()
    if not title or len(title) < 3:
        return None

    body = obs.get("body_md", "").strip()
    if not body:
        return None

    # Clean canonical fields — remove any fields not in the schema
    raw_fields = obs.get("canonical_fields", {})
    if not isinstance(raw_fields, dict):
        raw_fields = {}

    allowed = TYPE_FIELDS.get(etype, set())
    clean_fields = {k: v for k, v in raw_fields.items() if k in allowed}

    return {
        "entity_type": etype,
        "title": title,
        "body_md": body,
        "canonical_fields": clean_fields,
    }


def write_entity(conn: sqlite3.Connection, obs: dict) -> str:
    """Write a validated observation as a gargoyle entity."""
    eid = str(uuid.uuid4())
    now = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%S.000Z")
    run_id = str(uuid.uuid4())

    conn.execute(
        """INSERT INTO entities (id, entity_type, category, title, body_md, status, priority, due_at, created_at, updated_at, source, canonical_fields, _schema_version, provenance_run_id, deleted_at)
           VALUES (?, ?, NULL, ?, ?, NULL, NULL, NULL, ?, ?, 'agent', ?, 1, ?, NULL)""",
        (eid, obs["entity_type"], obs["title"], obs["body_md"], now, now,
         json.dumps(obs["canonical_fields"]), run_id)
    )

    # Update FTS index
    conn.execute(
        "INSERT OR REPLACE INTO entities_fts (rowid, title, body_md) "
        "SELECT rowid, title, body_md FROM entities WHERE id = ?",
        (eid,)
    )

    return eid


def main():
    try:
        raw = sys.stdin.read()
        hook_input = json.loads(raw)
    except (json.JSONDecodeError, Exception):
        sys.exit(0)

    session_id = hook_input.get("session_id", "unknown")
    last_message = hook_input.get("last_assistant_message", "")
    transcript_path = hook_input.get("transcript_path", "")

    # Skip if no assistant message
    if not last_message or len(last_message.strip()) < 20:
        sys.exit(0)

    # Build conversation context from transcript + last message
    recent = read_transcript_tail(transcript_path, TRANSCRIPT_TAIL)
    context = format_transcript_context(recent, last_message)

    if len(context.strip()) < 50:
        sys.exit(0)

    # Ask erasmus to extract observations
    observations = extract_observations(context)
    if not observations:
        sys.exit(0)

    # Load dedup set
    written = load_written(session_id)

    # Validate, dedup, and write
    try:
        conn = sqlite3.connect(DB_PATH)
    except Exception:
        sys.exit(0)

    new_count = 0
    for obs in observations:
        cleaned = validate_and_clean(obs)
        if not cleaned:
            continue

        # Dedup by title within session
        if cleaned["title"] in written:
            continue

        write_entity(conn, cleaned)
        written.add(cleaned["title"])
        new_count += 1

    conn.commit()
    conn.close()

    if new_count > 0:
        save_written(session_id, written)

    # Stop hooks don't return anything
    sys.exit(0)


if __name__ == "__main__":
    main()
