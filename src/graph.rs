use std::collections::HashMap;
use std::collections::HashSet;
use std::ops::{Index, IndexMut};
use crate::restrictions::{CourseCode, Qualification, PrerequisiteTree, Conjunctive};
use crate::process::Course;
use std::fmt::{self, Write, Formatter};
use crate::subject::{Subject, Subjects};
use rand::{thread_rng, Rng};
use std::io::{Write as _, Read};
use once_cell::sync::Lazy;
use std::process::{Command, Stdio};
use std::fmt::Write as FmtWrite;
use regex::{RegexBuilder, Regex};
use std::io;

fn graphviz_to_svg(graphviz: &str) -> io::Result<String> {
    let mut dotted = Command::new("dot")
        .arg("-Tsvg")
        .arg("/dev/stdin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    dotted.stdin.take().unwrap().write_all(graphviz.as_bytes())?;
    let mut svg = String::new();
    dotted.stdout.take().unwrap().read_to_string(&mut svg)?;
    dotted.wait()?;
    Ok(svg)
}

fn svg_box(code: &CourseCode, course: Option<&Course>, x: f32, y: f32) -> String {
    let mut ret = String::new();
    let x = x - 102.0;
    writeln!(ret, r#"<rect style="fill:#ffffff;stroke:#000000;stroke-width:3" width="102" height="44" x="{}" y="{}" />"#, x, y).unwrap();
    writeln!(ret, r#"<text x="{}" y="{}" style="font-family:monospace;font-size:16px">{}</text>"#, x+3.5, y+17.0, code).unwrap();
    if let Some(course) = course {
        let range = course.semester_range();
        if !range.is_full() {
            writeln!(ret, r#"<text x="{}" y="{}" style="font-family:monospace;font-size:8px">{range}</text>"#, x+20.5, y+30.0).unwrap();
        }
    }
    ret
}

fn svg_filter(svg: &mut String, courses: &HashMap<CourseCode, Course>) {
    // static REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r#"<g id=".*?" class="node qual_(.*?)">.*?points="(.*?),(.*?) .*?</g>"#).unwrap());
    static REGEX: Lazy<Regex> = Lazy::new(|| RegexBuilder::new(r#"<g id="node\d*" class="node qual_(.*?)".*?points="(.*?),(.*?) .*?</g>"#).dot_matches_new_line(true).build().unwrap());
    while let Some(location) = REGEX.captures(&svg) {
        let entire_range = location.get(0).unwrap().range();
        let code = location[1].try_into().unwrap();
        let top_left_x = location[2].parse::<f32>().unwrap();
        let top_left_y = location[3].parse().unwrap();
        let new_svg = svg_box(&code, courses.get(&code), top_left_x, top_left_y);
        svg.replace_range(entire_range, &new_svg);
    }
}

pub fn svg(courses: &HashMap<CourseCode, Course>) -> io::Result<String> {
    let mut id_generator = IdGenerator::default();
    let subjects: HashSet<&str> = courses.keys().map(|code| code.subject()).collect();
    let subject_graphs: Vec<_> = subjects.iter()
        .map(|subject| SubjectGraph::new(subject, courses, &mut id_generator))
        .collect();
    let mut graphviz = String::from("digraph {\npackmode=\"graph\"\n");
    for subject_graph in subject_graphs.iter() {
        subject_graph.graphviz_cluster(&mut graphviz);
    }
    graphviz.push_str("}");

    std::fs::write("viz.dot", &graphviz).unwrap();

    eprintln!("Filtering through graphviz");
    let mut svg = graphviz_to_svg(&graphviz)?;
    eprintln!("Fixup svg");
    svg_filter(&mut svg, courses);
    Ok(svg)
}

struct SubjectGraph {
    nodes: Vec<Node>,
    subject: String,
}

impl SubjectGraph {
    fn new(subject: &str, restrictions: &HashMap<CourseCode, Course>, id_generator: &mut IdGenerator) -> SubjectGraph {
        let mut ret = SubjectGraph { nodes: Vec::new(), subject: subject.to_string() };
        for (code, course) in restrictions.iter().filter(|(code, _)| code.subject() == subject) {
            let node_index = ret.insert_qualification(&Qualification::Course(code.clone()), id_generator);
            if let Some(prereq_tree) = course.prerequisites() {
                ret.insert(node_index, prereq_tree, id_generator);
            }
        }
        ret
    }

    fn iter(&self) -> impl Iterator<Item=(NodeIndex, &Node)> {
        self.nodes.iter().enumerate().map(|(i, node)| (NodeIndex(i), node))
    }

    fn insert(&mut self, location: NodeIndex, prereq_tree: &PrerequisiteTree, id_generator: &mut IdGenerator) {
        let to_insert = match prereq_tree {
            PrerequisiteTree::Qualification(qualification) => {
                self.insert_qualification(qualification, id_generator)
            }
            PrerequisiteTree::Conjunctive(conj, ref children) => {
                let found = self.nodes.iter()
                    .position(|n| n.is_conjunctive(*conj) && self.is_equal(&n.dependencies, children))
                    .map(NodeIndex);
                found.unwrap_or_else(|| {
                    let new_index = NodeIndex(self.nodes.len());
                    self.nodes.push(Node {
                        kind: NodeKind::Conjunctive(*conj),
                        dependencies: Vec::new(),
                        id: id_generator.next(),
                    });
                    for c in children {
                        self.insert(new_index, c, id_generator);
                    }
                    new_index
                })
            }
        };
        self[location].dependencies.push(to_insert);
    }

    fn is_equal(&self, dependencies: &[NodeIndex], prereq_tree: &[PrerequisiteTree]) -> bool {
        if dependencies.len() != prereq_tree.len() { return false }

        dependencies.iter().zip(prereq_tree)
            .all(|(&d, c)| {
                match c {
                    PrerequisiteTree::Qualification(q) => self[d].is_qualification(q),
                    PrerequisiteTree::Conjunctive(conj, children) => {
                        self[d].is_conjunctive(*conj)
                            && self.is_equal(&self[d].dependencies, children)
                    }
                }
            })
    }

    fn insert_qualification(&mut self, qualification: &Qualification, id_generator: &mut IdGenerator) -> NodeIndex {
        let result = self.iter()
            .find(|(_, node)| node.is_qualification(qualification))
            .map(|(i, _)| i);

        result.unwrap_or_else(|| {
            let new_index = NodeIndex(self.nodes.len());
            self.nodes.push(Node {
                kind: NodeKind::Qualification(qualification.clone()),
                dependencies: Vec::new(),
                id: id_generator.next(),
            });
            new_index
        })
    }

    fn is_singlet(&self, node_index: NodeIndex) -> bool {
        self[node_index].dependencies.is_empty()
            && self.nodes.iter().all(|o| !o.dependencies.contains(&node_index))
    }

    fn graphviz_cluster(&self, string: &mut String) {
        let abbreviation = self.subject.to_string();
        writeln!(string, "subgraph cluster_{} {{", abbreviation).unwrap();
        writeln!(string, "packmode=\"graph\"").unwrap();
        writeln!(string, "label=\"{}\"", self.subject).unwrap();

        let color = "808000";
        writeln!(string, "bgcolor=\"#{}\"", color).unwrap();

        for node in self.nodes.iter() {
            match node.kind() {
                NodeKind::Qualification(Qualification::ExamScore(q)) => {
                    writeln!(string, "{} [label=\"{}\",shape=box,color=blue]", node.id, q).unwrap();
                }
                NodeKind::Qualification(Qualification::Course(code)) => {
                    writeln!(string, "{} [label=\"\",shape=box, fixedsize=true, width=1.4, height=0.6, class=\"qual_{}\"]", node.id, code).unwrap();
                }
                NodeKind::Conjunctive(conjunctive) => {
                    writeln!(string, "{} [label={}]", node.id, conjunctive).unwrap();
                }
            }
        }

        let (singlets, others): (Vec<_>, Vec<_>) = self.iter()
            .partition(|&(i, _)| self.is_singlet(i));

        let singlets_sqrt = integer_square_root(singlets.len() as u64) as usize + 1;


        writeln!(string, "subgraph cluster{} {{\nstyle=\"invis\"", thread_rng().gen::<u32>()).unwrap();

        for (i, pair) in singlets.windows(2).enumerate() {
            if i % singlets_sqrt != 0 {
                writeln!(string, "{} -> {} [style=\"invis\"]", pair[0].1.id, pair[1].1.id).unwrap();
            }
        }

        writeln!(string, "}}").unwrap();


        for (_, node) in others {
            for &dependency in node.dependencies() {
                let dependency = &self[dependency];
                writeln!(string, "{} -> {}", dependency.id, node.id).unwrap();
            }
        }

        writeln!(string, "}}").unwrap();
    }
}

impl Index<NodeIndex> for SubjectGraph {
    type Output = Node;
    fn index(&self, index: NodeIndex) -> &Node {
        Index::index(&self.nodes, index.0)
    }
}

impl IndexMut<NodeIndex> for SubjectGraph {
    fn index_mut(&mut self, index: NodeIndex) -> &mut Node {
        IndexMut::index_mut(&mut self.nodes, index.0)
    }
}

#[derive(Clone, Debug)]
struct Id(u32);

impl fmt::Display for Id {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

#[derive(Default)]
struct IdGenerator(u32);

impl IdGenerator {
    fn next(&mut self) -> Id {
        self.0 = self.0.checked_add(1).unwrap();
        Id(self.0)
    }
}


#[derive(Debug, Clone)]
struct Node {
    kind: NodeKind,
    dependencies: Vec<NodeIndex>,
    id: Id,
}

impl Node {
    fn kind(&self) -> &NodeKind {
        &self.kind
    }

    fn dependencies(&self) -> &[NodeIndex] {
        &self.dependencies
    }

    fn is_conjunctive(&self, conj: Conjunctive) -> bool {
        self.kind == NodeKind::Conjunctive(conj)
    }

    fn is_qualification(&self, qualification: &Qualification) -> bool {
        match &self.kind {
            NodeKind::Qualification(qual) => qual == qualification,
            _ => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum NodeKind {
    Qualification(Qualification),
    Conjunctive(Conjunctive),
}

#[derive(Copy, Clone, PartialOrd, PartialEq, Eq, Ord, Hash)]
struct NodeIndex(pub usize);

impl fmt::Debug for NodeIndex {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

fn integer_square_root(n: u64) -> u64 {
    if n == 0 { return 0 }
    let mut x = n;
    let result = loop {
        let x_prev = x;
        x = (x + n / x) / 2;
        if x_prev == x || x_prev + 1 == x {
            break x_prev;
        }
    };
    result
}
