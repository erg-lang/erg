/// Calculates the Levenshtein distance (edit distance).
/// This shows how close the strings are to each other.
pub fn levenshtein(lhs: &str, rhs: &str) -> usize {
    let lhs = lhs.chars().collect::<Vec<char>>();
    let rhs = rhs.chars().collect::<Vec<char>>();
    let l_len = lhs.len();
    let r_len = rhs.len();
    // l_len+1 × r_len+1 array
    let mut table = vec![vec![0; r_len + 1]; l_len + 1];
    for i in 0..l_len + 1 {
        table[i][0] = i;
    }
    for i in 0..r_len + 1 {
        table[0][i] = i;
    }
    for i1 in 0..l_len {
        for i2 in 0..r_len {
            let cost = if lhs[i1] == rhs[i2] { 0 } else { 1 };
            table[i1 + 1][i2 + 1] = *[
                table[i1][i2 + 1] + 1, // delete cost
                table[i1 + 1][i2] + 1, // insert cost
                table[i1][i2] + cost,  // replace cost
            ]
            .iter()
            .min()
            .unwrap();
        }
    }
    return table[l_len][r_len]
}
