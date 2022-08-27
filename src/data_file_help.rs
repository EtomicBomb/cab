use std::fs;
use crate::restrictions::{CourseCode, RegistrationRestrictions};
use regex::Regex;
use std::collections::HashMap;
use once_cell::sync::Lazy;
use serde_json::Value;

pub fn look_for_override_corrections(restrictions: &HashMap<CourseCode, RegistrationRestrictions>) {
    for course in fs::read_dir("resources/scraped").unwrap() {
        let course = course.unwrap().path();
        let course_code: CourseCode = course.file_stem().unwrap().to_str().unwrap().parse().unwrap();

        let should_look = !restrictions[&course_code].override_required;

        if should_look {
            for variant in fs::read_dir(course).unwrap() {
                let variant = variant.unwrap().path();
                let root: Value = serde_json::from_str(&fs::read_to_string(variant).unwrap()).unwrap();

                if let Some(desc) = root["description"].as_str() {
                    static TM: Lazy<Regex> = Lazy::new(|| Regex::new(r#"override|permission|approv"#).unwrap());

                    if let Some(m) = TM.find(&desc) {
                        let desc = &desc[m.start().max(15)-15..];
                        static TAG_REMOVE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"<.*?>"#).unwrap());
                        let desc = TAG_REMOVE.replace_all(desc, "");
                        println!("{}\t{}", course_code, desc);
                        break;
                    }
                }
            }
        }
    }
}

pub fn look_for_prerequisite_corrections(restrictions: &HashMap<CourseCode, RegistrationRestrictions>) {
    for course in fs::read_dir("resources/scraped").unwrap() {
        let course = course.unwrap().path();
        let course_code: CourseCode = course.file_stem().unwrap().to_str().unwrap().parse().unwrap();

        let should_look = restrictions[&course_code].prerequisite_restrictions.is_none()
            && !restrictions[&course_code].informal_prerequisite
            && !restrictions[&course_code].override_required;

        if should_look {
            for variant in fs::read_dir(course).unwrap() {
                let variant = variant.unwrap().path();
                let root: Value = serde_json::from_str(&fs::read_to_string(variant).unwrap()).unwrap();

                if let Some(desc) = root["description"].as_str() {
                    if desc.contains("o prerequisite") { continue } // desc isn't 'No prerequisite.'
                    static TM: Lazy<Regex> = Lazy::new(|| Regex::new(r#"rerequisite|recommend|expect"#).unwrap()); // can add `experience`

                    if let Some(m) = TM.find(&desc) {
                        let desc = &desc[m.start().max(18)-18..];
                        static TAG_REMOVE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"<.*?>"#).unwrap());
                        let desc = TAG_REMOVE.replace_all(desc, "");
                        println!("{}\t{}", course_code, desc);
                        break;
                    }
                }

            }
        }
    }
}
