//
// Wordle solver
//

use std::collections::HashMap;
use std::process;

const WORD_LEN: usize = 5;
const DEPTH: usize = 4;

// Bucket can be stored as u8 - 3^5 <= 255.
type BucketId = u8;

////////////////////////////////////////////////////////////////////////
// Core scoring/classification algorithm
//

// Result of a guessed letter, as determined by Wordle
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum CharScore {
    Absent,
    Correct,
    Present,
}

// Compactly encode an arry of CharScores. Assumes the word isn't too long.
fn encode_score(cs: impl Iterator<Item = CharScore>) -> u8 {
    cs.map(|c| c as u8).fold(0, |acc, c| acc * 3 + c)
}

// Return the score for a guess against a specific actual answer, encoded.
fn score_wordle(guess: &[u8], answer: &[u8]) -> u8 {
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

////////////////////////////////////////////////////////////////////////
// The Scorer holds the data and caches scoring information
//

struct Scorer {
    // We load the strings from a file, and then convert to u8 slices for
    // efficiency.
    guess_strings: Vec<String>,
    answer_strings: Vec<String>,
    score_cache: Vec<Vec<u8>>,
}

impl Scorer {
    fn new() -> Scorer {
        // Load the strings...

        let guess_strings = std::fs::read_to_string("words/possible_guesses.txt")
            .unwrap()
            .lines()
            .filter(|s| !s.is_empty())
            .map(|s| String::from(s))
            .collect::<Vec<String>>();
        let guesses = guess_strings
            .iter()
            .map(|s| s.as_bytes())
            .collect::<Vec<&[u8]>>();

        let answer_strings = std::fs::read_to_string("words/possible_solutions.txt")
            .unwrap()
            .lines()
            .take(500) // TODO!
            .filter(|s| !s.is_empty())
            .map(|s| String::from(s))
            .collect::<Vec<String>>();
        let answers = answer_strings
            .iter()
            .map(|s| s.as_bytes())
            .collect::<Vec<&[u8]>>();

        // Score them all up-front.
        let score_cache = guesses
            .iter()
            .map(|g| {
                answers.iter().map(|a| score_wordle(g, a)).collect::<Vec<BucketId>>()
            })
            .collect::<Vec<Vec<BucketId>>>();

        Scorer {
            guess_strings,
            answer_strings,
            score_cache,
        }
    }

    // Given a guess, bucket the answer list entries by the score they return.
    //
    // What we'd like to do is have each bucket contain a single entry,
    // indicating that the guess has uniquely identified all possibile
    // answers. A bucket with more than one entry will require further
    // guessing to identify a unique answer.
    fn bucket_answers<'a>(&self, guess: usize, answers: &[usize]) -> Vec<Vec<usize>> {
        let mut buckets = HashMap::new();

        for answer in answers.iter() {
            let score = self.score_cache[guess][*answer];
            buckets
                .entry(score)
                .or_insert_with(|| Vec::new())
                .push(*answer);
        }

        let mut v: Vec<_> = buckets.into_iter().map(|(_k, v)| (v.len(), v)).collect();
        v.sort_by(|a, b| b.cmp(a));
        v.into_iter().map(|(_k, v)| v).collect()
    }
}

////////////////////////////////////////////////////////////////////////
// Bucketing algorithm
//

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

////////////////////////////////////////////////////////////////////////
// Depth-first search solver, biased towards trying best splitters first.
//

// Can we, in the given number of guesses, uniquely identify the
// solution from the given answer list? Guesses should be sorted to
// put best splitters first to make finding answers faster.
fn can_solve(
    s: &Scorer,
    num_guesses: usize,
    sorted_guesses: &[usize],
    answers: &[&[u8]]
) -> bool {
    if num_guesses == 1 {
        // With a single guess, can only identify one word.
        answers.len() == 1
    } else {
        sorted_guesses
            .iter()
            .any(|guess| can_solve_with_guess(s, *guess, num_guesses, sorted_guesses, answers))
    }
}

