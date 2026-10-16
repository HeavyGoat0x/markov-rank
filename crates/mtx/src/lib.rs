//! Lecture des fichiers Matrix Market (`.mtx`) de <https://sparse.tamu.edu>.
//!
//! Format (variante « coordinate ») :
//!
//! ```text
//! %%MatrixMarket matrix coordinate real general    <- en-tête
//! % commentaire                                    <- 0 ou plusieurs
//! 3 3 7                                            <- lignes colonnes nnz
//! 1 2 0.6                                          <- i j [valeur], indices à partir de 1
//! ...
//! ```
//!
//! Le crate ne dépend pas de `markov` : il renvoie une liste de triplets (format COO)
//! que l'appelant convertit dans la structure de son choix.
//!
//! Il lit aussi le format « liste d'adjacence » des fichiers `.txt` du TD (module
//! [`adjacency`]) ; [`read_graph_file`] choisit le bon lecteur d'après l'extension.

pub mod adjacency;

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub use adjacency::{read_adjacency, read_adjacency_file};

/// Type des valeurs stockées.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    /// Flottants.
    Real,
    /// Entiers (convertis en `f64`).
    Integer,
    /// Pas de valeur : chaque entrée vaut 1 (graphes non pondérés).
    Pattern,
}

/// Symétrie : seule la moitié de la matrice est stockée dans le fichier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Symmetry {
    /// Toutes les entrées sont présentes.
    General,
    /// `A[j][i] = A[i][j]` : chaque entrée hors diagonale vaut pour deux.
    Symmetric,
    /// `A[j][i] = -A[i][j]`.
    SkewSymmetric,
}

/// En-tête `%%MatrixMarket matrix coordinate <field> <symmetry>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub field: Field,
    pub symmetry: Symmetry,
}

/// Ligne de taille `lignes colonnes nnz`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size {
    pub rows: usize,
    pub cols: usize,
    /// Nombre d'entrées écrites dans le fichier (avant expansion des symétries).
    pub nnz: usize,
}

/// Matrice au format COO : triplets `(ligne, colonne, valeur)`, indices **à partir de 0**.
#[derive(Debug, Clone, PartialEq)]
pub struct Coo {
    pub rows: usize,
    pub cols: usize,
    pub entries: Vec<(usize, usize, f64)>,
}

/// Erreurs de lecture. Les numéros de ligne commencent à 1, comme dans un éditeur.
#[derive(Debug, thiserror::Error)]
pub enum MtxError {
    #[error("erreur d'entrée/sortie : {0}")]
    Io(#[from] std::io::Error),

    #[error("en-tête invalide (attendu `%%MatrixMarket matrix coordinate <field> <symmetry>`)")]
    InvalidHeader,

    #[error("variante non supportée : {0}")]
    Unsupported(String),

    #[error("ligne {line} : ligne de taille invalide (attendu `lignes colonnes nnz`)")]
    InvalidSize { line: usize },

    #[error("ligne {line} : entrée invalide")]
    InvalidEntry { line: usize },

    #[error("ligne {line} : indice ({row}, {col}) hors d'une matrice {rows}×{cols}")]
    OutOfBounds {
        line: usize,
        row: usize,
        col: usize,
        rows: usize,
        cols: usize,
    },

    #[error("fichier sans ligne de taille")]
    MissingSize,

    #[error("{expected} entrées annoncées, {found} lues")]
    WrongCount { expected: usize, found: usize },

    #[error(
        "ligne {line} : page {page} hors de 1..={pages} (l'en-tête annonce {pages} pages : \
         est-il juste ?)"
    )]
    PageOutOfRange {
        line: usize,
        page: usize,
        pages: usize,
    },
}

/// Analyse la première ligne du fichier.
///
/// Les mots-clés ne tiennent pas compte de la casse (`Real` = `real`).
///
/// # Errors
/// [`MtxError::InvalidHeader`] si la ligne n'a pas la forme attendue,
/// [`MtxError::Unsupported`] pour `array`, `complex` ou `hermitian`.
pub fn parse_header(line: &str) -> Result<Header, MtxError> {
    let words: Vec<String> = line.split_whitespace().map(str::to_lowercase).collect();
    if words.len() != 5 || words[0] != "%%matrixmarket" || words[1] != "matrix" {
        return Err(MtxError::InvalidHeader);
    }
    if words[2] != "coordinate" {
        return Err(MtxError::Unsupported(words[2].clone()));
    }

    let field = match words[3].as_str() {
        "real" => Field::Real,
        "integer" => Field::Integer,
        "pattern" => Field::Pattern,
        other => return Err(MtxError::Unsupported(other.to_string())),
    };
    let symmetry = match words[4].as_str() {
        "general" => Symmetry::General,
        "symmetric" => Symmetry::Symmetric,
        "skew-symmetric" => Symmetry::SkewSymmetric,
        other => return Err(MtxError::Unsupported(other.to_string())),
    };

    Ok(Header { field, symmetry })
}

