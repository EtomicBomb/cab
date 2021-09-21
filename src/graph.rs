use std::ops::{Index, IndexMut};
use crate::restrictions::{Qualification, PrerequisiteTree, Conjunctive};
use std::fmt::{self, Write, Formatter};
use crate::subject::{Subject, Subjects};
use crate::AllRestrictions;
use rand::{thread_rng, Rng};

pub struct SubjectGraphs {
    subject_graphs: Vec<SubjectGraph>,
}

impl SubjectGraphs {
    pub fn new(restrictions: &AllRestrictions) -> SubjectGraphs {
        let mut id_generator = IdGenerator::default();
        let subject_graphs = Subjects::all().iter()
            .map(|subject| SubjectGraph::new(subject, restrictions, &mut id_generator))
            .collect();

        SubjectGraphs { subject_graphs }
    }

    pub fn graphviz(&self) -> String {
        let mut ret = String::from("digraph {\npackmode=\"graph\"\n");

        for subject_graph in self.subject_graphs.iter() {
            subject_graph.graphviz_cluster(&mut ret);
        }

        ret.push_str("}");

        ret
    }
}

pub struct SubjectGraph {
    nodes: Vec<Node>,
    subject: Subject,
}

impl SubjectGraph {
    pub fn new(subject: Subject, restrictions: &AllRestrictions, id_generator: &mut IdGenerator) -> SubjectGraph {
        let mut ret = SubjectGraph { nodes: Vec::new(), subject };

        for (course, restrictions) in restrictions.iter().filter(|(course, _)| course.subject == subject) {
            let node_index = ret.insert_qualification(Qualification::Course(course), id_generator);

            if let Some(prereq_tree) = &restrictions.prerequisite_restrictions {
                ret.insert(node_index, prereq_tree, id_generator);
            }
        }

        ret
    }

    fn iter(&self) -> impl Iterator<Item=(NodeIndex, &Node)> {
        self.nodes.iter().enumerate().map(|(i, node)| (NodeIndex(i), node))
    }

    fn insert(&mut self, location: NodeIndex, prereq_tree: &PrerequisiteTree, id_generator: &mut IdGenerator) {
        let to_insert = match *prereq_tree {
            PrerequisiteTree::Qualification(qualification) => {
                self.insert_qualification(qualification, id_generator)
            }
            PrerequisiteTree::Conjunctive(conj, ref children) => {
                let found = self.nodes.iter()
                    .position(|n| n.is_conjunctive(conj) && self.is_equal(&n.dependencies, children))
                    .map(NodeIndex);

                found.unwrap_or_else(|| {
                    let new_index = NodeIndex(self.nodes.len());
                    self.nodes.push(Node {
                        kind: NodeKind::Conjunctive(conj),
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
                    PrerequisiteTree::Qualification(q) => self[d].is_qualification(*q),
                    PrerequisiteTree::Conjunctive(conj, children) => {
                        self[d].is_conjunctive(*conj)
                            && self.is_equal(&self[d].dependencies, children)
                    }
                }
            })
    }

    fn insert_qualification(&mut self, qualification: Qualification, id_generator: &mut IdGenerator) -> NodeIndex {
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

    pub fn graphviz_cluster(&self, string: &mut String) {
        let abbreviation = Subjects::all().abbreviation(self.subject);
        writeln!(string, "subgraph cluster_{} {{", abbreviation).unwrap();
        writeln!(string, "packmode=\"graph\"").unwrap();
        writeln!(string, "label=\"{}\"", Subjects::all().name(self.subject)).unwrap();

        let color = Subjects::all().color(self.subject);
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
pub struct IdGenerator(u32);

impl IdGenerator {
    fn next(&mut self) -> Id {
        self.0 = self.0.checked_add(1).unwrap();
        Id(self.0)
    }
}


#[derive(Debug, Clone)]
pub struct Node {
    kind: NodeKind,
    dependencies: Vec<NodeIndex>,
    id: Id,
}

impl Node {
    pub fn kind(&self) -> &NodeKind {
        &self.kind
    }

    pub fn dependencies(&self) -> &[NodeIndex] {
        &self.dependencies
    }

    fn is_conjunctive(&self, conj: Conjunctive) -> bool {
        self.kind == NodeKind::Conjunctive(conj)
    }

    fn is_qualification(&self, qualification: Qualification) -> bool {
        self.kind == NodeKind::Qualification(qualification)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum NodeKind {
    Qualification(Qualification),
    Conjunctive(Conjunctive),
}

#[derive(Copy, Clone, PartialOrd, PartialEq, Eq, Ord, Hash)]
pub struct NodeIndex(pub usize);

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