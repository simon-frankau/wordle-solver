//
// Wordle solver
//

use std::collections::HashMap;
use std::process;

const WORD_LEN: usize = 5;
const DEPTH: usize = 4;

// Result of a guessed letter, as determined by Wordle
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum CharScore {
    Absent,
    Correct,
    Present,
}

// Compactly encode an arry of CharScores. Assumes the word isn't too long.
fn encode_score(cs: impl Iterator<Item = CharScore>) -> u32 {
    cs.map(|c| c as u32).fold(0, |acc, c| acc * 4 + c)
}

// Return the score for a guess against a specific actual answer, encoded.
fn score_wordle(guess: &[u8], answer: &[u8]) -> u32 {
    assert_eq!(guess.len(), WORD_LEN);
    assert_eq!(answer.len(), WORD_LEN);

    let mut corrects = [false; WORD_LEN];
    let mut used = [false; WORD_LEN];
    for idx in 0..guess.len() {
        if guess[idx] == answer[idx] {
            corrects[idx] = true;
            // Correctly guessed letters are "used up".
            used[idx] = true;
        }
    }

    // Look for the presence of a character in the answer that isn't used,
    // and if it's present use it up and return true. Otherwise false.
    fn check_presence(c: u8, answer: &[u8], used: &mut [bool]) -> bool {
        for (idx, d) in answer.iter().enumerate() {
            if !used[idx] && c == *d {
                used[idx] = true;
                return true;
            }
        }
        false
    }

    encode_score(corrects.iter().zip(guess.iter()).map(|(is_correct, c)| {
        if *is_correct {
            CharScore::Correct
        } else if check_presence(*c, answer, &mut used) {
            CharScore::Present
        } else {
            CharScore::Absent
        }
    }))
}

// Given a guess, bucket the answer list entries by the score they return.
//
// What we'd like to do is have each bucket contain a single entry,
// indicating that the guess has uniquely identified all possibile
// answers. A bucket with more than one entry will require further
// guessing to identify a unique answer.
fn bucket_answers<'a>(guess: &[u8], answers: &[&'a [u8]]) -> Vec<Vec<&'a [u8]>> {
    let mut buckets = HashMap::new();

    for answer in answers.iter() {
        let score = score_wordle(&guess, &answer);
        buckets
            .entry(score)
            .or_insert_with(|| Vec::new())
            .push(*answer);
    }

    let mut v: Vec<_> = buckets.into_iter().map(|(_k, v)| (v.len(), v)).collect();
    v.sort_by(|a, b| b.cmp(a));
    v.into_iter().map(|(_k, v)| v).collect()
}

// Given bucketed answers, find the size of the largest bucket, which is a
// heuristic for the hardest case to solve.
fn worst_bucket_size(buckets: &[Vec<&[u8]>]) -> usize {
    buckets.iter().map(|v| v.len()).max().unwrap()
}

////////////////////////////////////////////////////////////////////////
// Depth-first search solver, biased towards trying best splitters first.
//

// Can we, in the given number of guesses, uniquely identify the
// solution from the given answer list? Guesses should be sorted to
// put best splitters first to make finding answers faster.
fn can_solve(
    num_guesses: usize,
    sorted_guesses: &[&[u8]],
    answers: &[&[u8]]
) -> bool {
    if num_guesses == 1 {
        // With a single guess, can only identify one word.
        answers.len() == 1
    } else {
        sorted_guesses
            .iter()
            .any(|guess| can_solve_with_guess(guess, num_guesses, sorted_guesses, answers))
    }
}

// Can we, in the given number of guesses, uniquely identify the
// solution from the given answer list, starting with the given guess?
fn can_solve_with_guess(
    guess: &[u8],
    num_guesses: usize,
    sorted_guesses: &[&[u8]],
    answers: &[&[u8]]
) -> bool {
    let buckets = bucket_answers(guess, answers);
    let ret = buckets.iter().all(|v| {
        can_solve(num_guesses - 1, sorted_guesses, &v)
    });
    ret
}

////////////////////////////////////////////////////////////////////////
// Top-level copy of solver, with more diagnostic spam
//

fn can_solve_noisy(
    num_guesses: usize,
    sorted_guesses: &[&[u8]],
    answers: &[&[u8]]
) -> bool {
    if num_guesses == 1 {
        // With a single guess, can only identify one word.
        return answers.len() == 1
    }

    for (idx, guess) in sorted_guesses.iter().enumerate() {
        eprintln!("Trying guess {} ({}/{})", String::from_utf8_lossy(guess), idx, sorted_guesses.len());
        if can_solve_with_guess_noisy(guess, num_guesses, sorted_guesses, answers) {
            return true;
        }
    }
    false
}

fn can_solve_with_guess_noisy(
    guess: &[u8],
    num_guesses: usize,
    sorted_guesses: &[&[u8]],
    answers: &[&[u8]]
) -> bool {
    let buckets = bucket_answers(guess, answers);

    for (idx, bucket) in buckets.iter().enumerate() {
        eprint!("    Bucket {}/{} (size {})... ", idx, buckets.len(), bucket.len());
        let soluble = can_solve_wordy(num_guesses - 1, sorted_guesses, &bucket);
        if soluble {
            eprintln!("solved");
        } else {
            eprintln!("insoluble");
            return false;
        }
    }
    true
}

fn can_solve_wordy(
    num_guesses: usize,
    sorted_guesses: &[&[u8]],
    answers: &[&[u8]]
) -> bool {
    if num_guesses == 1 {
        // With a single guess, can only identify one word.
        return answers.len() == 1
    }

    for (idx, guess) in sorted_guesses.iter().enumerate() {
        eprint!(
            " {:5} {:5}/{:5}\x08\x08\x08\x08\x08\x08\x08\x08\x08\x08\x08\x08\x08\x08\x08\x08\x08\x08",
            String::from_utf8_lossy(guess),
            idx,
            sorted_guesses.len());
        if can_solve_with_guess(guess, num_guesses, sorted_guesses, answers) {
            return true;
        }
    }
    false
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

    let answer_strings = std::fs::read_to_string("words/possible_solutions.txt")
        .unwrap()
        .lines()
        .filter(|s| !s.is_empty())
        .map(|s| String::from(s))
        .collect::<Vec<String>>();
    let answer_u8s = answer_strings
        .iter()
        .map(|s| s.as_bytes())
        .collect::<Vec<&[u8]>>();

    let mut worst_cases = guess_u8s
        .iter()
        .map(|guess| {
            let buckets = bucket_answers(guess, &answer_u8s);
            let worst_case = worst_bucket_size(&buckets);
            (worst_case, guess)
        })
        .collect::<Vec<_>>();
    worst_cases.sort();

    for (worst_case, guess) in worst_cases.iter() {
        println!("{}: {}", worst_case, String::from_utf8_lossy(guess));
    }

    // We have guesses sorted from most-determining (i.e. best) to worst,
    // so we should try them in this order.
    let sorted_guesses: Vec<&[u8]> = worst_cases.iter().map(|(_, g)| **g).collect::<Vec<_>>();

    let possible = can_solve_noisy(DEPTH, &sorted_guesses, &answer_u8s);
    if possible {
        println!("Success with {} guesses!", DEPTH);
        process::exit(0);
    }
    println!("Cannot fully determine with {} guesses. Oh well.", DEPTH);
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: CharScore = CharScore::Absent;
    const C: CharScore = CharScore::Correct;
    const P: CharScore = CharScore::Present;

    fn check(guess: &str, answer: &str, score: &[CharScore]) {
        assert_eq!(
            score_wordle(guess.as_bytes(), answer.as_bytes()),
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
