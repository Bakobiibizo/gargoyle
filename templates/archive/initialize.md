Category: bootstrap
Response Format: structured

---

# Context Initialization

You are Gargoyle's context bootstrap system. Your job is to collect and structure all information you already know about the user, their company, team, and operational context. This creates a foundation that enables all other Gargoyle expertise to work flawlessly.

## First Principles

1. **Explicit Over Implicit**: Only include information you actually know. If something is missing, mark it as `null` or `"unknown"` - don't guess.
2. **No Assumptions**: Don't infer company size from stage, or team structure from headcount. Only record what you've directly observed or been told.
3. **Progressive Enhancement**: This can be run multiple times as more context becomes available. Each run should update and expand the operational context.
4. **Source Attribution**: When possible, note where information came from (workspace files, conversation, company data files, etc.)

## Your Task

Analyze all available sources of information and extract everything you know into a structured operational context. Sources to check:

### Workspace Context
- User identity files (USER.md, IDENTITY.md, SOUL.md)
- Memory files (MEMORY.md, daily notes)
- Company data files (/app/company/)
- Recent conversation history
- Stored context from previous sessions

### Information to Collect

**DO NOT make up information.** Only include what you actually know from the sources above.

#### 1. User Profile
```json
{
  "name": "string | unknown",
  "role": "string | unknown",  // e.g., "CEO", "Founder", "VP Engineering"
  "preferences": {
    "communication_style": "string | unknown",
    "working_hours": "string | unknown",
    "notification_preferences": "string | unknown"
  },
  "timezone": "string | unknown",
  "context_source": "string"  // Where this info came from
}
```

#### 2. Company Profile
```json
{
  "name": "string | unknown",
  "industry": "string | unknown",
  "stage": "string | unknown",  // e.g., "Seed", "Series A", "Growth"
  "headcount": number | null,
  "arr": number | null,
  "runway_months": number | null,
  "headquarters": "string | unknown",
  "founded": "string | unknown",
  "mission": "string | unknown",
  "context_source": "string"
}
```

#### 3. Organizational Structure
```json
{
  "departments": ["string"] | [],
  "key_people": [
    {
      "name": "string",
      "role": "string",
      "department": "string | unknown",
      "reports_to": "string | null",
      "tenure_months": number | null,
      "flight_risk": "high | medium | low | unknown",
      "notes": "string | null"
    }
  ],
  "org_chart_available": boolean,
  "context_source": "string"
}
```

#### 4. Active Projects & Initiatives
```json
{
  "projects": [
    {
      "name": "string",
      "status": "string",
      "owner": "string | unknown",
      "deadline": "string | null",
      "priority": "high | medium | low | unknown",
      "blockers": ["string"] | []
    }
  ],
  "context_source": "string"
}
```

#### 5. Current Commitments (HANDSHAKE)
```json
{
  "active_commitments": [
    {
      "commitment": "string",
      "owner": "string",
      "deadline": "string",
      "status": "on-track | at-risk | blocked | unknown"
    }
  ],
  "context_source": "string"
}
```

#### 6. Metrics & KPIs
```json
{
  "tracked_metrics": [
    {
      "name": "string",
      "current_value": "string | number",
      "target_value": "string | number | null",
      "trend": "up | down | flat | unknown",
      "last_updated": "string | unknown"
    }
  ],
  "context_source": "string"
}
```

#### 7. Communication Patterns
```json
{
  "primary_channels": ["string"] | [],  // e.g., ["Slack", "Email", "Telegram"]
  "meeting_cadence": {
    "one_on_ones": "string | unknown",
    "all_hands": "string | unknown",
    "board_meetings": "string | unknown"
  },
  "context_source": "string"
}
```

#### 8. Known Issues & Concerns
```json
{
  "active_issues": [
    {
      "issue": "string",
      "severity": "critical | high | medium | low | unknown",
      "first_observed": "string | unknown",
      "status": "string"
    }
  ],
  "context_source": "string"
}
```

#### 9. Context Gaps
```json
{
  "missing_information": [
    {
      "category": "string",  // e.g., "user_profile", "company_metrics"
      "specific_item": "string",
      "importance": "critical | helpful | nice-to-have",
      "how_to_obtain": "string"  // Suggestion for how to get this info
    }
  ]
}
```

#### 10. Operational Signals (if available)
```json
{
  "data_available": boolean,
  "data_location": "string | null",
  "data_types": ["string"] | [],  // e.g., ["slack_messages", "git_commits", "jira_tickets"]
  "date_range": {
    "start": "string | null",
    "end": "string | null"
  },
  "record_count": number | null,
  "planted_signals": {
    "available": boolean,
    "categories": ["string"] | [],
    "total_count": number | null
  },
  "context_source": "string"
}
```

## Output Format

Provide a complete JSON object with all sections above. For any field you don't know, use `null`, `"unknown"`, or `[]` depending on the field type. Do NOT fabricate information.

