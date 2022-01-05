#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum CharScore {
    Absent,
    Correct,
    Present
}

fn score_wordle(guess: &[char], solution: &[char]) -> Vec<CharScore> {
    assert_eq!(guess.len(), solution.len());

    let corrects = guess
        .iter()
        .zip(solution.iter())
        .map(|(g, s)| g == s)
        .collect::<Vec<_>>();

    // Correct guess letters are "used up".
    let mut used = corrects.clone();

    // Look for the presence of a character in the solution that isn't used,
    // and if it's present use it up and return true. Otherwise false.
    fn check_presence(c: char, solution: &[char], used: &mut [bool]) -> bool {
        for (idx, d) in solution.iter().enumerate() {
            if !used[idx] && c == *d {
                used[idx] = true;
                return true;
            }
        }
        false
    }

    corrects
        .iter()
        .zip(guess.iter())
        .map(|(is_correct, c)| {
            if *is_correct {
                CharScore::Correct
            } else if check_presence(*c, solution, &mut used) {
                CharScore::Present
            } else {
                CharScore::Absent
            }
        })
        .collect::<Vec<_>>()
}

fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: CharScore = CharScore::Absent;
    const C: CharScore = CharScore::Correct;
    const P: CharScore = CharScore::Present;

    fn check(guess: &str, solution: &str, score: &[CharScore]) {
        let guess_vec = guess.chars().collect::<Vec<_>>();
        let solution_vec = solution.chars().collect::<Vec<_>>();
        let generated_score = score_wordle(&guess_vec, &solution_vec);
        assert_eq!(&generated_score, score);
    }

    #[test]
    fn test_simple_green() {
        check("weary", "wills", &[C, A, A, A, A]);
    }

    #[test]
    fn test_simple_yellow() {
        check("pilot", "leaks", &[A, A, P, A, A]);
    }

    #[test]
    fn test_double_yellow() {
        check("kazoo", "tools", &[A, A, A, P, P]);
    }

    #[test]
    fn test_green_overrides_yellow() {
        // Letters are 'used up' by exact matches.
        check("loose", "chore", &[A, A, C, A, C]);
    }

    #[test]
    fn test_yellow_overrides_yellow() {
        // Letters are 'used up' by inexact matches, too.
        // So, only one 'O' matches.
        check("spoon", "coats", &[P, A, P, A, A]);
    }

    #[test]
    fn test_success() {
        check("prize", "prize", &[C, C, C, C, C]);
    }
}
