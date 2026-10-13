//! Chaînes de Markov en temps discret et espace fini (TD 1).
//!
//! Une distribution est un `Vec<f64>` (vecteur ligne), une matrice de
//! transition est une [`DenseMatrix`]. L'évolution s'écrit `π(n+1) = π(n) · P`.

use std::fmt;

/// Tolérance utilisée pour comparer des flottants (sommes égales à 1, etc.).
pub const EPS: f64 = 1e-9;

/// Erreurs possibles à la construction d'une matrice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatrixError {
    /// Aucune ligne fournie.
    Empty,
    /// Une ligne n'a pas autant d'éléments qu'il y a de lignes.
    NotSquare,
}

/// Matrice carrée `n × n` stockée ligne par ligne dans un seul `Vec`
/// (« row-major ») : l'élément `(i, j)` est à l'indice `i * n + j`.
#[derive(Debug, Clone, PartialEq)]
pub struct DenseMatrix {
    n: usize,
    data: Vec<f64>,
}

impl DenseMatrix {
    /// Construit une matrice à partir de ses lignes.
    ///
    /// # Errors
    /// [`MatrixError::Empty`] si `rows` est vide,
    /// [`MatrixError::NotSquare`] si une ligne n'a pas `rows.len()` éléments.
    pub fn from_rows(rows: Vec<Vec<f64>>) -> Result<Self, MatrixError> {
        // Indice : vérifie les erreurs, puis aplatis les lignes dans `data`
        // (regarde `Vec::extend` ou `Iterator::flatten`).
        if rows.is_empty() {
            return Err(MatrixError::Empty);
        }

        if rows.iter().any(|row| row.len() != rows.len()) {
            return Err(MatrixError::NotSquare);
        }

        let n = rows.len();
        let data: Vec<f64> = rows.into_iter().flatten().collect();

        Ok(Self { n, data })
    }

    /// Matrice identité de taille `n`.
    #[must_use]
    pub fn identity(n: usize) -> Self {
        let mut data: Vec<f64> = vec![0.0; n * n];

        for i in 0..n {
            data[i * n + i] = 1.0;
        }
        Self { n, data }
    }

    /// Nombre de lignes (= nombre de colonnes).
    #[must_use]
    pub fn size(&self) -> usize {
        self.n
    }

    /// Élément `(i, j)`.
    ///
    /// # Panics
    /// Si `i` ou `j` est hors limites.
    #[must_use]
    pub fn get(&self, i: usize, j: usize) -> f64 {
        assert!(i < self.n && j < self.n, "élément ({i}, {j}) hors limites");

        self.data[i * self.n + j]
    }

    /// Exercice 1.1 q2 : coefficients ≥ 0 et chaque ligne somme à 1 (à `EPS` près).
    #[must_use]
    pub fn is_transition(&self) -> bool {
        for i in 0..self.n {
            let mut sum = 0.0;
            for j in 0..self.n {
                if self.get(i, j) < 0.0 {
                    return false;
                }
                sum += self.get(i, j);
            }
            if (sum - 1.0).abs() > EPS {
                return false;
            }
        }
        true
    }

    /// Produit matriciel `self × other`.
    ///
    /// # Panics
    /// Si les deux matrices n'ont pas la même taille.
    #[must_use]
    pub fn mul(&self, other: &Self) -> Self {
        assert_eq!(self.n, other.n, "tailles incompatibles");

        let mut data: Vec<f64> = vec![0.0; self.n * self.n];
        let n = self.n;
        for i in 0..n {
            for j in 0..n {
                for k in 0..n {
                    data[i * n + j] += self.get(i, k) * other.get(k, j);
                }
            }
        }

        Self { n, data }
    }

    /// Exercice 1.2 q7 : `self²`.
    #[must_use]
    pub fn square(&self) -> Self {
        self.mul(self)
    }
}

