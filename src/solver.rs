use crate::server::{self, Server};
use crate::{util, GuessOutcome, Letter, LetterOutcome, Word};
use std::collections::HashSet;

#[derive(Debug)]
pub struct Solver {
    guess_index: usize,
    guess_outcomes: [Option<GuessOutcome>; 6],
    letters_state: [LetterState; 26],
    dictionary: Vec<Word>,
}

impl Solver {
    pub fn new(dict: HashSet<Word>) -> Self {
        // sort words by number of distinct vowels for better picking
        let mut dictionary: Vec<Word> = dict.into_iter().collect();
        dictionary.sort_unstable_by_key(|w| w.distinct_vowels());
        Self {
            guess_index: 0,
            guess_outcomes: [None; 6],
            letters_state: [LetterState::Unknown; 26],
            dictionary,
        }
    }

    pub fn guess<S: Server>(&mut self, server: &mut S) -> Result<(Word, GuessOutcome), Error> {
        // Select a random word still in the dictionary
        let guess = self.dictionary.pop().ok_or(Error::Stumped)?;

        let outcome = server.submit(guess)?;
        self.guess_outcomes[self.guess_index] = Some(outcome);
        self.guess_index += 1;

        // Update knowledge about the letters
        for (i, (x, y)) in guess.iter().zip(outcome.iter()).enumerate() {
            let j = x.index() as usize;
            match y {
                LetterOutcome::Absent => match self.letters_state[j] {
                    LetterState::Unknown => self.letters_state[j] = LetterState::Absent,
                    LetterState::Positions(ref mut ps) => {
                        ps[i] = PositionState::No;
                        // We know Present -> Absent additionally means there is no further
                        // duplicates of that letter, so we'll immediately filter out words
                        // with too many instances of it.
                        let letter_count = guess.count(x);
                        self.dictionary.retain(|w| w.count(x) < letter_count);
                    }
                    // We knew were it was NOT located because of a correct letter, now
                    // we have evidence it might be nowhere at all, which is stronger, so
                    // we'll go with that.
                    LetterState::AntiPositions(_) => self.letters_state[j] = LetterState::Absent,
                    LetterState::Absent => (),
                },
                LetterOutcome::Present => match self.letters_state[j] {
                    LetterState::Unknown => {
                        let mut ps = [PositionState::Maybe; 5];
                        ps[i] = PositionState::No;
                        self.letters_state[j] = LetterState::Positions(ps);
                    }
                    LetterState::Positions(ref mut ps) => {
                        ps[i] = PositionState::No;
                    }
                    LetterState::AntiPositions(ps) => {
                        let mut new_ps = util::map_array(ps, PositionState::not);
                        new_ps[i] = PositionState::No;
                        self.letters_state[j] = LetterState::Positions(new_ps);
                    }
                    // If server is working properly, cannot go from Absent to Present
                    LetterState::Absent => unreachable!(),
                },
                LetterOutcome::Correct => {
                    // current letter is at position i
                    match self.letters_state[j] {
                        LetterState::Unknown => {
                            let mut ps = [PositionState::Maybe; 5];
                            ps[i] = PositionState::Yes;
                            self.letters_state[j] = LetterState::Positions(ps);
                        }
                        LetterState::Positions(ref mut ps) => {
                            ps[i] = PositionState::Yes;
                        }
                        LetterState::AntiPositions(ps) => {
                            let mut new_ps = util::map_array(ps, PositionState::not);
                            new_ps[i] = PositionState::Yes;
                            self.letters_state[j] = LetterState::Positions(new_ps);
                        }
                        // Absent -> Correct is possible if we guessed a word that has the letter
                        // multiple times, but the answer only has that letter once. In our guess
                        // one instance of the letter will be in an incorrect position, and the other in
                        // the correct position. Since there are no duplicates of that letter the former
                        // will be seen as Absent, while the latter as Correct.
                        LetterState::Absent => {
                            let mut ps = [PositionState::No; 5];
                            ps[i] = PositionState::Yes;
                            self.letters_state[j] = LetterState::Positions(ps);
                        }
                    }
                    // all other letters are not at position i
                    for (k, s) in self.letters_state.iter_mut().enumerate() {
                        if k == j {
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
        if self.dictionary.is_empty() && outcome != [LetterOutcome::Correct; 5] {
            return Err(Error::Stumped);
        }

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
            LetterState::AntiPositions(ps) => match ps[i] {
                PositionState::Yes => return false,
                PositionState::Maybe => (),
                PositionState::No => (),
            },
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

        let mut server = server::InMemoryServer::new(word, dict.clone());
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

    #[test]
    fn test_average_guesses() {
        let dict = load_dictionary();
        let mut total: u16 = 0;
        let mut count: u16 = 0;
        let mut fail_count: u16 = 0;
        for word in dict.iter() {
            count += 1;
            let result = run_solver(*word, dict.clone()) as u16;
            total += result;
            if result > 6 {
                fail_count += 1;
            }
        }
        let ratio = f64::from(total) / f64::from(count);
        println!("Average guesses to solve: {}", ratio);
        let ratio = f64::from(fail_count) / f64::from(count);
        println!("Failure rate: {}", ratio);
    }

    #[test]
    #[ignore]
    fn test_interactive_server() {
        let dict = load_dictionary();
        let mut server = server::InteractiveServer;
        let mut solver = solver::Solver::new(dict);

        loop {
            let (_, outcome) = solver.guess(&mut server).unwrap();
            if outcome == [LetterOutcome::Correct; 5] {
                break;
            }
        }
    }

    fn run_solver(word: Word, dict: HashSet<Word>) -> u8 {
        let mut server = server::InMemoryServer::new(word, dict.clone());
        let mut solver = solver::Solver::new(dict);

        let mut guess_counter = 0u8;
        loop {
            match solver.guess(&mut server) {
                Ok((_, outcome)) => {
                    guess_counter += 1;
                    if outcome == [LetterOutcome::Correct; 5] {
                        break;
                    }
                }
                Err(super::Error::Stumped) => panic!("{:?}\n{:?}", server, solver),
                Err(_) => {
                    // println!("{:?}", word);
                    guess_counter = 7;
                    break;
                }
            }
        }
        guess_counter
    }

    fn load_dictionary() -> HashSet<Word> {
        let text = std::fs::read_to_string("./res/words.txt").unwrap();
        text.split('\n').filter_map(Word::try_from_str).collect()
    }
}
