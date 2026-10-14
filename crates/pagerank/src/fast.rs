//! Phase 5 : PageRank parallèle et top-k sans tri complet.
//!
//! À mesurer avec `cargo bench -p pagerank` (benchmarks dans `benches/pagerank.rs`) AVANT et
//! APRÈS chaque optimisation : on n'optimise jamais à l'aveugle.

use markov::CsrMatrix;
use rayon::prelude::*;

use crate::{Options, PageRank, PageRankError, Transition, uniform};

/// Transition préparée pour le gather : on stocke `Pᵀ` (calculée une seule fois) au lieu de `P`.
#[derive(Debug, Clone, PartialEq)]
pub struct TransposedTransition {
    /// Transposée de la matrice de transition : `pt[j][i] = p[i][j]`.
    pub pt: CsrMatrix,
    /// Pages sans lien sortant (identique à [`Transition::dangling`]).
    pub dangling: Vec<bool>,
}

impl From<&Transition> for TransposedTransition {
    /// `From` est le trait standard de conversion : il donne aussi `t.into()` gratuitement.
    fn from(t: &Transition) -> Self {
        todo!()
    }
}

/// Même résultat que [`crate::google_step`], mais avec `pt.par_mul_vec(pi)` (gather parallèle,
/// car `π·P = Pᵀ·π`) et une somme parallèle pour la masse des pages dangling.
///
/// # Panics
/// Si `pi` ou `teleport` n'ont pas la bonne taille.
#[must_use]
pub fn google_step_parallel(
    pi: &[f64],
    tt: &TransposedTransition,
    alpha: f64,
    teleport: &[f64],
) -> Vec<f64> {
    // Indices rayon : `pi.par_iter().zip(&tt.dangling)` existe ; pour modifier `next` en
    // parallèle : `next.par_iter_mut().zip(teleport).for_each(|(x, &v)| …)`.
    todo!()
}

/// PageRank (départ et téléportation uniformes) calculé avec [`google_step_parallel`].
///
/// Mêmes erreurs et même boucle que [`crate::pagerank_from`]. Bonne occasion de refactoriser :
/// extraire la boucle de `pagerank_from` dans une fonction privée qui reçoit le pas sous forme
/// de closure (`impl Fn(&[f64]) -> Vec<f64>`), et l'appeler depuis les deux versions.
///
/// # Errors
/// Voir [`crate::pagerank_from`].
pub fn pagerank_parallel(t: &Transition, opts: &Options) -> Result<PageRank, PageRankError> {
    todo!()
}

/// Même résultat que [`crate::ranking::top_k`] (même ordre, mêmes égalités), sans trier toutes
/// les pages : `select_nth_unstable_by` place les k meilleures en tête en O(n), puis on ne trie
/// que ces k-là : O(n + k log k) au lieu de O(n log n).
#[must_use]
pub fn top_k_fast(scores: &[f64], k: usize) -> Vec<(usize, f64)> {
    // Attention aux cas k == 0 et k >= n (select_nth_unstable_by(k - 1) n'a de sens que pour
    // 1 <= k <= n).
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{google_step, pagerank, ranking::top_k};

    /// Graphe déterministe avec des pages dangling (une sur 10 n'a pas de lien sortant).
    fn web(n: usize) -> Transition {
        let mut triplets = Vec::new();
        for i in 0..n {
            if i % 10 == 9 {
                continue;
            }
            for j in [(i + 1) % n, i % 7, (i * 37 + 11) % n] {
                triplets.push((i, j, 1.0));
            }
        }
        let adjacency = CsrMatrix::from_triplets(n, n, triplets).unwrap();
        Transition::from_adjacency(&adjacency).unwrap()
    }

    fn assert_close(a: &[f64], b: &[f64], tol: f64) {
        assert_eq!(a.len(), b.len());
        for (i, (x, y)) in a.iter().zip(b).enumerate() {
            assert!((x - y).abs() < tol, "indice {i} : {x} != {y}");
        }
    }

    #[test]
    fn transposed_transition_swaps_entries() {
        let t = web(50);
        let tt = TransposedTransition::from(&t);
        assert_eq!(tt.dangling, t.dangling);
        for i in 0..50 {
            for (j, v) in t.p.row(i) {
                assert!((tt.pt.get(j, i) - v).abs() < 1e-15);
            }
        }
    }

    #[test]
    fn parallel_step_matches_sequential_step() {
        let t = web(1000);
        let tt = TransposedTransition::from(&t);
        let pi: Vec<f64> = {
            let raw: Vec<f64> = (0..1000).map(|i| f64::from(i % 13 + 1)).collect();
            let s: f64 = raw.iter().sum();
            raw.iter().map(|x| x / s).collect()
        };
        let u = uniform(1000);
        assert_close(
            &google_step_parallel(&pi, &tt, 0.85, &u),
            &google_step(&pi, &t, 0.85, &u),
            1e-12,
        );
    }

    #[test]
    fn parallel_pagerank_matches_sequential() {
        let t = web(5000);
        let opts = Options::default();
        let seq = pagerank(&t, &opts).unwrap();
        let par = pagerank_parallel(&t, &opts).unwrap();
        assert!(par.converged);
        assert_close(&par.scores, &seq.scores, 1e-9);
        // Les sommes ne sont pas faites dans le même ordre : les arrondis diffèrent un peu,
        // le test d'arrêt peut basculer une itération plus tôt ou plus tard.
        assert!(par.iterations.abs_diff(seq.iterations) <= 1);
    }

    #[test]
    fn parallel_pagerank_validates_alpha() {
        let opts = Options {
            alpha: 2.0,
            ..Options::default()
        };
        assert!(matches!(
            pagerank_parallel(&web(10), &opts),
            Err(PageRankError::InvalidAlpha(_))
        ));
    }

    #[test]
    fn top_k_fast_matches_top_k() {
        // Beaucoup d'égalités (scores entiers de 0 à 9) pour vérifier le départage par indice.
        let scores: Vec<f64> = (0..500).map(|i| f64::from((i * 7919) % 10)).collect();
        for k in [0, 1, 5, 50, 499, 500, 600] {
            assert_eq!(top_k_fast(&scores, k), top_k(&scores, k), "k = {k}");
        }
        assert_eq!(top_k_fast(&[], 3), vec![]);
    }
}