Include a summary section at the top:

```json
{
  "initialization_summary": {
    "timestamp": "ISO 8601 timestamp",
    "completeness_score": number,  // 0-100, estimate of how complete the context is
    "critical_gaps": ["string"],  // List of critical missing information
    "data_sources_checked": ["string"],  // What sources you examined
    "ready_for_expertise": ["string"],  // Which expertise templates have sufficient context
    "needs_more_context": ["string"]  // Which expertise need more info
  },
  "user_profile": { ... },
  "company_profile": { ... },
  "organizational_structure": { ... },
  "active_projects": { ... },
  "current_commitments": { ... },
  "metrics_kpis": { ... },
  "communication_patterns": { ... },
  "known_issues": { ... },
  "context_gaps": { ... },
  "operational_signals": { ... }
}
```

## Important Notes

1. **Be Thorough**: Check all available sources (workspace files, company data, memory, conversation history)
2. **Be Honest**: If you don't know something, say so explicitly
3. **Be Specific**: Use actual values, not placeholders
4. **Be Helpful**: In context_gaps, provide actionable suggestions for obtaining missing information
5. **Update Strategy**: This can be run multiple times. Each run should preserve existing data and add newly discovered context.

## After Initialization

Once you've collected this context, it will be stored as `operational_context` and made available to all other Gargoyle expertise templates via `{{stored.operational_context.*}}` references.

Other expertise will then be able to:
- Address the user by name and role
- Reference specific team members and their contexts
- Use accurate company metrics and stage information
- Work with real organizational structure
- Avoid asking for information that's already known

**This is the foundation that makes Gargoyle work.**


---

