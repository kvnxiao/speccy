//! Deterministic provenance scan (DESIGN § Provenance Hygiene).
//!
//! Shipped product-file contents must carry no Speccy terminology or run
//! identifiers. This is layer 1: a zero-token deny-list scan over added diff
//! lines, run every round. Hits become blocking findings that feed the normal
//! repair round. Rendered packs, `.speccy/`, and exports are exempt.

/// A deny-list hit: an added line in a non-exempt file matching a deny term.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvenanceHit {
    pub file: String,
    pub line: usize,
    pub term: String,
}

/// Path prefixes exempt from the scan: rendered harness packs and `.speccy/`.
const EXEMPT_PREFIXES: &[&str] = &[
    ".speccy/",
    ".claude/",
    ".codex/",
    ".agents/",
    ".codex-plugin/",
];

/// Build the deny-list for a run: the universal `speccy` term plus this run's
/// identifiers (spec ref, spec/run IDs, requirement IDs) and any configured
/// extra terms. Bare task IDs are deliberately excluded (too short/common).
pub fn deny_terms(
    spec_ref: &str,
    spec_id: &str,
    run_id: &str,
    requirement_ids: impl IntoIterator<Item = String>,
    extra: &[String],
) -> Vec<String> {
    let mut terms = vec![
        "speccy".to_string(),
        spec_ref.to_string(),
        spec_id.to_string(),
        run_id.to_string(),
    ];
    terms.extend(requirement_ids);
    terms.extend(extra.iter().cloned());
    terms.retain(|t| !t.trim().is_empty());
    terms
}

/// Scan a unified diff's added lines for deny terms in non-exempt files.
pub fn scan_diff(diff: &str, terms: &[String]) -> Vec<ProvenanceHit> {
    let lowered: Vec<(String, String)> = terms
        .iter()
        .map(|t| (t.to_lowercase(), t.clone()))
        .collect();
    let mut hits = Vec::new();
    let mut current_file: Option<String> = None;
    let mut exempt = false;
    let mut new_line = 0usize;

    for line in diff.lines() {
        if let Some(path) = line.strip_prefix("+++ ") {
            let path = path.strip_prefix("b/").unwrap_or(path).trim();
            exempt = path == "/dev/null" || is_exempt(path);
            current_file = Some(path.to_string());
            continue;
        }
        if line.starts_with("--- ") {
            continue;
        }
        if let Some(rest) = line.strip_prefix("@@") {
            new_line = parse_hunk_new_start(rest);
            continue;
        }
        if let Some(added) = line.strip_prefix('+') {
            // (Not a `+++` header — those are handled above.)
            if !exempt {
                if let Some(file) = &current_file {
                    let lowered_added = added.to_lowercase();
                    if let Some((_, original)) = lowered
                        .iter()
                        .find(|(t, _)| lowered_added.contains(t.as_str()))
                    {
                        hits.push(ProvenanceHit {
                            file: file.clone(),
                            line: new_line,
                            term: original.clone(),
                        });
                    }
                }
            }
            new_line += 1;
        } else if line.starts_with(' ') || line.is_empty() {
            new_line += 1;
        }
        // Removed lines ('-') do not advance the new-file line counter.
    }
    hits
}

fn is_exempt(path: &str) -> bool {
    let norm = path.replace('\\', "/");
    EXEMPT_PREFIXES.iter().any(|p| norm.starts_with(p))
}

/// Parse the new-file start line from a hunk header `@@ -a,b +c,d @@`.
fn parse_hunk_new_start(rest: &str) -> usize {
    rest.split('+')
        .nth(1)
        .and_then(|s| s.split([',', ' ']).next())
        .and_then(|s| s.trim().parse::<usize>().ok())
        .unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIFF: &str = "\
diff --git a/src/auth.ts b/src/auth.ts
--- a/src/auth.ts
+++ b/src/auth.ts
@@ -1,2 +1,3 @@
 line one
+// satisfies R-AUTH-003 for the speccy run
 line two
diff --git a/.claude/agents/x.md b/.claude/agents/x.md
--- a/.claude/agents/x.md
+++ b/.claude/agents/x.md
@@ -0,0 +1,1 @@
+speccy reviewer persona references SPEC-20260630-A7F4
";

    #[test]
    fn flags_product_file_but_not_pack_file() {
        let terms = deny_terms(
            "SPEC-20260630-A7F4",
            "spec_x",
            "run_x",
            ["R-AUTH-003".into()],
            &[],
        );
        let hits = scan_diff(DIFF, &terms);
        // The product-file leak is flagged; the .claude pack leak is exempt.
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(hits[0].file, "src/auth.ts");
        assert_eq!(hits[0].line, 2);
    }

    #[test]
    fn clean_diff_has_no_hits() {
        let diff = "+++ b/src/ok.ts\n@@ -0,0 +1,1 @@\n+const x = 1;\n";
        let terms = deny_terms("SPEC-1", "spec_x", "run_x", [], &[]);
        assert!(scan_diff(diff, &terms).is_empty());
    }

    #[test]
    fn bare_task_ids_are_not_terms() {
        // T1 is not in the deny-list, so a diff mentioning it is clean.
        let diff = "+++ b/src/t.ts\n@@ -0,0 +1,1 @@\n+const T1 = compute();\n";
        let terms = deny_terms("SPEC-1", "spec_x", "run_x", ["R-AUTH-003".into()], &[]);
        assert!(scan_diff(diff, &terms).is_empty());
    }
}
