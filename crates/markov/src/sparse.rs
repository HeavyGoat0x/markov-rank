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
        mut triplets: Vec<(usize, usize, f64)>,
    ) -> Result<Self, MatrixError> {
        // Étapes :
        // 1) vérifier les indices ;
        // 2) trier par (ligne, colonne) : `triplets.sort_unstable_by_key(|&(i, j, _)| (i, j))` ;
        //    (il faut que `triplets` soit `mut` : ajoute `mut` devant le paramètre)
        // 3) parcourir dans l'ordre : si (i, j) est le même que le précédent, additionner
        //    dans la dernière case de `values` ; sinon pousser dans `col_idx` et `values`
        //    et compter une entrée de plus pour la ligne i ;
        // 4) `row_ptr` = sommes cumulées des nombres d'entrées par ligne.
        // 1) Indices : on refuse AVANT de trier, inutile de trier des données invalides.
        for &(i, j, _) in &triplets {
            if i >= n_rows || j >= n_cols {
                return Err(MatrixError::OutOfBounds { row: i, col: j });
            }
        }

        // 2) Tri sur place (on possède `triplets`, aucune copie).
        triplets.sort_unstable_by_key(|&(i, j, _)| (i, j));

        let mut row_ptr: Vec<usize> = Vec::with_capacity(n_rows + 1);
        row_ptr.push(0);
        let mut col_idx: Vec<usize> = Vec::with_capacity(triplets.len());
        let mut values: Vec<f64> = Vec::with_capacity(triplets.len());
        let mut prev: Option<(usize, usize)> = None;

        for (i, j, value) in triplets {
            // 3) Doublon (même position que le précédent, ils sont côte à côte après le tri) :
            //    on additionne dans la dernière case, et on NE stocke PAS une deuxième fois.
            if prev == Some((i, j)) {
                if let Some(last) = values.last_mut() {
                    *last += value;
                }
                continue;
            }
            prev = Some((i, j));

            // 4) Fermer toutes les lignes avant i (y compris les vides) AVANT de stocker :
            //    `row_ptr.len() - 1` = nombre de lignes déjà fermées.
            while row_ptr.len() <= i {
                row_ptr.push(values.len());
            }
            col_idx.push(j);
            values.push(value);
        }

        // 5) Fermer la dernière ligne et les éventuelles lignes vides de la fin.
        while row_ptr.len() <= n_rows {
            row_ptr.push(values.len());
        }

        Ok(CsrMatrix {
            n_rows,
            n_cols,
            row_ptr,
            col_idx,
            values,
        })
    }

    /// Convertit une matrice dense, en ignorant les zéros.
    ///
    /// # Panics
    /// Jamais en pratique : les indices viennent de `m`, ils sont valides par construction.
    #[must_use]
    pub fn from_dense(m: &DenseMatrix) -> Self {
        let n = m.size(); // DenseMatrix est carrée : un seul n suffit
        let mut triplets: Vec<(usize, usize, f64)> = Vec::new();

        for i in 0..n {
            for j in 0..n {
                let v = m.get(i, j);
                if v != 0.0 {
                    triplets.push((i, j, v)); // on réutilise v au lieu de relire m.get
                }
            }
        }

        // `from_triplets` renvoie un `Result`, mais les indices viennent de `m` :
        // l'erreur est impossible. Le message d'`expect` dit ce qu'on SUPPOSE vrai.
        Self::from_triplets(n, n, triplets).expect("indices valides par construction")
    }

    /// Convertit en matrice dense (utile pour les tests sur de petites matrices).
    ///
    /// # Panics
    /// Si la matrice n'est pas carrée ([`DenseMatrix`] est toujours carrée).
    #[must_use]
    pub fn to_dense(&self) -> DenseMatrix {
        assert_eq!(self.n_rows, self.n_cols, "DenseMatrix doit être carrée");
        let n = self.n_rows;
        let mut data = vec![0.0; n * n];

        for i in 0..n {
            // Les entrées de la ligne i sont aux positions k = row_ptr[i] .. row_ptr[i + 1]
            // de `col_idx` et `values` : on y accède directement par l'indice k,
            // sans itérateur, donc sans `Option`.
            for k in self.row_ptr[i]..self.row_ptr[i + 1] {
                let j = self.col_idx[k];
                data[i * n + j] = self.values[k]; // élément (i, j), stockage row-major
            }
        }

        // `n` et `data` sont privés, mais `sparse` est un sous-module du crate qui définit
        // `DenseMatrix` : il a le droit de les utiliser.
        DenseMatrix { n, data }
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
        // Les entrées de la ligne i occupent les positions row_ptr[i] .. row_ptr[i + 1]
        // de `col_idx` et `values`. `zip` associe chaque colonne à sa valeur ; les deux
        // tranches ont la même longueur, donc aucun `None` possible.
        let range = self.row_ptr[i]..self.row_ptr[i + 1];
        let cols = self.col_idx[range.clone()].iter().copied();
        cols.zip(self.values[range].iter().copied())
    }

    /// Élément `(i, j)`, `0.0` s'il n'est pas stocké.
    ///
    /// # Panics
    /// Si `i` ou `j` est hors limites.
    #[must_use]
    pub fn get(&self, i: usize, j: usize) -> f64 {
        assert!(
            i < self.n_rows && j < self.n_cols,
            "élément ({i}, {j}) hors d'une matrice {}×{}",
            self.n_rows,
            self.n_cols
        );

        // Les colonnes d'une ligne sont triées : recherche dichotomique, O(log k).
        // `binary_search` renvoie `Ok(position)` si `j` est trouvé, `Err(_)` sinon.
        let range = self.row_ptr[i]..self.row_ptr[i + 1];
        match self.col_idx[range.clone()].binary_search(&j) {
            Ok(k) => self.values[range.start + k], // k est relatif au début de la tranche
            Err(_) => 0.0,                         // non stocké = coefficient nul
        }
    }

    /// Transposée : l'entrée `(i, j)` devient `(j, i)`, une matrice `m × n` devient `n × m`.
    ///
    /// # Panics
    /// Jamais en pratique : les indices inversés d'une matrice valide sont valides.
    #[must_use]
    pub fn transpose(&self) -> Self {
        let mut triplets = Vec::with_capacity(self.nnz()); // taille connue : une seule allocation
        for i in 0..self.n_rows {
            for (j, v) in self.row(i) {
                triplets.push((j, i, v));
            }
        }
        // Dimensions échangées : n_cols lignes, n_rows colonnes.
        Self::from_triplets(self.n_cols, self.n_rows, triplets)
            .expect("indices transposés valides par construction")
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
        assert_eq!(
            x.len(),
            self.n_rows,
            "x doit avoir une valeur par ligne de la matrice"
        );
        let mut result: Vec<f64> = vec![0.0; self.n_cols];

        for (i, &xi) in x.iter().enumerate() {
            for (j, v) in self.row(i) {
                result[j] += xi * v;
            }
        }

        result
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
