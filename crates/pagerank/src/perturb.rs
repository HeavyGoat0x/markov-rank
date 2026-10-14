//! Modifications aléatoires d'un graphe et expérience du TD 2.
//!
//! Question du TD : après une petite modification du graphe (liens supprimés ou ajoutés,
//! nouvelles pages), combien d'itérations faut-il pour recalculer PageRank
//! - en partant de la distribution uniforme ?
//! - en partant de l'ancien PageRank (*warm start*) ?
//!
//! Toutes les fonctions reçoivent le générateur aléatoire en paramètre : avec la même graine
//! (`StdRng::seed_from_u64(42)`), on obtient exactement les mêmes résultats. Une expérience
//! reproductible, c'est la base d'un bon rapport.

use std::collections::HashSet;
use std::path::Path;

use markov::CsrMatrix;
use rand::prelude::*;

use crate::{Options, PageRankError, Transition, pagerank, pagerank_from, uniform};

/// Graphe orienté non pondéré : `n` nœuds et une liste de liens `(de, vers)`.
///
/// Plus pratique que [`CsrMatrix`] pour ajouter ou retirer des liens ; on convertit en
/// [`Transition`] seulement au moment de calculer PageRank.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Graph {
    pub n: usize,
    pub links: Vec<(usize, usize)>,
}

impl Graph {
    /// Lit un fichier `.mtx` : chaque entrée `(i, j)` devient un lien `i → j` (valeur ignorée).
    ///
    /// # Errors
    /// Erreur de lecture, ou [`PageRankError::NotSquare`] si la matrice n'est pas carrée.
    pub fn from_mtx_file<P: AsRef<Path>>(path: P) -> Result<Self, PageRankError> {
        // `mtx::read_file(path)?`, vérifier rows == cols, puis garder (i, j) de chaque entrée
        // (indice : `.into_iter().map(|(i, j, _)| (i, j)).collect()`).
        todo!()
    }

    /// Matrice de transition (chaque lien a le poids 1).
    ///
    /// # Errors
    /// Celles de [`Transition::from_adjacency`].
    pub fn transition(&self) -> Result<Transition, PageRankError> {
        // Construire les triplets (i, j, 1.0), `CsrMatrix::from_triplets(self.n, self.n, …)?`,
        // puis `Transition::from_adjacency(&…)`.
        todo!()
    }
}

/// Supprime `count` liens choisis au hasard (tous si `count` dépasse le nombre de liens).
#[must_use]
pub fn remove_random_links<R: Rng + ?Sized>(g: &Graph, count: usize, rng: &mut R) -> Graph {
    // Indice : copier les liens, les mélanger (`links.shuffle(rng)`, trait `SliceRandom`),
    // puis `truncate` pour garder `len - count` liens (attention : `count` peut dépasser `len`,
    // regarde `usize::saturating_sub`).
    todo!()
}

/// Ajoute `count` liens tirés au hasard : jamais de boucle (`i → i`), jamais un lien qui
/// existe déjà.
///
/// # Panics
/// S'il n'y a pas assez de liens possibles : au plus `n·(n−1)` liens distincts sans boucle.
#[must_use]
pub fn add_random_links<R: Rng + ?Sized>(g: &Graph, count: usize, rng: &mut R) -> Graph {
    // Étapes :
    // 1) `let mut seen: HashSet<(usize, usize)> = g.links.iter().copied().collect();`
    // 2) vérifier qu'il reste assez de place, sinon paniquer avec un message contenant
    //    « liens » (le test le vérifie), par ex. `assert!(…, "pas assez de liens possibles")` ;
    // 3) tant qu'on n'a pas ajouté `count` liens : tirer i et j avec `rng.random_range(0..n)` ;
    //    si i != j et `seen.insert((i, j))` renvoie true (lien nouveau), le garder.
    todo!()
}

/// Ajoute `count` nouvelles pages, numérotées `n, n+1, …`. Chacune reçoit `links_per_node`
/// liens sortants vers des pages existantes distinctes, et `links_per_node` liens entrants
/// depuis des pages existantes distinctes (tirées au hasard).
///
/// # Panics
/// Si `links_per_node` dépasse le nombre de pages existantes.
#[must_use]
pub fn add_random_nodes<R: Rng + ?Sized>(
    g: &Graph,
    count: usize,
    links_per_node: usize,
    rng: &mut R,
) -> Graph {
    // Indice : `rand::seq::index::sample(rng, g.n, links_per_node)` tire `links_per_node`
    // indices DISTINCTS dans 0..g.n ; `.iter()` les parcourt.
    todo!()
}

