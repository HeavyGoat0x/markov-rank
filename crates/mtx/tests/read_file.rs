//! Tests d'intégration : lecture de vrais fichiers, via l'API publique uniquement.

use mtx::{MtxError, read_file, read_graph_file};

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

#[test]
fn read_graph_file_picks_the_reader_from_the_extension() {
    // `.txt` : liste d'adjacence. Pages 1 à 4 → 5, et 5 → 1.
    let star = read_graph_file(data("star.txt")).unwrap();
    assert_eq!((star.rows, star.cols), (5, 5));
    assert_eq!(star.entries.len(), 5);
    assert_eq!(star.entries[4], (4, 0, 1.0));

    // `.mtx` : même résultat que `read_file`.
    assert_eq!(
        read_graph_file(data("ex12.mtx")).unwrap(),
        read_file(data("ex12.mtx")).unwrap()
    );

    assert!(matches!(
        read_graph_file(data("graph.csv")),
        Err(MtxError::Unsupported(_))
    ));
}
