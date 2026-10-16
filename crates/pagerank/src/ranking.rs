//! Exploitation des scores : les pages les mieux classées.

/// Les `k` meilleures pages, par score décroissant : `(indice, score)`.
///
/// À score égal, la plus petite page d'abord (résultat déterministe). Si `k` dépasse le
/// nombre de pages, toutes sont renvoyées.
#[must_use]
pub fn top_k(scores: &[f64], k: usize) -> Vec<(usize, f64)> {
    // Indice : `scores.iter().copied().enumerate().collect::<Vec<_>>()`, puis
    // `sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)))` et `truncate(k)`.
    // `total_cmp` compare des f64 (même NaN) ; `then` départage les égalités.
    // Bonus phase 5 : pour 1 million de pages et k = 10, trier tout est du gâchis
    // (`select_nth_unstable_by` fait mieux).
    let mut pages: Vec<(usize, f64)> = scores.iter().copied().enumerate().collect();

    pages.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));

    pages.truncate(k);
    pages
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn best_pages_first() {
        let top = top_k(&[0.1, 0.5, 0.15, 0.25], 2);
        assert_eq!(top, vec![(1, 0.5), (3, 0.25)]);
    }

    #[test]
    fn ties_are_broken_by_index() {
        assert_eq!(
            top_k(&[0.2, 0.4, 0.2, 0.2], 3),
            vec![(1, 0.4), (0, 0.2), (2, 0.2)]
        );
    }

    #[test]
    fn k_larger_than_n_returns_everything() {
        assert_eq!(top_k(&[0.3, 0.7], 10).len(), 2);
        assert_eq!(top_k(&[], 3), vec![]);
    }
}
