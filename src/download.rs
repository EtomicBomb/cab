use bytes::Bytes;
use std::marker::Unpin;
use crate::restrictions::CourseCode; 
use std::{io, fs};
use std::path::Path;
use std::io::Write as IoWrite;
use std::fmt::Write as FmtWrite;
use reqwest::Client;
use serde_json::json;
use serde::Deserialize;
use futures::prelude::*;
use std::convert::AsRef;
use std::iter::IntoIterator;
use tokio::io::AsyncWriteExt;
use tokio::io::AsyncWrite;
    
// CSCI 0200 
//{"group":"code:VISA 1110","key":"","srcdb":"202210","matched":"crn:17685,18097"}
//{"group":"code:VISA 1110","key":"crn:17685","srcdb":"202210","matched":"crn:17685,18097"}

pub async fn download<'a, 'b, W, I>(
    client: &'b Client,
    databases: I,
    max_connections: usize,
    destination: W,
) 
where 
    W: AsyncWrite + Unpin,
    I: IntoIterator<Item=&'a str>,
{
    let stubs = stubs(client, databases, max_connections).await;
    course_details(client, stubs, max_connections, destination).await;
}

struct Stub<'a> {
    key: String,
    database: &'a str,
}

async fn stubs<'a, 'b, I>(
    client: &'b Client,
    databases: I, 
    max_connections: usize,
) -> impl Stream<Item=Stub<'a>> + 'b 
where 
    'a: 'b,
    I: IntoIterator<Item=&'a str>,
    <I as IntoIterator>::IntoIter: 'b,
{
    stream::iter(databases)
        .map(move |database| async move {
            let keys = stub_keys(client, database).await?;
            let stubs: Vec<_> = keys.into_iter()
                .map(|StubKey { key }| Stub { key, database })
                .collect();
            Ok::<_, reqwest::Error>(stubs)
        })
        .buffer_unordered(max_connections)
        .filter_map(|stubs| async { stubs.ok() })
        .flat_map(stream::iter)
}

#[derive(Debug, Deserialize)]
struct StubKey {
    key: String,
}

async fn stub_keys(client: &Client, database: &str) -> reqwest::Result<Vec<StubKey>> {
    #[derive(Debug, Deserialize)]
    pub struct SearchResults {
        results: Vec<StubKey>,
    }

    let result = client.post("https://cab.brown.edu/api/?page=fose&route=search")
        .json(&json!({
            "other": database,
            "criteria": [{
                "field": "is_ind_study",
                "value": "N"
            }],
        }))
        .send()
        .await?
        .json::<SearchResults>()
        .await?
        .results;

    Ok(result)
}

async fn course_details<'a, W, S>(
    client: &Client, 
    stubs: S,
    max_connections: usize,
    mut destination: W,
) 
where
    W: AsyncWrite + Unpin,
    S: Stream<Item=Stub<'a>>,
{
    let mut json_chunks = stubs
        .map(|stub| course_detail(client, stub))
        .buffer_unordered(max_connections)
        .filter_map(|b| async { b.ok() })
        .boxed_local();

    while let Some(mut json) = json_chunks.next().await {
        let _ = destination.write_all_buf(&mut json).await;
        let _ = destination.write_all(b"\n").await;
    }
}

async fn course_detail<'a, 'b, 'c>(
    client: &'c Client, 
    stub: Stub<'a>,
) -> reqwest::Result<Bytes> {

    eprintln!("new request {}/{}", stub.database, stub.key);
    client.post("https://cab.brown.edu/api/?page=fose&route=details")
        .json(&json!({
            "srcdb": &stub.database,
            "key": format!("key:{}", stub.key),
        }))
        .send()
        .await?
        .bytes()
        .await
}
