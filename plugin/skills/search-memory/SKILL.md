---
name: search-memory
description: Deep search the gargoyle knowledge graph for entities, docs, specs, and memories related to a topic
userInvocable: true
argumentHint: "<search query>"
allowedTools: ["Read", "Grep", "Glob", "Agent"]
---

Search the gargoyle knowledge graph thoroughly for information about "$ARGUMENTS".

Use the gargoyle MCP tools to:
1. Run `search_fts` with relevant keywords from the query
2. Run `search_similar` for semantic matches
3. For the most relevant results, use `get_entity_graph` to find connected entities (specs, docs, related projects)
4. Summarize what you found in a concise, structured format

Focus on surfacing: project descriptions, specs, architecture docs, decision context, and relationships between entities.
