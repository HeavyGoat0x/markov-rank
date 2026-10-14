//! `markov-rank` : PageRank en ligne de commande sur des fichiers `.mtx`.
//!
//! ```text
//! cargo run --release -p markov-rank -- info data/web-Stanford.mtx
//! cargo run --release -p markov-rank -- rank data/web-Stanford.mtx --top 10
//! cargo run --release -p markov-rank -- perturb data/web-Stanford.mtx --remove 1000 --runs 5 --csv out.csv
//! ```
//!
//! `--release` : sans lui, Rust compile sans optimisations et le calcul est 10 à 50× plus lent.
//! Le `--` sépare les options de cargo de celles du programme.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use pagerank::Options;

/// Ligne de commande, décrite par des types : `clap` génère l'analyse des arguments,
/// les messages d'erreur et `--help` à partir des `///` et des `#[arg]`.
#[derive(Debug, Parser)]
#[command(
    name = "markov-rank",
    version,
    about = "PageRank sur des matrices creuses (.mtx)"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Taille du graphe, nombre de liens, pages sans lien sortant.
    Info {
        /// Fichier Matrix Market (`.mtx`).
        file: PathBuf,
    },
    /// Calcule PageRank et affiche les meilleures pages.
    Rank {
        file: PathBuf,
        #[command(flatten)]
        pagerank: PageRankArgs,
        /// Nombre de pages à afficher.
        #[arg(long, default_value_t = 10)]
        top: usize,
    },
    /// Expérience du TD 2 : modifie le graphe au hasard et compare les itérations
    /// (départ uniforme contre ancien PageRank).
    Perturb(PerturbArgs),
}

/// Options communes de PageRank, réutilisées par plusieurs sous-commandes (`flatten`).
#[derive(Debug, Args)]
struct PageRankArgs {
    /// Probabilité de suivre un lien.
    #[arg(long, default_value_t = 0.85)]
    alpha: f64,
    /// Critère d'arrêt sur la distance L1 entre deux itérations.
    #[arg(long, default_value_t = 1e-10)]
    epsilon: f64,
    /// Nombre maximal d'itérations.
    #[arg(long, default_value_t = 1000)]
    max_iter: usize,
}

impl PageRankArgs {
    fn options(&self) -> Options {
        Options {
            alpha: self.alpha,
            epsilon: self.epsilon,
            max_iter: self.max_iter,
        }
    }
}

#[derive(Debug, Args)]
struct PerturbArgs {
    file: PathBuf,
    #[command(flatten)]
    pagerank: PageRankArgs,
    /// Nombre de liens supprimés au hasard.
    #[arg(long, default_value_t = 0)]
    remove: usize,
    /// Nombre de liens ajoutés au hasard.
    #[arg(long, default_value_t = 0)]
    add: usize,
    /// Nombre de pages ajoutées.
    #[arg(long, default_value_t = 0)]
    new_nodes: usize,
    /// Liens entrants et sortants de chaque nouvelle page.
    #[arg(long, default_value_t = 5)]
    links_per_node: usize,
    /// Nombre de répétitions (graines `seed`, `seed + 1`, …).
    #[arg(long, default_value_t = 1)]
    runs: u64,
    /// Graine du générateur aléatoire : même graine = mêmes résultats.
    #[arg(long, default_value_t = 42)]
    seed: u64,
    /// Écrit aussi les résultats dans ce fichier CSV.
    #[arg(long)]
    csv: Option<PathBuf>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("erreur : {e}");
            ExitCode::FAILURE
        }
    }
}

/// `Box<dyn Error>` : « n'importe quelle erreur ». Pratique dans un programme (pas dans une
/// bibliothèque) : `?` y convertit `PageRankError`, `io::Error`… sans `map_err`.
fn run(cli: Cli) -> Result<(), Box<dyn Error>> {
    match cli.command {
        Command::Info { file } => cmd_info(&file),
        Command::Rank {
            file,
            pagerank,
            top,
        } => cmd_rank(&file, &pagerank.options(), top),
        Command::Perturb(args) => cmd_perturb(&args),
    }
}

/// Affiche : nombre de pages, nombre de liens, nombre et pourcentage de pages dangling.
fn cmd_info(file: &Path) -> Result<(), Box<dyn Error>> {
    // `Transition::from_mtx_file(file)?` donne tout : `t.n()`, `t.p.nnz()`, `t.dangling_count()`.
    todo!()
}

/// Calcule PageRank, puis affiche : itérations, résidu, temps (`std::time::Instant`),
/// et les `top` meilleures pages avec leur score (`pagerank::ranking::top_k`).
/// Attention : les pages sont numérotées à partir de 1 dans le fichier, 0 dans le code.
fn cmd_rank(file: &Path, opts: &Options, top: usize) -> Result<(), Box<dyn Error>> {
    todo!()
}

/// Pour chaque run r (graine `seed + r`) : part de `Graph::from_mtx_file`, applique
/// `remove_random_links`, `add_random_links` puis `add_random_nodes`, appelle
/// `compare_starts`, et affiche les deux nombres d'itérations. Avec `--csv`, écrit une ligne
/// par run : `run,seed,removed,added,new_nodes,uniform_iterations,warm_iterations`
/// (`std::fs::File::create` + `writeln!`).
fn cmd_perturb(args: &PerturbArgs) -> Result<(), Box<dyn Error>> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_valid() {
        // Vérifie toute la définition (noms en double, valeurs par défaut invalides…).
        Cli::command().debug_assert();
    }

    #[test]
    fn parses_rank_with_defaults() {
        let cli = Cli::try_parse_from(["markov-rank", "rank", "g.mtx", "--top", "5"]).unwrap();
        match cli.command {
            Command::Rank {
                file,
                pagerank,
                top,
            } => {
                assert_eq!(file, PathBuf::from("g.mtx"));
                assert_eq!(top, 5);
                assert_eq!(pagerank.options(), Options::default());
            }
            other => panic!("mauvaise sous-commande : {other:?}"),
        }
    }

    #[test]
    fn parses_perturb_options() {
        let cli = Cli::try_parse_from([
            "markov-rank",
            "perturb",
            "g.mtx",
            "--remove",
            "100",
            "--new-nodes",
            "3",
            "--seed",
            "7",
            "--alpha",
            "0.9",
        ])
        .unwrap();
        let Command::Perturb(args) = cli.command else {
            panic!("mauvaise sous-commande");
        };
        assert_eq!((args.remove, args.add, args.new_nodes), (100, 0, 3));
        assert_eq!(args.seed, 7);
        assert!((args.pagerank.alpha - 0.9).abs() < 1e-12);
        assert!(args.csv.is_none());
    }

    #[test]
    fn rejects_unknown_options() {
        assert!(Cli::try_parse_from(["markov-rank", "rank", "g.mtx", "--oops"]).is_err());
    }
}
