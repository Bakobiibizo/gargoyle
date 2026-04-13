// DedupPipeline: exact -> fuzzy -> embedding
//
// Runs post-commit (after entity creation succeeds) and is non-blocking.
// Entity creation is NEVER blocked by dedup failures.

use rusqlite::{params, Connection};
use std::collections::HashSet;
use strsim::levenshtein;

use crate::config::GargoyleConfig;
use crate::error::{GargoyleError, Result};
use crate::models::dedup::{DedupSuggestion, DetectionMethod};
use crate::services::indexer::IndexerService;

pub struct DedupPipeline;

impl DedupPipeline {
    /// Main entry point. Runs the 3-stage dedup pipeline in order:
    ///   1. Exact title match (case-insensitive) — confidence from config
    ///   2. Fuzzy title match (Levenshtein/trigram) — confidence from config
    ///   3. Embedding proximity — confidence from config
    ///
    /// Short-circuits after Stage 1 if a high-confidence (>= 0.95) match is found.
    ///
    /// For each duplicate found, inserts a row into `dedup_suggestions` with
    /// status='pending' and returns the suggestions.
    ///
    /// CRITICAL: Entity creation is never blocked. If any stage fails,
    /// the error is swallowed and an empty Vec is returned.
    pub fn check_for_duplicates(
        conn: &Connection,
        new_entity_id: &str,
    ) -> Result<Vec<DedupSuggestion>> {
        match Self::run_pipeline(conn, new_entity_id) {
            Ok(suggestions) => Ok(suggestions),
            Err(_) => {
                // Dedup must never block entity creation.
                // Swallow all errors and return empty vec.
                Ok(vec![])
            }
        }
    }

    /// List dedup suggestions, optionally filtered by status.
    pub fn get_suggestions(
        conn: &Connection,
        status: Option<&str>,
    ) -> Result<Vec<DedupSuggestion>> {
        let suggestions = match status {
            Some(s) => {
                let mut stmt = conn.prepare(
                    "SELECT suggestion_id, new_entity_id, existing_entity_id,
                            detection_method, confidence, status, created_at
                     FROM dedup_suggestions
                     WHERE status = ?1
                     ORDER BY confidence DESC",
                )?;
                let rows = stmt.query_map(params![s], Self::map_suggestion_row)?;
                rows.collect::<std::result::Result<Vec<_>, _>>()?
            }
            None => {
                let mut stmt = conn.prepare(
                    "SELECT suggestion_id, new_entity_id, existing_entity_id,
                            detection_method, confidence, status, created_at
                     FROM dedup_suggestions
                     ORDER BY confidence DESC",
                )?;
                let rows = stmt.query_map([], Self::map_suggestion_row)?;
                rows.collect::<std::result::Result<Vec<_>, _>>()?
            }
        };
        Ok(suggestions)
    }

