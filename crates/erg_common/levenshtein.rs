/// copied and modified from https://doc.rust-lang.org/beta/nightly-rustc/src/rustc_span/lev_distance.rs.html
///
/// Finds the Levenshtein distance between two strings.
///
/// Returns None if the distance exceeds the limit.
pub fn levenshtein(a: &str, b: &str, limit: usize) -> Option<usize> {
    if a == b {
        return Some(0);
    }

    let n = a.chars().count();
    let m = b.chars().count();
    let min_dist = m.abs_diff(n);

    if min_dist > limit {
        return None;
    }
    if n == 0 || m == 0 {
        return (min_dist <= limit).then_some(min_dist);
    }

    let mut dcol: Vec<_> = (0..=m).collect();

    for (i, sc) in a.chars().enumerate() {
        let mut current = i;
        dcol[0] = current + 1;

        for (j, tc) in b.chars().enumerate() {
            let next = dcol[j + 1];
            if sc == tc {
                dcol[j + 1] = current;
            } else {
                dcol[j + 1] = current.min(next);
                dcol[j + 1] = dcol[j + 1].min(dcol[j]) + 1;
            }
            current = next;
        }
    }

    (dcol[m] <= limit).then_some(dcol[m])
}

pub fn get_similar_name<'a, S: ?Sized, I: Iterator<Item = &'a S>>(
    candidates: I,
    name: &str,
) -> Option<&'a S>
where
    S: std::borrow::Borrow<str>,
{
    let limit = (name.len() as f64).sqrt().round() as usize;
    let most_similar_name =
        candidates.min_by_key(|v| levenshtein(v.borrow(), name, limit).unwrap_or(usize::MAX))?;
    let dist = levenshtein(most_similar_name.borrow(), name, limit);
    if dist.map_or(true, |d| d >= limit) {
        None
    } else {
        Some(most_similar_name)
    }
}

pub fn get_similar_name_and_some<'a, S: ?Sized, T, I: Iterator<Item = (&'a T, &'a S)>>(
    candidates: I,
    name: &str,
) -> Option<(&'a T, &'a S)>
where
    S: std::borrow::Borrow<str>,
{
    let limit = (name.len() as f64).sqrt().round() as usize;
    let most_similar_name_and_some = candidates
        .min_by_key(|(_, v)| levenshtein(v.borrow(), name, limit).unwrap_or(usize::MAX))?;
    let dist = levenshtein(most_similar_name_and_some.1.borrow(), name, limit);
    if dist.map_or(true, |d| d >= limit) {
        None
    } else {
        Some(most_similar_name_and_some)
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
            get_similar_name(["True", "False", "c"].into_iter(), "Falze"),
            Some("False")
        );
    }
}
