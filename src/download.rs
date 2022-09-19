use bytes::Bytes;
use std::marker::Unpin;

use futures::prelude::*;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::io::Write as IoWrite;

use std::iter::IntoIterator;
use tokio::io::AsyncWrite;
use tokio::io::AsyncWriteExt;

// CSCI 0200
//{"group":"code:VISA 1110","key":"","srcdb":"202210","matched":"crn:17685,18097"}
//{"group":"code:VISA 1110","key":"crn:17685","srcdb":"202210","matched":"crn:17685,18097"}

pub async fn download<'a, W: AsyncWrite + Unpin>(
    client: &Client,
    terms: &'a [&'a str],
    max_connections: usize,
    mut destination: W,
) {
    let stubs = stubs(client, terms, max_connections).await;
    let mut json_chunks = course_details(client, &stubs, max_connections)
        .await
        .boxed_local();

    while let Some(mut json) = json_chunks.next().await {
        let _ = destination.write_all_buf(&mut json).await;
        let _ = destination.write_all(b"\n").await;
    }
}

struct Stub<'a> {
    crn: String,
    term: &'a str,
}

async fn stubs<'a>(client: &Client, terms: &'a [&'a str], max_connections: usize) -> Vec<Stub<'a>> {
    stream::iter(terms)
        .enumerate()
        .map(move |(i, term)| async move {
            eprint!("[{}/{}] requesting stub {term}\r", i + 1, terms.len());
            std::io::stdout().flush().unwrap();
            let crns = crns(client, term).await?;
            let stubs: Vec<_> = crns
                .into_iter()
                .map(|Crn { crn }| Stub { crn, term })
                .collect();
            Ok::<_, reqwest::Error>(stubs)
        })
        .buffer_unordered(max_connections)
        .filter_map(|b| async {
            match b {
                Ok(b) => Some(b),
                Err(e) => {
                    eprintln!("stub lookup failed: {e:?}");
                    None
                }
            }
        })
        .flat_map(stream::iter)
        .collect()
        .await
}

#[derive(Debug, Deserialize)]
struct Crn {
    crn: String,
}

async fn crns(client: &Client, term: &str) -> reqwest::Result<Vec<Crn>> {
    #[derive(Debug, Deserialize)]
    struct SearchResults {
        results: Vec<Crn>,
    }

    let result = client
        .post("https://cab.brown.edu/api/?page=fose&route=search")
        .json(&json!({
            "other": {"srcdb": term},
            "criteria": [
                {"field":"is_ind_study","value":"N"},
                {"field":"is_canc","value":"N"}
            ],
        }))
        .send()
        .await?
        .json::<SearchResults>()
        .await?
        .results;

    Ok(result)
}

async fn course_details<'a>(
    client: &'a Client,
    stubs: &'a [Stub<'_>],
    max_connections: usize,
) -> impl Stream<Item = Bytes> + 'a
where
{
    stream::iter(stubs)
        .enumerate()
        .map(move |(i, stub)| {
            eprint!(
                "[{}/{}] requesting detail {}/{}\r",
                i + 1,
                stubs.len(),
                stub.term,
                stub.crn
            );
            std::io::stdout().flush().unwrap();
            course_detail(client, stub)
        })
        .buffer_unordered(max_connections)
        .filter_map(|b| async {
            match b {
                Ok(b) => Some(b),
                Err(e) => {
                    eprintln!("course detail lookup failed: {e:?}");
                    None
                }
            }
        })
}

async fn course_detail(client: &Client, stub: &Stub<'_>) -> reqwest::Result<Bytes> {
    client
        .post("https://cab.brown.edu/api/?page=fose&route=details")
        .json(&json!({
            "srcdb": stub.term,
            "key": format!("crn:{}", stub.crn),
        }))
        .send()
        .await?
        .bytes()
        .await
}