    /// Resolve a suggestion by updating its status to 'accepted' or 'dismissed'.
    pub fn resolve_suggestion(
        conn: &Connection,
        suggestion_id: &str,
        new_status: &str,
    ) -> Result<()> {
        if new_status != "accepted" && new_status != "dismissed" {
            return Err(GargoyleError::Schema(format!(
                "Invalid dedup suggestion status: '{}'. Must be 'accepted' or 'dismissed'.",
                new_status
            )));
        }

        let rows_affected = conn.execute(
            "UPDATE dedup_suggestions SET status = ?1 WHERE suggestion_id = ?2",
            params![new_status, suggestion_id],
        )?;

        if rows_affected == 0 {
            return Err(GargoyleError::NotFound {
                entity_type: "dedup_suggestion".to_string(),
                id: suggestion_id.to_string(),
            });
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Run the full 3-stage pipeline. This is the inner function that may error;
    /// `check_for_duplicates` wraps it to swallow errors.
    fn run_pipeline(conn: &Connection, new_entity_id: &str) -> Result<Vec<DedupSuggestion>> {
        // Load the new entity's type and title
        let (entity_type, title): (String, String) = conn
            .query_row(
                "SELECT entity_type, title FROM entities WHERE id = ?1 AND deleted_at IS NULL",
                params![new_entity_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => GargoyleError::NotFound {
                    entity_type: "entity".to_string(),
                    id: new_entity_id.to_string(),
                },
                other => GargoyleError::Database(other),
            })?;

        let config = &GargoyleConfig::global().dedup;
        let mut suggestions: Vec<DedupSuggestion> = Vec::new();
        let mut found_ids: HashSet<String> = HashSet::new();

        // --- Stage 1: Exact title match (case-insensitive) ---
        {
            let mut stmt = conn.prepare(
                "SELECT id, title FROM entities
                 WHERE entity_type = ?1
                   AND id != ?2
                   AND deleted_at IS NULL
                   AND LOWER(title) = LOWER(?3)",
            )?;

            let rows = stmt.query_map(params![&entity_type, new_entity_id, &title], |row| {
                let id: String = row.get(0)?;
                let _title: String = row.get(1)?;
                Ok(id)
            })?;

            for row in rows {
                let existing_id = row?;
                found_ids.insert(existing_id.clone());
                let suggestion = Self::insert_suggestion(
                    conn,
                    new_entity_id,
                    &existing_id,
                    DetectionMethod::ExactTitle,
                    config.exact_match_confidence,
                )?;
                suggestions.push(suggestion);
            }
        }

        // Short-circuit: if Stage 1 found a high-confidence match, skip remaining stages
        if suggestions
            .iter()
            .any(|s| s.confidence >= config.exact_match_confidence)
        {
            return Ok(suggestions);
        }

        // --- Stage 2: Fuzzy title match (Levenshtein distance + trigram similarity) ---
        {
            let mut stmt = conn.prepare(
                "SELECT id, title FROM entities
                 WHERE entity_type = ?1
                   AND id != ?2
                   AND deleted_at IS NULL",
            )?;

            let rows = stmt.query_map(params![&entity_type, new_entity_id], |row| {
                let id: String = row.get(0)?;
                let t: String = row.get(1)?;
                Ok((id, t))
            })?;

            let new_title_lower = title.to_lowercase();

            for row in rows {
                let (existing_id, existing_title) = row?;

                // Skip entities already found by exact match
                if found_ids.contains(&existing_id) {
                    continue;
                }

                let existing_title_lower = existing_title.to_lowercase();

                // Criterion A: raw Levenshtein distance
                let lev_distance = levenshtein(&new_title_lower, &existing_title_lower);
                let is_lev_match = lev_distance <= config.levenshtein_max_distance;

                // Criterion B: trigram (Jaccard) similarity
                let is_trigram_match = trigram_similarity(&new_title_lower, &existing_title_lower)
                    > config.trigram_similarity_threshold;

                if is_lev_match || is_trigram_match {
                    found_ids.insert(existing_id.clone());
                    let suggestion = Self::insert_suggestion(
                        conn,
                        new_entity_id,
                        &existing_id,
                        DetectionMethod::FuzzyTitle,
                        config.fuzzy_match_confidence,
                    )?;
                    suggestions.push(suggestion);
                }
            }
        }

        // --- Stage 3: Embedding proximity ---
        {
            // Short acronym titles produce embeddings too semantically adjacent to
            // disambiguate reliably. Skip embedding proximity for short titles.
            if title.len() < config.min_title_length_for_embedding {
                return Ok(suggestions);
            }

            // Generate embedding for the new entity (best-effort)
            if let Err(_) = IndexerService::generate_embedding(conn, new_entity_id) {
                // If embedding generation fails, skip this stage
                return Ok(suggestions);
            }

            let search_results = match IndexerService::search_similar(
                conn,
                &title,
                10,
                Some(config.embedding_proximity_threshold),
            ) {
                Ok(results) => results,
                Err(_) => {
                    // If similarity search fails, skip this stage
                    return Ok(suggestions);
                }
            };

            for result in search_results {
                // Skip self
                if result.entity_id == new_entity_id {
                    continue;
                }
                // Skip already found
                if found_ids.contains(&result.entity_id) {
                    continue;
                }
                // CRITICAL: Skip cross-type matches
                if result.entity_type != entity_type {
                    continue;
                }

                found_ids.insert(result.entity_id.clone());
                let suggestion = Self::insert_suggestion(
                    conn,
                    new_entity_id,
                    &result.entity_id,
                    DetectionMethod::EmbeddingProximity,
                    config.embedding_match_confidence,
                )?;
                suggestions.push(suggestion);
            }
        }

        Ok(suggestions)
    }

    /// Insert a single dedup suggestion into the database and return it.
    fn insert_suggestion(
        conn: &Connection,
        new_entity_id: &str,
        existing_entity_id: &str,
        detection_method: DetectionMethod,
        confidence: f64,
    ) -> Result<DedupSuggestion> {
        let suggestion_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string();
        let method_str = detection_method.to_string();

        conn.execute(
            "INSERT INTO dedup_suggestions
                (suggestion_id, new_entity_id, existing_entity_id, detection_method, confidence, status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'pending', ?6)",
            params![
                &suggestion_id,
                new_entity_id,
                existing_entity_id,
                &method_str,
                confidence,
                &now,
            ],
        )?;

        Ok(DedupSuggestion {
            suggestion_id,
            new_entity_id: new_entity_id.to_string(),
            existing_entity_id: existing_entity_id.to_string(),
            detection_method,
            confidence,
            status: "pending".to_string(),
            created_at: now,
        })
    }

    /// Map a database row to a DedupSuggestion.
    fn map_suggestion_row(row: &rusqlite::Row) -> rusqlite::Result<DedupSuggestion> {
        let method_str: String = row.get(3)?;
        let detection_method = match method_str.as_str() {
            "exact_title" => DetectionMethod::ExactTitle,
            "fuzzy_title" => DetectionMethod::FuzzyTitle,
            "embedding_proximity" => DetectionMethod::EmbeddingProximity,
            _ => DetectionMethod::ExactTitle, // fallback, shouldn't happen with CHECK constraint
        };

        Ok(DedupSuggestion {
            suggestion_id: row.get(0)?,
            new_entity_id: row.get(1)?,
            existing_entity_id: row.get(2)?,
            detection_method,
            confidence: row.get(4)?,
            status: row.get(5)?,
            created_at: row.get(6)?,
        })
    }
}

/// Compute trigram (character 3-gram) Jaccard similarity between two strings.
///
/// Splits each string into overlapping character trigrams, then computes
/// |intersection| / |union|. Returns 0.0 if both strings are too short
/// to produce any trigrams.
fn trigram_similarity(a: &str, b: &str) -> f64 {
    fn trigrams(s: &str) -> HashSet<String> {
        let chars: Vec<char> = s.chars().collect();
        if chars.len() < 3 {
            return HashSet::new();
        }
        (0..chars.len() - 2)
            .map(|i| chars[i..i + 3].iter().collect::<String>())
            .collect()
    }

    let set_a = trigrams(a);
    let set_b = trigrams(b);

    if set_a.is_empty() && set_b.is_empty() {
        return 0.0;
    }

    let intersection = set_a.intersection(&set_b).count() as f64;
    let union = set_a.union(&set_b).count() as f64;

    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::create_memory_connection;
    use crate::db::migrations::run_migrations;

    /// Create an in-memory DB with the full schema applied.
    fn setup_db() -> Connection {
        let conn = create_memory_connection().expect("Failed to create in-memory connection");
        run_migrations(&conn).expect("Failed to run migrations");
        conn
    }

    /// Insert a test entity into the database.
    fn insert_entity(conn: &Connection, id: &str, entity_type: &str, title: &str) {
        let now = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string();
        conn.execute(
            "INSERT INTO entities (id, entity_type, title, body_md, source, canonical_fields, _schema_version, created_at, updated_at)
             VALUES (?1, ?2, ?3, '', 'manual', '{}', 1, ?4, ?4)",
            params![id, entity_type, title, &now],
        )
        .expect("Failed to insert test entity");
    }

    // -----------------------------------------------------------------------
    // Test 1: Exact title match
    // -----------------------------------------------------------------------
    #[test]
    fn test_exact_title_match() {
        let conn = setup_db();
        insert_entity(&conn, "m-existing", "metric", "Daily Users");
        insert_entity(&conn, "m-new", "metric", "Daily Users");

        let suggestions = DedupPipeline::check_for_duplicates(&conn, "m-new").unwrap();

        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].existing_entity_id, "m-existing");
        assert_eq!(suggestions[0].new_entity_id, "m-new");
        assert_eq!(suggestions[0].detection_method, DetectionMethod::ExactTitle);
        assert!((suggestions[0].confidence - 0.95).abs() < f64::EPSILON);
        assert_eq!(suggestions[0].status, "pending");
    }