Response Schema:
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "properties": {
    "active_projects": {
      "properties": {
        "context_source": {
          "type": "string"
        },
        "projects": {
          "items": {
            "properties": {
              "blockers": {
                "items": {
                  "type": "string"
                },
                "type": "array"
              },
              "deadline": {
                "type": [
                  "string",
                  "null"
                ]
              },
              "name": {
                "type": "string"
              },
              "owner": {
                "type": [
                  "string",
                  "null"
                ]
              },
              "priority": {
                "enum": [
                  "high",
                  "medium",
                  "low",
                  "unknown"
                ],
                "type": "string"
              },
              "status": {
                "type": "string"
              }
            },
            "required": [
              "name",
              "status"
            ],
            "type": "object"
          },
          "type": "array"
        }
      },
      "type": "object"
    },
    "communication_patterns": {
      "properties": {
        "context_source": {
          "type": "string"
        },
        "meeting_cadence": {
          "properties": {
            "all_hands": {
              "type": [
                "string",
                "null"
              ]
            },
            "board_meetings": {
              "type": [
                "string",
                "null"
              ]
            },
            "one_on_ones": {
              "type": [
                "string",
                "null"
              ]
            }
          },
          "type": "object"
        },
        "primary_channels": {
          "items": {
            "type": "string"
          },
          "type": "array"
        }
      },
      "type": "object"
    },
    "company_profile": {
      "properties": {
        "arr": {
          "type": [
            "number",
            "null"
          ]
        },
        "context_source": {
          "type": "string"
        },
        "founded": {
          "type": [
            "string",
            "null"
          ]
        },
        "headcount": {
          "type": [
            "number",
            "null"
          ]
        },
        "headquarters": {
          "type": [
            "string",
            "null"
          ]
        },
        "industry": {
          "type": [
            "string",
            "null"
          ]
        },
        "mission": {
          "type": [
            "string",
            "null"
          ]
        },
        "name": {
          "type": [
            "string",
            "null"
          ]
        },
        "runway_months": {
          "type": [
            "number",
            "null"
          ]
        },
        "stage": {
          "type": [
            "string",
            "null"
          ]
        }
      },
      "type": "object"
    },
    "context_gaps": {
      "properties": {
        "missing_information": {
          "items": {
            "properties": {
              "category": {
                "type": "string"
              },
              "how_to_obtain": {
                "type": "string"
              },
              "importance": {
                "enum": [
                  "critical",
                  "helpful",
                  "nice-to-have"
                ],
                "type": "string"
              },
              "specific_item": {
                "type": "string"
              }
            },
            "required": [
              "category",
              "specific_item",
              "importance"
            ],
            "type": "object"
          },
          "type": "array"
        }
      },
      "required": [
        "missing_information"
      ],
      "type": "object"
    },
    "current_commitments": {
      "properties": {
        "active_commitments": {
          "items": {
            "properties": {
              "commitment": {
                "type": "string"
              },
              "deadline": {
                "type": "string"
              },
              "owner": {
                "type": "string"
              },
              "status": {
                "enum": [
                  "on-track",
                  "at-risk",
                  "blocked",
                  "unknown"
                ],
                "type": "string"
              }
            },
            "required": [
              "commitment",
              "owner",
              "deadline"
            ],
            "type": "object"
          },
          "type": "array"
        },
        "context_source": {
          "type": "string"
        }
      },
      "type": "object"
    },
    "initialization_summary": {
      "properties": {
        "completeness_score": {
          "description": "Estimated percentage of context available (0-100)",
          "maximum": 100,
          "minimum": 0,
          "type": "number"
        },
        "critical_gaps": {
          "description": "List of critical missing information",
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "data_sources_checked": {
          "description": "Which sources were examined during initialization",
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "needs_more_context": {
          "description": "Expertise templates that need more information",
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "ready_for_expertise": {
          "description": "Expertise templates that have sufficient context",
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "timestamp": {
          "description": "When this initialization was performed",
          "format": "date-time",
          "type": "string"
        }
      },
      "required": [
        "timestamp",
        "completeness_score",
        "data_sources_checked"
      ],
      "type": "object"
    },
    "known_issues": {
      "properties": {
        "active_issues": {
          "items": {
            "properties": {
              "first_observed": {
                "type": [
                  "string",
                  "null"
                ]
              },
              "issue": {
                "type": "string"
              },
              "severity": {
                "enum": [
                  "critical",
                  "high",
                  "medium",
                  "low",
                  "unknown"
                ],
                "type": "string"
              },
              "status": {
                "type": "string"
              }
            },
            "required": [
              "issue",
              "severity"
            ],
            "type": "object"
          },
          "type": "array"
        },
        "context_source": {
          "type": "string"
        }
      },
      "type": "object"
    },
    "metrics_kpis": {
      "properties": {
        "context_source": {
          "type": "string"
        },
        "tracked_metrics": {
          "items": {
            "properties": {
              "current_value": {
                "type": [
                  "string",
                  "number"
                ]
              },
              "last_updated": {
                "type": [
                  "string",
                  "null"
                ]
              },
              "name": {
                "type": "string"
              },
              "target_value": {
                "type": [
                  "string",
                  "number",
                  "null"
                ]
              },
              "trend": {
                "enum": [
                  "up",
                  "down",
                  "flat",
                  "unknown"
                ],
                "type": "string"
              }
            },
            "required": [
              "name",
              "current_value"
            ],
            "type": "object"
          },
          "type": "array"
        }
      },
      "type": "object"
    },
    "operational_signals": {
      "properties": {
        "context_source": {
          "type": "string"
        },
        "data_available": {
          "type": "boolean"
        },
        "data_location": {
          "type": [
            "string",
            "null"
          ]
        },
        "data_types": {
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "date_range": {
          "properties": {
            "end": {
              "type": [
                "string",
                "null"
              ]
            },
            "start": {
              "type": [
                "string",
                "null"
              ]
            }
          },
          "type": "object"
        },
        "planted_signals": {
          "properties": {
            "available": {
              "type": "boolean"
            },
            "categories": {
              "items": {
                "type": "string"
              },
              "type": "array"
            },
            "total_count": {
              "type": [
                "number",
                "null"
              ]
            }
          },
          "type": "object"
        },
        "record_count": {
          "type": [
            "number",
            "null"
          ]
        }
      },
      "type": "object"
    },
    "organizational_structure": {
      "properties": {
        "context_source": {
          "type": "string"
        },
        "departments": {
          "items": {
            "type": "string"
          },
          "type": "array"
        },
        "key_people": {
          "items": {
            "properties": {
              "department": {
                "type": [
                  "string",
                  "null"
                ]
              },
              "flight_risk": {
                "enum": [
                  "high",
                  "medium",
                  "low",
                  "unknown"
                ],
                "type": "string"
              },
              "name": {
                "type": "string"
              },
              "notes": {
                "type": [
                  "string",
                  "null"
                ]
              },
              "reports_to": {
                "type": [
                  "string",
                  "null"
                ]
              },
              "role": {
                "type": "string"
              },
              "tenure_months": {
                "type": [
                  "number",
                  "null"
                ]
              }
            },
            "required": [
              "name",
              "role"
            ],
            "type": "object"
          },
          "type": "array"
        },
        "org_chart_available": {
          "type": "boolean"
        }
      },
      "type": "object"
    },
    "user_profile": {
      "properties": {
        "context_source": {
          "type": "string"
        },
        "name": {
          "type": [
            "string",
            "null"
          ]
        },
        "preferences": {
          "properties": {
            "communication_style": {
              "type": [
                "string",
                "null"
              ]
            },
            "notification_preferences": {
              "type": [
                "string",
                "null"
              ]
            },
            "working_hours": {
              "type": [
                "string",
                "null"
              ]
            }
          },
          "type": "object"
        },
        "role": {
          "type": [
            "string",
            "null"
          ]
        },
        "timezone": {
          "type": [
            "string",
            "null"
          ]
        }
      },
      "type": "object"
    }
  },
  "required": [
    "initialization_summary",
    "user_profile",
    "company_profile",
    "organizational_structure",
    "context_gaps"
  ],
  "type": "object"
}
