//! Lecture du format « liste d'adjacence » des graphes du TD (`wb-edu.txt`,
//! `Stanford_BerkeleyV2.txt`) :
//!
//! ```text
//! 5                  <- nombre de pages
//! 5                  <- nombre de liens
//! 1 1 5 1.000000     <- page, degré sortant, puis (cible, probabilité) × degré
//! 2 0                <- page sans lien sortant (dangling)
//! ...
//! ```
//!
//! Pages numérotées à partir de 1. La probabilité vaut `1 / degré` : on la garde telle quelle
//! dans le COO (de toute façon `Transition::from_adjacency` renormalise chaque ligne).

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::{Coo, MtxError};

/// Lit une ligne d'en-tête : un seul entier positif.
fn parse_count(line: &str, line_no: usize) -> Result<usize, MtxError> {
    let err = || MtxError::InvalidSize { line: line_no };
    let mut words = line.split_whitespace();
    match (words.next(), words.next()) {
        (Some(word), None) => word.parse::<usize>().map_err(|_| err()),
        _ => Err(err()),
    }
}

/// Analyse une ligne `page degré (cible probabilité)…`, ajoute ses liens à `entries`
/// (indices **à partir de 0**) et renvoie le degré.
///
/// # Errors
/// [`MtxError::InvalidEntry`] si un nombre ne se lit pas ou si le nombre de couples ne
/// correspond pas au degré, [`MtxError::PageOutOfRange`] si une page vaut 0 ou dépasse `n`.
pub fn parse_page_line(
    line: &str,
    line_no: usize,
    n: usize,
    entries: &mut Vec<(usize, usize, f64)>,
) -> Result<usize, MtxError> {
    let bad = || MtxError::InvalidEntry { line: line_no };
    // Numéro du fichier (1..=n) → indice (0..n), ou erreur.
    let index = |page: usize| {
        if page == 0 || page > n {
            Err(MtxError::PageOutOfRange {
                line: line_no,
                page,
                pages: n,
            })
        } else {
            Ok(page - 1)
        }
    };

    let words: Vec<&str> = line.split_whitespace().collect();
    if words.len() < 2 {
        return Err(bad());
    }
    let from = index(words[0].parse::<usize>().map_err(|_| bad())?)?;
    let degree = words[1].parse::<usize>().map_err(|_| bad())?;

    // `as_chunks::<2>()` découpe le reste de la ligne en couples `[cible, probabilité]`, et
    // renvoie à part ce qui ne remplit pas un couple complet (`rest`, qui doit être vide).
    // Il faut exactement `degree` couples.
    let (pairs, rest) = words[2..].as_chunks::<2>();
    if !rest.is_empty() || pairs.len() != degree {
        return Err(bad());
    }

    // Déstructuration : chaque couple `[&str; 2]` est découpé en deux variables.
    for [target, probability] in pairs {
        let to = index(target.parse::<usize>().map_err(|_| bad())?)?;
        let probability = probability.parse::<f64>().map_err(|_| bad())?;
        entries.push((from, to, probability));
    }
    Ok(degree)
}

/// Lit un graphe au format liste d'adjacence depuis n'importe quelle source ligne par ligne.
///
/// Les lignes vides sont ignorées. La matrice renvoyée est carrée (`n × n`).
///
/// # Errors
/// Erreur d'E/S, [`MtxError::InvalidSize`] si une ligne d'en-tête n'est pas un entier seul,
/// [`MtxError::MissingSize`] s'il manque l'en-tête, les erreurs de [`parse_page_line`], et
/// [`MtxError::WrongCount`] si le nombre de liens lus diffère de celui annoncé.
pub fn read_adjacency<R: BufRead>(reader: R) -> Result<Coo, MtxError> {
    let mut pages: Option<usize> = None;
    let mut links: Option<usize> = None;
    let mut entries = Vec::new();
    let mut found = 0;

    for (index, line) in reader.lines().enumerate() {
        let line = line?;
        let text = line.trim();
        let line_no = index + 1;
        if text.is_empty() {
            continue;
        }

        // Les deux premières lignes utiles sont l'en-tête, toutes les suivantes des pages.
        match (pages, links) {
            (None, _) => pages = Some(parse_count(text, line_no)?),
            (Some(_), None) => {
                let m = parse_count(text, line_no)?;
                entries.reserve(m); // alloue la mémoire une seule fois
                links = Some(m);
            }
            (Some(n), Some(_)) => found += parse_page_line(text, line_no, n, &mut entries)?,
        }
    }

    let n = pages.ok_or(MtxError::MissingSize)?;
    let m = links.ok_or(MtxError::MissingSize)?;
    if found != m {
        return Err(MtxError::WrongCount { expected: m, found });
    }

    Ok(Coo {
        rows: n,
        cols: n,
        entries,
    })
}