/// Exercice 1.2 q8 : affichage lisible, une ligne de la matrice par ligne de texte.
impl fmt::Display for DenseMatrix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Indice : `write!(f, "{:8.5}", x)?` pour chaque élément, `writeln!(f)?` en fin de ligne.

        for i in 0..self.n {
            for j in 0..self.n {
                write!(f, "{:8.5}", self.get(i, j))?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

/// Exercice 1.1 q1 : coefficients ≥ 0 et somme égale à 1 (à `EPS` près).
#[must_use]
pub fn is_distribution(pi: &[f64]) -> bool {
    let mut sum = 0.0;
    for &element in pi {
        if element < 0.0 {
            return false;
        }
        sum += element;
    }

    if (sum - 1.0).abs() > EPS {
        return false;
    }
    true
}

/// Exercice 1.2 q3 : un pas de la chaîne, `π · P`.
/// Rappel : `(π · P)[j] = Σᵢ π[i] · P[i][j]`.
///
/// # Panics
/// Si `pi` n'a pas la taille de `p`.
#[must_use]
pub fn step(pi: &[f64], p: &DenseMatrix) -> Vec<f64> {
    assert_eq!(pi.len(), p.size(), "tailles incompatibles");
    let mut result: Vec<f64> = vec![0.0; p.size()];

    for (j, r) in result.iter_mut().enumerate() {
        for (i, &x) in pi.iter().enumerate() {
            *r += x * p.get(i, j);
        }
    }
    result
}

/// Exercice 1.2 q4 : renvoie `[π(0), π(1), …, π(n)]` (soit `n + 1` vecteurs).
#[must_use]
pub fn iterate(pi0: &[f64], p: &DenseMatrix, n: usize) -> Vec<Vec<f64>> {
    let mut result = Vec::with_capacity(n + 1);
    result.push(pi0.to_vec());
    for i in 0..n {
        let next = step(&result[i], p);
        result.push(next);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(a: &[f64], b: &[f64], tol: f64) {
        assert_eq!(a.len(), b.len(), "tailles différentes");
        for (i, (x, y)) in a.iter().zip(b).enumerate() {
            assert!((x - y).abs() < tol, "indice {i} : {x} != {y}");
        }
    }

    /// Matrice de l'exercice 1.2.
    fn p_ex12() -> DenseMatrix {
        DenseMatrix::from_rows(vec![
            vec![0.0, 0.6, 0.4],
            vec![0.2, 0.8, 0.0],
            vec![0.3, 0.2, 0.5],
        ])
        .unwrap()
    }

    // ---------- Construction ----------

    #[test]
    fn from_rows_rejects_empty() {
        assert_eq!(DenseMatrix::from_rows(vec![]), Err(MatrixError::Empty));
    }

    #[test]
    fn from_rows_rejects_non_square() {
        let rows = vec![vec![1.0, 0.0], vec![1.0]];
        assert_eq!(DenseMatrix::from_rows(rows), Err(MatrixError::NotSquare));
    }

    #[test]
    fn get_reads_row_major() {
        let p = p_ex12();
        assert_eq!(p.size(), 3);
        assert!((p.get(0, 1) - 0.6).abs() < EPS);
        assert!((p.get(2, 0) - 0.3).abs() < EPS);
    }

    #[test]
    fn identity_is_neutral() {
        let p = p_ex12();
        assert_eq!(p.mul(&DenseMatrix::identity(3)), p);
    }

    // ---------- Exercice 1.1 ----------

    #[test]
    fn distributions() {
        assert!(is_distribution(&[0.2, 0.3, 0.5]));
        assert!(is_distribution(&[1.0]));
        assert!(!is_distribution(&[0.5, 0.6]), "somme ≠ 1");
        assert!(!is_distribution(&[1.5, -0.5]), "coefficient négatif");
    }

    #[test]
    fn transition_matrices() {
        assert!(p_ex12().is_transition());
        assert!(DenseMatrix::identity(4).is_transition());
        let bad = DenseMatrix::from_rows(vec![vec![0.5, 0.6], vec![0.0, 1.0]]).unwrap();
        assert!(!bad.is_transition());
    }

    #[test]
    fn step_preserves_distribution() {
        let pi = step(&[0.2, 0.3, 0.5], &p_ex12());
        assert!(is_distribution(&pi));
    }

    #[test]
    fn powers_stay_transition() {
        let mut p = p_ex12();
        for _ in 0..10 {
            p = p.square();
            assert!(p.is_transition());
        }
    }

    // ---------- Exercice 1.2 ----------

    #[test]
    fn step_from_dirac() {
        // Partir de l'état 1 avec certitude donne la première ligne de P.
        assert_close(&step(&[1.0, 0.0, 0.0], &p_ex12()), &[0.0, 0.6, 0.4], EPS);
    }

    #[test]
    fn square_ex12() {
        let p2 = p_ex12().square();
        let expected = [[0.24, 0.56, 0.20], [0.16, 0.76, 0.08], [0.19, 0.44, 0.37]];
        for (i, row) in expected.iter().enumerate() {
            for (j, &v) in row.iter().enumerate() {
                assert!((p2.get(i, j) - v).abs() < EPS, "P²[{i}][{j}]");
            }
        }
    }

    #[test]
    fn iterate_returns_n_plus_one_vectors() {
        let traj = iterate(&[1.0, 0.0, 0.0], &p_ex12(), 12);
        assert_eq!(traj.len(), 13);
        assert_close(&traj[0], &[1.0, 0.0, 0.0], EPS);
    }

    /// Distribution stationnaire de l'exercice 1.2, calculée à la main : (5, 19, 4) / 28.
    const PI_EX12: [f64; 3] = [5.0 / 28.0, 19.0 / 28.0, 4.0 / 28.0];

    #[test]
    fn converges_from_any_start() {
        let p = p_ex12();
        for pi0 in [
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0],
        ] {
            let last = iterate(&pi0, &p, 200).pop().unwrap();
            assert_close(&last, &PI_EX12, 1e-9);
        }
    }

    #[test]
    fn p1024_rows_equal_stationary() {
        let mut p = p_ex12();
        for _ in 0..10 {
            p = p.square(); // P², P⁴, …, P¹⁰²⁴
        }
        for i in 0..3 {
            let row: Vec<f64> = (0..3).map(|j| p.get(i, j)).collect();
            assert_close(&row, &PI_EX12, 1e-9);
        }
    }

    // ---------- Exercices 1.3 et 1.4 ----------

    #[test]
    fn two_states_closed_form() {
        // Exercice 1.3 : 0 → 1 avec proba p, 1 → 0 avec proba q.
        let (p, q) = (0.3, 0.1);
        let m = DenseMatrix::from_rows(vec![vec![1.0 - p, p], vec![q, 1.0 - q]]).unwrap();
        let last = iterate(&[1.0, 0.0], &m, 500).pop().unwrap();
        assert_close(&last, &[q / (p + q), p / (p + q)], 1e-9);
    }

    #[test]
    fn machine_works_80_percent() {
        // Exercice 1.4 : états (F, P).
        let m = DenseMatrix::from_rows(vec![vec![0.9, 0.1], vec![0.4, 0.6]]).unwrap();
        let last = iterate(&[1.0, 0.0], &m, 200).pop().unwrap();
        assert_close(&last, &[0.8, 0.2], 1e-9);
    }
}
