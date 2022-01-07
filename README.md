# Wordle solver

The idea of this is to find the optimal solution to Wordle puzzles.

The current implementation, for each guess, works out the worst case
number of possible solutions if we receive a particular result. We can
use this to heuristically steer finding optimal solutions.

# Solutions

A word can be uniquely determined in 5 guesses (that is, the fifth
guess is guaranteed to be correct). There are probably lots of words
that can be used as starters to guess successfully in 5 guesses, but I
haven't brute-forced that list. I know "aesir" can be used as a first
guess.

A word cannot be uniquely determind in 3 guesses - an exhaustive
search shows this.

I don't think a word cannot be uniquely determined in 4 guesses, but I
want to optimise enough to make the exhaustive search reasonably
quick.

# Sources

The words in the `words` directory are sourced from the Wordle game
itself, so that we're solving the actual real game. The scoring
algorithm is reverse-engineered from its JS.