/// Ouvre et lit un fichier au format liste d'adjacence.
///
/// # Errors
/// Voir [`read_adjacency`] ; [`MtxError::Io`] si le fichier ne s'ouvre pas.
pub fn read_adjacency_file<P: AsRef<Path>>(path: P) -> Result<Coo, MtxError> {
    let file = File::open(path)?;
    read_adjacency(BufReader::new(file))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 3 pages : 1 → 2 et 3, 2 dangling, 3 → 1. Espaces en fin de ligne comme dans les
    /// vrais fichiers.
    const SMALL: &str = "\
3
3
1 2 2 0.500000 3 0.500000
2 0
3 1 1 1.000000
";

    #[test]
    fn reads_small_graph_zero_based() {
        let coo = read_adjacency(SMALL.as_bytes()).unwrap();
        assert_eq!((coo.rows, coo.cols), (3, 3));
        assert_eq!(coo.entries, vec![(0, 1, 0.5), (0, 2, 0.5), (2, 0, 1.0)]);
    }

    #[test]
    fn tolerates_trailing_spaces_and_blank_lines() {
        let text = "3    \n3  \n\n1 2 2 0.5 3 0.5  \n2 0  \n3 1 1 1.0\n\n";
        assert_eq!(read_adjacency(text.as_bytes()).unwrap().entries.len(), 3);
    }

    #[test]
    fn wrong_page_count_in_header_is_reported() {
        // En-tête faux : il annonce 2 pages, mais la ligne 3 parle de la page 3.
        let text = "2\n3\n1 2 2 0.5 3 0.5\n2 0\n3 1 1 1.0\n";
        assert!(matches!(
            read_adjacency(text.as_bytes()),
            Err(MtxError::PageOutOfRange {
                line: 3,
                page: 3,
                pages: 2
            })
        ));
    }

    #[test]
    fn page_zero_is_out_of_range() {
        let text = "2\n1\n0 1 2 1.0\n";
        assert!(matches!(
            read_adjacency(text.as_bytes()),
            Err(MtxError::PageOutOfRange { page: 0, .. })
        ));
    }

    #[test]
    fn degree_must_match_the_pairs() {
        for line in ["1 2 2 0.5", "1 1 2 0.5 3", "1 1 2 x", "1 x"] {
            let text = format!("3\n1\n{line}\n");
            assert!(
                matches!(
                    read_adjacency(text.as_bytes()),
                    Err(MtxError::InvalidEntry { line: 3 })
                ),
                "{line}"
            );
        }
    }

    #[test]
    fn wrong_link_count_is_reported() {
        let text = "3\n5\n1 2 2 0.5 3 0.5\n2 0\n3 1 1 1.0\n";
        assert!(matches!(
            read_adjacency(text.as_bytes()),
            Err(MtxError::WrongCount {
                expected: 5,
                found: 3
            })
        ));
    }

    #[test]
    fn bad_or_missing_header() {
        assert!(matches!(
            read_adjacency("3 3\n3\n".as_bytes()),
            Err(MtxError::InvalidSize { line: 1 })
        ));
        assert!(matches!(
            read_adjacency("".as_bytes()),
            Err(MtxError::MissingSize)
        ));
        assert!(matches!(
            read_adjacency("3\n".as_bytes()),
            Err(MtxError::MissingSize)
        ));
    }
}
