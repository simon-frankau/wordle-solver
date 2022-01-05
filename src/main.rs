//
// Wordle solver
//

use std::collections::HashMap;
use std::process;

const WORD_LEN: usize = 5;
// 3.pow(WORD_LEN) doesn't work as pow not yet available in consts.
const TABLE_SIZE: usize = 3 * 3 * 3 * 3 * 3;

// Result of a guessed letter, as determined by Wordle
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum CharScore {
    Absent,
    Correct,
    Present
}

// Return the score for a guess, encoded.
fn score_wordle(guess: &[u8], solution: &[u8]) -> u32 {
    assert_eq!(guess.len(), WORD_LEN);
    assert_eq!(solution.len(), WORD_LEN);

    let mut corrects = [false; WORD_LEN];
    let mut used = [false; WORD_LEN];
    for idx in 0..guess.len() {
        if guess[idx] == solution[idx] {
            corrects[idx] = true;
            // Correctly guessed letters are "used up".
            used[idx] = true;
        }
    }

    // Look for the presence of a character in the solution that isn't used,
    // and if it's present use it up and return true. Otherwise false.
    fn check_presence(c: u8, solution: &[u8], used: &mut [bool]) -> bool {
        for (idx, d) in solution.iter().enumerate() {
            if !used[idx] && c == *d {
                used[idx] = true;
                return true;
            }
        }
        false
    }

    encode_score(corrects
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
        }))
}

// Compactly encode an arry of CharScores. Assumes the word isn't too long.
fn encode_score(cs: impl Iterator<Item = CharScore>) -> u32 {
   cs.map(|c| c as u32).fold(0, |acc, c| acc * 4 + c)
}

// Given a guess, bucket the solution list entries by the score they return
fn bucket_solutions<'a>(guess: &[u8], solutions: &[&'a [u8]]) -> HashMap<u32, Vec<&'a [u8]>> {
    let mut buckets = HashMap::with_capacity(TABLE_SIZE);

    for sol in solutions.iter() {
        let score = score_wordle(&guess, &sol);
        buckets
            .entry(score)
            .or_insert_with(|| Vec::new())
            .push(*sol);
    }

    buckets
}

// Given bucketed solutions, find the size of the largest bucket, which is a
// heuristic for the hardest case to solve.
fn worst_bucket_size(buckets: &HashMap<u32, Vec<&[u8]>>) -> usize {
    buckets
        .iter()
        .map(|(_k, v)| v.len())
        .max()
        .unwrap()
}

// Can we, given the list of guesses, find a guess that will uniquely
// determine the solution?
fn can_fully_solve(sorted_guesses: &[&[u8]], solutions: &[&[u8]]) -> bool {
    for guess in sorted_guesses.iter() {
        let buckets = bucket_solutions(guess, solutions);
        if buckets.iter().all(|(_k, v)| v.len() <= 1) {
            return true;
        }
    }
    false
}

// Bucket solutions based on first guess, and then try to find a guess
// that solves each bucket. Guesses should be sorted by the most
// effective ones first, to make this more efficient.
fn attempt_second_guess(guess: &[u8], sorted_guesses: &[&[u8]], solutions: &[&[u8]]) -> bool {
    let buckets = bucket_solutions(guess, solutions);

    // Sort the buckets by size - largest buckets are going to be
    // hardest to solve, so try them first to avoid wasting time.
    let mut sorted_buckets = buckets
        .into_iter()
        .map(|(_k, v)| (v.len(), v))
        .collect::<Vec<(usize, Vec<&[u8]>)>>();
    sorted_buckets.sort();

    for (_, bucket_sols) in sorted_buckets.iter() {
        if !can_fully_solve(sorted_guesses, bucket_sols) {
            return false;
        }
    }

    true
}

fn main() {
    let guess_strings = std::fs::read_to_string("words/possible_guesses.txt")
        .unwrap()
        .lines()
        .filter(|s| !s.is_empty())
        .map(|s| String::from(s))
        .collect::<Vec<String>>();
    let guess_u8s = guess_strings
        .iter()
        .map(|s| s.as_bytes())
        .collect::<Vec<&[u8]>>();

    let solution_strings = std::fs::read_to_string("words/possible_solutions.txt")
        .unwrap()
        .lines()
        .filter(|s| !s.is_empty())
        .map(|s| String::from(s))
        .collect::<Vec<String>>();
    let solution_u8s = solution_strings
        .iter()
        .map(|s| s.as_bytes())
        .collect::<Vec<&[u8]>>();

    let mut worst_cases = guess_u8s
        .iter()
        .map(|guess| {
            let buckets = bucket_solutions(guess, &solution_u8s);
            let worst_case = worst_bucket_size(&buckets);
            (worst_case, guess)
        })
        .collect::<Vec<_>>();
    worst_cases.sort();

    for (worst_case, guess) in worst_cases.iter() {
        println!("{}: {}", worst_case, String::from_utf8_lossy(guess));
    }

    if worst_cases[0].0 > 1 {
        println!("Cannot fully determine with one guess")
    } else {
        println!(
            "Can fully determine with guess '{}'",
            String::from_utf8_lossy(worst_cases[0].1)
        );
        process::exit(0);
    }

    // We have guesses sorted from most-determining (i.e. best) to worst,
    // so we should try them in this order.
    let sorted_guesses: Vec<&[u8]> = worst_cases
        .iter()
        .map(|(_, g)| **g)
        .collect::<Vec<_>>();

    for guess in sorted_guesses.iter() {
        println!("Trying '{}' as first guess...", String::from_utf8_lossy(guess));
        if attempt_second_guess(guess, &*sorted_guesses, &solution_u8s) {
            println!(
                "Success! Can solve with two guesses starting with '{}'",
                String::from_utf8_lossy(guess));
            process::exit(0);
        }
    }
    println!("Cannot fully determine with two guesses. Oh well.");
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: CharScore = CharScore::Absent;
    const C: CharScore = CharScore::Correct;
    const P: CharScore = CharScore::Present;

    fn check(guess: &str, solution: &str, score: &[CharScore]) {
        assert_eq!(
            score_wordle(guess.as_bytes(), solution.as_bytes()),
            encode_score(score.iter().cloned())
        );
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
