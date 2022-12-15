/// Calculates the Levenshtein distance (edit distance).
/// This shows how close the strings are to each other.
pub fn levenshtein(lhs: &str, rhs: &str) -> usize {
    let lhs = lhs.chars().collect::<Vec<char>>();
    let rhs = rhs.chars().collect::<Vec<char>>();
    let l_len = lhs.len();
    let r_len = rhs.len();
    // l_len+1 × r_len+1 array
    let mut table = vec![vec![0; r_len + 1]; l_len + 1];
    table
        .iter_mut()
        .take(l_len + 1)
        .enumerate()
        .for_each(|(i, row)| row[0] = i);
    table[0]
        .iter_mut()
        .take(r_len + 1)
        .enumerate()
        .for_each(|(i, elem)| *elem = i);
    for i1 in 0..l_len {
        #[allow(clippy::needless_range_loop)]
        for i2 in 0..r_len {
            let cost = usize::from(lhs[i1] != rhs[i2]);
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
    table[l_len][r_len]
}

pub fn get_similar_name<'a, I: Iterator<Item = &'a str> + Clone>(
    candidates: I,
    name: &str,
) -> Option<&'a str> {
    let most_similar_name = candidates.min_by_key(|v| levenshtein(v, name))?;
    let len = most_similar_name.len();
    let dist = levenshtein(most_similar_name, name) as f64;
    if dist >= (len as f64).sqrt() {
        None
    } else {
        Some(most_similar_name)
    }
}

#[cfg(test)]
mod tests {
    use crate::levenshtein::get_similar_name;

    #[test]
    fn test_get_similar_name() {
        assert_eq!(get_similar_name(["a", "b", "c"].into_iter(), "k"), None);
        assert_eq!(
            get_similar_name(["True", "b", "c"].into_iter(), "true"),
            Some("True")
        );
        assert_eq!(
            get_similar_name(["True", "b", "c"].into_iter(), "truth"),
            None
        );
        assert_eq!(
            get_similar_name(["True", "False", "c"].into_iter(), "Felze"),
            Some("False")
        );
    }
}
