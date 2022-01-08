//
// Wordle solver
//

use std::collections::HashMap;
use std::process;

const WORD_LEN: usize = 5;
const DEPTH: usize = 4;

const MAX_BUCKET: usize = 3 * 3 * 3 * 3 * 3;

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
    // Once the scores are precalculated, we refer to everything by indices.
    guesses: Vec<String>,
    answers: Vec<String>,
    score_cache: Vec<Vec<u8>>,

    // Awkward place to put reused vector.
    bucket_vec: Vec<Vec<usize>>,
}

impl Scorer {
    fn new() -> Scorer {
        // Load the strings...
        let guesses = std::fs::read_to_string("words/possible_guesses.txt")
            .unwrap()
            .lines()
            .filter(|s| !s.is_empty())
            .map(|s| String::from(s))
            .collect::<Vec<String>>();
        let answers = std::fs::read_to_string("words/possible_solutions.txt")
            .unwrap()
            .lines()
            .filter(|s| !s.is_empty())
            .map(|s| String::from(s))
            .collect::<Vec<String>>();

        // Score them all up-front.
        let score_cache = guesses
            .iter()
            .map(|g| {
                let gbs = g.as_bytes();
                answers
                    .iter()
                    .map(|a| {
                        let abs = a.as_bytes();
                        score_wordle(gbs, abs)
                    })
                    .collect::<Vec<BucketId>>()
            })
            .collect::<Vec<Vec<BucketId>>>();

        let bucket_vec = (0..MAX_BUCKET).map(|_| Vec::new()).collect::<Vec<_>>();

        Scorer {
            guesses,
            answers,
            score_cache,
            bucket_vec
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

    // Version of bucket_answers used for the 3-guess case.
    fn bucket_answers3<'a>(&mut self, guess: usize, answers: &[usize]) {
        for bucket in self.bucket_vec.iter_mut() {
            bucket.clear();
        }

        for answer in answers.iter() {
            let score = self.score_cache[guess][*answer];
            self.bucket_vec[score as usize].push(*answer);
        }

        self.bucket_vec.sort_by(|a, b| b.len().cmp(&a.len()));
    }

    // Optimise the order in which guesses are made, so that those
    // that minimise the largest bucket come first.
    fn optimise_guess_order(&mut self)  {
        let answer_nums = self
            .answers
            .iter()
            .enumerate()
            .map(|(idx, _)| idx)
            .collect::<Vec<usize>>();

        let mut worst_cases: Vec<(usize, usize, String)> = self.guesses
            .iter()
            .enumerate()
            .map(|(idx, guess)| {
                // Bucket the answers by score for this guess.
                let buckets = self.bucket_answers(idx, &answer_nums);
                // Given bucketed answers, find the size of the largest
                // bucket, which is a heuristic for the hardest case to
                // solve.
                let largest_bucket_size = buckets.iter().map(|v| v.len()).max().unwrap();
                (largest_bucket_size, idx, guess.clone())
            })
            .collect();
        worst_cases.sort();

        for (worst_case, _idx, guess) in worst_cases.iter() {
            println!("{}: {}", worst_case, guess);
        }

        // Sort the guess list and the score cache to match the
        // improved search order.
        self.guesses = worst_cases.iter().map(|(_, _, g)| g.clone()).collect();
        self.score_cache = worst_cases
            .iter()
            .map(|(_, idx, _)| self.score_cache[*idx].clone())
            .collect();
    }
}

////////////////////////////////////////////////////////////////////////
// Depth-first search solver, biased towards trying best splitters first.
//

// Specialise last layers of search as an optimisation.

// Allocated once to optimise leaf case.
pub static mut SEEN_TABLE: &'static mut [u8] = &mut [0; MAX_BUCKET];
pub static mut COUNTER: u8 = 0;

// Can we solve with 2 guesses? 2nd guess must be correct answer, which
// means all we need to do is check that the first guess full determines
// - there can be at most one possible solution per bucket.
fn can_solve2(s: &Scorer, answers: &[usize]) -> bool {
    (0..s.guesses.len()).any(|guess| can_solve_with_guess2(s, guess, answers))
}

fn can_solve_with_guess2(
    s: &Scorer,
    guess: usize,
    answers: &[usize]
) -> bool {
    unsafe {
        // Special case - next guess has to be final, so check if each bucket
        // contains at most one entry.
        //
        // Set counter to a value not seen in the array.
        if COUNTER == u8::MAX {
            COUNTER = 0;
            for entry in SEEN_TABLE.iter_mut() {
                *entry = 255;
            }
        } else {
            COUNTER += 1;
        }

        // Iterate over the answers, early-outing if a bucket is used twice.
        for answer in answers.iter() {
            let score = s.score_cache[guess][*answer];
            if SEEN_TABLE[score as usize] == COUNTER {
                return false;
            }
            SEEN_TABLE[score as usize] = COUNTER;
        }
        true
    }
}

fn can_solve3(
    s: &mut Scorer,
    answers: &[usize]
) -> bool {
    (0..s.guesses.len())
        .any(|guess| {
            s.bucket_answers3(guess, answers);
            s.bucket_vec.iter().all(|v| {
                v.is_empty() || can_solve2(s, &v)
            })
        })
}

// General solver
//
// Can we, in the given number of guesses, uniquely identify the
// solution from the given answer list? Guesses should be sorted to
// put best splitters first to make finding answers faster.
fn can_solve(
    s: &mut Scorer,
    num_guesses: usize,
    answers: &[usize]
) -> bool {
    if num_guesses == 3 {
        return can_solve3(s, answers)
    } else if num_guesses == 2 {
        return can_solve2(s, answers)
    }

    for idx in 0..s.guesses.len() {
        eprint!(
            " {:5} {:5}/{:5}\x08\x08\x08\x08\x08\x08\x08\x08\x08\x08\x08\x08\x08\x08\x08\x08\x08\x08",
            s.guesses[idx],
            idx,
            s.guesses.len());

            let buckets = s.bucket_answers(idx, answers);
        if buckets.iter().all(|v| { can_solve(s, num_guesses - 1, &v) }) {
            return true;
        }
    }
    false
}

////////////////////////////////////////////////////////////////////////
// Top-level copy of solver, with more diagnostic spam
//

fn can_solve_noisy(
    s: &mut Scorer,
    num_guesses: usize,
    answers: &[usize]
) -> bool {
    for idx in 0..10 { // TODO s.guesses.len() {
        eprintln!("Trying guess {} ({}/{})", s.guesses[idx], idx, s.guesses.len());
        if can_solve_with_guess_noisy(s, idx, num_guesses, answers) {
            return true;
        }
    }
    false
}

fn can_solve_with_guess_noisy(
    s: &mut Scorer,
    guess: usize,
    num_guesses: usize,
    answers: &[usize]
) -> bool {
    let buckets = s.bucket_answers(guess, answers);

    for (idx, bucket) in buckets.iter().enumerate() {
        eprint!("    Bucket {}/{} (size {})... ", idx, buckets.len(), bucket.len());
        assert_eq!(num_guesses - 1, 3);
        let soluble = can_solve(s, num_guesses - 1, &bucket);
        if soluble {
            eprintln!("solved");
        } else {
            eprintln!("insoluble");
            return false;
        }
    }
    true
}

////////////////////////////////////////////////////////////////////////
// Entry point
//

fn main() {
    let mut s = Scorer::new();
    s.optimise_guess_order();

    let answer_idxs = s
        .answers
        .iter()
        .enumerate()
        .map(|(idx, _)| idx)
        .collect::<Vec<usize>>();

    let possible = can_solve_noisy(&mut s, DEPTH, &answer_idxs);
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
