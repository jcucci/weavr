//! Shared confidence-scoring logic used by all language-specific mergers.

/// Computes confidence for pure import/using-directive merges.
///
/// `is_all_imports` — whether every item in the merged result is an import.
/// `has_base` — whether a common ancestor was available (3-way merge).
pub(crate) fn compute_import_confidence(is_all_imports: bool, has_base: bool) -> f32 {
    let base_score: f32 = if is_all_imports { 0.95 } else { 0.75 };
    let bonus: f32 = if has_base { 0.05 } else { 0.0 };
    (base_score + bonus).min(1.0)
}

/// Computes confidence for mixed merges (imports + structural items).
///
/// `has_import_merge` — whether the merge included import/using resolution.
/// `has_structural_merge` — whether the merge included structural merging (impl blocks, class members).
/// `has_base` — whether a common ancestor was available (3-way merge).
pub(crate) fn compute_mixed_confidence(
    has_import_merge: bool,
    has_structural_merge: bool,
    has_base: bool,
) -> f32 {
    let base: f32 = if has_import_merge && !has_structural_merge {
        0.95 // Pure import merge
    } else if has_structural_merge {
        0.80 // Structural member additions
    } else {
        0.75 // Mixed disjoint items
    };
    let bonus: f32 = if has_base { 0.05 } else { 0.0 };
    (base + bonus).min(1.0)
}