/// Analyse la ligne `lignes colonnes nnz`. `line_no` sert aux messages d'erreur.
///
/// # Errors
/// [`MtxError::InvalidSize`] s'il n'y a pas exactement trois entiers positifs.
pub fn parse_size(line: &str, line_no: usize) -> Result<Size, MtxError> {
    // Closure qui fabrique notre erreur : évite de répéter `InvalidSize { line: line_no }`.
    let err = || MtxError::InvalidSize { line: line_no };

    // `&str` et non `String` : les mots pointent dans `line`, aucune copie.
    let words: Vec<&str> = line.split_whitespace().collect();
    if words.len() != 3 {
        return Err(err());
    }

    // `parse` renvoie un `Result` : `map_err` remplace son erreur par la nôtre,
    // puis `?` sort de la fonction si c'est un `Err`, sinon donne le nombre.
    // "-1" échoue tout seul : un `usize` ne peut pas être négatif.
    let rows = words[0].parse::<usize>().map_err(|_| err())?;
    let cols = words[1].parse::<usize>().map_err(|_| err())?;
    let nnz = words[2].parse::<usize>().map_err(|_| err())?;

    Ok(Size { rows, cols, nnz })
}

/// Analyse une ligne d'entrée `i j [valeur]` et renvoie des indices **à partir de 0**.
///
/// Pour [`Field::Pattern`], il n'y a pas de valeur et l'entrée vaut `1.0`.
///
/// # Errors
/// [`MtxError::InvalidEntry`] si un nombre manque, est en trop ou ne se lit pas,
/// [`MtxError::OutOfBounds`] si un indice vaut 0 ou dépasse la taille.
pub fn parse_entry(
    line: &str,
    line_no: usize,
    field: Field,
    size: Size,
) -> Result<(usize, usize, f64), MtxError> {
    let bad = || MtxError::InvalidEntry { line: line_no };

    let words: Vec<&str> = line.split_whitespace().collect();
    let expected = if field == Field::Pattern { 2 } else { 3 };
    if words.len() != expected {
        return Err(bad());
    }

    let row = words[0].parse::<usize>().map_err(|_| bad())?;
    let col = words[1].parse::<usize>().map_err(|_| bad())?;
    let value = if field == Field::Pattern {
        1.0
    } else {
        words[2].parse::<f64>().map_err(|_| bad())?
    };

    // Indices du fichier : de 1 à rows/cols inclus.
    if row == 0 || row > size.rows || col == 0 || col > size.cols {
        return Err(MtxError::OutOfBounds {
            line: line_no,
            row,
            col,
            rows: size.rows,
            cols: size.cols,
        });
    }

    Ok((row - 1, col - 1, value))
}

/// Lit une matrice depuis n'importe quelle source ligne par ligne (fichier, chaîne en mémoire…).
///
/// Ignore les lignes vides et les commentaires (`%`). Développe les symétries :
/// pour une entrée `(i, j, v)` hors diagonale, ajoute aussi `(j, i, v)` (ou `-v` si antisymétrique).
///
/// # Errors
/// Toute erreur des fonctions `parse_*`, une erreur d'E/S,
/// [`MtxError::MissingSize`] ou [`MtxError::WrongCount`].
pub fn read<R: BufRead>(reader: R) -> Result<Coo, MtxError> {
    // `mut` : l'itérateur avance à chaque `next()` et se souvient de sa position.
    let mut lines = reader.lines().enumerate();

    // 1) En-tête : la toute première ligne, lue AVANT la boucle.
    //    `line?` : erreur d'E/S → `MtxError::Io` (grâce à `#[from]`).
    //    `parse_header(...)?` : en-tête invalide → on renvoie l'erreur telle quelle.
    let header = match lines.next() {
        Some((_, line)) => parse_header(&line?)?,
        None => return Err(MtxError::InvalidHeader), // texte vide : c'est nous qui décidons
    };

    // Déclarées AVANT la boucle pour survivre d'un tour à l'autre.
    let mut size: Option<Size> = None; // `None` tant que la ligne de taille n'est pas lue
    let mut entries = Vec::new();
    let mut found = 0; // lignes d'entrée lues (sans compter les doublons dus à la symétrie)

    // 2) La boucle reprend là où `next()` s'est arrêté : à la deuxième ligne.
    for (index, line) in lines {
        let line = line?;
        let text = line.trim();
        let line_no = index + 1; // `enumerate` compte depuis 0, un éditeur depuis 1

        // Lignes vides et commentaires : on passe à la suivante.
        if text.is_empty() || text.starts_with('%') {
            continue;
        }

        match size {
            // Pas encore de taille : cette première ligne utile EST la taille.
            None => {
                let s = parse_size(text, line_no)?;
                entries.reserve(s.nnz); // alloue la mémoire une seule fois
                size = Some(s);
            }
            // Taille connue : toutes les lignes suivantes sont des entrées.
            Some(s) => {
                let (i, j, v) = parse_entry(text, line_no, header.field, s)?;
                found += 1;
                entries.push((i, j, v));

                // Symétrie : le fichier ne stocke qu'une moitié, on ajoute l'autre.
                // La diagonale (i == j) n'a pas de « moitié miroir ».
                if i != j {
                    match header.symmetry {
                        Symmetry::General => {}
                        Symmetry::Symmetric => entries.push((j, i, v)),
                        Symmetry::SkewSymmetric => entries.push((j, i, -v)),
                    }
                }
            }
        }
    }

    // 3) Vérifications finales.
    //    `ok_or` transforme l'`Option` en `Result` : `None` → `Err(MissingSize)`.
    let size = size.ok_or(MtxError::MissingSize)?;
    if found != size.nnz {
        return Err(MtxError::WrongCount {
            expected: size.nnz,
            found,
        });
    }

    Ok(Coo {
        rows: size.rows,
        cols: size.cols,
        entries,
    })
}

