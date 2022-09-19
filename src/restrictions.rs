use serde::de::Error;
use serde::Deserializer;
use serde::de;
use serde::de::MapAccess;
use serde::ser::{Serializer, SerializeSeq, SerializeMap};
use serde::Serialize;
use serde::ser;
use serde::{Deserialize};
use std::fmt;
use std::fmt::{Write};
use std::str::FromStr;
use std::io::{BufRead};
use std::iter::Sum;
use crate::logic::IntoProduct;
use crate::logic::Visitor;
use crate::logic::Product;

#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd)]
pub enum PrerequisiteTree {
    Qualification(Qualification),
    Operator(Operator, Vec<PrerequisiteTree>),
}

impl IntoProduct for PrerequisiteTree {
    type Node = Qualification;
    fn into_product(&self, visitor: &mut Visitor<Self::Node>) -> Product {
        match self {
            PrerequisiteTree::Qualification(qualification) => visitor.visit_node(qualification.clone()),
            PrerequisiteTree::Operator(Operator::All, children) => visitor.visit_all(children),
            PrerequisiteTree::Operator(Operator::Any, children) => visitor.visit_any(children),
        }
    }

    fn node(node: &Self::Node) -> Self {
        PrerequisiteTree::Qualification(node.clone())
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
            PrerequisiteTree::Qualification(Qualification::ExamScore(ExamScore { exam, score })) => {
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
                    "any" => Ok(PrerequisiteTree::Operator(
                        Operator::Any,
                        map.next_value()?,
                    )),
                    "all" => Ok(PrerequisiteTree::Operator(
                        Operator::All,
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

    pub fn number(&self) -> &str {
        &self.number        
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

