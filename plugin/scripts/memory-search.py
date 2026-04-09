"""
Gargoyle memory search hook for Claude Code.

Runs on UserPromptSubmit: reads the user's prompt from stdin,
extracts search terms via erasmus inference, queries gargoyle
SQLite (FTS + embeddings), and returns relevant memories as
additionalContext.

Input (stdin JSON):
  {"session_id": "...", "cwd": "...", "hook_event_name": "UserPromptSubmit", "prompt": "..."}

Output (stdout JSON):
  {"hookSpecificOutput": {"hookEventName": "UserPromptSubmit", "additionalContext": "..."}}
"""

import json
import os
import sqlite3
import sys
import struct
import math

# --- Configuration (from plugin userConfig env vars or fallbacks) ---
DB_PATH = os.environ.get("CLAUDE_PLUGIN_OPTION_DB_PATH", "D:/apps/gargoyle/src-tauri/gargoyle.db")
ERASMUS_TEXT_URL = os.environ.get("CLAUDE_PLUGIN_OPTION_ERASMUS_TEXT_URL", "https://text-erasmus.ngrok.dev/v1/chat/completions")
ERASMUS_EMBED_URL = os.environ.get("CLAUDE_PLUGIN_OPTION_ERASMUS_EMBED_URL", "https://erasmus.ngrok.dev/embed")
SEEN_DIR = os.path.join(os.environ.get("TEMP", "/tmp"), "gargoyle-memory-hook")
MAX_RESULTS = 8
MAX_CONTEXT_CHARS = 3000
MIN_PROMPT_LENGTH = 10  # Skip very short prompts like "yes", "ok"


# --- Session dedup ---

def _seen_path(session_id: str) -> str:
    """File path for this session's seen-entity set."""
    os.makedirs(SEEN_DIR, exist_ok=True)
    safe_id = "".join(c if c.isalnum() or c in "-_" else "_" for c in session_id)
    return os.path.join(SEEN_DIR, f"{safe_id}.json")


def load_seen(session_id: str) -> set[str]:
    """Load entity IDs already surfaced in this session."""
    path = _seen_path(session_id)
    try:
        with open(path, "r") as f:
            return set(json.load(f))
    except (FileNotFoundError, json.JSONDecodeError):
        return set()


def save_seen(session_id: str, seen: set[str]) -> None:
    """Persist the seen set for this session."""
    path = _seen_path(session_id)
    with open(path, "w") as f:
        json.dump(list(seen), f)

# --- Helpers ---

def extract_keywords_local(prompt: str) -> list[str]:
    """Fallback keyword extraction without LLM — just split and filter."""
    stop_words = {
        "the", "a", "an", "is", "are", "was", "were", "be", "been", "being",
        "have", "has", "had", "do", "does", "did", "will", "would", "could",
        "should", "may", "might", "can", "shall", "to", "of", "in", "for",
        "on", "with", "at", "by", "from", "as", "into", "through", "during",
        "before", "after", "above", "below", "between", "out", "off", "over",
        "under", "again", "further", "then", "once", "here", "there", "when",
        "where", "why", "how", "all", "both", "each", "few", "more", "most",
        "other", "some", "such", "no", "nor", "not", "only", "own", "same",
        "so", "than", "too", "very", "just", "because", "but", "and", "or",
        "if", "while", "about", "this", "that", "these", "those", "what",
        "which", "who", "whom", "its", "it", "i", "me", "my", "we", "our",
        "you", "your", "he", "she", "they", "them", "his", "her", "their",
        "up", "also", "any", "let", "make", "like", "well", "get", "got",
        "know", "think", "want", "need", "use", "try", "tell", "give",
        "look", "find", "go", "come", "take", "see", "show", "help",
        "please", "thanks", "thank", "hi", "hello", "hey",
    }
    words = []
    for word in prompt.lower().split():
        cleaned = "".join(c for c in word if c.isalnum() or c in "-_")
        if cleaned and cleaned not in stop_words and len(cleaned) > 2:
            words.append(cleaned)
    # Deduplicate preserving order
    seen = set()
    unique = []
    for w in words:
        if w not in seen:
            seen.add(w)
            unique.append(w)
    return unique[:8]


