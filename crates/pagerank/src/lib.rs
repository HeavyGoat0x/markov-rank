//! PageRank : distribution stationnaire de la matrice de Google, calculée sur une matrice creuse.
//!
//! Modèle du surfeur aléatoire : à chaque pas, avec probabilité `alpha` il suit un lien sortant
//! de la page courante (au hasard, proportionnellement aux poids), sinon il se « téléporte » sur
//! une page tirée selon la distribution `teleport` (uniforme par défaut). Une page sans lien
//! sortant (*dangling*) téléporte toujours.
//!
//! Matrice de Google : `G = α·(P + d·vᵀ) + (1 − α)·e·vᵀ`, avec `P` la matrice de transition,
//! `d` l'indicatrice des pages dangling, `v` la distribution de téléportation et `e` le vecteur
//! de 1. On ne construit JAMAIS `G` (n² cases, presque toutes non nulles). Un pas s'écrit :
//!
//! ```text
//! π·G = α·(π·P) + (α·Σ_{i dangling} π[i] + 1 − α) · v
//! ```
//!
//! Coût O(nnz) par itération grâce à [`CsrMatrix::vec_mul`].

pub mod fast; // phase 5 : PageRank parallèle, top-k rapide
pub mod perturb; // phase 4 : modifications du graphe et expérience du TD 2
pub mod ranking; // phase 4 : meilleures pages

use std::path::Path;

use markov::{CsrMatrix, MatrixError};
use mtx::MtxError;