// Can we, in the given number of guesses, uniquely identify the
// solution from the given answer list, starting with the given guess?
fn can_solve_with_guess(
    s: &Scorer,
    guess: usize,
    num_guesses: usize,
    sorted_guesses: &[usize],
    answers: &[&[u8]]
) -> bool {
    let buckets = bucket_answers(s.guess_strings[guess].as_bytes(), answers);
    let ret = buckets.iter().all(|v| {
        can_solve(s, num_guesses - 1, sorted_guesses, &v)
    });
    ret
}

////////////////////////////////////////////////////////////////////////
// Top-level copy of solver, with more diagnostic spam
//

fn can_solve_noisy(
    s: &Scorer,
    num_guesses: usize,
    sorted_guesses: &[usize],
    answers: &[&[u8]]
) -> bool {
    if num_guesses == 1 {
        // With a single guess, can only identify one word.
        return answers.len() == 1
    }

    for (idx, guess) in sorted_guesses.iter().enumerate() {
        eprintln!("Trying guess {} ({}/{})", s.guess_strings[*guess], idx, sorted_guesses.len());
        if can_solve_with_guess_noisy(s, guess, num_guesses, sorted_guesses, answers) {
            return true;
        }
    }
    false
}

fn can_solve_with_guess_noisy(
    s: &Scorer,
    guess: &usize,
    num_guesses: usize,
    sorted_guesses: &[usize],
    answers: &[&[u8]]
) -> bool {
    let buckets = bucket_answers(s.guess_strings[*guess].as_bytes(), answers);

    for (idx, bucket) in buckets.iter().enumerate() {
        eprint!("    Bucket {}/{} (size {})... ", idx, buckets.len(), bucket.len());
        let soluble = can_solve_wordy(s, num_guesses - 1, sorted_guesses, &bucket);
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
    s: &Scorer,
    num_guesses: usize,
    sorted_guesses: &[usize],
    answers: &[&[u8]]
) -> bool {
    if num_guesses == 1 {
        // With a single guess, can only identify one word.
        return answers.len() == 1
    }

    for (idx, guess) in sorted_guesses.iter().enumerate() {
        eprint!(
            " {:5} {:5}/{:5}\x08\x08\x08\x08\x08\x08\x08\x08\x08\x08\x08\x08\x08\x08\x08\x08\x08\x08",
            s.guess_strings[*guess],
            idx,
            sorted_guesses.len());
        if can_solve_with_guess(s, *guess, num_guesses, sorted_guesses, answers) {
            return true;
        }
    }
    false
}

////////////////////////////////////////////////////////////////////////
// Entry point
//

fn main() {
    let s = Scorer::new();

    let answers = s.answer_strings
        .iter()
        .map(|s| s.as_bytes())
        .collect::<Vec<&[u8]>>();

    let answer_nums = (0..answers.len()).collect::<Vec<usize>>();
    let mut worst_cases: Vec<(usize, usize)> = (0..s.guess_strings.len())
        .map(|guess| {
            // Bucket the answers by score for this guess.
            let buckets = s.bucket_answers(guess, &answer_nums);
            // Given bucketed answers, find the size of the largest
            // bucket, which is a heuristic for the hardest case to
            // solve.
            let largest_bucket_size = buckets.iter().map(|v| v.len()).max().unwrap();
            (largest_bucket_size, guess)
        })
        .collect();
    worst_cases.sort();

    for (worst_case, guess) in worst_cases.iter() {
        println!("{}: {}", worst_case, s.guess_strings[*guess]);
    }

    // We have guesses sorted from most-determining (i.e. best) to worst,
    // so we should try them in this order.
    let sorted_guesses: Vec<usize> = worst_cases.iter().map(|(_, g)| *g).collect::<Vec<_>>();

    let possible = can_solve_noisy(&s, DEPTH, &sorted_guesses, &answers);
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
