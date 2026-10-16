//! Tests d'intégration : du fichier `.mtx` jusqu'au classement, via l'API publique.

use pagerank::{Options, PageRankError, Transition, pagerank};

fn data(name: &str) -> String {
    format!("{}/tests/data/{name}", env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn star_graph_ranks_the_hub_first() {
    let t = Transition::from_mtx_file(data("star.mtx")).unwrap();
    assert_eq!(t.n(), 5);
    assert_eq!(t.dangling_count(), 0);

    let r = pagerank(&t, &Options::default()).unwrap();
    assert!(r.converged);
    let best = (0..5)
        .max_by(|&a, &b| r.scores[a].total_cmp(&r.scores[b]))
        .unwrap();
    assert_eq!(
        best, 4,
        "la page 5 du fichier (indice 4) reçoit tous les liens"
    );
}

#[test]
fn txt_and_mtx_give_the_same_transition() {
    // Le même petit web, dans les deux formats.
    let from_txt = Transition::from_mtx_file(data("star.txt")).unwrap();
    let from_mtx = Transition::from_mtx_file(data("star.mtx")).unwrap();
    assert_eq!(from_txt, from_mtx);
}

#[test]
fn rectangular_file_is_rejected() {
    assert!(matches!(
        Transition::from_mtx_file(data("rect.mtx")),
        Err(PageRankError::NotSquare { rows: 2, cols: 3 })
    ));
}

#[test]
fn missing_file_is_a_read_error() {
    assert!(matches!(
        Transition::from_mtx_file(data("missing.mtx")),
        Err(PageRankError::Mtx(_))
    ));
}
