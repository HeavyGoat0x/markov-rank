use markov::{DenseMatrix, iterate};

fn main() {
    let p = DenseMatrix::from_rows(vec![
        vec![0.0, 0.6, 0.4],
        vec![0.2, 0.8, 0.0],
        vec![0.3, 0.2, 0.5],
    ])
    .expect("matrice valide");

    let starts = [
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0],
    ];
    for pi0 in starts {
        println!("Départ π(0) = {pi0:.5?}");

        for (index, pi) in iterate(&pi0, &p, 12).iter().enumerate() {
            println!("  π({index:2}) = {pi:.5?}");
        }
        println!();
    }

    // Q7-Q8 : P², P⁴, P⁸, …, P¹⁰²⁴ par mises au carré successives.
    let mut pk = p.clone();
    for k in 1..=10 {
        pk = pk.square();
        println!("P^{}", 1 << k);
        println!("{pk}");
    }
}