/// Ouvre et lit un fichier `.mtx`.
///
/// # Errors
/// Voir [`read`] ; [`MtxError::Io`] si le fichier ne s'ouvre pas.
pub fn read_file<P: AsRef<Path>>(path: P) -> Result<Coo, MtxError> {
    let file = File::open(path)?;
    read(BufReader::new(file))
}

/// Lit un graphe en choisissant le format d'après l'extension : `.mtx` (Matrix Market) ou
/// `.txt` (liste d'adjacence du TD, voir [`adjacency`]).
///
/// # Errors
/// Celles de [`read_file`] ou de [`read_adjacency_file`] ;
/// [`MtxError::Unsupported`] pour une autre extension.
pub fn read_graph_file<P: AsRef<Path>>(path: P) -> Result<Coo, MtxError> {
    let path = path.as_ref();
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_lowercase);
    match extension.as_deref() {
        Some("mtx") => read_file(path),
        Some("txt") => read_adjacency_file(path),
        _ => Err(MtxError::Unsupported(format!(
            "extension de {} (attendu .mtx ou .txt)",
            path.display()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZE_3X3: Size = Size {
        rows: 3,
        cols: 3,
        nnz: 7,
    };

    // ---------- En-tête ----------

    #[test]
    fn header_real_general() {
        let h = parse_header("%%MatrixMarket matrix coordinate real general").unwrap();
        assert_eq!(
            h,
            Header {
                field: Field::Real,
                symmetry: Symmetry::General
            }
        );
    }

    #[test]
    fn header_is_case_insensitive() {
        let h = parse_header("%%MatrixMarket Matrix Coordinate Pattern Symmetric").unwrap();
        assert_eq!(h.field, Field::Pattern);
        assert_eq!(h.symmetry, Symmetry::Symmetric);
    }

    #[test]
    fn header_integer_skew() {
        let h = parse_header("%%MatrixMarket matrix coordinate integer skew-symmetric").unwrap();
        assert_eq!(h.field, Field::Integer);
        assert_eq!(h.symmetry, Symmetry::SkewSymmetric);
    }

    #[test]
    fn header_rejects_garbage() {
        assert!(matches!(
            parse_header("hello world"),
            Err(MtxError::InvalidHeader)
        ));
        assert!(matches!(
            parse_header("%%MatrixMarket matrix coordinate real"),
            Err(MtxError::InvalidHeader)
        ));
    }

    #[test]
    fn header_rejects_unsupported_variants() {
        for line in [
            "%%MatrixMarket matrix array real general",
            "%%MatrixMarket matrix coordinate complex general",
            "%%MatrixMarket matrix coordinate real hermitian",
        ] {
            assert!(
                matches!(parse_header(line), Err(MtxError::Unsupported(_))),
                "{line}"
            );
        }
    }

    // ---------- Taille ----------

    #[test]
    fn size_ok() {
        assert_eq!(parse_size("3 3 7", 2).unwrap(), SIZE_3X3);
        assert_eq!(parse_size("  10   20  5 ", 2).unwrap().cols, 20);
    }

    #[test]
    fn size_rejects_bad_lines() {
        for line in ["3 3", "3 3 7 1", "a b c", "-1 3 7"] {
            assert!(
                matches!(parse_size(line, 4), Err(MtxError::InvalidSize { line: 4 })),
                "{line}"
            );
        }
    }

    // ---------- Entrées ----------

    #[test]
    fn entry_real_is_zero_based() {
        let e = parse_entry("1 2 0.6", 5, Field::Real, SIZE_3X3).unwrap();
        assert_eq!(e, (0, 1, 0.6));
    }

    #[test]
    fn entry_integer_and_pattern() {
        assert_eq!(
            parse_entry("3 3 5", 5, Field::Integer, SIZE_3X3).unwrap(),
            (2, 2, 5.0)
        );
        assert_eq!(
            parse_entry("2 1", 5, Field::Pattern, SIZE_3X3).unwrap(),
            (1, 0, 1.0)
        );
    }

    #[test]
    fn entry_rejects_malformed() {
        for (line, field) in [
            ("1 2", Field::Real),        // valeur manquante
            ("1 2 0.5", Field::Pattern), // valeur en trop
            ("1 x 0.5", Field::Real),    // pas un nombre
            ("1 2 abc", Field::Real),
        ] {
            assert!(
                matches!(
                    parse_entry(line, 7, field, SIZE_3X3),
                    Err(MtxError::InvalidEntry { line: 7 })
                ),
                "{line}"
            );
        }
    }

    #[test]
    fn entry_rejects_out_of_bounds() {
        for line in ["0 1 0.5", "1 0 0.5", "4 1 0.5", "1 4 0.5"] {
            assert!(
                matches!(
                    parse_entry(line, 9, Field::Real, SIZE_3X3),
                    Err(MtxError::OutOfBounds { line: 9, .. })
                ),
                "{line}"
            );
        }
    }

    // ---------- Fichier complet ----------

    /// Matrice de l'exercice 1.2 du TD 1.
    const EX12: &str = "\
%%MatrixMarket matrix coordinate real general
% Exercice 1.2 du TD 1
%
3 3 7
1 2 0.6
1 3 0.4
2 1 0.2
2 2 0.8
3 1 0.3

3 2 0.2
3 3 0.5
";

    #[test]
    fn read_ex12() {
        // `&[u8]` implémente `BufRead` : pratique pour tester sans fichier.
        let coo = read(EX12.as_bytes()).unwrap();
        assert_eq!((coo.rows, coo.cols), (3, 3));
        assert_eq!(coo.entries.len(), 7);
        assert_eq!(coo.entries[0], (0, 1, 0.6));
        assert_eq!(coo.entries[6], (2, 2, 0.5));
    }

    #[test]
    fn read_expands_symmetric() {
        let text = "\
%%MatrixMarket matrix coordinate pattern symmetric
3 3 3
1 1
2 1
3 2
";
        let mut entries = read(text.as_bytes()).unwrap().entries;
        entries.sort_by_key(|&(i, j, _)| (i, j));
        // La diagonale (1,1) n'est pas dupliquée ; (2,1) et (3,2) le sont.
        assert_eq!(
            entries,
            vec![
                (0, 0, 1.0),
                (0, 1, 1.0),
                (1, 0, 1.0),
                (1, 2, 1.0),
                (2, 1, 1.0)
            ]
        );
    }

    #[test]
    fn read_expands_skew_symmetric() {
        let text = "\
%%MatrixMarket matrix coordinate real skew-symmetric
2 2 1
2 1 3.0
";
        let mut entries = read(text.as_bytes()).unwrap().entries;
        entries.sort_by_key(|&(i, j, _)| (i, j));
        assert_eq!(entries, vec![(0, 1, -3.0), (1, 0, 3.0)]);
    }

    #[test]
    fn read_rejects_wrong_count() {
        let text = "%%MatrixMarket matrix coordinate real general\n2 2 3\n1 1 1.0\n2 2 1.0\n";
        assert!(matches!(
            read(text.as_bytes()),
            Err(MtxError::WrongCount {
                expected: 3,
                found: 2
            })
        ));
    }

    #[test]
    fn read_rejects_missing_header_or_size() {
        assert!(matches!(
            read("3 3 7\n".as_bytes()),
            Err(MtxError::InvalidHeader)
        ));
        assert!(matches!(read("".as_bytes()), Err(MtxError::InvalidHeader)));
        let only_header = "%%MatrixMarket matrix coordinate real general\n% rien\n";
        assert!(matches!(
            read(only_header.as_bytes()),
            Err(MtxError::MissingSize)
        ));
    }

    #[test]
    fn read_reports_line_numbers() {
        // L'entrée invalide est à la ligne 4 du texte (en comptant l'en-tête et le commentaire).
        let text = "%%MatrixMarket matrix coordinate real general\n% c\n2 2 1\n1 oops 1.0\n";
        assert!(matches!(
            read(text.as_bytes()),
            Err(MtxError::InvalidEntry { line: 4 })
        ));
    }
}
