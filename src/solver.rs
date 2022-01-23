use crate::server::{self, Server};
use crate::{util, GuessOutcome, LetterOutcome, Word, Letter};
use rand::seq::IteratorRandom;
use std::collections::HashSet;

pub struct Solver {
    guess_index: usize,
    guess_outcomes: [Option<GuessOutcome>; 6],
    letters_state: [LetterState; 26],
    dictionary: HashSet<Word>,
}

impl Solver {
    pub fn new(dictionary: HashSet<Word>) -> Self {
        Self {
            guess_index: 0,
            guess_outcomes: [None; 6],
            letters_state: [LetterState::Unknown; 26],
            dictionary,
        }
    }

    pub fn guess(&mut self, server: &mut Server) -> Result<(Word, GuessOutcome), Error> {
        // Select a random word still in the dictionary
        let mut rng = rand::thread_rng();
        let guess = *self
            .dictionary
            .iter()
            .choose(&mut rng)
            .ok_or(Error::Stumped)?;
        self.dictionary.remove(&guess);

        let outcome = server.submit(guess)?;
        self.guess_outcomes[self.guess_index] = Some(outcome);
        self.guess_index += 1;

        // Update knowledge about the letters
        for (i, (x, y)) in guess.iter().zip(outcome.iter()).enumerate() {
            let j = x.index();
            match y {
                LetterOutcome::Absent => self.letters_state[j as usize] = LetterState::Absent,
                LetterOutcome::Present => match self.letters_state[j as usize] {
                    LetterState::Unknown => {
                        let mut ps = [PositionState::Maybe; 5];
                        ps[i] = PositionState::No;
                        self.letters_state[j as usize] = LetterState::Positions(ps);
                    }
                    LetterState::Positions(ref mut ps) => {
                        ps[i] = PositionState::No;
                    }
                    LetterState::AntiPositions(ps) => {
                        let mut new_ps = util::map_array(ps, PositionState::not);
                        new_ps[i] = PositionState::No;
                        self.letters_state[j as usize] = LetterState::Positions(new_ps);
                    }
                    // If server is working properly, cannot go from Absent to Present
                    LetterState::Absent => unreachable!(),
                },
                LetterOutcome::Correct => {
                    // current letter is at position i
                    match self.letters_state[j as usize] {
                        LetterState::Unknown => {
                            let mut ps = [PositionState::Maybe; 5];
                            ps[i] = PositionState::Yes;
                            self.letters_state[j as usize] = LetterState::Positions(ps);
                        }
                        LetterState::Positions(ref mut ps) => {
                            ps[i] = PositionState::Yes;
                        }
                        LetterState::AntiPositions(ps) => {
                            let mut new_ps = util::map_array(ps, PositionState::not);
                            new_ps[i] = PositionState::Yes;
                            self.letters_state[j as usize] = LetterState::Positions(new_ps);
                        }
                        // If server is working properly, cannot go from Absent to Correct
                        LetterState::Absent => unreachable!(),
                    }
                    // all other letters are not at position i
                    for (k, s) in self.letters_state.iter_mut().enumerate() {
                        if k == j.into() {
                            continue;
                        }
                        match s {
                            LetterState::Absent => (), // nothing to change
                            LetterState::Unknown => {
                                let mut ps = [PositionState::Maybe; 5];
                                ps[i] = PositionState::Yes;
                                *s = LetterState::AntiPositions(ps);
                            }
                            LetterState::AntiPositions(ps) => {
                                ps[i] = PositionState::Yes;
                            }
                            LetterState::Positions(ps) => {
                                ps[i] = PositionState::No;
                            }
                        }
                    }
                }
            }
        }

        // Filter dictionary based on information
        let state = &self.letters_state;
        self.dictionary.retain(|w| satisfies(w, state));

        Ok((guess, outcome))
    }
}

fn satisfies(word: &Word, state: &[LetterState; 26]) -> bool {
    for (i, l) in word.iter().enumerate() {
        let j = l.index();
        match state[j as usize] {
            LetterState::Absent => return false,
            LetterState::Unknown => (), // not sure
            LetterState::Positions(ps) => {
                match ps[i] {
                    PositionState::Yes => (),   // definitely right
                    PositionState::Maybe => (), // not sure
                    PositionState::No => return false,
                }
            }
            LetterState::AntiPositions(ps) => {
                match ps[i] {
                    PositionState::Yes => return false,
                    PositionState::Maybe => (),
                    PositionState::No => (),
                }
            }
        }
    }
    // Check everything that is present is in the candidate word
    for (l, s) in Letter::LETTERS.iter().zip(state.iter()) {
        if let LetterState::Positions(_) = s {
            if !word.contains(l) {
                return false;
            }
        }
    }
    true
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum LetterState {
    /// No information yet
    Unknown,
    /// Definitely know the letter is in at least some positions
    Positions([PositionState; 5]),
    /// Definitely know the letter is not in some positions
    AntiPositions([PositionState; 5]),
    /// Definitely know the letter is not in the word at all
    Absent,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PositionState {
    Yes,
    Maybe,
    No,
}

impl PositionState {
    pub fn not(self) -> Self {
        match self {
            Self::Yes => Self::No,
            Self::No => Self::Yes,
            Self::Maybe => Self::Maybe,
        }
    }
}

impl Default for PositionState {
    fn default() -> Self {
        Self::Maybe
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Stumped,
    Server(server::Error),
}

impl From<server::Error> for Error {
    fn from(e: server::Error) -> Self {
        Self::Server(e)
    }
}

#[cfg(test)]
mod tests {
    use crate::{server, solver, LetterOutcome, Word};
    use rand::seq::IteratorRandom;
    use std::collections::HashSet;

    #[test]
    fn test_solver() {
        let dict = load_dictionary();
        let word = *dict.iter().choose(&mut rand::thread_rng()).unwrap();

        let mut server = server::Server::new(word, dict.clone());
        let mut solver = solver::Solver::new(dict);

        println!("Answer: {:?}", word);
        loop {
            let (guess, outcome) = solver.guess(&mut server).unwrap();
            println!("{:?} {:?}", guess, outcome);
            if outcome == [LetterOutcome::Correct; 5] {
                break;
            }
        }
    }

    fn load_dictionary() -> HashSet<Word> {
        let text = std::fs::read_to_string("./res/words.txt").unwrap();
        text.split('\n').filter_map(Word::try_from_str).collect()
    }
}
