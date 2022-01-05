# Wordle solver

The idea of this is to find the optimal solution to Wordle puzzles.

The current implementation, for each guess, works out the worst case
number of possible solutions if we receive a particular result. We can
use this to heuristically steer finding optimal solutions.

# Sources

The words in the `words` directory are sourced from the Wordle game
itself, so that we're solving the actual real game. The scoring
algorithm is reverse-engineered from its JS.
