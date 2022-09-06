use serde::de::Error;
use serde::Deserializer;
use serde::de::Visitor;
use serde::de::MapAccess;
use serde::ser::{Serializer, SerializeSeq, SerializeMap};
use serde::Serialize;
use serde::ser;
use std::convert::Infallible;
use once_cell::sync::Lazy;
use serde::{Deserialize};
use regex::Regex;
use std::fmt;
use std::collections::{HashMap, HashSet};
use std::fmt::{Formatter, Write};
use std::str::FromStr;
use std::io::{BufReader, BufRead};
use std::fs::File;
use crate::subject::{Subject, Subjects};
use crate::normalize::normalize;
use std::iter::Sum;
use std::ops::Add;
use serde_json::Value;

#[derive(Debug, Default, Clone)]
pub struct RegistrationRestrictions {
    pub prerequisite_restrictions: Option<PrerequisiteTree>,
    pub program_restrictions: Option<ProgramRestriction>,
    pub level_restrictions: Option<LevelRestriction>,
    pub semester_restriction: Option<SemesterRestriction>,
    pub override_required: bool,
    pub informal_prerequisite: bool,
}

impl RegistrationRestrictions {
    pub fn from_json(course: &CourseCode, root: &Value) -> RegistrationRestrictions {
        match root["registration_restrictions"].as_str() {
            Some(restrictions) => RegistrationRestrictions {
                prerequisite_restrictions: prerequisite_tree_from_correction(course)
                    .or_else(|| PrerequisiteTree::from_restrictions_string(restrictions)),
                program_restrictions: ProgramRestriction::from_restrictions_string(restrictions),
                level_restrictions: LevelRestriction::from_restrictions_string(restrictions),
                semester_restriction: SemesterRestriction::from_restrictions_string(restrictions),
                override_required: override_required(course, root),
                informal_prerequisite: informal_prerequisite(course),
            },
            None => RegistrationRestrictions {
                prerequisite_restrictions: prerequisite_tree_from_correction(course),
                program_restrictions: None,
                level_restrictions: None,
                semester_restriction: None,
                override_required: override_required(course, root),
                informal_prerequisite: informal_prerequisite(course),
            },
        }
    }

    pub fn normalize(mut self) -> RegistrationRestrictions {
        self.prerequisite_restrictions = self.prerequisite_restrictions.map(normalize);
        self
    }
}

impl Add for RegistrationRestrictions {
    type Output = RegistrationRestrictions;
    /// Combines the two to get the most restrictive
    fn add(self, other: RegistrationRestrictions) -> RegistrationRestrictions {
        RegistrationRestrictions {
            program_restrictions: self.program_restrictions.or(other.program_restrictions),
            level_restrictions: self.level_restrictions.or(other.level_restrictions),
            semester_restriction: match self.semester_restriction {
                Some(s) => match other.semester_restriction {
                    Some(o) => s.intersection(o),
                    None => Some(s),
                },
                None => other.semester_restriction,
            },
            override_required: self.override_required | other.override_required,
            prerequisite_restrictions: self.prerequisite_restrictions.or(other.prerequisite_restrictions),
            informal_prerequisite: self.informal_prerequisite | other.informal_prerequisite,
        }
    }
}

impl Sum for RegistrationRestrictions {
    fn sum<I: Iterator<Item=RegistrationRestrictions>>(iter: I) -> RegistrationRestrictions {
        iter.fold(Default::default(), Add::add)
    }
}

fn override_required(course_code: &CourseCode, root: &Value) -> bool {
    static OVERRIDE_CORRECTIONS: Lazy<HashSet<CourseCode>> = Lazy::new(|| {
        let file = BufReader::new(File::open("resources/override_corrections.txt").unwrap());
        file.lines()
            .filter_map(|line| line.ok())
            .filter(|line| !line.is_empty())
            .map(|line| line.as_str().try_into().unwrap())
            .collect()
    });

    if OVERRIDE_CORRECTIONS.contains(course_code) { return true }

    match root["permreq"] {
        Value::Null => false,
        Value::String(ref s) if s == "N" => false,
        Value::String(ref s) if s == "Y" => true,
        ref e => panic!("{:?}", e),
    }
}

