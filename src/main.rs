#![allow(unused_imports)]
#![allow(dead_code)]

#[macro_use]
mod json;
mod request;
mod restrictions;
mod parse_prerequisite_string;
mod subject;
mod draw;
mod graph;
mod data_file_help;
mod normalize;

use crate::json::{Json, JsonString};
use std::{fmt, io, fs};
use regex::Regex;
use once_cell::sync::Lazy;
use std::str::FromStr;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fmt::Formatter;
use crate::restrictions::{RegistrationRestrictions, SemesterRange, CourseCode, Qualification, PrerequisiteTree, Conjunctive};
use crate::subject::{Subject, Subjects};
use rand::{thread_rng, Rng};
use std::path::{PathBuf, Path};
use crate::graph::{SubjectGraphs};
use std::io::{Write, Read};
use std::num::NonZeroU8;
use std::ops::Index;
use std::process::{Command, Stdio};
use std::fs::File;

// FIX ITALIAN, FIX APMA 1160, FIX FREN 0400, fix HISP 0200 HISP 0300 not recognize cab
// latn 200 0100 and 110 have two any's

// INFORMAL PREREQUISITE - its a prerequisite that cannot be represented with the tree structure.
// Examples of informal prerequisites include auditions, demonstrated experience in the field, some programming experience, a specific level of knowledge in a foreign language, etc.
// Check the course description or contact the instructor for more information.

// Courses listed as prerequisites may be required, recommended or included in error.

// The dependencies in this graphic are based on, but not the same as, the ones listed in Courses@Brown. I corrected mistakes, made many myself, and simplified the graphic.
// The course faculty are the definitive source of information for the course you're interested in.

// This visualization does not support circular dependencies. These are occasionally appropriate, and used by courses like VISA 1510. There are just not shown here.

fn main() -> io::Result<()> {
    let restrictions = AllRestrictions::new()?;
    let graph = SubjectGraphs::new(&restrictions);

    let dot_string = graph.graphviz();

    eprintln!("Running Graphviz...");

    let mut dotted = Command::new("dot")
        .arg("-Tsvg")
        .arg("/dev/stdin")
        .arg("-o")
        .arg("output/graph14.svg")
        .stdin(Stdio::piped())
        .spawn()?;

    dotted.stdin.take().unwrap().write_all(dot_string.as_bytes())?;

    Ok(())
}

pub struct AllRestrictions {
    map: HashMap<CourseCode, RegistrationRestrictions>,
}

impl AllRestrictions {
    fn new() -> io::Result<AllRestrictions> {
        AllRestrictions::new_from_directory("resources/scraped")
    }

    fn new_from_directory<P: AsRef<Path>>(path: P) -> io::Result<AllRestrictions> {
        let map = fs::read_dir(path)?
            .map(|course| {
                let course = course?.path();
                let course_code: CourseCode = course.file_stem().unwrap().to_str().unwrap().parse().unwrap();

                let restriction = fs::read_dir(course)?
                    .map(|variant| {
                        let path = variant?.path();
                        let json: Json = fs::read_to_string(path)?.parse().unwrap();
                        Ok(RegistrationRestrictions::from_json(course_code, &json))
                    })
                    .sum::<io::Result<RegistrationRestrictions>>()?
                    .normalize();

                Ok((course_code, restriction))
            })
            .collect::<io::Result<_>>()?;

        Ok(AllRestrictions { map })
    }

    fn iter(&self) -> impl Iterator<Item=(CourseCode, &RegistrationRestrictions)> {
        self.map.iter().map(|(&k, v)| ((k, v)))
    }
}
