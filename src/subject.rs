use std::collections::HashMap;
use std::str::FromStr;
use std::fmt::{Write, Formatter};
use std::fmt;
use once_cell::sync::Lazy;
use std::io::{BufReader, BufRead};
use std::fs::File;
use std::convert::Infallible;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Subject {
    inner: String,
}

impl FromStr for Subject {
    type Err = Infallible;
    fn from_str(string: &str) -> Result<Self, Self::Err> {
        Ok(Self { inner: string.to_string() })
    }
}

impl fmt::Display for Subject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.inner)
    }
}

pub struct Subjects {
    info: HashMap<Subject, SubjectInfo>,
}

impl Subjects {
    pub fn all() -> &'static Subjects {
        static SUBJECTS: Lazy<Subjects> = Lazy::new(|| {
            let file = BufReader::new(File::open("resources/subjects.txt").unwrap());

            let info = file.lines()
                .filter_map(Result::ok)
                .filter(|line| !line.is_empty())
                .map(|line| {
                    let mut split = line.split(";");
                    let inner = split.next().unwrap().to_string();
                    let info = SubjectInfo {
                        name: split.next().unwrap().to_string(),
                        category: split.next().unwrap().parse().unwrap(),
                        color: split.next().unwrap().to_string(),
                    };
                    (Subject { inner }, info)
                })
                .collect();

            Subjects { info }
        });

        &SUBJECTS
    }

    pub fn iter(&self) -> impl Iterator<Item=&'_ Subject> + '_ {
        self.info.keys()
    }

    pub fn name(&self, code: &Subject) -> &str {
        &self.info[&code].name
    }

    pub fn category(&self, code: &Subject) -> SubjectCategory {
        self.info[&code].category
    }

    pub fn color(&self, code: &Subject) -> &str {
        &self.info[&code].color
    }
}

struct SubjectInfo {
    name: String,
    category: SubjectCategory,
    color: String,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum SubjectCategory {
    Language,
    Culture,
    AbstractScience,
    PhysicalScience,
    Other,  // todo: put more specific categories in subject.dat
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
//
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