    // -----------------------------------------------------------------------
    // Test 2: Exact title match is case-insensitive
    // -----------------------------------------------------------------------
    #[test]
    fn test_exact_title_case_insensitive() {
        let conn = setup_db();
        insert_entity(&conn, "m-existing", "metric", "Daily Users");
        insert_entity(&conn, "m-new", "metric", "daily users");

        let suggestions = DedupPipeline::check_for_duplicates(&conn, "m-new").unwrap();

        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].existing_entity_id, "m-existing");
        assert_eq!(suggestions[0].detection_method, DetectionMethod::ExactTitle);
        assert!((suggestions[0].confidence - 0.95).abs() < f64::EPSILON);
    }

    // -----------------------------------------------------------------------
    // Test 3: No cross-type dedup (CRITICAL)
    // -----------------------------------------------------------------------
    #[test]
    fn test_no_cross_type_dedup() {
        let conn = setup_db();
        insert_entity(&conn, "m-revenue", "metric", "Revenue");
        insert_entity(&conn, "e-revenue", "experiment", "Revenue");

        // Check for dupes of the experiment -- it should NOT match the metric
        let suggestions = DedupPipeline::check_for_duplicates(&conn, "e-revenue").unwrap();

        assert!(
            suggestions.is_empty(),
            "Cross-type matches must never be created. Got: {:?}",
            suggestions
                .iter()
                .map(|s| &s.existing_entity_id)
                .collect::<Vec<_>>()
        );
    }

    // -----------------------------------------------------------------------
    // Test 4: Fuzzy title match (Levenshtein distance <= 3)
    // -----------------------------------------------------------------------
    #[test]
    fn test_fuzzy_title_match() {
        let conn = setup_db();
        // "monthly revenue" vs "monthly revenues" -> Levenshtein distance = 1 (<=3), should match
        insert_entity(&conn, "m-existing", "metric", "Monthly Revenue");
        insert_entity(&conn, "m-new", "metric", "Monthly Revenues");

        let suggestions = DedupPipeline::check_for_duplicates(&conn, "m-new").unwrap();

        // Should find a fuzzy match (not exact, since titles differ)
        assert!(!suggestions.is_empty(), "Should find a fuzzy match");

        // Find the fuzzy match specifically
        let fuzzy = suggestions
            .iter()
            .find(|s| s.detection_method == DetectionMethod::FuzzyTitle);
        assert!(
            fuzzy.is_some(),
            "Should have a fuzzy match for 'Monthly Revenue' vs 'Monthly Revenues'"
        );

        let fuzzy = fuzzy.unwrap();
        // Confidence is fixed at 0.70 for all fuzzy matches per spec
        assert!(
            (fuzzy.confidence - 0.70).abs() < f64::EPSILON,
            "Fuzzy confidence should be fixed at 0.70"
        );
        assert_eq!(fuzzy.existing_entity_id, "m-existing");
    }

    // -----------------------------------------------------------------------
    // Test 5: Fuzzy below threshold produces no match
    // -----------------------------------------------------------------------
    #[test]
    fn test_fuzzy_below_threshold() {
        let conn = setup_db();
        insert_entity(&conn, "m-existing", "metric", "Revenue");
        insert_entity(&conn, "m-new", "metric", "Completely Different Title XYZ");

        let suggestions = DedupPipeline::check_for_duplicates(&conn, "m-new").unwrap();

        // No exact or fuzzy match should be found (titles are very different)
        let non_embedding = suggestions
            .iter()
            .filter(|s| {
                s.detection_method == DetectionMethod::ExactTitle
                    || s.detection_method == DetectionMethod::FuzzyTitle
            })
            .count();
        assert_eq!(
            non_embedding, 0,
            "Very different titles should not produce exact or fuzzy matches"
        );
    }

    // -----------------------------------------------------------------------
    // Test 6: Embedding proximity — short-circuit on exact match
    // -----------------------------------------------------------------------
    #[test]
    fn test_embedding_proximity() {
        let conn = setup_db();
        // Two entities with identical titles -> exact match in Stage 1
        insert_entity(&conn, "m-existing", "metric", "User Retention Rate");
        insert_entity(&conn, "m-new", "metric", "User Retention Rate");

        // Generate embedding for the existing entity so search_similar could find it
        IndexerService::generate_embedding(&conn, "m-existing").unwrap();

        let suggestions = DedupPipeline::check_for_duplicates(&conn, "m-new").unwrap();

        // The exact title match (confidence 0.95) triggers short-circuit,
        // so Stages 2 and 3 are skipped entirely. Only 1 suggestion.
        assert_eq!(
            suggestions.len(),
            1,
            "Short-circuit should yield exactly 1 suggestion"
        );
        assert_eq!(suggestions[0].detection_method, DetectionMethod::ExactTitle);
        assert!((suggestions[0].confidence - 0.95).abs() < f64::EPSILON);
    }

    // -----------------------------------------------------------------------
    // Test 6b: Short titles skip embedding proximity stage
    // -----------------------------------------------------------------------
    #[test]
    fn test_short_title_skips_embedding_proximity() {
        let conn = setup_db();
        // "MRR" is 3 chars (< 4), so embedding proximity should be skipped.
        // These titles are different enough that exact and fuzzy won't match either.
        insert_entity(&conn, "m-existing", "metric", "ARR");
        insert_entity(&conn, "m-new", "metric", "MRR");

        // Generate embedding for existing so it would be findable
        let _ = IndexerService::generate_embedding(&conn, "m-existing");

        let suggestions = DedupPipeline::check_for_duplicates(&conn, "m-new").unwrap();

        // Should NOT find an embedding proximity match because title is too short
        let embedding_matches: Vec<_> = suggestions
            .iter()
            .filter(|s| s.detection_method == DetectionMethod::EmbeddingProximity)
            .collect();
        assert!(
            embedding_matches.is_empty(),
            "Short titles (< 4 chars) should not trigger embedding proximity. Got {} matches.",
            embedding_matches.len()
        );
    }

    // -----------------------------------------------------------------------
    // Test 7: Dedup excludes self
    // -----------------------------------------------------------------------
    #[test]
    fn test_dedup_excludes_self() {
        let conn = setup_db();
        insert_entity(&conn, "m-only", "metric", "Unique Metric Title");

        let suggestions = DedupPipeline::check_for_duplicates(&conn, "m-only").unwrap();

        assert!(
            suggestions.is_empty(),
            "Entity should not match itself. Got {} suggestions.",
            suggestions.len()
        );
    }

    // -----------------------------------------------------------------------
    // Test 8: Dedup excludes soft-deleted entities
    // -----------------------------------------------------------------------
    #[test]
    fn test_dedup_excludes_deleted() {
        let conn = setup_db();
        insert_entity(&conn, "m-deleted", "metric", "Archived Metric");
        // Soft-delete it
        conn.execute(
            "UPDATE entities SET deleted_at = '2026-01-01T00:00:00.000Z' WHERE id = 'm-deleted'",
            [],
        )
        .unwrap();

        insert_entity(&conn, "m-new", "metric", "Archived Metric");

        let suggestions = DedupPipeline::check_for_duplicates(&conn, "m-new").unwrap();

        assert!(
            suggestions.is_empty(),
            "Soft-deleted entities should not be matched. Got {} suggestions.",
            suggestions.len()
        );
    }

    // -----------------------------------------------------------------------
    // Test 9: Get all suggestions
    // -----------------------------------------------------------------------
    #[test]
    fn test_get_suggestions_all() {
        let conn = setup_db();
        insert_entity(&conn, "m-1", "metric", "Alpha");
        insert_entity(&conn, "m-2", "metric", "Alpha");
        insert_entity(&conn, "m-3", "metric", "Alpha");

        // Create suggestions via the pipeline
        DedupPipeline::check_for_duplicates(&conn, "m-3").unwrap();

        let all = DedupPipeline::get_suggestions(&conn, None).unwrap();
        assert!(
            all.len() >= 2,
            "Should have at least 2 suggestions (m-3 matches m-1 and m-2). Got {}",
            all.len()
        );
    }

    // -----------------------------------------------------------------------
    // Test 10: Get suggestions filtered by status
    // -----------------------------------------------------------------------
    #[test]
    fn test_get_suggestions_filtered() {
        let conn = setup_db();
        insert_entity(&conn, "m-a", "metric", "Beta");
        insert_entity(&conn, "m-b", "metric", "Beta");

        let suggestions = DedupPipeline::check_for_duplicates(&conn, "m-b").unwrap();
        assert!(!suggestions.is_empty());

        // All should be pending initially
        let pending = DedupPipeline::get_suggestions(&conn, Some("pending")).unwrap();
        assert_eq!(pending.len(), suggestions.len());

        // Dismiss one
        DedupPipeline::resolve_suggestion(&conn, &suggestions[0].suggestion_id, "dismissed")
            .unwrap();

        // Now pending should be fewer
        let pending_after = DedupPipeline::get_suggestions(&conn, Some("pending")).unwrap();
        assert_eq!(pending_after.len(), suggestions.len() - 1);

        let dismissed = DedupPipeline::get_suggestions(&conn, Some("dismissed")).unwrap();
        assert_eq!(dismissed.len(), 1);
    }

    // -----------------------------------------------------------------------
    // Test 11: Resolve suggestion - accept
    // -----------------------------------------------------------------------
    #[test]
    fn test_resolve_suggestion_accept() {
        let conn = setup_db();
        insert_entity(&conn, "m-x", "metric", "Gamma");
        insert_entity(&conn, "m-y", "metric", "Gamma");

        let suggestions = DedupPipeline::check_for_duplicates(&conn, "m-y").unwrap();
        assert!(!suggestions.is_empty());

        DedupPipeline::resolve_suggestion(&conn, &suggestions[0].suggestion_id, "accepted")
            .unwrap();

        let accepted = DedupPipeline::get_suggestions(&conn, Some("accepted")).unwrap();
        assert_eq!(accepted.len(), 1);
        assert_eq!(accepted[0].suggestion_id, suggestions[0].suggestion_id);
        assert_eq!(accepted[0].status, "accepted");
    }

    // -----------------------------------------------------------------------
    // Test 12: Resolve suggestion - dismiss
    // -----------------------------------------------------------------------
    #[test]
    fn test_resolve_suggestion_dismiss() {
        let conn = setup_db();
        insert_entity(&conn, "m-p", "metric", "Delta");
        insert_entity(&conn, "m-q", "metric", "Delta");

        let suggestions = DedupPipeline::check_for_duplicates(&conn, "m-q").unwrap();
        assert!(!suggestions.is_empty());

        DedupPipeline::resolve_suggestion(&conn, &suggestions[0].suggestion_id, "dismissed")
            .unwrap();

        let dismissed = DedupPipeline::get_suggestions(&conn, Some("dismissed")).unwrap();
        assert_eq!(dismissed.len(), 1);
        assert_eq!(dismissed[0].status, "dismissed");
    }

    // -----------------------------------------------------------------------
    // Test 13: Invalid status returns error
    // -----------------------------------------------------------------------
    #[test]
    fn test_resolve_invalid_status() {
        let conn = setup_db();
        insert_entity(&conn, "m-inv1", "metric", "Epsilon");
        insert_entity(&conn, "m-inv2", "metric", "Epsilon");

        let suggestions = DedupPipeline::check_for_duplicates(&conn, "m-inv2").unwrap();
        assert!(!suggestions.is_empty());

        let result = DedupPipeline::resolve_suggestion(
            &conn,
            &suggestions[0].suggestion_id,
            "invalid_status",
        );
        assert!(result.is_err(), "Invalid status should return an error");

        match result.unwrap_err() {
            GargoyleError::Schema(msg) => {
                assert!(
                    msg.contains("invalid_status"),
                    "Error message should mention the invalid status"
                );
            }
            other => panic!("Expected Schema error, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Test 14: Dedup does not block entity creation (embedding failure)
    // -----------------------------------------------------------------------
    #[test]
    fn test_dedup_does_not_block_creation() {
        let conn = setup_db();

        // Call check_for_duplicates with a nonexistent entity id.
        // This will fail internally (entity not found), but should
        // NOT propagate the error -- it should return Ok(empty vec).
        let result = DedupPipeline::check_for_duplicates(&conn, "nonexistent-entity");
        assert!(
            result.is_ok(),
            "Dedup failure must not block entity creation"
        );
        assert!(
            result.unwrap().is_empty(),
            "Failed dedup should return empty vec"
        );
    }
}
