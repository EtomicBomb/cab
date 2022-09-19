use serde::de::Error;
use serde::Deserializer;
use serde::de;
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
use std::iter::Sum;
use std::ops::Add;
use serde_json::Value;
use crate::logic::IntoProduct;
use crate::logic::Visitor;
use crate::logic::Product;

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

impl IntoProduct for PrerequisiteTree {
    type Node = Qualification;
    fn into_product(&self, visitor: &mut Visitor<Self::Node>) -> Product {
        match self {
            PrerequisiteTree::Qualification(qualification) => visitor.visit_node(qualification.clone()),
            PrerequisiteTree::Conjunctive(Conjunctive::All, children) => visitor.visit_all(children),
            PrerequisiteTree::Conjunctive(Conjunctive::Any, children) => visitor.visit_any(children),
        }
    }

    fn node(node: &Self::Node) -> Self {
        PrerequisiteTree::Qualification(node.clone())
    }

    fn all(trees: Vec<Self>) -> Self {
        PrerequisiteTree::Conjunctive(Conjunctive::All, trees)
    }

    fn any(trees: Vec<Self>) -> Self {
        PrerequisiteTree::Conjunctive(Conjunctive::Any, trees)
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Conjunctive::Any => "any",
            Conjunctive::All => "all",
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