fn informal_prerequisite(course_code: &CourseCode) -> bool {
    static INFORMAL_PREREQUISITES: Lazy<HashSet<CourseCode>> = Lazy::new(|| {
        let file = BufReader::new(File::open("resources/informal.txt").unwrap());
        file.lines()
            .filter_map(|line| line.ok())
            .filter(|line| !line.is_empty())
            .map(|line| line.as_str().try_into().unwrap())
            .collect()
    });

    INFORMAL_PREREQUISITES.contains(course_code)
}

#[derive(Debug, Clone, Copy)]
pub enum SemesterRestriction {
    Must(SemesterRange),
    CannotBe(SemesterRange),
}

impl SemesterRestriction {
    fn from_restrictions_string(restrictions: &str) -> Option<SemesterRestriction> {
        static SEMESTER_RESTRICTIONS: Lazy<Regex> = Lazy::new(|| Regex::new(r#"<p class="cls">(Enrollment limited to students with a semester level of (?P<must>.*?)\.)|(Students with a semester level of (?P<cannot>.*?) may <strong>not</strong> enroll\.)</p>"#).unwrap());
        SEMESTER_RESTRICTIONS.captures(restrictions).map(|captures| {
            if let Some(list) = captures.name("must") {
                SemesterRestriction::Must(list.as_str().parse().unwrap())
            } else if let Some(list) = captures.name("cannot") {
                SemesterRestriction::CannotBe(list.as_str().parse().unwrap())
            } else {
                unreachable!()
            }
        })
    }

    fn intersection(self, other: SemesterRestriction) -> Option<SemesterRestriction> {
        match (self, other) {
            (SemesterRestriction::Must(s), SemesterRestriction::Must(o)) =>
                Some(SemesterRestriction::Must(s.intersection(o)?)),
            (SemesterRestriction::CannotBe(s), SemesterRestriction::CannotBe(o)) =>
                Some(SemesterRestriction::CannotBe(s.intersection(o)?)),
            _ => None,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct SemesterRange {
    pub start: SemesterLevel,
    pub end: SemesterLevel,
}

impl SemesterRange {
    // returns the most restrictive of the two
    fn intersection(self, other: SemesterRange) -> Option<SemesterRange> {
        let start = self.start.max(other.start);
        let end = self.end.min(other.end);
        if start < end { Some(SemesterRange { start, end }) } else { None }
    }
}

impl FromStr for SemesterRange {
    type Err = ();
    fn from_str(string: &str) -> Result<SemesterRange, ()> {
        static DELIMITER: Lazy<Regex> = Lazy::new(|| Regex::new(", | or ").unwrap());

        let levels = DELIMITER.split(string)
            .map(SemesterLevel::from_str)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert!(levels.windows(2).all(|a| a[0].precedes(a[1])), "{}", string);

        Ok(SemesterRange { start: *levels.first().unwrap(), end: *levels.last().unwrap() })
    }
}

impl fmt::Display for SemesterRange {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}-{}", self.start, self.end)
    }
}

#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Ord, Eq)]
pub struct SemesterLevel {
    level: u8, // 1 to 15: 14 = GM, 15 = GP
}

impl SemesterLevel {
    fn precedes(self, other: SemesterLevel) -> bool {
        self.level + 1 == other.level
    }
}

impl FromStr for SemesterLevel {
    type Err = ();

    fn from_str(string: &str) -> Result<SemesterLevel, ()> {
        match string {
            "GM" => Ok(SemesterLevel { level: 14 }),
            "GP" => Ok(SemesterLevel { level: 15 }),
            _ => {
                let level = string.parse().map_err(|_| ())?;
                if 0 < level && level < 14 { Ok(SemesterLevel { level }) } else { Err(()) }
            }
        }
    }
}

impl fmt::Display for SemesterLevel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.level {
            1..=13 => write!(f, "{:02}", self.level),
            14 => f.write_str("GM"),
            15 => f.write_str("GP"),
            _ => unreachable!(),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum ProgramRestriction {
    BiologyGraduate,
    PublicHealthGraduate,
    StemUndergraduate,
    EducationGraduate,
    LiteraryArtsGraduate,
}

impl ProgramRestriction {
    fn from_restrictions_string(restrictions: &str) -> Option<ProgramRestriction> {
        static PROGRAM_RESTRICTIONS_LIST: Lazy<Regex> = Lazy::new(|| Regex::new(r#"<p class="prg">Enrollment limited to students in the following programs:<ul>(.*?)</ul></p>"#).unwrap());
        static PROGRAM_RESTRICTIONS_SINGLE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"<p class="prg">Enrollment limited to students in the (.*?) program.</p>"#).unwrap());
        static BUCKETS: Lazy<HashMap<String, ProgramRestriction>> = Lazy::new(|| {
            let file = BufReader::new(File::open("resources/program_buckets.txt").unwrap());
            let mut buckets: HashMap<String, ProgramRestriction> = HashMap::new();

            for line in file.lines() {
                let line = line.unwrap();
                if line.is_empty() { continue }

                let mut line = line.split(": ");
                let bucket_name = line.next().unwrap();
                let bucket = match bucket_name {
                    "education graduate" => ProgramRestriction::EducationGraduate,
                    "literary arts graduate" => ProgramRestriction::LiteraryArtsGraduate,
                    "biology graduate" => ProgramRestriction::BiologyGraduate,
                    "public health graduate" => ProgramRestriction::PublicHealthGraduate,
                    "stem undergraduate" => ProgramRestriction::StemUndergraduate,
                    _ => panic!("{}", bucket_name),
                };

                buckets.extend({
                    line.next().unwrap().split(",")
                        .map(|bucket_content| (bucket_content.to_string(), bucket))
                });
            }

            buckets
        });

        if let Some(cap) = PROGRAM_RESTRICTIONS_LIST.captures(restrictions) {
            let program_list = &cap[1];
            let program = program_list.split_terminator("</li>")
                .map(|program| program.trim_start_matches("<li>"))
                .next()
                .unwrap();
            BUCKETS.get(program).copied()
        } else if let Some(cap) = PROGRAM_RESTRICTIONS_SINGLE.captures(restrictions) {
            BUCKETS.get(&cap[1]).copied()
        } else {
            None
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum LevelRestriction {
    Graduate,
    Undergraduate,
}

impl LevelRestriction {
    fn from_restrictions_string(restrictions: &str) -> Option<LevelRestriction> {
        static LEVEL_RESTRICTIONS: Lazy<Regex> = Lazy::new(|| Regex::new(r#"<p class="lvl">Enrollment is limited to (.*?) level students.</p>"#).unwrap());
        LEVEL_RESTRICTIONS.captures(restrictions).map(|cap| match &cap[1] {
            "Graduate" => LevelRestriction::Graduate,
            "Undergraduate" => LevelRestriction::Undergraduate,
            _ => panic!("{}", &cap[1]),
        })
    }
}

fn prerequisite_tree_from_correction(course_code: &CourseCode) -> Option<PrerequisiteTree> {
    static PREREQUISITE_CORRECTIONS: Lazy<HashMap<CourseCode, PrerequisiteTree>> = Lazy::new(|| {
        let file = BufReader::new(File::open("resources/prerequisite_corrections.txt").unwrap());

        let mut ret = HashMap::new();

        for line in file.lines() {
            let line = line.unwrap();
            if line.is_empty() { continue }
            let mut columns = line.split(";");
            let course: CourseCode = columns.next().unwrap().try_into().unwrap();
            let tree = columns.next().unwrap().try_into().unwrap();
            ret.insert(course, tree);
        }

        ret
    });

    PREREQUISITE_CORRECTIONS.get(course_code).cloned()
}

#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd)]
pub enum PrerequisiteTree {
    Qualification(Qualification),
    Conjunctive(Conjunctive, Vec<PrerequisiteTree>),
}

impl PrerequisiteTree {
    fn from_restrictions_string(restrictions: &str) -> Option<PrerequisiteTree> {
        static PREREQ_INNER: Lazy<Regex> = Lazy::new(|| Regex::new(r#"<p class="prereq">Prerequisites?: (.*?)\.<"#).unwrap());
        static TAG_REMOVE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"<.*?>"#).unwrap());
        let prerequisites = &PREREQ_INNER.captures(restrictions)?[1];
        let tags_removed = TAG_REMOVE.replace_all(prerequisites, "");
        let tree = tags_removed.as_ref().try_into().unwrap();
        Some(tree)
    }

    pub fn qualifications_set(&self) -> HashSet<Qualification> {
        let mut ret = HashSet::new();

        let mut stack = Vec::new();
        stack.push(self);

        while let Some(tree) = stack.pop() {
            match tree {
                PrerequisiteTree::Qualification(qualification) => { ret.insert(qualification.clone()); },
                PrerequisiteTree::Conjunctive(_, children) => stack.extend(children),
            }
        }

        ret
    }
}

impl fmt::Display for PrerequisiteTree {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PrerequisiteTree::Qualification(qual) => fmt::Display::fmt(qual, f),
            PrerequisiteTree::Conjunctive(conj, children) => {
                fmt::Display::fmt(conj, f)?;
                f.write_char('(')?;
                let mut comma = "";
                for child in children {
                    f.write_str(comma)?;
                    fmt::Display::fmt(child, f)?;
                    comma = ",";
                }
                f.write_char(')')
            },
        }
    }
}

impl ser::Serialize for PrerequisiteTree {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            PrerequisiteTree::Qualification(Qualification::Course(course)) => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("course", course)?;
                map.end()
            }
            PrerequisiteTree::Qualification(Qualification::ExamScore(ExamScore { exam, score })) => {
                let mut map = serializer.serialize_map(Some(2))?;
                map.serialize_entry("exam", exam)?;
                map.serialize_entry("score", score)?;
                map.end()
            }
            PrerequisiteTree::Conjunctive(conjunctive, children) => {
                let mut map = serializer.serialize_map(Some(1))?;
                let conjunctive = conjunctive.to_string();
                map.serialize_entry(conjunctive.as_str(), children)?; 
                map.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for PrerequisiteTree {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct PrerequisiteTreeVisitor;

        impl<'de> Visitor<'de> for PrerequisiteTreeVisitor {
            type Value = PrerequisiteTree;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(r#"{"code": "<>"} or {"exam": "<>", "score": <>}"#)
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let missing_field = "missing `code`, `exam`, `score`, `or`, or `and`";
                let key: String = map.next_key()?.ok_or(Error::missing_field(missing_field))?;

                match key.as_str() {
                    "course" => Ok(PrerequisiteTree::Qualification(Qualification::Course(
                        map.next_value::<CourseCode>()?
                        ))),
                    "exam" => Ok(PrerequisiteTree::Qualification(Qualification::ExamScore(ExamScore { 
                        exam: map.next_value()?,
                        score: {
                            let (key, value): (String, _) = map.next_entry()?.ok_or(Error::missing_field("score"))?;
                            if key != "score" {
                                return Err(Error::missing_field("thing"));
                            }
                            value
                        }
                    }))),
                    "any" => Ok(PrerequisiteTree::Conjunctive(
                        Conjunctive::Any,
                        map.next_value()?,
                    )),
                    "all" => Ok(PrerequisiteTree::Conjunctive(
                        Conjunctive::All,
                        map.next_value()?,
                    )),
                    _ => Err(Error::missing_field(missing_field)),
                }
            }
        }

        deserializer.deserialize_map(PrerequisiteTreeVisitor)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum Qualification {
    Course(CourseCode),
    ExamScore(ExamScore),
}

impl fmt::Display for Qualification {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Qualification::Course(c) => fmt::Display::fmt(c, f),
            Qualification::ExamScore(e) => fmt::Display::fmt(e, f),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Conjunctive {
    Any,
    All,
}

impl fmt::Display for Conjunctive {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match self {
            Conjunctive::Any => "any",
            Conjunctive::All => "all",
        })
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Ord, PartialOrd, Hash, Debug)]
pub struct ExamScore {
    exam: Exam,
    score: u16,
}

impl fmt::Display for ExamScore {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} in {}", self.score, self.exam)
    }
}

impl ExamScore {
    pub fn from_exam_score(exam: &str, score: &str) -> Result<Self, ()> {
        let exam = exam.parse().map_err(|_| ())?;
        let score = score.parse().map_err(|_| ())?;
        Ok(ExamScore { exam, score })
    }
}

impl FromStr for ExamScore {
    type Err = ();
    fn from_str(string: &str) -> Result<ExamScore, ()> {
        static REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"minimum score of (.*?) in '(.*?)'").unwrap());
        let captures = REGEX.captures(string).unwrap();
        ExamScore::from_exam_score(&captures[1], &captures[2])
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Ord, PartialOrd, Hash, Debug)]
#[serde(transparent)]
pub struct Exam {
    inner: String,
}

impl FromStr for Exam {
    type Err = ();
    fn from_str(string: &str) -> Result<Exam, ()> {
        Ok(Exam { inner: string.to_string() })
    }
}


impl fmt::Display for Exam {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.inner)
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(try_from = "String")]
#[serde(into = "String")]
pub struct CourseCode {
    pub subject: Subject,
    pub number: CourseNumber,
}

impl TryFrom<String> for CourseCode {
    type Error = Infallible;
    fn try_from(string: String) -> Result<Self, Self::Error> {
//        let mut split = string.split(" ");
//        let subject = split.next().unwrap().parse().unwrap();
//        let number = split.next().unwrap().parse().unwrap();
//        Ok(CourseCode { subject, number })
        TryFrom::try_from(string.as_str())
    }
}

impl<'a> TryFrom<&'a str> for CourseCode {
    type Error = Infallible;
    fn try_from(string: &'a str) -> Result<Self, Self::Error> {
        let mut split = string.split(" ");
        let subject = split.next().unwrap().parse().unwrap();
        let number = split.next().unwrap().parse().unwrap();
        Ok(CourseCode { subject, number })
    }
}

impl fmt::Debug for CourseCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for CourseCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.subject, self.number)
    }
}

impl From<CourseCode> for String {
    fn from(item: CourseCode) -> String {
        item.to_string()
    }
}

#[derive(Clone, PartialEq, Debug, Ord, PartialOrd, Eq, Hash)]
pub struct CourseNumber {
    four_digit: u16,
    suffix: Option<char>,
}

impl fmt::Display for CourseNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}", self.four_digit)?;
        if let Some(c) = self.suffix { f.write_char(c)? }
        Ok(())
    }
}

impl FromStr for CourseNumber {
    type Err = ();
    fn from_str(string: &str) -> Result<Self, ()> {
        match string.len() {
            4 => {
                Ok(CourseNumber {
                    four_digit: string.parse().map_err(|_| ())?,
                    suffix: None,
                })
            },
            5 => {
                Ok(CourseNumber {
                    four_digit: string[..4].parse().map_err(|_| ())?,
                    suffix: string.chars().last(),
                })
            },
            _ => Err(()),
        }
    }
}



