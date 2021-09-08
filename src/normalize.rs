use crate::restrictions::{PrerequisiteTree, Qualification, ScoreQualification, CourseCode, Conjunctive};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::io::{BufReader, BufRead};
use std::fs::File;
use std::str::FromStr;
use std::cmp::Reverse;
use crate::parse_prerequisite_string::parse_prerequisite_string;

// needs: distributive laws

/// Normalization Steps:
/// replacing courses with their equivalents
/// all(all(a's), b's) -> all(a's, b's)
/// any(any(a's), b's) -> any(a's, b's)
/// all(a, any(a, c's)) -> a
/// any(a, all(a, c's)) -> a
/// sort descending
/// exam score overlap / dedup
/// all(a) -> a
/// any(a) -> a
pub fn normalize(tree: PrerequisiteTree) -> PrerequisiteTree {
    let tree = equivalent(tree);
    let tree = flatten(tree);
    let tree = exam_score_overlap(&tree);
    let tree = unbox_singlets(tree);
    tree
}

fn equivalent(tree: PrerequisiteTree) -> PrerequisiteTree {
    static EQUIVALENT_MAP: Lazy<HashMap<Qualification, PrerequisiteTree>> = Lazy::new(|| {
        let file = BufReader::new(File::open("resources/equivalent.txt").unwrap());
        let mut ret = HashMap::new();
        for line in file.lines() {
            let line = line.unwrap();
            let tree = parse_prerequisite_string(&line).unwrap();
            let set = tree.qualifications_set();
            ret.extend(set.into_iter().map(|q| (q, tree.clone())));
        }

        ret
    });

    match tree {
        PrerequisiteTree::Qualification(qual) => match EQUIVALENT_MAP.get(&qual) {
            Some(t) => t.clone(),
            None => tree,
        },
        PrerequisiteTree::Conjunctive(conj, children) => {
            let children = children.into_iter().map(equivalent).collect();
            PrerequisiteTree::Conjunctive(conj, children)
        },
    }
}

fn flatten(tree: PrerequisiteTree) -> PrerequisiteTree {
    match tree {
        PrerequisiteTree::Qualification(_) => tree,
        PrerequisiteTree::Conjunctive(conj, children) => {
            let mut new_children = Vec::new();
            for child in children {
                let child = flatten(child);
                match child {
                    PrerequisiteTree::Conjunctive(c, mut sub_branches) if c == conj => new_children.append(&mut sub_branches),
                    _ => new_children.push(child),
                }
            }

            new_children.sort_by(|a, b| b.cmp(a));

            PrerequisiteTree::Conjunctive(conj, new_children)
        },
    }
}

fn exam_score_overlap(tree: &PrerequisiteTree) -> PrerequisiteTree {
    match tree {
        PrerequisiteTree::Qualification(_) => tree.clone(),
        PrerequisiteTree::Conjunctive(conj, children) => {
            let mut children: Vec<_> = children.iter().map(exam_score_overlap).collect();

            children.dedup_by(|a, b| match (a, b) {
                (x, y) if x == y => true,
                (
                    PrerequisiteTree::Qualification(Qualification::ExamScore(ScoreQualification::ExamScore(a0, _))),
                    PrerequisiteTree::Qualification(Qualification::ExamScore(ScoreQualification::ExamScore(b0, _)))
                ) => a0 == b0,
                _ => false,
            });

            PrerequisiteTree::Conjunctive(*conj, children)
        },
    }
}

fn unbox_singlets(tree: PrerequisiteTree) -> PrerequisiteTree {
    match tree {
        PrerequisiteTree::Qualification(_) => tree,
        PrerequisiteTree::Conjunctive(conj, children) => {
            let mut children: Vec<_> = children.into_iter().map(unbox_singlets).collect();

            if children.len() == 1 {
                children.pop().unwrap()
            } else {
                PrerequisiteTree::Conjunctive(conj, children)
            }
        }
    }
}