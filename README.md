# Wordle solver

The idea of this is to find the optimal solution to Wordle puzzles.

The current implementation, for each guess, works out the worst case
number of possible solutions if we receive a particular result. We can
use this to heuristically steer finding optimal solutions.

# What's this branch about?

As a result of https://twitter.com/gcapell/status/1480003186836398080
, I'm looking at how many steps the obvious greedy algorithm takes to
guess, in the worst case.

# Solutions

A word can be uniquely determined in 5 guesses (that is, the fifth
guess is guaranteed to be correct). There are probably lots of words
that can be used as starters to guess successfully in 5 guesses, but I
haven't brute-forced that list. I know "aesir" can be used as a first
guess.

A word cannot be uniquely determind in 4 guesses - an exhaustive
search shows this. This search took circa 11 hours on the 4 cores of
my early 2015 MacBook Air, demonstrating the trade-off between ancient
hardware, time optimising code, and waiting for a computation to
complete.

It's much faster to show that some word can be used as a starting
point for a 5-guess solution than to show no 4-guess solution exists,
because the former allows a heuristic to find an actual answer
quickly, while the latter requires an exhaustive search to show no
solution exists.

# Next steps

Potential follow-up projects:

 * Find the best starting words, for some metric, like "fraction of
   words that can be solved by guess number 4".
 * Build an actually useful solver, rather than something that just
   says "a solution exists". It would take your guesses and the
   replies, and list potential solutions and suggested guesses.
 * Write up the algorithm in a bit more detail in this README.md.

# Sources

The words in the `words` directory are sourced from the Wordle game
itself, so that we're solving the actual real game. The scoring
algorithm is reverse-engineered from its JS.
