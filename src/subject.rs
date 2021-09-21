use std::str::FromStr;
use std::fmt::{Write, Formatter};
use std::fmt;
use once_cell::sync::Lazy;
use std::io::{BufReader, BufRead};
use std::fs::File;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Subject {
    inner: u8,
}

impl Subject {
    fn get_usize(self) -> usize {
        self.inner as usize
    }
}

pub struct Subjects {
    info: Vec<SubjectInfo>,
}

impl Subjects {
    pub fn all() -> &'static Subjects {
        static SUBJECTS: Lazy<Subjects> = Lazy::new(|| {
            let file = BufReader::new(File::open("resources/subjects.txt").unwrap());

            let mut info: Vec<SubjectInfo> = Vec::new();

            for (i, line) in file.lines().enumerate() {
                assert!(i <= 255);
                let line = line.unwrap();
                if line.is_empty() { continue }
                info.push(line.parse().unwrap());
            }

            info.sort_by(|a, b| a.abbreviation.cmp(&b.abbreviation));

            Subjects { info }
        });

        &SUBJECTS
    }

    pub fn iter(&self) -> impl Iterator<Item=Subject> {
        (0..self.info.len() as u8).map(|inner| Subject { inner })
    }

    pub fn code_from_abbreviation(&self, abbreviation: &str) -> Option<Subject> {
        self.info.binary_search_by_key(&abbreviation, |i| i.abbreviation.as_str())
            .ok()
            .map(|i| Subject { inner: i as u8 })
    }

    pub fn abbreviation(&self, code: Subject) -> &str {
        &self.info[code.get_usize()].abbreviation
    }

    pub fn name(&self, code: Subject) -> &str {
        &self.info[code.get_usize()].name
    }

    pub fn category(&self, code: Subject) -> SubjectCategory {
        self.info[code.get_usize()].category
    }

    pub fn color(&self, code: Subject) -> &str {
        &self.info[code.get_usize()].color
    }
}

impl fmt::Display for Subject {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(Subjects::all().name(*self))
    }
}

struct SubjectInfo {
    abbreviation: String,
    name: String,
    category: SubjectCategory,
    color: String,
}

impl FromStr for SubjectInfo {
    type Err = ();
    fn from_str(string: &str) -> Result<SubjectInfo, ()> {
        let mut split = string.split(";");
        Ok(SubjectInfo {
            abbreviation: split.next().unwrap().to_string(),
            name: split.next().unwrap().to_string(),
            category: split.next().unwrap().parse()?,
            color: split.next().unwrap().to_string(),
        })
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum SubjectCategory {
    Language,
    Culture,
    AbstractScience,
    PhysicalScience,
    Other, // todo: put more specific categories in subject.dat
}

impl fmt::Display for SubjectCategory {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            SubjectCategory::Language => "Language",
            SubjectCategory::Culture => "Culture",
            SubjectCategory::AbstractScience => "abstract science",
            SubjectCategory::PhysicalScience => "Physical Science",
            SubjectCategory::Other => "Other",
        })
    }
}

impl FromStr for SubjectCategory {
    type Err = ();
    fn from_str(string: &str) -> Result<SubjectCategory, ()> {
        match string {
            "language" => Ok(SubjectCategory::Language),
            "culture" => Ok(SubjectCategory::Culture),
            "abstract science" => Ok(SubjectCategory::AbstractScience),
            "physical science" => Ok(SubjectCategory::PhysicalScience),
            "other" => Ok(SubjectCategory::Other),
            _ => Err(()),
        }
    }
}

// #[derive(Copy, Clone, Debug)]
// pub struct Color {
//     r: u8,
//     g: u8,
//     b: u8,
// }
//
// impl FromStr for Color {
//     type Err = ();
//     fn from_str(string: &str) -> Result<Color, ()> {
//         if string.len() != 6 { return Err(()) }
//         Ok(Color {
//             r: u8::from_str_radix(&string[0..2], 16).map_err(|_| ())?,
//             g: u8::from_str_radix(&string[2..4], 16).map_err(|_| ())?,
//             b: u8::from_str_radix(&string[4..6], 16).map_err(|_| ())?,
//         })
//     }
// }
//
// impl fmt::Display for Color {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "{:02x}{:02x}{:02x}", self.r, self.g, self.b)
//     }
// }