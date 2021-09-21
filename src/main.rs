#![allow(dead_code)]
#![allow(unused_imports)]

#[macro_use]
mod json;
mod request;
mod restrictions;
mod parse_prerequisite_string;
mod subject;
mod graph;
mod data_file_help;
mod normalize;

use regex::{RegexBuilder, Regex};
use once_cell::sync::Lazy;
use crate::json::{Json};
use std::{io, fs};
use std::collections::{HashMap};
use crate::restrictions::{ProgramRestriction, SemesterRestriction, RegistrationRestrictions, SemesterRange, CourseCode, Qualification, PrerequisiteTree, Conjunctive, LevelRestriction};
use std::path::{Path, PathBuf};
use crate::graph::{SubjectGraphs};
use std::io::{Write, Read};
use std::process::{Command, Stdio};
use std::fmt::Write as FmtWrite;

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
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    dotted.stdin.take().unwrap().write_all(dot_string.as_bytes())?;

    let mut svg = String::new();
    dotted.stdout.take().unwrap().read_to_string(&mut svg)?;

    dotted.wait()?;

    eprintln!("Filtering Graphviz output...");
    svg_filter(&mut svg, &restrictions);

    let output_path = output_svg_path();
    eprintln!("Writing new svg to {}", &output_path);
    fs::write(&output_path, svg)
}

fn svg_filter(svg: &mut String, restrictions: &AllRestrictions) {
    // static REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r#"<g id=".*?" class="node qual_(.*?)">.*?points="(.*?),(.*?) .*?</g>"#).unwrap());
    static REGEX: Lazy<Regex> = Lazy::new(|| RegexBuilder::new(r#"<g id="node\d*" class="node qual_(.*?)".*?points="(.*?),(.*?) .*?</g>"#).dot_matches_new_line(true).build().unwrap());

    while let Some(location) = REGEX.captures(&svg) {
        let entire_range = location.get(0).unwrap().range();
        let course_code = location[1].parse().unwrap();
        let top_left_x = location[2].parse::<f32>().unwrap()-102.0;
        let top_left_y = location[3].parse().unwrap();
        let new_text = restrictions.svg(course_code, top_left_x, top_left_y);
        svg.replace_range(entire_range, &new_text);
    }
}

pub struct AllRestrictions {
    map: HashMap<CourseCode, RegistrationRestrictions>,
}

impl AllRestrictions {
    fn new() -> io::Result<AllRestrictions> {
        AllRestrictions::new_from_directory("resources/scraped")
    }

    fn svg(&self, course_code: CourseCode, x: f32, y: f32) -> String {
        let mut ret = String::new();

        writeln!(ret, r#"<rect style="fill:#ffffff;stroke:#000000;stroke-width:3" width="102" height="44" x="{}" y="{}" />"#, x, y).unwrap();
        writeln!(ret, r#"<text x="{}" y="{}" style="font-family:monospace;font-size:16px">{}</text>"#, x+3.5, y+17.0, course_code).unwrap();

        if let Some(c) = self.map.get(&course_code) {
            if c.override_required {

            }
            if c.informal_prerequisite {

            }
            if let Some(s) = c.semester_restriction {
                let (cross, text) = match s {
                    SemesterRestriction::Must(a) => (false, a),
                    SemesterRestriction::CannotBe(a) => (true, a),
                };
                writeln!(ret, r#"<text x="{}" y="{}" style="font-family:monospace;font-size:8px">{}</text>"#, x+20.5, y+30.0, text).unwrap();
                if cross {
                    writeln!(ret, r#"<rect style="fill:#ff0000" width="20" height=1" x="{}" y="{}" />"#, x+21.5, y+25.0).unwrap();
                }
            }
            if let Some(r) = c.level_restrictions {
                let (color, text) = match r {
                    LevelRestriction::Undergraduate => ("c83771", "U"),
                    LevelRestriction::Graduate => ("71c837", "G"),
                };
                writeln!(ret, r#"<circle style="fill:#{};stroke:#000000;stroke-width:0.5" width="102" r="8" cx="{}" cy="{}" />"#, color, x+14.5, y+30.0).unwrap();
                writeln!(ret, r#"<text x="{}" y="{}" style="font-family:monospace;font-size:8px">{}</text>"#, x+16.5, y+30.0, text).unwrap();
            }
            if let Some(c) = c.program_restrictions {
                let (color, text) = match c {
                    ProgramRestriction::StemUndergraduate => ("000000", "stem"),
                    ProgramRestriction::PublicHealthGraduate => ("000000", "phG"),
                    ProgramRestriction::LiteraryArtsGraduate => ("000000", "litG"),
                    ProgramRestriction::BiologyGraduate => ("0000000", "bioG"),
                    ProgramRestriction::EducationGraduate => ("000000", "eduG"),
                };
                writeln!(ret, r#"<text x="{}" y="{}" style="font-family:monospace;font-size:8px">{}</text>"#, x+50.5, y+30.0, text).unwrap();
            }
        }

        ret
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

fn output_svg_path() -> String {
    // choose a file that doesn't exist

    let mut file_name_ext = 0;
    let mut path = format!("output/graph1.svg");

    while fs::metadata(&path).is_ok() {
        file_name_ext += 1;
        path = format!("output/graph{}.svg", file_name_ext);
    }

    path
}