#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum CharScore {
    Absent,
    Correct,
    Present
}

fn score_wordle(guess: &[char], solution: &[char]) -> Vec<CharScore> {
    assert_eq!(guess.len(), solution.len());

    let mut score = vec![CharScore::Absent; solution.len()];
    let mut used = vec![false; solution.len()];

    for r in 0..guess.len() {
        if guess[r] == solution[r] && !used[r] {
            score[r] = CharScore::Correct;
            used[r] = true;
        }
    }

    for n in 0..guess.len() {
        if score[n] != CharScore::Correct {
            let i = guess[n];
            for l in 0..solution.len() {
                let d = solution[l];
                if !used[l] && i == d {
                    score[n] = CharScore::Present;
                    used[l] = true;
                    break;
                }
            }
        }
    }

    score
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
