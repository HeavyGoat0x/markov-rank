//! Matrice creuse au format CSR (*Compressed Sparse Row*).
//!
//! On ne stocke que les coefficients non nuls, ligne par ligne :
//!
//! ```text
//!         | 0   0.6 0.4 |      values  = [0.6, 0.4, 0.2, 0.8, 0.3, 0.2, 0.5]
//!     P = | 0.2 0.8 0   |  ->  col_idx = [1,   2,   0,   1,   0,   1,   2  ]
//!         | 0.3 0.2 0.5 |      row_ptr = [0, 2, 4, 7]
//! ```
//!
//! Les entrées de la ligne `i` sont aux positions `row_ptr[i] .. row_ptr[i + 1]`
//! de `col_idx` et `values`. Mémoire : O(n + nnz) au lieu de O(n²).

use crate::{DenseMatrix, MatrixError};

/// Matrice creuse `n_rows × n_cols` au format CSR.
///
/// Invariants : `row_ptr.len() == n_rows + 1`, `row_ptr[0] == 0`,
/// `row_ptr[n_rows] == nnz`, et dans chaque ligne les colonnes sont
/// strictement croissantes (donc sans doublon).
#[derive(Debug, Clone, PartialEq)]
pub struct CsrMatrix {
    n_rows: usize,
    n_cols: usize,
    row_ptr: Vec<usize>,
    col_idx: Vec<usize>,
    values: Vec<f64>,
}

impl CsrMatrix {
    /// Construit une matrice à partir de triplets `(ligne, colonne, valeur)`, dans n'importe
    /// quel ordre. Les doublons (même ligne et même colonne) sont **additionnés**.
    ///
    /// Prend le `Vec` par valeur pour pouvoir le trier sur place, sans copie.
    ///
    /// # Errors
    /// [`MatrixError::OutOfBounds`] si un indice sort de la matrice.
    pub fn from_triplets(
        n_rows: usize,
        n_cols: usize,
        triplets: Vec<(usize, usize, f64)>,
    ) -> Result<Self, MatrixError> {
        // Étapes :
        // 1) vérifier les indices ;
        // 2) trier par (ligne, colonne) : `triplets.sort_unstable_by_key(|&(i, j, _)| (i, j))` ;
        //    (il faut que `triplets` soit `mut` : ajoute `mut` devant le paramètre)
        // 3) parcourir dans l'ordre : si (i, j) est le même que le précédent, additionner
        //    dans la dernière case de `values` ; sinon pousser dans `col_idx` et `values`
        //    et compter une entrée de plus pour la ligne i ;
        // 4) `row_ptr` = sommes cumulées des nombres d'entrées par ligne.
        todo!()
    }

    /// Convertit une matrice dense, en ignorant les zéros.
    #[must_use]
    pub fn from_dense(m: &DenseMatrix) -> Self {
        // Indice : construis les triplets des coefficients non nuls puis appelle `from_triplets`.
        // Elle renvoie un `Result`, mais ici les indices sont valides par construction :
        // `.expect("indices valides")` est légitime. Clippy demandera alors une section
        // `# Panics` dans la doc, comme pour `get`.
        todo!()
    }

    /// Convertit en matrice dense (utile pour les tests sur de petites matrices).
    ///
    /// # Panics
    /// Si la matrice n'est pas carrée ([`DenseMatrix`] est toujours carrée).
    #[must_use]
    pub fn to_dense(&self) -> DenseMatrix {
        todo!()
    }

    /// Nombre de lignes.
    #[must_use]
    pub fn n_rows(&self) -> usize {
        self.n_rows
    }

    /// Nombre de colonnes.
    #[must_use]
    pub fn n_cols(&self) -> usize {
        self.n_cols
    }

    /// Nombre de coefficients stockés (*number of non-zeros*).
    #[must_use]
    pub fn nnz(&self) -> usize {
        self.values.len()
    }

