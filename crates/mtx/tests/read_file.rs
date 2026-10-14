//! Tests d'intégration : lecture de vrais fichiers, via l'API publique uniquement.

use mtx::{MtxError, read_file};

/// Chemin d'un fichier de `tests/data/`, quel que soit le dossier d'où l'on lance `cargo test`.
fn data(name: &str) -> String {
    format!("{}/tests/data/{name}", env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn reads_ex12_file() {
    let coo = read_file(data("ex12.mtx")).unwrap();
    assert_eq!((coo.rows, coo.cols), (3, 3));
    assert_eq!(coo.entries.len(), 7);

    // Chaque ligne d'une matrice de transition somme à 1.
    let mut row_sums = [0.0; 3];
    for &(i, _, v) in &coo.entries {
        row_sums[i] += v;
    }
    for s in row_sums {
        assert!((s - 1.0).abs() < 1e-12);
    }
}

#[test]
fn missing_file_is_io_error() {
    assert!(matches!(
        read_file(data("does-not-exist.mtx")),
        Err(MtxError::Io(_))
    ));
}