/// Erreurs du calcul de PageRank.
#[derive(Debug, thiserror::Error)]
pub enum PageRankError {
    /// Erreur de lecture du fichier `.mtx`. `transparent` : on affiche le message d'origine.
    #[error(transparent)]
    Mtx(#[from] MtxError),

    /// Erreur de construction de la matrice creuse.
    #[error(transparent)]
    Matrix(#[from] MatrixError),

    #[error("la matrice d'adjacence doit être carrée, reçu {rows}×{cols}")]
    NotSquare { rows: usize, cols: usize },

    #[error("le graphe n'a aucun nœud")]
    EmptyGraph,

    #[error("alpha doit être dans [0, 1], reçu {0}")]
    InvalidAlpha(f64),

    #[error("{what} : taille {found}, attendu {expected}")]
    SizeMismatch {
        what: &'static str,
        expected: usize,
        found: usize,
    },

    #[error("{0} n'est pas une distribution (coefficients ≥ 0, somme égale à 1)")]
    NotDistribution(&'static str),
}

/// Graphe prêt pour PageRank : matrice de transition et pages sans lien sortant.
#[derive(Debug, Clone, PartialEq)]
pub struct Transition {
    /// Matrice stochastique : chaque ligne non vide somme à 1.
    pub p: CsrMatrix,
    /// `dangling[i]` vaut `true` si la page `i` n'a aucun lien sortant (ligne vide).
    pub dangling: Vec<bool>,
}

impl Transition {
    /// Construit la matrice de transition à partir d'une matrice d'adjacence :
    /// `adjacency[i][j]` = poids (≥ 0) du lien de `i` vers `j`.
    ///
    /// Chaque ligne est divisée par sa somme pour sommer à 1. Une ligne de somme nulle
    /// correspond à une page dangling : elle reste vide dans `p`.
    ///
    /// # Errors
    /// [`PageRankError::NotSquare`] si la matrice n'est pas carrée,
    /// [`PageRankError::EmptyGraph`] si elle est de taille 0.
    pub fn from_adjacency(adjacency: &CsrMatrix) -> Result<Self, PageRankError> {
        // Étapes :
        // 1) vérifier carré et non vide ;
        // 2) pour chaque ligne i : somme = Σ des valeurs de `adjacency.row(i)` ;
        //    si somme == 0 → dangling[i] = true, sinon pousser (i, j, v / somme) dans des triplets ;
        // 3) `CsrMatrix::from_triplets(n, n, triplets)?` : le `?` convertit `MatrixError`
        //    en `PageRankError::Matrix` grâce à `#[from]`.
        let rows = adjacency.n_rows();
        let cols = adjacency.n_cols();
        if rows == 0 {
            return Err(PageRankError::EmptyGraph);
        }
        if rows != cols {
            return Err(PageRankError::NotSquare { rows, cols });
        }
        let mut triplets: Vec<(usize, usize, f64)> = Vec::with_capacity(adjacency.nnz());
        let mut dangling: Vec<bool> = Vec::with_capacity(rows);
        for i in 0..rows {
            let somme: f64 = adjacency.row(i).map(|(_, val)| val).sum();
            if somme > 0.0 {
                for (col, val) in adjacency.row(i) {
                    triplets.push((i, col, val / somme));
                }
            }
            dangling.push(somme == 0.0);
        }
        let p = CsrMatrix::from_triplets(rows, cols, triplets)?;
        Ok(Self { p, dangling })
    }

    /// Lit un fichier `.mtx` où l'entrée `(i, j)` est un lien de la page `i` vers la page `j`
    /// (convention des graphes web SNAP de <https://sparse.tamu.edu>), ou un fichier `.txt`
    /// au format liste d'adjacence du TD (voir [`mtx::read_graph_file`]).
    ///
    /// # Errors
    /// Erreurs de lecture ([`PageRankError::Mtx`]), puis celles de [`Transition::from_adjacency`].
    pub fn from_mtx_file<P: AsRef<Path>>(path: P) -> Result<Self, PageRankError> {
        // Trois lignes, un `?` chacune : `mtx::read_graph_file`, `CsrMatrix::from_triplets`
        // (avec `coo.rows`, `coo.cols`, `coo.entries`), puis `Self::from_adjacency`.
        // Les `?` convertissent `MtxError` et `MatrixError` automatiquement (`#[from]`).
        let coo = mtx::read_graph_file(path)?;
        let adjacency: CsrMatrix = CsrMatrix::from_triplets(coo.rows, coo.cols, coo.entries)?;

        Transition::from_adjacency(&adjacency)
    }

    /// Nombre de pages.
    #[must_use]
    pub fn n(&self) -> usize {
        self.dangling.len()
    }

    /// Nombre de pages sans lien sortant.
    #[must_use]
    pub fn dangling_count(&self) -> usize {
        self.dangling.iter().filter(|&&d| d).count()
    }
}

/// Distribution uniforme sur `n` pages : `[1/n, …, 1/n]` (vide si `n == 0`).
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn uniform(n: usize) -> Vec<f64> {
    // Attention : clippy pedantic signale `n as f64` (`cast_precision_loss`) car un `usize`
    // très grand ne tient pas exactement dans un `f64`. Au-delà de 2⁵² pages seulement :
    // ici c'est sans risque, on peut le dire à clippy avec
    // `#[allow(clippy::cast_precision_loss)]` au-dessus de la fonction.
    if n == 0 {
        return Vec::new();
    }
    vec![1.0 / (n as f64); n]
}

/// Distance L1 : `Σ |a[i] − b[i]|`. C'est le critère d'arrêt de PageRank.
///
/// # Panics
/// Si `a` et `b` n'ont pas la même taille.
#[must_use]
pub fn l1_distance(a: &[f64], b: &[f64]) -> f64 {
    assert_eq!(
        a.len(),
        b.len(),
        "Les deux vecteurs n'ont pas la même taille"
    );
    a.iter().zip(b).map(|(x, y)| (x - y).abs()).sum()
}

/// Un pas de la chaîne de Google : renvoie `π·G` sans construire `G`.
///
/// `π·G = α·(π·P) + (α·Σ_{i dangling} π[i] + 1 − α) · teleport`
///
/// # Panics
/// Si `pi` ou `teleport` n'ont pas `t.n()` éléments.
#[must_use]
pub fn google_step(pi: &[f64], t: &Transition, alpha: f64, teleport: &[f64]) -> Vec<f64> {
    // Étapes :
    // 1) `let mut next = t.p.vec_mul(pi);` puis multiplier chaque case par alpha ;
    // 2) `dangling_mass` = somme des `pi[i]` pour les i dangling
    //    (indice : `pi.iter().zip(&t.dangling).filter(...).map(...).sum()`) ;
    // 3) `let jump = alpha * dangling_mass + 1.0 - alpha;`
    //    puis `next[j] += jump * teleport[j]` pour tout j.
    let mut next = t.p.vec_mul(pi);
    for x in &mut next {
        *x *= alpha;
    }
    let dangling_mass: f64 = pi
        .iter()
        .zip(&t.dangling)
        .filter(|&(_, &b)| b)
        .map(|(&e, _)| e)
        .sum();
    let jump = alpha * dangling_mass + 1.0 - alpha;
    for (x, &tp) in next.iter_mut().zip(teleport) {
        *x += jump * tp;
    }
    next
}

/// Paramètres de l'itération.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Options {
    /// Probabilité de suivre un lien (0.85 dans l'article original de Brin et Page).
    pub alpha: f64,
    /// On s'arrête dès que `‖π(k+1) − π(k)‖₁ < epsilon`.
    pub epsilon: f64,
    /// Nombre maximal d'itérations.
    pub max_iter: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            alpha: 0.85,
            epsilon: 1e-10,
            max_iter: 1000,
        }
    }
}

