use crate::restrictions::CourseCode;
use crate::restrictions::PrerequisiteTree;
use std::collections::HashMap;
use std::collections::HashSet;
use std::num::ParseIntError;

use once_cell::sync::Lazy;
use regex::NoExpand;
use regex::Regex;
use serde::Deserialize;
use serde::Serialize;
use serde_json::de;
use serde_json::StreamDeserializer;
use std::convert::Infallible;
use std::fmt;
use std::iter;
use std::str::FromStr;

fn yes_or_no(string: &str) -> Option<bool> {
    match string {
        "Y" => Some(true),
        "N" => Some(false),
        _ => None,
    }
}

fn enrollment_from_seats(string: &str) -> Option<u16> {
    static SEATS_MAX: Lazy<Regex> =
        Lazy::new(|| Regex::new(r#"<span class="seats_max">(\d+?)</span>"#).unwrap());
    static SEATS_AVAILABLE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r#"<span class="seats_avail">(-?\d+?)</span>"#).unwrap());
    let max: i16 = match SEATS_MAX.captures(string) {
        Some(captures) => captures.get(1).unwrap().as_str().parse().unwrap(),
        None => return None,
    };
    let available: i16 = match SEATS_AVAILABLE.captures(string) {
        Some(captures) => captures.get(1).unwrap().as_str().parse().unwrap(),
        None => return None,
    };
    Some((max - available) as u16)
}

fn enrollment_from_html(string: &str) -> Option<u16> {
    static ENROLLMENT: Lazy<Regex> =
        Lazy::new(|| Regex::new(r#"Current enrollment: (\d+)"#).unwrap());
    ENROLLMENT
        .captures(string)
        .map(|captures| captures.get(1).unwrap().as_str().parse().unwrap())
}

fn section(string: &str) -> Option<u8> {
    static SECTION: Lazy<Regex> = Lazy::new(|| Regex::new(r#"^S(\d{2})$"#).unwrap());
    SECTION
        .captures(string)
        .map(|captures| captures.get(1).unwrap().as_str().parse().unwrap())
}

#[derive(Clone, Debug)]
enum Title {
    AliasOf(CourseCode),
    Title(String),
}

impl FromStr for Title {
    type Err = Infallible;
    fn from_str(string: &str) -> Result<Self, Self::Err> {
        static COURSE_CODE: Lazy<Regex> =
            Lazy::new(|| Regex::new(r#"[A-Z]+ \d{4}[A-Z]?"#).unwrap());
        Ok(match COURSE_CODE.find(string) {
            None => Title::Title(string.to_string()),
            Some(cannonical) => Title::AliasOf(CourseCode::try_from(cannonical.as_str()).unwrap()),
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Demographics {
    #[serde(default)]
    #[serde(alias = "FY")]
    freshmen: u16,
    #[serde(default)]
    #[serde(alias = "So")]
    sophomores: u16,
    #[serde(default)]
    #[serde(alias = "Jr")]
    juniors: u16,
    #[serde(default)]
    #[serde(alias = "Sr")]
    seniors: u16,
    #[serde(default)]
    #[serde(alias = "Gr")]
    graduates: u16,
    #[serde(default)]
    #[serde(alias = "Oth")]
    others: u16,
}

fn strip_html(string: &str) -> String {
    static TAG: Lazy<Regex> = Lazy::new(|| Regex::new(r#"<.*?>"#).unwrap());
    static AMP: Lazy<Regex> = Lazy::new(|| Regex::new(r#"&amp;"#).unwrap());
    static LT: Lazy<Regex> = Lazy::new(|| Regex::new(r#"&lt;"#).unwrap());
    static GT: Lazy<Regex> = Lazy::new(|| Regex::new(r#"&gt;"#).unwrap());
    let string = TAG.replace_all(&string, NoExpand(""));
    let string = AMP.replace_all(&string, NoExpand("&"));
    let string = LT.replace_all(&string, NoExpand("<"));
    let string = GT.replace_all(&string, NoExpand(">"));
    string.to_string()
}

#[derive(Serialize, Deserialize)]
struct Semester {
    inner: u16,
}

impl fmt::Display for Semester {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.inner {
            13 => f.write_str("GM"),
            14 => f.write_str("GP"),
            s => write!(f, "{:02}", s + 1),
        }
    }
}

impl FromStr for Semester {
    type Err = ParseIntError;
    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let semester_number = match string {
            "GM" => 14,
            "GP" => 15,
            "F2" => 2,
            s => s.parse()?,
        };
        Ok(Semester {
            inner: semester_number - 1,
        })
    }
}

#[derive(Serialize, Deserialize, Copy, Debug, Clone, PartialEq)]
#[serde(try_from = "Vec<u16>")]
#[serde(into = "Vec<u16>")]
pub struct SemesterRange {
    inner: u16,
}

impl SemesterRange {
    const FULL: SemesterRange = SemesterRange::to(15);
    const EMPTY: SemesterRange = SemesterRange::to(0);
    const UNDERGRADUATE: SemesterRange = SemesterRange::to(8);
    const GRADUATE: SemesterRange = SemesterRange::UNDERGRADUATE.complement();

    const fn to(semester: u16) -> SemesterRange {
        SemesterRange {
            inner: (1 << semester) - 1,
        }
    }

    pub fn is_full(&self) -> bool {
        self == &SemesterRange::FULL
    }

    fn add(self, semester: Semester) -> Self {
        SemesterRange {
            inner: self.inner | (1 << (semester.inner)),
        }
    }

    const fn complement(self) -> Self {
        SemesterRange {
            inner: self.inner ^ SemesterRange::FULL.inner,
        }
    }

    fn intersection(self, other: Self) -> Self {
        SemesterRange {
            inner: self.inner & other.inner,
        }
    }

    fn semesters(self) -> impl Iterator<Item = Semester> {
        let mut inner = self.inner;
        iter::from_fn(move || {
            if inner == 0 {
                return None;
            }
            let semester = inner.trailing_zeros();
            inner &= !(1 << semester);
            Some(Semester {
                inner: semester as u16,
            })
        })
    }
}

impl TryFrom<Vec<u16>> for SemesterRange {
    type Error = Infallible;
    fn try_from(semesters: Vec<u16>) -> Result<Self, Self::Error> {
        Ok(semesters
            .into_iter()
            .fold(SemesterRange::EMPTY, |accum, inner| {
                accum.add(Semester { inner })
            }))
    }
}

impl From<SemesterRange> for Vec<u16> {
    fn from(range: SemesterRange) -> Vec<u16> {
        range.semesters().map(|semester| semester.inner).collect()
    }
}

impl<'a> TryFrom<&'a str> for SemesterRange {
    type Error = Infallible;
    fn try_from(string: &'a str) -> Result<Self, Self::Error> {
        static DELIM: Lazy<Regex> = Lazy::new(|| Regex::new(r#", | or "#).unwrap());
        Ok(DELIM
            .split(string)
            .map(Semester::from_str)
            .map(Result::unwrap)
            .fold(SemesterRange::EMPTY, SemesterRange::add))
    }
}

impl fmt::Display for SemesterRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut sep = "";
        for semester in self.semesters() {
            write!(f, "{sep}{semester}")?;
            sep = ", ";
        }
        Ok(())
    }
}

impl Default for SemesterRange {
    fn default() -> SemesterRange {
        SemesterRange::FULL
    }
}

#[cfg(test)]
mod tests {
    use super::{Semester, SemesterRange};
    use std::str::FromStr;

    #[test]
    fn semseter_range() {
        let text = "05, 06, 07, 08, 09, 10, 11, 12 or 13";
        let range = SemesterRange::try_from(text).unwrap();
        assert_eq!(range.to_string(), "05, 06, 07, 08, 09, 10, 11, 12, 13");
        let compl = range.complement();
        assert_eq!(compl.to_string(), "01, 02, 03, 04, GM, GP");
    }

    #[test]
    fn semseter_range2() {
        let text = "05, 06, 07, 08, 09, 10, 11, 12 or 13";
        let range = SemesterRange::try_from(text).unwrap();
        assert_eq!(
            range.to_string(),
            "05, 06, 07, 08, 09, 10, 11, 12, 13",
            "{}",
            range.inner
        );
    }

    #[test]
    fn semseter_range3() {
        let range = SemesterRange::EMPTY;
        let range = range.add(Semester::from_str("05").unwrap());
        assert_eq!(range.to_string(), "05", "{}", range.inner);
    }

    #[test]
    fn semseter_range4() {
        let range = SemesterRange::to(4);
        assert_eq!(range.to_string(), "01, 02, 03, 04", "{}", range.inner);
    }
}

fn program_string(string: &str) -> Vec<String> {
    static DELIM: Lazy<Regex> = Lazy::new(|| Regex::new(r#", | or "#).unwrap());
    DELIM.split(string).map(str::to_string).collect()
}

#[derive(Debug)]
struct Qualifications {
    prerequisites: Option<PrerequisiteTree>,
    programs: Option<Vec<String>>,
    semester_range: SemesterRange,
}

impl FromStr for Qualifications {
    type Err = Infallible;
    fn from_str(string: &str) -> Result<Self, Self::Err> {
        static TAG: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r#"^(<p class="prereq">Prerequisites?: (?P<prereq>.*?)\.(<br/><sup>\*</sup> May be taken concurrently\.)?</p>)?(<p class="cls">Enrollment limited to students with a semester level of (?P<cls>.*?)\.</p>)?(<p class="cls">Students with a semester level of (?P<clsc>.*?) may <strong>not</strong> enroll\.</p>)?(<p class="maj">Enrollment is limited to students with a major in (?P<maj>.*?)\.</p>)?(<p class="maj">Students cannot enroll who have a concentration in (.*?)\.</p>)?(<p class="prg">Enrollment limited to students in the (?P<prg>.*?) programs\.</p>)?(<p class="prg">Enrollment limited to students in the following programs:<ul>(?P<prgl>.*?)</ul></p>)?(<p class="prg">Enrollment limited to students in the (?P<prgs>.*?) program.</p>)?(<p class="prg">Enrollment limited to students in the (?P<prg1>.*?) or (?P<prg2>.*?) programs.</p>)?(<p class="prg">Students in the (.*?) program may <strong>not</strong> enroll.</p>)?(<p class="lvl">Enrollment is limited to (?P<lvl>Undergraduate|Graduate) level students\.</p>)?(<p class="lvl">(?P<lvlc>Undergraduate|Graduate) level students may <strong>not</strong> enroll\.</p>)?(<p class="chr">Enrollment limited to students in the (?P<chr>.*?) chohort\.</p>)?$"#).unwrap()
        });
        let captures = TAG.captures(string).unwrap();
        let prerequisites = captures
            .name("prereq")
            .as_ref()
            .map(regex::Match::as_str)
            .map(strip_html)
            .as_deref()
            .map(PrerequisiteTree::try_from)
            .map(Result::unwrap);
        let semester_level = captures
            .name("cls")
            .as_ref()
            .map(regex::Match::as_str)
            .map(SemesterRange::try_from)
            .map(Result::unwrap)
            .unwrap_or_default();
        let semester_level_complement = captures
            .name("clsc")
            .as_ref()
            .map(regex::Match::as_str)
            .map(SemesterRange::try_from)
            .map(Result::unwrap)
            .map(SemesterRange::complement)
            .unwrap_or_default();
        let programs = captures
            .name("prg")
            .as_ref()
            .map(regex::Match::as_str)
            .map(program_string);
        let level = captures
            .name("lvl")
            .as_ref()
            .map(regex::Match::as_str)
            .and_then(|level| match level {
                "Undergraduate" => Some(SemesterRange::UNDERGRADUATE),
                "Graduate" => Some(SemesterRange::GRADUATE),
                _ => None,
            })
            .unwrap_or_default();
        let semester_range = semester_level
            .intersection(semester_level_complement)
            .intersection(level);
        Ok(Qualifications {
            prerequisites,
            programs,
            semester_range,
        })
    }
}

fn instructors(string: &str) -> Vec<String> {
    static NAME: Lazy<Regex> = Lazy::new(|| Regex::new(r#"<h4>.*?</h4>"#).unwrap());
    NAME.find_iter(string)
        .map(|name| strip_html(name.as_str()))
        .filter(|name| name != "TBD")
        .collect()
}

#[derive(Debug)]
struct Record {
    restricted: bool,
    code: CourseCode,
    section: Option<u8>,
    title: Title,
    description: String,
    qualifications: Qualifications,
    enrollment: Option<u16>,
    instructors: Vec<String>,
    demographics: Option<Demographics>,
    srcdb: String,
}

impl FromStr for Record {
    type Err = ();
    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let raw: Raw = serde_json::from_str(string).unwrap();
        Ok(Record::from(raw))
    }
}

impl From<Raw> for Record {
    fn from(raw: Raw) -> Record {
        let restricted = yes_or_no(&raw.permreq).unwrap();
        let code = CourseCode::try_from(raw.code.as_str()).unwrap();
        let section = section(&raw.section);
        let title = Title::from_str(&raw.title).unwrap();
        let description = strip_html(&raw.description);
        let qualifications = Qualifications::from_str(&raw.registration_restrictions).unwrap();
        let enrollment_seats = enrollment_from_seats(&raw.seats);
        let enrollment_html = enrollment_from_html(&raw.regdemog_html);
        let enrollment = enrollment_seats.or(enrollment_html);
        let instructors = instructors(&raw.instructordetail_html);
        let demographics = serde_json::from_str(&raw.regdemog_json).ok();
        let srcdb = raw.srcdb;
        Record {
            restricted,
            code,
            section,
            title,
            description,
            qualifications,
            enrollment,
            instructors,
            demographics,
            srcdb,
        }
    }
}

#[derive(Deserialize, Debug)]
struct Raw {
    permreq: String,
    code: String,
    section: String,
    title: String,
    description: String,
    registration_restrictions: String,
    seats: String,
    instructordetail_html: String,
    regdemog_html: String,
    regdemog_json: String,
    srcdb: String,
}

#[derive(Serialize, Deserialize)]
pub struct Offering {
    date: String,
    section: u8,
    instructors: Vec<String>,
    enrollment: Option<u16>,
    demographics: Option<Demographics>,
}

#[derive(Serialize, Deserialize)]
pub struct Course {
    code: CourseCode,
    title: String,
    description: String,
    prerequisites: Option<PrerequisiteTree>,
    semester_range: SemesterRange,
    restricted: bool,
    aliases: Vec<CourseCode>,
    offerings: Vec<Offering>,
}

impl Course {
    pub fn code(&self) -> &CourseCode {
        &self.code
    }

    pub fn prerequisites(&self) -> Option<&PrerequisiteTree> {
        self.prerequisites.as_ref()
    }

    pub fn prerequisites_mut(&mut self) -> &mut Option<PrerequisiteTree> {
        &mut self.prerequisites
    }

    pub fn semester_range(&self) -> &SemesterRange {
        &self.semester_range
    }

    fn from_offerings(
        code: CourseCode,
        mut offerings: Vec<Record>,
        aliases: Vec<CourseCode>,
    ) -> Course {
        offerings.sort_by(|a, b| a.srcdb.cmp(&b.srcdb).reverse()); // recent first
        let latest = offerings.first().unwrap();
        let title = match latest.title {
            Title::Title(ref t) => t.clone(),
            _ => unreachable!("method precondition"),
        };
        let description = latest.description.clone();
        let prerequisites = offerings
            .iter()
            .find_map(|offering| offering.qualifications.prerequisites.as_ref())
            .cloned();
        let semester_range = latest.qualifications.semester_range;
        let restricted = latest.restricted;
        let offerings = offerings
            .into_iter()
            .map(|offering| Offering {
                date: offering.srcdb,
                section: offering.section.unwrap(),
                instructors: offering.instructors,
                enrollment: offering.enrollment,
                demographics: offering.demographics,
            })
            .collect();
        Course {
            code,
            title,
            description,
            prerequisites,
            semester_range,
            restricted,
            aliases,
            offerings,
        }
    }
}

pub fn process<'a, R: de::Read<'a>>(source: R) -> Vec<Course> {
    #[derive(Default)]
    struct Details {
        offerings: Vec<Record>,
        aliases: HashSet<CourseCode>,
    }
    let mut map: HashMap<CourseCode, Details> = HashMap::new();
    StreamDeserializer::<_, Raw>::new(source)
        .filter_map(Result::ok)
        .map(Record::from)
        .for_each(|record| match record.title {
            Title::Title(_) if record.section.is_some() => {
                map.entry(record.code.clone())
                    .or_default()
                    .offerings
                    .push(record);
            }
            Title::AliasOf(cannonical) => {
                map.entry(cannonical)
                    .or_default()
                    .aliases
                    .insert(record.code);
            }
            _ => {}
        });
    map.into_iter()
        .filter(|(_, Details { offerings, .. })| !offerings.is_empty())
        .map(|(code, Details { offerings, aliases })| {
            let aliases = aliases.into_iter().collect();
            Course::from_offerings(code, offerings, aliases)
        })
        .collect()
}
