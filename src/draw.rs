// use crate::restrictions::{CourseCode, RegistrationRestrictions, Qualification, Conjunctive, ScoreQualification};
// use crate::all_restrictions;
// use std::collections::HashMap;
// use crate::graph::{NodeIndex, NodeKind};
// use rand::{thread_rng, Rng};
// use crate::subject::Subjects;
// use std::fmt::Write;
//
//
// // DESIGN GOALS FOR GRAPH DRAWING:
// // similar types of subjects are near
// // same subject classes are near
// // class's dependencies are on the left of a class
// //
//
//
//
// pub struct Layout<'a> {
//     positions: HashMap<NodeIndex, GridPosition>,
//     graph: &'a Graph,
//     restrictions: &'a HashMap<CourseCode, RegistrationRestrictions>,
//
// }
//
// impl<'a> Layout<'a> {
//     pub fn from_graph(graph: &'a Graph, restrictions: &'a HashMap<CourseCode, RegistrationRestrictions>) -> Layout<'a> {
//         let positions = graph.iter()
//             .map(|(i, _)| (i, GridPosition {
//                 x: thread_rng().gen_range(-2000..2000),
//                 y: thread_rng().gen_range(-2000..2000),
//             }))
//             .collect();
//
//         Layout { positions, graph, restrictions }
//     }
//
//     pub fn graphviz(&self) -> String {
//         let mut ret = String::from("digraph {\n");
//
//         for (i, node) in self.graph.iter() {
//             match node.kind() {
//                 NodeKind::Qualification(Qualification::ExamScore(q)) => {
//                     writeln!(ret, r#"{} [label="{}",shape=box,color=blue]"#, i.0, q).unwrap();
//                 }
//                 NodeKind::Qualification(Qualification::Course(code)) => {
//                     writeln!(ret, r#"{} [label="{}",shape=box]"#, i.0, code).unwrap();
//                 }
//                 NodeKind::Conjunctive(conjunctive) => {
//                     writeln!(ret, "{} [label={}]", i.0, conjunctive).unwrap();
//                 }
//             }
//
//             for dependency in node.dependencies() {
//                 writeln!(ret, "{} -> {}", dependency.0, i.0).unwrap();
//             }
//         }
//
//         ret.push_str("}");
//
//         ret
//     }
//
//     pub fn swap_min_length(&mut self) {
//         let a = NodeIndex(thread_rng().gen_range(0..self.graph.nodes.len()));
//
//         let b = (0..self.graph.nodes.len())
//             .map(NodeIndex)
//             .min_by_key(|&b| {
//                 let b_position = self.positions[&b];
//                 self.graph[a].dependencies().iter()
//                     .map(|arrow_start_index| {
//                         let other_position = self.positions[arrow_start_index];
//                         b_position.distance_squared(other_position)
//                     })
//                     .sum::<i32>()
//             })
//             .unwrap();
//
//         let a_position = self.positions[&a];
//         let b_position = self.positions[&b];
//         self.positions.insert(a, b_position);
//         self.positions.insert(b, a_position);
//     }
//
//     pub fn svg(&self) -> String {
//         let mut min_x = i32::MAX;
//         let mut min_y = i32::MAX;
//         let mut max_x = i32::MIN;
//         let mut max_y = i32::MIN;
//
//         let mut svg_inner = String::new();
//
//         // draw lines
//         for (&node_index, &arrow_end) in self.positions.iter() {
//             for arrow_start_index in self.graph[node_index].dependencies() {
//                 let arrow_start = self.positions[arrow_start_index];
//                 writeln!(svg_inner, r##"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke="black" />"##, arrow_start.x, arrow_start.y, arrow_end.x, arrow_end.y).unwrap();
//             }
//         }
//
//         // draw nodes
//         for (&node_index, &position) in self.positions.iter() {
//             let GridPosition { x, y } = position;
//             min_x = min_x.min(x);
//             min_y = min_y.min(y);
//             max_x = max_x.max(x);
//             max_y = max_y.max(y);
//
//             match self.graph[node_index].kind() {
//                 NodeKind::Qualification(Qualification::ExamScore(q)) => {
//                     writeln!(svg_inner, r##"<rect x="{}" y="{}" width="50" height="10" style="fill:#{}" />"##, x-25, y-5, EXAM_COLOR).unwrap();
//                     writeln!(svg_inner, r##"<text x="{}" y="{}" font-size="8.0" font-family="monospace" dominant-baseline="middle" text-anchor="middle">{}</text>"##, x, y, q).unwrap();
//                 },
//                 NodeKind::Qualification(Qualification::Course(code)) => {
//                     let color = Subjects::all().color(code.subject);
//                     writeln!(svg_inner, r##"<rect x="{}" y="{}" width="50" height="10" style="fill:#{}" />"##, x-25, y-5, color).unwrap();
//                     writeln!(svg_inner, r##"<text x="{}" y="{}" font-size="8.0" font-family="monospace" dominant-baseline="middle" text-anchor="middle">{}</text>"##, x, y, code).unwrap();
//                 },
//                 NodeKind::Conjunctive(conj) => {
//                     let color = conjunctive_color(conj);
//                     writeln!(svg_inner, r##"<circle cx="{}" cy="{}" r="10" style="fill:#{}" />"##, x, y, color).unwrap();
//                     writeln!(svg_inner, r##"<text x="{}" y="{}" font-size="8.0" font-family="monospace" dominant-baseline="middle" text-anchor="middle">{}</text>"##, x, y, conj).unwrap();
//                 },
//             }
//         }
//
//         format!(r#"<svg viewBox="{} {} {} {}">{}</svg>"#, min_x, min_y, max_x-min_x, max_y-min_y, svg_inner)
//     }
// }
// //
// // #[derive(Debug, Copy, Clone)]
// // struct LineSegment {
// //     start: GridPosition,
// //     end: GridPosition,
// // }
// //
// // impl LineSegment {
// //     fn slope_intercept(self) -> (f64, f64) {
// //         let slope = (self.start.y-self.end.y) as f64 / (self.start.x-self.end.x) as f64;
// //         let intercept = self.start.y as f64 - slope*self.start.x as f64;
// //         (slope, intercept)
// //     }
// //
// //     // precondition: (x, y) must be a point on the theoretical infinitely extending line
// //     fn line_contains(self, x: f64, y: f64) -> bool {
// //         fn min_max(a: f64, b: f64) -> (f64, f64) {
// //             if a <= b { (a, b) } else { (b, a) }
// //         }
// //
// //         let (min_x, max_x) = min_max(self.start.x as f64, self.end.x as f64);
// //         let (min_y, max_y) = min_max(self.start.y as f64, self.end.y as f64);
// //
// //         min_x <= x && x <= max_x
// //             && min_y <= y && y <= max_y
// //     }
// //
// //     fn intersects_with(self, other: LineSegment) -> bool {
// //         let (self_slope, self_intercept) = self.slope_intercept();
// //         let (other_slope, other_intercept) = other.slope_intercept();
// //         if self_slope == other_slope { return false }
// //
// //         let intersection_x = (other_intercept-self_intercept) / (self_slope-other_slope);
// //         let intersection_y = self_slope * intersection_x + self_intercept;
// //
// //         self.line_contains(intersection_x, intersection_y)
// //             && other.line_contains(intersection_x, intersection_y)
// //     }
// // }
//
// const EXAM_COLOR: &'static str = "0000d0"; // blue
//
// fn conjunctive_color(conjunctive: Conjunctive) -> &'static str {
//     match conjunctive {
//         Conjunctive::All => "00d000", // green
//         Conjunctive::Any => "d00000", // red
//     }
// }
//
// #[derive(Copy, Clone)]
// pub struct GridPosition {
//     x: i32,
//     y: i32,
// }
//
// impl GridPosition {
//     fn distance_squared(self, other: GridPosition) -> i32 {
//         let dx = self.x - other.x;
//         let dy = self.y - other.y;
//         dx*dx + dy*dy
//     }
//
// }
//