def extract_keywords_llm(prompt: str) -> list[str]:
    """Use erasmus endpoint to extract search terms from prompt."""
    try:
        import httpx
        resp = httpx.post(
            ERASMUS_TEXT_URL,
            json={
                "model": "qwen2.5:7b-instruct",
                "messages": [
                    {
                        "role": "system",
                        "content": (
                            "Extract 3-6 search keywords from the user's message. "
                            "Focus on project names, technical terms, library names, "
                            "and concepts. Return ONLY a JSON array of strings, nothing else. "
                            "Example: [\"erasmus\", \"pipeline\", \"fastapi\"]"
                        ),
                    },
                    {"role": "user", "content": prompt},
                ],
                "max_tokens": 100,
                "temperature": 0,
            },
            timeout=3.0,
        )
        if resp.status_code == 200:
            content = resp.json()["choices"][0]["message"]["content"].strip()
            # Parse the JSON array from the response
            if content.startswith("["):
                return json.loads(content)
    except Exception:
        pass
    return []


def search_fts(conn: sqlite3.Connection, keywords: list[str]) -> list[dict]:
    """Full-text search across entities."""
    if not keywords:
        return []

    # Build FTS query: OR between keywords
    fts_query = " OR ".join(keywords)
    results = []

    try:
        rows = conn.execute(
            """
            SELECT e.id, e.entity_type, e.title, e.category, e.status,
                   substr(e.body_md, 1, 500) as body_preview,
                   e.canonical_fields
            FROM entities_fts fts
            JOIN entities e ON e.rowid = fts.rowid
            WHERE entities_fts MATCH ?
              AND e.deleted_at IS NULL
            ORDER BY rank
            LIMIT ?
            """,
            (fts_query, MAX_RESULTS * 2),
        ).fetchall()

        for row in rows:
            results.append({
                "id": row[0],
                "type": row[1],
                "title": row[2],
                "category": row[3],
                "status": row[4],
                "body_preview": row[5],
                "canonical_fields": row[6],
                "source": "fts",
            })
    except Exception:
        pass

    return results


def search_embeddings(conn: sqlite3.Connection, prompt: str) -> list[dict]:
    """Semantic similarity search via embeddings."""
    try:
        import httpx

        # Get embedding for the prompt
        resp = httpx.post(
            ERASMUS_EMBED_URL,
            json={"texts": [prompt]},
            timeout=3.0,
        )
        if resp.status_code != 200:
            return []

        query_embedding = resp.json()["embeddings"][0]

        # Get all entity embeddings
        rows = conn.execute(
            """
            SELECT emb.entity_id, emb.embedding, e.entity_type, e.title,
                   e.category, substr(e.body_md, 1, 500) as body_preview,
                   e.canonical_fields
            FROM embeddings emb
            JOIN entities e ON e.id = emb.entity_id
            WHERE e.deleted_at IS NULL
              AND emb.embedding IS NOT NULL
            """
        ).fetchall()

        scored = []
        for row in rows:
            entity_id, emb_blob, etype, title, category, body_preview, canonical = row
            if not emb_blob:
                continue

            # Decode embedding blob (array of f32)
            n_floats = len(emb_blob) // 4
            stored_emb = list(struct.unpack(f"<{n_floats}f", emb_blob))

            # Cosine similarity
            if len(stored_emb) != len(query_embedding):
                continue

            dot = sum(a * b for a, b in zip(query_embedding, stored_emb))
            norm_a = math.sqrt(sum(a * a for a in query_embedding))
            norm_b = math.sqrt(sum(b * b for b in stored_emb))
            if norm_a == 0 or norm_b == 0:
                continue
            similarity = dot / (norm_a * norm_b)

            if similarity > 0.3:  # Minimum threshold
                scored.append((similarity, {
                    "id": entity_id,
                    "type": etype,
                    "title": title,
                    "category": category,
                    "body_preview": body_preview,
                    "canonical_fields": canonical,
                    "source": "embedding",
                    "similarity": round(similarity, 3),
                }))

        scored.sort(key=lambda x: x[0], reverse=True)
        return [item for _, item in scored[:MAX_RESULTS]]

    except Exception:
        return []


