//
// Wordle solver
//

use std::collections::HashMap;

const WORD_LEN: usize = 5;

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
}

impl Scorer {
    fn new() -> Scorer {
        // Load the strings...
        let mut guesses = std::fs::read_to_string("words/possible_guesses.txt")
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

        // Answers are also possible guesses!
        for answer in answers.iter() {
            guesses.push(answer.clone());
        }

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

        Scorer {
            guesses,
            answers,
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

    // Returns the worst case bucket size for the guess, and the bucketing.
    fn find_greedy_worst_case<'a>(&self, guess: usize, answers: &[usize]) -> (usize, HashMap<u8, Vec<usize>>) {
        let mut buckets = HashMap::new();

        for answer in answers.iter() {
            let score = self.score_cache[guess][*answer];
            buckets
                .entry(score)
                .or_insert_with(|| Vec::new())
                .push(*answer);
        }

        let worst_case = buckets.values().map(|b| b.len()).max().unwrap();
        (worst_case, buckets)
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
// Greedy guesser

// Returns the number of guesses it needed.
fn guess_greedily(s: &Scorer, depth: usize, answers: &[usize], target: usize) -> usize {
    // Final guess?
    if answers.len() == 1 {
        return depth + 1;
    }

    // Try all words, and find the one with the smallest worst case set.
    let ((num_poss, buckets), greedy_guess) = (0..s.guesses.len())
        .map(|guess| (s.find_greedy_worst_case(guess, answers), guess))
        .min_by(|((a, _), ai), ((b, _), bi)| (*a, *ai).cmp(&(*b, *bi)))
        .unwrap();

    eprintln!(" Guessing {}, worst case {} possibilities", s.guesses[greedy_guess], num_poss);

    // And recurse
    let target_score = s.score_cache[greedy_guess][target];
    let target_answers = buckets.get(&target_score).unwrap();
    assert!(target_answers.iter().any(|t| *t == target));

    guess_greedily(s, depth + 1, &target_answers, target)
}

////////////////////////////////////////////////////////////////////////
// Entry point
//

fn main() {
    let mut s = Scorer::new();
    s.optimise_guess_order();

    let answers = (0..s.answers.len()).collect::<Vec<usize>>();

    for answer in 0..s.answers.len() {
        eprintln!("Trying to greedliy solve {}", s.answers[answer]);
        let steps = guess_greedily(&s, 0, &answers, answer);
        eprintln!("Took {} guesses", steps);
    }
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
