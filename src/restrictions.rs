use crate::logic::Tree;
use crate::logic::Symbol;
use crate::logic::Product;
use crate::logic::{visit_symbol, visit_all, visit_any};
use serde::de;
use serde::de::Error;
use serde::de::MapAccess;
use serde::ser;
use serde::ser::{SerializeMap, Serializer};
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use std::fmt;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CourseCode {
    subject: String,
    number: String,
}

impl CourseCode {
    pub fn new(subject: String, number: String) -> Result<CourseCode, ()> {
        Ok(CourseCode { subject, number })
    }

    pub fn subject(&self) -> &str {
        &self.subject
    }
}

impl<'a> TryFrom<&'a str> for CourseCode {
    type Error = ();
    fn try_from(string: &'a str) -> Result<Self, Self::Error> {
        let mut split = string.split(" ");
        let subject = split.next().ok_or(())?.to_string();
        let number = split.next().ok_or(())?.to_string();
        if split.next().is_some() {
            return Err(());
        }
        Ok(CourseCode { subject, number })
    }
}

impl fmt::Display for CourseCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.subject, self.number)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub struct ExamScore {
    pub exam: String,
    pub score: u32,
}

impl fmt::Display for ExamScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} on '{}'", self.exam, self.score)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum Qualification {
    Course(CourseCode),
    ExamScore(ExamScore),
}

impl Symbol for Qualification {
    fn rank(&self) -> Option<u32> {
        match self {
            Qualification::Course(..) => None,
            Qualification::ExamScore(ExamScore { score, .. }) => Some(*score),
        }
    }
}

impl fmt::Display for Qualification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Qualification::Course(c) => fmt::Display::fmt(c, f),
            Qualification::ExamScore(e) => fmt::Display::fmt(e, f),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Operator {
    Any,
    All,
}

impl fmt::Display for Operator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Operator::Any => "any",
            Operator::All => "all",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd)]
pub enum PrerequisiteTree {
    Qualification(Qualification),
    Operator(Operator, Vec<PrerequisiteTree>),
}

impl Tree for PrerequisiteTree {
    type Symbol = Qualification;
    fn into_product(&self) -> Product<Self::Symbol> {
        match self {
            PrerequisiteTree::Qualification(qualification) => visit_symbol(qualification.clone()),
            PrerequisiteTree::Operator(Operator::All, children) => visit_all(children),
            PrerequisiteTree::Operator(Operator::Any, children) => visit_any(children),
        }
    }

    fn symbol(symbol: Self::Symbol) -> Self {
        PrerequisiteTree::Qualification(symbol)
    }

    fn all(trees: Vec<Self>) -> Self {
        PrerequisiteTree::Operator(Operator::All, trees)
    }

    fn any(trees: Vec<Self>) -> Self {
        PrerequisiteTree::Operator(Operator::Any, trees)
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
            PrerequisiteTree::Qualification(Qualification::ExamScore(ExamScore {
                exam,
                score,
            })) => {
                let mut map = serializer.serialize_map(Some(2))?;
                map.serialize_entry("exam", exam)?;
                map.serialize_entry("score", score)?;
                map.end()
            }
            PrerequisiteTree::Operator(conjunctive, children) => {
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

        impl<'de> de::Visitor<'de> for PrerequisiteTreeVisitor {
            type Value = PrerequisiteTree;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(r#"{"code": "<>"} or {"exam": "<>", "score": <>}"#)
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let missing_field = "missing `code`, `exam`, `score`, `or`, or `and`";
                let key: String = map.next_key()?.ok_or(Error::missing_field(missing_field))?;

                match key.as_str() {
                    "course" => Ok(PrerequisiteTree::Qualification(Qualification::Course(
                        map.next_value::<CourseCode>()?,
                    ))),
                    "exam" => Ok(PrerequisiteTree::Qualification(Qualification::ExamScore(
                        ExamScore {
                            exam: map.next_value()?,
                            score: {
                                let (key, value): (String, _) =
                                    map.next_entry()?.ok_or(Error::missing_field("score"))?;
                                if key != "score" {
                                    return Err(Error::missing_field("thing"));
                                }
                                value
                            },
                        },
                    ))),
                    "any" => Ok(PrerequisiteTree::Operator(Operator::Any, map.next_value()?)),
                    "all" => Ok(PrerequisiteTree::Operator(Operator::All, map.next_value()?)),
                    _ => Err(Error::missing_field(missing_field)),
                }
            }
        }

        deserializer.deserialize_map(PrerequisiteTreeVisitor)
    }
}