/// Résultat de PageRank.
#[derive(Debug, Clone, PartialEq)]
pub struct PageRank {
    /// Score de chaque page (une distribution : somme égale à 1).
    pub scores: Vec<f64>,
    /// Nombre de pas effectués (c'est ce que le TD 2 demande de comparer).
    pub iterations: usize,
    /// `‖π(k) − π(k−1)‖₁` au dernier pas (`f64::INFINITY` si aucun pas).
    pub residual: f64,
    /// `true` si `residual < epsilon` avant `max_iter`.
    pub converged: bool,
}

/// PageRank classique : départ uniforme, téléportation uniforme.
///
/// # Errors
/// Voir [`pagerank_from`].
pub fn pagerank(t: &Transition, opts: &Options) -> Result<PageRank, PageRankError> {
    let start = uniform(t.n());
    pagerank_from(t, opts, &start, &start)
}

/// PageRank avec un point de départ et une téléportation choisis.
///
/// - `start` : distribution initiale π(0). Partir de l'ancien PageRank après une petite
///   modification du graphe (*warm start*) converge beaucoup plus vite : c'est la question du TD 2.
/// - `teleport` : distribution de téléportation `v`. Concentrée sur quelques pages, elle donne le
///   **PageRank personnalisé** (pages importantes « vues depuis » ces pages-là) : on s'en servira
///   pour trouver les pages utiles aux étudiants internationaux sur un site d'université.
///
/// Boucle : pour k = 1, 2, …, `max_iter` : `next = google_step(π)`, `residual = ‖next − π‖₁`,
/// `π = next` ; si `residual < epsilon`, on s'arrête avec `iterations = k` et `converged = true`.
///
/// # Errors
/// Dans cet ordre : [`PageRankError::EmptyGraph`], [`PageRankError::InvalidAlpha`] (hors de
/// [0, 1] ou NaN), [`PageRankError::SizeMismatch`] (`"start"` puis `"teleport"`),
/// [`PageRankError::NotDistribution`] (`"start"` puis `"teleport"`).
pub fn pagerank_from(
    t: &Transition,
    opts: &Options,
    start: &[f64],
    teleport: &[f64],
) -> Result<PageRank, PageRankError> {
    // Indices :
    // - alpha valide : `(0.0..=1.0).contains(&opts.alpha)` (faux aussi pour NaN) ;
    // - distribution : `markov::is_distribution` existe déjà (phase 1) ;
    // - si max_iter == 0 : renvoyer `start` avec iterations 0, residual INFINITY, converged false.
    if t.n() == 0 {
        return Err(PageRankError::EmptyGraph);
    }

    if !(0.0..=1.0).contains(&opts.alpha) {
        return Err(PageRankError::InvalidAlpha(opts.alpha));
    }

    if start.len() != t.n() {
        return Err(PageRankError::SizeMismatch {
            what: "start",
            expected: t.n(),
            found: start.len(),
        });
    }

    if teleport.len() != t.n() {
        return Err(PageRankError::SizeMismatch {
            what: "teleport",
            expected: t.n(),
            found: teleport.len(),
        });
    }

    if !markov::is_distribution(start) {
        return Err(PageRankError::NotDistribution("start"));
    }

    if !markov::is_distribution(teleport) {
        return Err(PageRankError::NotDistribution("teleport"));
    }

    let mut iterations: usize = 0;

    let mut pi: Vec<f64> = start.to_vec();
    let mut residual = f64::INFINITY;
    for _ in 0..opts.max_iter {
        let next = google_step(&pi, t, opts.alpha, teleport);
        iterations += 1;
        residual = l1_distance(&next, &pi);
        if residual < opts.epsilon {
            return Ok(PageRank {
                scores: next,
                iterations,
                residual,
                converged: true,
            });
        }
        pi = next;
    }
    Ok(PageRank {
        scores: pi,
        iterations,
        residual,
        converged: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use markov::{DenseMatrix, iterate, step};
    use proptest::prelude::*;

    const TOL: f64 = 1e-9;

    fn assert_close(a: &[f64], b: &[f64], tol: f64) {
        assert_eq!(a.len(), b.len(), "tailles différentes");
        for (i, (x, y)) in a.iter().zip(b).enumerate() {
            assert!((x - y).abs() < tol, "indice {i} : {x} != {y}");
        }
    }

    /// Graphe non pondéré à `n` nœuds à partir d'une liste de liens `(de, vers)`.
    fn graph(n: usize, links: &[(usize, usize)]) -> Transition {
        let triplets = links.iter().map(|&(i, j)| (i, j, 1.0)).collect();
        let adjacency = CsrMatrix::from_triplets(n, n, triplets).unwrap();
        Transition::from_adjacency(&adjacency).unwrap()
    }

    /// Graphe de test avec une page dangling (3) et des liens non symétriques.
    fn web4() -> Transition {
        graph(4, &[(0, 1), (0, 2), (1, 2), (1, 3), (2, 0), (2, 1)])
    }

    /// Matrice de Google construite explicitement (dense) : la référence des tests.
    #[allow(clippy::cast_precision_loss)]
    fn dense_google(t: &Transition, alpha: f64) -> DenseMatrix {
        let n = t.n();
        let nf = n as f64;
        let rows = (0..n)
            .map(|i| {
                (0..n)
                    .map(|j| {
                        let d = if t.dangling[i] { 1.0 / nf } else { 0.0 };
                        alpha * (t.p.get(i, j) + d) + (1.0 - alpha) / nf
                    })
                    .collect()
            })
            .collect();
        DenseMatrix::from_rows(rows).unwrap()
    }

    // ---------- Transition ----------

    #[test]
    fn from_adjacency_normalizes_rows() {
        let t = graph(3, &[(0, 1), (0, 2), (1, 2), (2, 0)]);
        assert!((t.p.get(0, 1) - 0.5).abs() < TOL);
        assert!((t.p.get(0, 2) - 0.5).abs() < TOL);
        assert!((t.p.get(1, 2) - 1.0).abs() < TOL);
        assert!((t.p.get(2, 0) - 1.0).abs() < TOL);
        assert_eq!(t.dangling, vec![false, false, false]);
        assert_eq!(t.n(), 3);
    }

    #[test]
    fn from_adjacency_uses_weights() {
        let adjacency =
            CsrMatrix::from_triplets(2, 2, vec![(0, 0, 2.0), (0, 1, 6.0), (1, 0, 1.0)]).unwrap();
        let t = Transition::from_adjacency(&adjacency).unwrap();
        assert!((t.p.get(0, 0) - 0.25).abs() < TOL);
        assert!((t.p.get(0, 1) - 0.75).abs() < TOL);
        assert!((t.p.get(1, 0) - 1.0).abs() < TOL);
    }

    #[test]
    fn from_adjacency_detects_dangling() {
        let t = web4();
        assert_eq!(t.dangling, vec![false, false, false, true]);
        assert_eq!(t.dangling_count(), 1);
        assert_eq!(t.p.row(3).count(), 0);
    }

    #[test]
    fn from_adjacency_rejects_bad_shapes() {
        let rect = CsrMatrix::from_triplets(2, 3, vec![]).unwrap();
        assert!(matches!(
            Transition::from_adjacency(&rect),
            Err(PageRankError::NotSquare { rows: 2, cols: 3 })
        ));
        let empty = CsrMatrix::from_triplets(0, 0, vec![]).unwrap();
        assert!(matches!(
            Transition::from_adjacency(&empty),
            Err(PageRankError::EmptyGraph)
        ));
    }

    // ---------- Petites fonctions ----------

    #[test]
    fn uniform_distribution() {
        assert_close(&uniform(4), &[0.25; 4], TOL);
        assert_eq!(uniform(0), Vec::<f64>::new());
    }

    #[test]
    fn l1_distance_sums_absolute_differences() {
        assert!((l1_distance(&[0.5, 0.5], &[1.0, 0.0]) - 1.0).abs() < TOL);
        assert!(l1_distance(&[0.1, 0.2], &[0.1, 0.2]).abs() < TOL);
    }

    // ---------- Un pas ----------

    #[test]
    fn google_step_matches_dense_google_matrix() {
        let t = web4();
        let pi = [0.1, 0.2, 0.3, 0.4];
        let expected = step(&pi, &dense_google(&t, 0.85));
        assert_close(&google_step(&pi, &t, 0.85, &uniform(4)), &expected, TOL);
    }

    #[test]
    fn google_step_preserves_distribution() {
        let t = web4();
        let next = google_step(&[0.0, 0.0, 0.0, 1.0], &t, 0.85, &uniform(4));
        assert!(markov::is_distribution(&next));
    }

    #[test]
    fn alpha_one_without_dangling_is_plain_markov_step() {
        let t = graph(3, &[(0, 1), (1, 2), (2, 0), (2, 1)]);
        let pi = [0.2, 0.3, 0.5];
        assert_close(
            &google_step(&pi, &t, 1.0, &uniform(3)),
            &t.p.vec_mul(&pi),
            TOL,
        );
    }

    #[test]
    fn alpha_zero_always_teleports() {
        let t = web4();
        let teleport = [0.7, 0.1, 0.1, 0.1];
        assert_close(&google_step(&[0.25; 4], &t, 0.0, &teleport), &teleport, TOL);
    }

    // ---------- PageRank ----------

    #[test]
    fn ring_gives_uniform_scores() {
        let t = graph(5, &[(0, 1), (1, 2), (2, 3), (3, 4), (4, 0)]);
        let r = pagerank(&t, &Options::default()).unwrap();
        assert_close(&r.scores, &[0.2; 5], 1e-9);
    }

    #[test]
    fn matches_dense_power_iteration() {
        let t = web4();
        let r = pagerank(&t, &Options::default()).unwrap();
        let reference = iterate(&uniform(4), &dense_google(&t, 0.85), 2000)
            .pop()
            .unwrap();
        assert_close(&r.scores, &reference, 1e-8);
    }

    #[test]
    fn reports_convergence() {
        let opts = Options::default();
        let r = pagerank(&web4(), &opts).unwrap();
        assert!(r.converged);
        assert!(r.residual < opts.epsilon);
        assert!(r.iterations > 0 && r.iterations < opts.max_iter);
    }

    #[test]
    fn warm_start_from_solution_converges_in_one_step() {
        let t = web4();
        let opts = Options::default();
        let first = pagerank(&t, &opts).unwrap();
        let again = pagerank_from(&t, &opts, &first.scores, &uniform(4)).unwrap();
        assert_eq!(again.iterations, 1);
        assert!(again.converged);
    }

    #[test]
    fn max_iter_is_respected() {
        let opts = Options {
            epsilon: 0.0, // jamais atteint : on fait exactement max_iter pas
            max_iter: 5,
            ..Options::default()
        };
        let r = pagerank(&web4(), &opts).unwrap();
        assert_eq!(r.iterations, 5);
        assert!(!r.converged);

        let zero = Options {
            max_iter: 0,
            ..Options::default()
        };
        let r = pagerank(&web4(), &zero).unwrap();
        assert_eq!(r.iterations, 0);
        assert!(r.residual.is_infinite());
        assert_close(&r.scores, &uniform(4), TOL);
    }

    #[test]
    fn personalized_pagerank_favours_teleport_target() {
        // Anneau 0 → 1 → 2 → 3 → 0, téléportation toujours vers 0 :
        // les scores décroissent en s'éloignant de 0 le long de l'anneau.
        let t = graph(4, &[(0, 1), (1, 2), (2, 3), (3, 0)]);
        let r = pagerank_from(&t, &Options::default(), &uniform(4), &[1.0, 0.0, 0.0, 0.0]).unwrap();
        assert!(r.scores[0] > r.scores[1]);
        assert!(r.scores[1] > r.scores[2]);
        assert!(r.scores[2] > r.scores[3]);
        assert!(markov::is_distribution(&r.scores));
    }

    #[test]
    fn rejects_invalid_parameters() {
        let t = web4();
        let u = uniform(4);
        for alpha in [-0.1, 1.5, f64::NAN] {
            let opts = Options {
                alpha,
                ..Options::default()
            };
            assert!(matches!(
                pagerank(&t, &opts),
                Err(PageRankError::InvalidAlpha(_))
            ));
        }
        let opts = Options::default();
        assert!(matches!(
            pagerank_from(&t, &opts, &[0.5, 0.5], &u),
            Err(PageRankError::SizeMismatch {
                what: "start",
                expected: 4,
                found: 2
            })
        ));
        assert!(matches!(
            pagerank_from(&t, &opts, &u, &[1.0]),
            Err(PageRankError::SizeMismatch {
                what: "teleport",
                ..
            })
        ));
        assert!(matches!(
            pagerank_from(&t, &opts, &[1.0, 1.0, 0.0, 0.0], &u),
            Err(PageRankError::NotDistribution("start"))
        ));
        assert!(matches!(
            pagerank_from(&t, &opts, &u, &[0.5, 0.5, 0.5, -0.5]),
            Err(PageRankError::NotDistribution("teleport"))
        ));
    }

    #[test]
    fn rejects_empty_graph() {
        let t = Transition {
            p: CsrMatrix::from_triplets(0, 0, vec![]).unwrap(),
            dangling: vec![],
        };
        assert!(matches!(
            pagerank(&t, &Options::default()),
            Err(PageRankError::EmptyGraph)
        ));
    }

    // ---------- Propriétés sur des graphes aléatoires (proptest) ----------

    /// Génère un nombre de nœuds puis une liste de liens entre ces nœuds.
    fn random_graph() -> impl Strategy<Value = (usize, Vec<(usize, usize)>)> {
        (1usize..40).prop_flat_map(|n| (Just(n), prop::collection::vec((0..n, 0..n), 0..200)))
    }

    proptest! {
        /// Quel que soit le graphe : scores ≥ 0, somme 1, convergence.
        #[test]
        fn scores_form_a_distribution((n, links) in random_graph()) {
            let r = pagerank(&graph(n, &links), &Options::default()).unwrap();
            prop_assert!(r.converged);
            prop_assert!(r.scores.iter().all(|&s| s >= 0.0));
            prop_assert!((r.scores.iter().sum::<f64>() - 1.0).abs() < 1e-9);
        }
    }
}
