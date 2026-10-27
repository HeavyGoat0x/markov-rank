//! Phase 5 : produit matrice × vecteur en « gather », séquentiel et parallèle.
//!
//! `vec_mul` (phase 2) calcule `x · A` en « scatter » : chaque ligne i *disperse* `x[i] · v`
//! dans plusieurs cases `result[j]`. Deux lignes peuvent écrire dans la même case : impossible
//! à paralléliser sans verrou.
//!
//! `mul_vec` calcule `A · x` en « gather » : la case `result[i]` *rassemble* les produits de la
//! ligne i, et chaque ligne n'écrit que dans SA case. Les lignes sont indépendantes : on peut les
//! répartir sur tous les cœurs du processeur avec `rayon`.
//!
//! Astuce : `x · A = Aᵀ · x`. Avec la transposée calculée une fois au départ, PageRank peut donc
//! utiliser le gather parallèle à chaque itération.
//!
//! Ce fichier n'utilise que l'API publique de [`CsrMatrix`] (`row`, `n_rows`, …) : un bloc
//! `impl` peut se trouver dans n'importe quel module du crate qui définit le type.

use rayon::prelude::*;

use crate::CsrMatrix;

impl CsrMatrix {
    /// Produit matrice × vecteur colonne : `(A · x)[i] = Σⱼ A[i][j] · x[j]`.
    ///
    /// # Panics
    /// Si `x.len() != n_cols` : `x` doit avoir une valeur par **colonne** de la matrice
    /// (à l'inverse de [`CsrMatrix::vec_mul`], qui en veut une par ligne).
    #[must_use]
    pub fn mul_vec(&self, x: &[f64]) -> Vec<f64> {
        // Une ligne = une case du résultat :
        // `(0..self.n_rows()).map(|i| self.row(i).map(|(j, v)| v * x[j]).sum()).collect()`
        // (sans oublier le `assert_eq!` sur la taille de x).
        // Vérifié avant la boucle : sinon `x[j]` paniquerait plus loin avec un message
        // « index out of bounds » qui ne dit pas d'où vient l'erreur.
        assert_eq!(
            x.len(),
            self.n_cols(),
            "A · x : x doit avoir une valeur par colonne de la matrice"
        );

        (0..self.n_rows())
            .map(|i| self.row(i).map(|(j, v)| v * x[j]).sum())
            .collect()
    }

    /// Même calcul que [`CsrMatrix::mul_vec`], réparti sur tous les cœurs.
    ///
    /// # Panics
    /// Comme [`CsrMatrix::mul_vec`] : si `x.len() != n_cols`.
    #[must_use]
    pub fn par_mul_vec(&self, x: &[f64]) -> Vec<f64> {
        // Avec `rayon`, il suffit souvent de remplacer `(0..n)` par `(0..n).into_par_iter()` :
        // rayon découpe la plage en morceaux et les donne à ses threads. Le compilateur vérifie
        // que c'est sûr (`CsrMatrix` et `&[f64]` sont `Sync` : lisibles depuis plusieurs threads).
        // Vérifié dans le thread appelant, avant de lancer les threads de rayon.
        assert_eq!(
            x.len(),
            self.n_cols(),
            "A · x : x doit avoir une valeur par colonne de la matrice"
        );
        (0..self.n_rows())
            .into_par_iter()
            .map(|i| self.row(i).map(|(j, v)| v * x[j]).sum())
            .collect()
    }
}

#[cfg(test)]
#[allow(clippy::cast_precision_loss)] // petits entiers convertis en f64 : aucune perte
mod tests {
    use crate::{CsrMatrix, DenseMatrix};

    fn assert_close(a: &[f64], b: &[f64]) {
        assert_eq!(a.len(), b.len());
        for (i, (x, y)) in a.iter().zip(b).enumerate() {
            assert!((x - y).abs() < 1e-9, "indice {i} : {x} != {y}");
        }
    }

    /// Matrice déterministe assez grosse pour que le parallélisme serve vraiment.
    fn big(n: usize) -> CsrMatrix {
        let mut triplets = Vec::new();
        for i in 0..n {
            for k in 1..=5 {
                triplets.push((i, (i * 31 + k * 17) % n, 1.0 / (k as f64)));
            }
        }
        CsrMatrix::from_triplets(n, n, triplets).unwrap()
    }

    #[test]
    fn mul_vec_matches_dense() {
        let dense = DenseMatrix::from_rows(vec![
            vec![0.0, 0.6, 0.4],
            vec![0.2, 0.8, 0.0],
            vec![0.3, 0.2, 0.5],
        ])
        .unwrap();
        let x = [1.0, 2.0, 3.0];
        let expected: Vec<f64> = (0..3)
            .map(|i| (0..3).map(|j| dense.get(i, j) * x[j]).sum())
            .collect();
        assert_close(&CsrMatrix::from_dense(&dense).mul_vec(&x), &expected);
    }

    #[test]
    fn transposed_gather_equals_scatter() {
        // x · A == Aᵀ · x : c'est ce qui permet d'utiliser le gather dans PageRank.
        let a = big(300);
        let x: Vec<f64> = (0..300).map(|i| f64::from(i % 7)).collect();
        assert_close(&a.transpose().mul_vec(&x), &a.vec_mul(&x));
    }

    #[test]
    fn rectangular_mul_vec() {
        let a =
            CsrMatrix::from_triplets(2, 3, vec![(0, 2, 2.0), (1, 0, 1.0), (1, 1, 1.0)]).unwrap();
        assert_close(&a.mul_vec(&[1.0, 10.0, 100.0]), &[200.0, 11.0]);
    }

    #[test]
    fn parallel_equals_sequential() {
        let a = big(20_000);
        let x: Vec<f64> = (0..20_000).map(|i| f64::from((i * 13) % 101)).collect();
        assert_close(&a.par_mul_vec(&x), &a.mul_vec(&x));
    }
}
