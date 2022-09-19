//#![allow(dead_code)]
//#![allow(unused_imports)]

mod restrictions;
mod parse_prerequisite_string;
mod graph;
mod download;
mod process;
mod logic;

use serde_json::StreamDeserializer;
use crate::process::Course;
use std::{io};
use std::collections::{HashMap};
use crate::restrictions::{Qualification};
use std::path::{Path};
use std::io::{Write};
use reqwest::Client;
use tokio::io::AsyncWriteExt;
use std::fs::File;
use serde_json::de::IoRead;

#[tokio::main]
async fn main() -> io::Result<()> {
    stage2("output/cab.jsonl", "output/minimized.jsonl")?;
    courses_to_svg("output/minimized.jsonl")?;
    stage1("output/cab.jsonl").await?;
    Ok(())
}

fn courses_to_svg<I: AsRef<Path>>(input: I) -> io::Result<()> {
    let input = File::open(input)?;
    let courses: Vec<Course> = StreamDeserializer::new(IoRead::new(&input))
        .into_iter()
        .collect::<serde_json::Result<_>>()?;
    let courses = courses.into_iter().map(|course| (course.code().clone(), course)).collect();
    let svg = crate::graph::svg(&courses)?;
    let mut output = file_at("output/graphs/graph", ".svg").unwrap();
    output.write_all(svg.as_bytes()).unwrap();
    Ok(())
}

/// Input is cab.jsonl, output is courses
fn stage2<I: AsRef<Path>, O: AsRef<Path>>(input: I, output: O) -> io::Result<()> {
    let input = File::open(input)?;
    eprintln!("Reading from file");
    let mut courses = process::process(IoRead::new(&input));
    eprintln!("Read {}", courses.len());
    let minimized = courses.iter()
        .filter_map(|course| Some((Qualification::Course(course.code().clone()), course.prerequisites()?)));
    eprintln!("Minimizing");
    let minimized: HashMap<_, _> = logic::minimize(minimized).collect();
    for course in courses.iter_mut() {
        if let Some(new_tree) = minimized.get(&Qualification::Course(course.code().clone())) {
            *course.prerequisites_mut() = new_tree.clone();
        }
    }
    eprintln!("Writing");
    let mut output = File::create(output)?;
    for result in courses.iter() {
        serde_json::to_writer(&mut output, result)?;
        output.write_all(b"\n")?;
    }
    Ok(())
}

async fn stage1<P: AsRef<Path>>(output: P) -> io::Result<()> {
    let terms = [
        "201600", // Summer 2016
        "201610", // Fall 2016
        "201615", // Winter 2017
        "201620", // Spring 2017
        "201700", // Summer 2017
        "201710", // Fall 2017
        "201715", // Winter 2018
        "201720", // Spring 2018
        "201800", // Summer 2018
        "201810", // Fall 2018
        "201815", // Winter 2019
        "201820", // Spring 2019
        "201900", // Summer 2019
        "201910", // Fall 2019
        "201915", // Winter 2020
        "201920", // Spring 2020
        "202000", // Summer 2020
        "202010", // Fall 2020
        "202020", // Spring 2021
        "202100", // Summer 2021
        "202110", // Fall 2021
        "202115", // Winter 2022
        "202120", // Spring 2022
        "202200", // Summer 2022
        "202210", // Fall 2022
        "202215", // Winter 2023
        "202220", // Spring 2023
    ];
    let client = Client::builder()
        .build()
        .expect("client not available");
    let mut output = tokio::fs::File::create(output).await.unwrap();
    download::download(&client, &terms, 10, &mut output).await;
    output.shutdown().await.unwrap();
    Ok(())
}

fn file_at(path: &str, extension: &str) -> io::Result<File> {
    let mut number = 0;
    loop {
        number += 1;
        let file = File::options()
            .create_new(true)
            .write(true)
            .open(format!("{path}{number}{extension}"));
        match file {
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {},
            file => return file,
        }
    }
}
