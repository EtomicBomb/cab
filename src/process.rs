use std::num::ParseIntError;
use crate::restrictions::PrerequisiteTree;
use std::borrow::Cow;
use once_cell::sync::Lazy;
use crate::parse_prerequisite_string::parse_prerequisite_string;
use regex::Regex;
use regex::NoExpand;
use std::convert::Infallible;
use serde::Deserializer;
use serde::Deserialize;
use serde_json::StreamDeserializer;
use serde_json::de;
use std::str::FromStr;
use std::fmt;

fn yes_or_no(string: &str) -> Option<bool> {
    match string {
        "Y" => Some(true),
        "N" => Some(false),
        _ => None,
    }
}

fn enrollment_from_seats(string: &str) -> Option<u16> {
    static SEATS_MAX: Lazy<Regex> = Lazy::new(|| Regex::new(r#"<span class="seats_max">(\d+?)</span>"#).unwrap());
    static SEATS_AVAILABLE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"<span class="seats_avail">(-?\d+?)</span>"#).unwrap());

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
    static ENROLLMENT: Lazy<Regex> = Lazy::new(|| Regex::new(r#"Current enrollment: (\d+)"#).unwrap());

    ENROLLMENT.captures(string).map(|captures| captures.get(1).unwrap().as_str().parse().unwrap())
}

#[derive(Debug)]
struct CourseCode {
    inner: String,
}

impl FromStr for CourseCode {
    type Err = Infallible;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        Ok(CourseCode { inner: string.to_string() })
    }
}

fn section(string: &str) -> Option<u8> {
    static SECTION: Lazy<Regex> = Lazy::new(|| Regex::new(r#"^S(\d{2})$"#).unwrap());

    SECTION.captures(string).map(|captures| captures.get(1).unwrap().as_str().parse().unwrap())
}

#[derive(Debug)]
enum Title {
    AliasOf(CourseCode),
    Title(String),
}

impl FromStr for Title {
    type Err = Infallible;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        static COURSE_CODE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"[A-Z]+ \d{4}[A-Z]?"#).unwrap());
        Ok(match COURSE_CODE.find(string) {
            None => Title::Title(string.to_string()),
            Some(cannonical) => Title::AliasOf(CourseCode {
                inner: cannonical.as_str().to_string(),
            })
        })
    }
}

#[derive(Deserialize, Debug)]
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

#[derive(Copy, Debug, Clone)]
struct SemesterRange {
    inner: u16,
}

struct Semester { 
    inner: u16,
}

impl FromStr for Semester {
    type Err = ParseIntError;
    fn from_str(string: &str) -> Result<Self, Self::Err> {
        Ok(Semester { inner: match string {
            "GM" => 14,
            "GP" => 15,
            "F2" => 2,
            s => s.parse()?,
        }})
    }
}

impl SemesterRange {
    const FULL: SemesterRange = SemesterRange::to(15);
    const EMPTY: SemesterRange = SemesterRange::to(0);
    const UNDERGRADUATE: SemesterRange = SemesterRange::to(8);
    const GRADUATE: SemesterRange = SemesterRange::UNDERGRADUATE.complement();

    const fn to(semester: u16) -> SemesterRange {
        SemesterRange { inner: (1 << semester) - 1 }    
    }

    fn add(self, semester: Semester) -> Self {
        SemesterRange { inner: self.inner | (semester.inner - 1) }
    }

    fn from_str(string: &str) -> SemesterRange {
        static DELIM: Lazy<Regex> = Lazy::new(|| Regex::new(r#", | or "#).unwrap());
        DELIM.split(string)
            .map(Semester::from_str)
            .map(Result::unwrap)
            .fold(SemesterRange::EMPTY, SemesterRange::add)
    }

    const fn complement(self) -> Self {
        SemesterRange { inner: !self.inner & SemesterRange::FULL.inner }
    }

    fn intersection(self, other: Self) -> Self {
        SemesterRange { inner: self.inner & other.inner }
    }

    fn contiguous(self) -> Option<(u16, u16)> {
        todo!()
    }
}

impl Default for SemesterRange {
    fn default() -> SemesterRange {
        SemesterRange::FULL
    }
}

fn program_string(string: &str) -> Vec<String> {
    static DELIM: Lazy<Regex> = Lazy::new(|| Regex::new(r#", | or "#).unwrap());
    DELIM.split(string)
        .map(str::to_string)
        .collect()
}

#[derive(Debug)]
struct Qualifications {
    prerequisites: PrerequisiteTree,
    programs: Option<Vec<String>>,
    semester_range: SemesterRange,
}

impl FromStr for Qualifications {
    type Err = Infallible;
    fn from_str(string: &str) -> Result<Self, Self::Err> {
        static TAG: Lazy<Regex> = Lazy::new(|| Regex::new(r#"^(<p class="prereq">Prerequisites?: (?P<prereq>.*?)\.(<br/><sup>\*</sup> May be taken concurrently\.)?</p>)?(<p class="cls">Enrollment limited to students with a semester level of (?P<cls>.*?)\.</p>)?(<p class="cls">Students with a semester level of (?P<clsc>.*?) may <strong>not</strong> enroll\.</p>)?(<p class="maj">Enrollment is limited to students with a major in (?P<maj>.*?)\.</p>)?(<p class="maj">Students cannot enroll who have a concentration in (.*?)\.</p>)?(<p class="prg">Enrollment limited to students in the (?P<prg>.*?) programs\.</p>)?(<p class="prg">Enrollment limited to students in the following programs:<ul>(?P<prgl>.*?)</ul></p>)?(<p class="prg">Enrollment limited to students in the (?P<prgs>.*?) program.</p>)?(<p class="prg">Enrollment limited to students in the (?P<prg1>.*?) or (?P<prg2>.*?) programs.</p>)?(<p class="prg">Students in the (.*?) program may <strong>not</strong> enroll.</p>)?(<p class="lvl">Enrollment is limited to (?P<lvl>Undergraduate|Graduate) level students\.</p>)?(<p class="lvl">(?P<lvlc>Undergraduate|Graduate) level students may <strong>not</strong> enroll\.</p>)?(<p class="chr">Enrollment limited to students in the (?P<chr>.*?) chohort\.</p>)?$"#).unwrap());
        
        let captures = TAG.captures(string).unwrap();

        let prerequisites = captures.name("prereq")
            .as_ref()
            .map(regex::Match::as_str)
            .map(strip_html)
            .as_deref()
            .map(parse_prerequisite_string)
            .map(Result::unwrap)
            .unwrap_or_default();

        if let Some(captures) = captures.name("cls") {
            println!("{}", captures.as_str());
        }

        let semester_level = captures.name("cls")
            .as_ref()
            .map(regex::Match::as_str)
            .map(SemesterRange::from_str)
            .unwrap_or_default();

        let semester_level_complement = captures.name("clsc")
            .as_ref()
            .map(regex::Match::as_str)
            .map(SemesterRange::from_str)
            .map(SemesterRange::complement)
            .unwrap_or_default();

        let programs = captures.name("prg")
            .as_ref()
            .map(regex::Match::as_str)
            .map(program_string);

        let level = captures.name("lvl")
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


        Ok(Qualifications { prerequisites, programs, semester_range })
    }
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
        let code = CourseCode::from_str(&raw.code).unwrap();
        let section = section(&raw.section);
        let title = Title::from_str(&raw.title).unwrap();
        let description = strip_html(&raw.description);
        let qualifications = Qualifications::from_str(&raw.registration_restrictions).unwrap();
        let enrollment_seats = enrollment_from_seats(&raw.seats);
        let enrollment_html = enrollment_from_html(&raw.regdemog_html);
        let enrollment = enrollment_seats.or(enrollment_html);
        let demographics = serde_json::from_str(&raw.regdemog_json).ok();
        let srcdb = raw.srcdb;
        Record { restricted, code, section, title, description, qualifications, enrollment, demographics, srcdb }
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
    regdemog_html: String,
    regdemog_json: String,
    srcdb: String,
}

pub fn process<'a, R: de::Read<'a>>(
    source: R
) {
    let raws = StreamDeserializer::<_, Raw>::new(source);

    for raw in raws {
        if let Ok(raw) = raw {
            println!("{raw:#?}");
            let record = Record::from(raw);
            println!("{record:#?}");
        }
    }
}
    
    