/// Étend une distribution de `pi.len()` pages à `new_n` pages, pour un warm start après
/// ajout de pages : les anciens scores sont multipliés par `old_n / new_n`, chaque nouvelle
/// page reçoit `1 / new_n`. Le résultat somme toujours à 1.
///
/// # Panics
/// Si `new_n < pi.len()`, avec un message contenant « réduire » (le test le vérifie).
#[must_use]
pub fn extend_distribution(pi: &[f64], new_n: usize) -> Vec<f64> {
    todo!()
}

/// Nombre d'itérations pour recalculer PageRank après modification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Comparison {
    /// En partant de la distribution uniforme.
    pub uniform_iterations: usize,
    /// En partant de l'ancien PageRank (étendu si des pages ont été ajoutées).
    pub warm_iterations: usize,
}

/// L'expérience du TD 2 : PageRank de `before`, puis de `after` depuis les deux départs.
///
/// # Errors
/// Celles de [`Graph::transition`] et de [`pagerank_from`].
///
/// # Panics
/// Si `after` a moins de pages que `before` (on ne sait étendre une distribution que vers
/// plus de pages).
pub fn compare_starts(
    before: &Graph,
    after: &Graph,
    opts: &Options,
) -> Result<Comparison, PageRankError> {
    // 1) `pagerank(&before.transition()?, opts)?` → ancien PageRank ;
    // 2) `after.transition()?` puis `pagerank(…)` → itérations depuis l'uniforme ;
    // 3) `extend_distribution(&ancien.scores, after.n)` puis
    //    `pagerank_from(&t_after, opts, &warm, &uniform(after.n))` → itérations warm.
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rng(seed: u64) -> StdRng {
        StdRng::seed_from_u64(seed)
    }

    /// Graphe déterministe de `n` nœuds : un anneau, un lien vers l'une des pages 0, 1 ou 2
    /// (très populaires, comme une page d'accueil) et un raccourci. Son PageRank est donc
    /// loin d'être uniforme, ce qui rend le warm start utile.
    fn chords(n: usize) -> Graph {
        let mut links = Vec::new();
        for i in 0..n {
            for j in [(i + 1) % n, i % 3, (i * 7 + 3) % n] {
                if i != j && !links.contains(&(i, j)) {
                    links.push((i, j));
                }
            }
        }
        Graph { n, links }
    }

    fn as_set(links: &[(usize, usize)]) -> HashSet<(usize, usize)> {
        links.iter().copied().collect()
    }

    // ---------- Graph ----------

    #[test]
    fn graph_from_mtx_file() {
        let path = format!("{}/tests/data/star.mtx", env!("CARGO_MANIFEST_DIR"));
        let g = Graph::from_mtx_file(path).unwrap();
        assert_eq!(g.n, 5);
        assert_eq!(
            as_set(&g.links),
            as_set(&[(0, 4), (1, 4), (2, 4), (3, 4), (4, 0)])
        );
    }

    #[test]
    fn graph_to_transition() {
        let g = Graph {
            n: 3,
            links: vec![(0, 1), (0, 2), (1, 2)],
        };
        let t = g.transition().unwrap();
        assert!((t.p.get(0, 2) - 0.5).abs() < 1e-12);
        assert_eq!(t.dangling, vec![false, false, true]);
    }

    // ---------- Suppression ----------

    #[test]
    fn remove_keeps_a_subset_of_the_right_size() {
        let g = chords(20);
        let after = remove_random_links(&g, 5, &mut rng(1));
        assert_eq!(after.n, 20);
        assert_eq!(after.links.len(), g.links.len() - 5);
        assert!(as_set(&after.links).is_subset(&as_set(&g.links)));
    }

    #[test]
    fn remove_more_than_available_empties_the_graph() {
        let g = chords(5);
        assert_eq!(remove_random_links(&g, 1000, &mut rng(1)).links, vec![]);
    }

    #[test]
    fn same_seed_same_result() {
        let g = chords(30);
        let a = remove_random_links(&g, 10, &mut rng(42));
        let b = remove_random_links(&g, 10, &mut rng(42));
        let c = remove_random_links(&g, 10, &mut rng(43));
        assert_eq!(as_set(&a.links), as_set(&b.links));
        assert_ne!(as_set(&a.links), as_set(&c.links));
    }

    // ---------- Ajout de liens ----------

    #[test]
    fn add_links_without_self_loops_or_duplicates() {
        let g = chords(10);
        let after = add_random_links(&g, 15, &mut rng(3));
        assert_eq!(after.links.len(), g.links.len() + 15);
        assert!(after.links.iter().all(|&(i, j)| i != j));
        assert_eq!(as_set(&after.links).len(), after.links.len(), "doublon");
        assert!(as_set(&g.links).is_subset(&as_set(&after.links)));
    }

    #[test]
    fn add_links_can_fill_the_graph() {
        // 3 nœuds : 3·2 = 6 liens possibles, 1 existe déjà, on peut en ajouter 5.
        let g = Graph {
            n: 3,
            links: vec![(0, 1)],
        };
        assert_eq!(add_random_links(&g, 5, &mut rng(0)).links.len(), 6);
    }

    #[test]
    #[should_panic(expected = "liens")]
    fn add_links_panics_when_impossible() {
        let g = Graph {
            n: 2,
            links: vec![(0, 1)],
        };
        let _ = add_random_links(&g, 2, &mut rng(0));
    }

    // ---------- Ajout de pages ----------

    #[test]
    fn add_nodes_links_them_to_existing_pages() {
        let g = chords(10);
        let after = add_random_nodes(&g, 2, 3, &mut rng(5));
        assert_eq!(after.n, 12);
        assert_eq!(after.links.len(), g.links.len() + 2 * 3 * 2);
        for new in [10, 11] {
            let out: Vec<usize> = after
                .links
                .iter()
                .filter(|l| l.0 == new)
                .map(|l| l.1)
                .collect();
            let inc: Vec<usize> = after
                .links
                .iter()
                .filter(|l| l.1 == new)
                .map(|l| l.0)
                .collect();
            assert_eq!(out.len(), 3);
            assert_eq!(inc.len(), 3);
            assert!(
                out.iter().chain(&inc).all(|&k| k < 10),
                "vers les anciennes pages"
            );
            assert_eq!(
                out.iter().collect::<HashSet<_>>().len(),
                3,
                "cibles distinctes"
            );
        }
    }

    // ---------- Warm start ----------

    #[test]
    fn extend_distribution_keeps_a_distribution() {
        let e = extend_distribution(&[0.8, 0.2], 4);
        let expected = [0.4, 0.1, 0.25, 0.25];
        for (x, y) in e.iter().zip(expected) {
            assert!((x - y).abs() < 1e-12);
        }
        assert_eq!(extend_distribution(&[0.3, 0.7], 2), vec![0.3, 0.7]);
    }

    #[test]
    #[should_panic(expected = "réduire")]
    fn extend_distribution_cannot_shrink() {
        let _ = extend_distribution(&[0.5, 0.5], 1);
    }

    /// Somme des itérations (uniforme, warm) sur 8 graines : sur un seul tirage, les deux
    /// départs peuvent être à égalité ; en moyenne, le warm start doit gagner.
    fn total_iterations(modify: impl Fn(&Graph, &mut StdRng) -> Graph) -> (usize, usize) {
        let before = chords(60);
        let (mut uniform_total, mut warm_total) = (0, 0);
        for seed in 0..8 {
            let after = modify(&before, &mut rng(seed));
            let c = compare_starts(&before, &after, &Options::default()).unwrap();
            uniform_total += c.uniform_iterations;
            warm_total += c.warm_iterations;
        }
        (uniform_total, warm_total)
    }

    #[test]
    fn warm_start_beats_uniform_after_removing_links() {
        let (uniform_total, warm_total) = total_iterations(|g, r| remove_random_links(g, 3, r));
        assert!(
            warm_total < uniform_total,
            "warm {warm_total} vs uniforme {uniform_total}"
        );
    }

    #[test]
    fn warm_start_beats_uniform_after_adding_links() {
        let (uniform_total, warm_total) = total_iterations(|g, r| add_random_links(g, 3, r));
        assert!(
            warm_total < uniform_total,
            "warm {warm_total} vs uniforme {uniform_total}"
        );
    }

    #[test]
    fn warm_start_beats_uniform_after_adding_pages() {
        let (uniform_total, warm_total) = total_iterations(|g, r| add_random_nodes(g, 2, 3, r));
        assert!(
            warm_total < uniform_total,
            "warm {warm_total} vs uniforme {uniform_total}"
        );
    }
}
