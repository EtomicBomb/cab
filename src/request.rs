use crate::restrictions::CourseCode;
use crate::json::Json;
use curl::easy::Easy;
use crate::jsons;
use std::{io, fs};
use std::path::Path;
use std::io::Write as IoWrite;
use std::fmt::Write as FmtWrite;

/// Scrapes Courses@Brown and saves the results locally to the specified path, printing progress messages to stderr
pub fn scrape_course_info() -> io::Result<()> {
    let save_path = Path::new("resources/scraped");

    let course_stubs = scrape_course_stubs()?;
    let course_stubs_array = course_stubs.object("results").array();

    for (i, course_stub) in course_stubs_array.iter().enumerate() {
        let course_code_string = course_stub.object("code").string();
        if course_code_string.ends_with("_XLST") { continue }
        let course_code = course_code_string.parse()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, format!("Unknown course: {}", course_code_string)))?;

        let crn: u32 = course_stub.object("crn").string().parse().unwrap();

        let percent = 100 * i / course_stubs_array.len();
        eprint!("{}% - {}\r", percent, course_code);
        io::stdout().flush()?;

        match course_details(course_code, crn) {
            Ok(details_string) => {
                let course_dir = save_path.join(course_code.to_string());
                fs::create_dir_all(&course_dir)?;
                fs::write(
                    course_dir.join(crn.to_string()).with_extension("json"),
                    details_string,
                )?;
            },
            Err(e) => eprintln!("Couldn't find course '{}': {}", course_code, e),
        }
    }

    eprintln!("100%");

    Ok(())
}

fn scrape_course_stubs() -> io::Result<Json> {
    let request = percent_encode(&jsons!({
        other: {srcdb: "999999"},
        criteria: [{field:"is_ind_study",value:"N"}],
    }));

    let result = make_request(request.as_bytes(), "https://cab.brown.edu/api/?page=fose&route=search")?;

    result.parse()
        .map_err(|_| io::Error::new(
            io::ErrorKind::InvalidData,
            format!("C@B returned invalid json: {}", result)
        ))

}

fn course_details(course_code: CourseCode, crn: u32) -> io::Result<String> {
    let request = percent_encode(&jsons!({
        srcdb: "999999",
        group: (format!("code:{}", course_code)),
        key: (format!("crn:{}", crn))
    }));

    make_request(request.as_bytes(), "https://cab.brown.edu/api/?page=fose&route=details")
}

fn make_request(request: &[u8], url: &str) -> io::Result<String> {
    let mut response = Vec::new();

    let mut easy = Easy::new();
    easy.url(url).unwrap();
    easy.post(true)?;
    easy.post_field_size(request.len() as u64)?;

    {
        let mut transfer = easy.transfer();
        transfer.read_function(|buf| {
            buf[..request.len()].copy_from_slice(request);
            Ok(request.len())
        })?;
        transfer.write_function(|buf| {
            response.extend_from_slice(buf);
            Ok(buf.len())
        })?;
        transfer.perform()?;
    }

    String::from_utf8(response)
        .map_err(|_| io::Error::new(
            io::ErrorKind::InvalidData,
            format!("C@B returned invalid utf8")
        ))

}

fn percent_encode(string: &str) -> String {
    let mut ret = String::with_capacity(2*string.len());

    for b in string.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => ret.push(b as char),
            _ => write!(ret, "%{:02X}", b).unwrap(),
        }
    }

    ret
}
