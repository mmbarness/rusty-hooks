use glob::{glob, GlobError};
use regex::Regex;
use std::io::{self, BufRead, BufReader, Error, Lines};
use std::ops::Add;
use std::{fs::File, path::PathBuf};

pub trait Parser {
    fn linux_regex() -> Regex {
        Regex::new(r"^(/[^/ ]*)+/?$")
    }

    fn lines_iterator(path: PathBuf) -> Result<Lines<BufReader<File>>, Error> {
        let file: File = File::open(path)?;
        let reader = BufReader::new(file);
        reader.lines()
    }

    fn pull_filepaths_from_line(line: String) -> () {
        let possible_filepath_accumulator: Option<String> = None;
        let accumulated_filepaths: Vec<String> = [];
        for char in line.chars() {
            if let Some(filepath_acc) = possible_filepath_accumulator {
                let acc_valid = Self::linux_regex().is_match(filepath_acc);
                if !acc_valid {
                    possible_filepath_accumulator = None;
                    break;
                }
                let new_filepath_acc = filepath_acc.add(char);
                let still_valid = Self::linux_regex().is_match(new_filepath_acc);
            }
        }
    }

    fn glob_it(unvalidated_path: String) -> Result<Vec<PathBuf>, GlobError> {
        let paths = glob(unvalidated_path)?;
    }

    fn search_for_file_paths(string: Result<String, Box<dyn Error>>) -> Vec<Path> {}
}

impl Parser for Script {}
