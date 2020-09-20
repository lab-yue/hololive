use clap::{App, Arg};
use colored::*;
use pad::PadStr;
use regex::{Captures,Match, Regex};

#[derive(Debug)]
struct Stream {
    member: String,
    url: String,
    start: String,
    is_streaming: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = App::new("hololive")
        .version("v0.0.1")
        .arg(Arg::with_name("all").long("all").help("show all"))
        .get_matches();

    let res = reqwest::get("https://schedule.hololive.tv/")
        .await?
        .text()
        .await
        .unwrap();
    let mut schedule = get_schedule(&res);
    if matches.occurrences_of("all") == 0 {
        schedule.retain(|s| s.is_streaming);
    }

    for stream in schedule {
        println!(
            "{:<6} {} {:<9} {}",
            stream.start.magenta(),
            stream.member.bold().pad_to_width_with_char(15, ' '),
            if stream.is_streaming { "streaming".bright_green().to_string() } else { "".to_string() },
            stream.url,
        );
    }
    Ok(())
}

fn get_schedule(base: &str) -> Vec<Stream> {
    let re = Regex::new(
        r#"(?x)
    thumbnail"[\s\S]+?
    event_category':'(?P<member>.+?)'[\s\S]+?'
    event_label':'(?P<url>.+?)'[\s\S]+?
    height:17px;">\s+(?P<start>\S+)\s+</
    "#,
    )
    .unwrap();
    re.captures_iter(base)
        .map(|cap| Stream {
            member: match_or_empty(cap.name("member") ),
            url: match_or_empty(cap.name("url") ),
            start: match_or_empty(cap.name("start")),
            is_streaming: match_or_empty(cap.get(0)).contains("red solid"),
        })
        .collect::<Vec<Stream>>()
}

fn match_or_empty(maybe_match: Option<Match>) -> String {
    match maybe_match {
        Some(matched) => matched.as_str().to_string(),
        _ => "".to_string(),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_get_schedule() {
        let test_html = include_str!("./test.html");
        let mut schedule = crate::get_schedule(test_html);
        assert_eq!(schedule.len(), 69);
        schedule.retain(|s| s.is_streaming);
        assert_eq!(schedule.len(), 6);
    }
}