def merge_results(fts_results: list[dict], emb_results: list[dict]) -> list[dict]:
    """Merge and deduplicate FTS + embedding results, boosting items found in both."""
    by_id = {}

    for r in fts_results:
        by_id[r["id"]] = {**r, "score": 1.0, "sources": {"fts"}}

    for r in emb_results:
        if r["id"] in by_id:
            # Boost items found in both searches
            by_id[r["id"]]["score"] = 1.5 + r.get("similarity", 0.5)
            by_id[r["id"]]["sources"].add("embedding")
        else:
            by_id[r["id"]] = {**r, "score": r.get("similarity", 0.5), "sources": {"embedding"}}

    results = list(by_id.values())
    results.sort(key=lambda x: x["score"], reverse=True)
    return results[:MAX_RESULTS]


def format_context(results: list[dict]) -> str:
    """Format results into a readable context string."""
    if not results:
        return ""

    lines = ["## Relevant memories from knowledge graph\n"]

    for r in results:
        type_label = r.get("type", "entity")
        title = r.get("title", "Untitled")
        category = r.get("category") or ""
        category_str = f" [{category}]" if category else ""

        lines.append(f"### {title}{category_str} ({type_label})")

        # Show tags if available
        try:
            fields = json.loads(r.get("canonical_fields") or "{}")
            tags = fields.get("tags", [])
            if tags:
                lines.append(f"Tags: {', '.join(tags[:6])}")
        except (json.JSONDecodeError, TypeError):
            pass

        # Show body preview
        body = (r.get("body_preview") or "").strip()
        if body:
            # Truncate to keep total context manageable
            if len(body) > 300:
                body = body[:297] + "..."
            lines.append(body)

        lines.append("")

    context = "\n".join(lines)

    # Hard cap on total context size
    if len(context) > MAX_CONTEXT_CHARS:
        context = context[:MAX_CONTEXT_CHARS] + "\n...(truncated)"

    return context


def main():
    # Read hook input from stdin
    try:
        raw = sys.stdin.read()
        hook_input = json.loads(raw)
    except (json.JSONDecodeError, Exception):
        # Can't parse input — exit silently
        sys.exit(0)

    prompt = hook_input.get("prompt", "")
    session_id = hook_input.get("session_id", "unknown")

    # Skip very short or empty prompts
    if len(prompt.strip()) < MIN_PROMPT_LENGTH:
        print(json.dumps({
            "hookSpecificOutput": {
                "hookEventName": "UserPromptSubmit",
            }
        }))
        sys.exit(0)

    # Extract search keywords
    keywords = extract_keywords_llm(prompt)
    if not keywords:
        keywords = extract_keywords_local(prompt)

    if not keywords:
        print(json.dumps({
            "hookSpecificOutput": {
                "hookEventName": "UserPromptSubmit",
            }
        }))
        sys.exit(0)

    # Search gargoyle
    try:
        conn = sqlite3.connect(f"file:{DB_PATH}?mode=ro", uri=True)
    except Exception:
        sys.exit(0)

    fts_results = search_fts(conn, keywords)
    emb_results = search_embeddings(conn, prompt)
    conn.close()

    merged = merge_results(fts_results, emb_results)

    # Filter out entities already surfaced in this session
    seen = load_seen(session_id)
    new_results = [r for r in merged if r["id"] not in seen]

    context = format_context(new_results)

    # Track newly surfaced entities
    if new_results:
        seen.update(r["id"] for r in new_results)
        save_seen(session_id, seen)

    output = {
        "hookSpecificOutput": {
            "hookEventName": "UserPromptSubmit",
        }
    }
    if context:
        output["hookSpecificOutput"]["additionalContext"] = context

    print(json.dumps(output))


if __name__ == "__main__":
    main()
