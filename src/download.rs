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

struct Stub<'a> {
    key: String,
    srcdb: &'a str,
}

#[derive(Debug, Deserialize)]
struct StubKey {
    key: String,
}

async fn stub_keys(client: &Client, srcdb: &str) -> reqwest::Result<Vec<StubKey>> {
    #[derive(Debug, Deserialize)]
    pub struct SearchResults {
        results: Vec<StubKey>,
    }

    let result = client.post("https://cab.brown.edu/api/?page=fose&route=search")
        .json(&json!({
            "other": srcdb,
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

async fn stubs<'a: 'b, 'b, I: IntoIterator<Item=&'a str>>(
    client: &'b Client,
    srcdbs: I, 
    max_connections: usize,
) -> impl Stream<Item=Stub<'a>> + 'b 
    where <I as IntoIterator>::IntoIter: 'b
{
    stream::iter(srcdbs)
        .map(move |srcdb| async move {
            let keys = stub_keys(client, srcdb).await?;
            let stubs: Vec<_> = keys.into_iter()
                .map(|StubKey { key }| Stub { key, srcdb })
                .collect();
            Ok::<_, reqwest::Error>(stubs)
        })
        .buffer_unordered(max_connections)
        .filter_map(|stubs| async { stubs.ok() })
        .flat_map(stream::iter)
}

async fn course_detail<'a, 'b, 'c>(
    client: &'c Client, 
    stub: Stub<'a>,
) -> reqwest::Result<Bytes> {
    eprintln!("new request");
    client.post("https://cab.brown.edu/api/?page=fose&route=details")
        .json(&json!({
            "srcdb": &stub.srcdb,
            "key": format!("key:{}", stub.key),
        }))
        .send()
        .await?
        .bytes()
        .await
}

async fn course_details<'a, W: AsyncWrite + Unpin, S: Stream<Item=Stub<'a>>>(
    client: &Client, 
    stubs: S,
    max_connections: usize,
    mut destination: W,
) {
    let mut json_chunks = stubs
        .map(|stub| course_detail(client, stub))
        .buffer_unordered(max_connections)
        .filter_map(|b| async { b.ok() })
        .boxed_local();

    while let Some(mut json_chunk) = json_chunks.next().await {
        let _ = destination.write_all_buf(&mut json_chunk).await;
    }
}

pub async fn download<'a, 'b, W: AsyncWrite + Unpin, I: IntoIterator<Item=&'a str>>(
    client: &'b Client,
    srcdbs: I,
    max_connections: usize,
    destination: W,
) 
where <I as IntoIterator>::IntoIter: 'b
{
    let stubs = stubs(client, srcdbs, max_connections).await;
    course_details(client, stubs, max_connections, destination).await;
}
    
// CSCI 0200 
//{"group":"code:VISA 1110","key":"","srcdb":"202210","matched":"crn:17685,18097"}
//{"group":"code:VISA 1110","key":"crn:17685","srcdb":"202210","matched":"crn:17685,18097"}