    /// Entrées `(colonne, valeur)` de la ligne `i`, par colonne croissante.
    ///
    /// `'_` : l'itérateur emprunte `self`, il ne peut pas vivre plus longtemps que la matrice.
    ///
    /// # Panics
    /// Si `i >= n_rows`.
    pub fn row(&self, i: usize) -> impl Iterator<Item = (usize, f64)> + '_ {
        // Indice : avec `let range = self.row_ptr[i]..self.row_ptr[i + 1];`,
        // `self.col_idx[range.clone()].iter().copied()` donne les colonnes ;
        // `.zip(...)` les associe aux valeurs.
        //
        // À REMPLACER : `todo!()` ne marche pas ici, car Rust doit connaître le type concret
        // de l'itérateur renvoyé. En attendant, on renvoie un itérateur vide.
        let _ = i;
        std::iter::empty()
    }

    /// Élément `(i, j)`, `0.0` s'il n'est pas stocké.
    ///
    /// # Panics
    /// Si `i` ou `j` est hors limites.
    #[must_use]
    pub fn get(&self, i: usize, j: usize) -> f64 {
        // Version simple : parcourir `self.row(i)`.
        // Version rapide (bonus) : les colonnes étant triées, `binary_search` sur la tranche
        // `col_idx` de la ligne.
        todo!()
    }

    /// Transposée : l'entrée `(i, j)` devient `(j, i)`.
    #[must_use]
    pub fn transpose(&self) -> Self {
        // Version simple : construire les triplets inversés et appeler `from_triplets`
        // (même remarque que `from_dense` pour le `.expect`).
        todo!()
    }

    /// Produit vecteur ligne × matrice : `x · A`, le même calcul que `step` en dense.
    ///
    /// Coût O(nnz) au lieu de O(n²) : c'est ce qui rendra PageRank possible sur des
    /// millions de pages.
    ///
    /// # Panics
    /// Si `x.len() != n_rows`.
    #[must_use]
    pub fn vec_mul(&self, x: &[f64]) -> Vec<f64> {
        // `(x · A)[j] = Σᵢ x[i] · A[i][j]`. En CSR, on parcourt les lignes :
        // pour chaque ligne i et chaque entrée (j, v) de la ligne, `result[j] += x[i] * v`.
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EPS, step};

    /// Matrice de l'exercice 1.2, en triplets volontairement dans le désordre.
    fn ex12() -> CsrMatrix {
        CsrMatrix::from_triplets(
            3,
            3,
            vec![
                (2, 2, 0.5),
                (0, 1, 0.6),
                (1, 0, 0.2),
                (2, 0, 0.3),
                (0, 2, 0.4),
                (2, 1, 0.2),
                (1, 1, 0.8),
            ],
        )
        .unwrap()
    }

    fn ex12_dense() -> DenseMatrix {
        DenseMatrix::from_rows(vec![
            vec![0.0, 0.6, 0.4],
            vec![0.2, 0.8, 0.0],
            vec![0.3, 0.2, 0.5],
        ])
        .unwrap()
    }

    // ---------- Construction ----------

    #[test]
    fn from_triplets_builds_csr_arrays() {
        let m = ex12();
        assert_eq!(m.row_ptr, vec![0, 2, 4, 7]);
        assert_eq!(m.col_idx, vec![1, 2, 0, 1, 0, 1, 2]);
        assert_eq!(m.values, vec![0.6, 0.4, 0.2, 0.8, 0.3, 0.2, 0.5]);
        assert_eq!((m.n_rows(), m.n_cols(), m.nnz()), (3, 3, 7));
    }

    #[test]
    fn from_triplets_sums_duplicates() {
        let m =
            CsrMatrix::from_triplets(2, 2, vec![(0, 1, 0.25), (1, 0, 1.0), (0, 1, 0.5)]).unwrap();
        assert_eq!(m.nnz(), 2);
        assert!((m.get(0, 1) - 0.75).abs() < EPS);
    }

    #[test]
    fn from_triplets_handles_empty_rows() {
        // Ligne 1 vide (un nœud sans lien sortant, très courant sur le web).
        let m = CsrMatrix::from_triplets(3, 3, vec![(0, 0, 1.0), (2, 1, 2.0)]).unwrap();
        assert_eq!(m.row_ptr, vec![0, 1, 1, 2]);
        assert_eq!(m.row(1).count(), 0);
    }

    #[test]
    fn from_triplets_rejects_out_of_bounds() {
        assert_eq!(
            CsrMatrix::from_triplets(2, 3, vec![(2, 0, 1.0)]),
            Err(MatrixError::OutOfBounds { row: 2, col: 0 })
        );
        assert_eq!(
            CsrMatrix::from_triplets(2, 3, vec![(0, 3, 1.0)]),
            Err(MatrixError::OutOfBounds { row: 0, col: 3 })
        );
    }

    #[test]
    fn rectangular_and_empty_matrices() {
        let m = CsrMatrix::from_triplets(2, 4, vec![(1, 3, 1.0)]).unwrap();
        assert_eq!((m.n_rows(), m.n_cols()), (2, 4));
        let empty = CsrMatrix::from_triplets(0, 0, vec![]).unwrap();
        assert_eq!(empty.nnz(), 0);
        assert_eq!(empty.row_ptr, vec![0]);
    }

    // ---------- Lecture ----------

    #[test]
    fn row_iterates_sorted_columns() {
        let row: Vec<(usize, f64)> = ex12().row(2).collect();
        assert_eq!(row, vec![(0, 0.3), (1, 0.2), (2, 0.5)]);
    }

    #[test]
    fn get_matches_dense() {
        let (m, d) = (ex12(), ex12_dense());
        for i in 0..3 {
            for j in 0..3 {
                assert!((m.get(i, j) - d.get(i, j)).abs() < EPS, "({i}, {j})");
            }
        }
    }

    // ---------- Conversions ----------

    #[test]
    fn from_dense_skips_zeros() {
        let m = CsrMatrix::from_dense(&ex12_dense());
        assert_eq!(m, ex12());
    }

    #[test]
    fn dense_round_trip() {
        assert_eq!(ex12().to_dense(), ex12_dense());
    }

    #[test]
    fn transpose_swaps_indices() {
        let m = ex12();
        let t = m.transpose();
        assert_eq!((t.n_rows(), t.n_cols()), (3, 3));
        for i in 0..3 {
            for j in 0..3 {
                assert!((m.get(i, j) - t.get(j, i)).abs() < EPS, "({i}, {j})");
            }
        }
        assert_eq!(t.transpose(), m);
    }

    #[test]
    fn transpose_rectangular() {
        let t = CsrMatrix::from_triplets(2, 4, vec![(1, 3, 1.0)])
            .unwrap()
            .transpose();
        assert_eq!((t.n_rows(), t.n_cols()), (4, 2));
        assert!((t.get(3, 1) - 1.0).abs() < EPS);
    }

    // ---------- Produit ----------

    #[test]
    fn vec_mul_matches_dense_step() {
        let pi = [0.2, 0.3, 0.5];
        let sparse = ex12().vec_mul(&pi);
        let dense = step(&pi, &ex12_dense());
        for (s, d) in sparse.iter().zip(&dense) {
            assert!((s - d).abs() < EPS);
        }
    }

    #[test]
    fn vec_mul_rectangular() {
        // x (taille 2) · A (2 × 4) = vecteur de taille 4.
        let a = CsrMatrix::from_triplets(2, 4, vec![(0, 0, 1.0), (1, 3, 2.0)]).unwrap();
        assert_eq!(a.vec_mul(&[3.0, 5.0]), vec![3.0, 0.0, 0.0, 10.0]);
    }
}
