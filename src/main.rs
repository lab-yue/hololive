use clap::{App, Arg};
use colored::*;
use futures::StreamExt;
use pad::PadStr;
use regex::{Captures, Match, Regex};
use std::fmt;
use tokio;

#[derive(Debug, Clone)]
struct Stream {
    member: String,
    url: String,
    start: String,
    is_streaming: bool,
    title: String,
}
impl fmt::Display for Stream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:<6} {} {:<11} {} {}",
            self.start.magenta(),
            self.member.bold().pad_to_width_with_char(15, ' '),
            if self.is_streaming {
                " streaming ".black().on_bright_green().to_string()
            } else {
                "".to_string()
            },
            self.url.replace("https://www.youtube.com/watch?v=", ""),
            self.title.replace(" - YouTube", "").yellow(),
        )
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = App::new("hololive")
        .version("v0.0.1")
        .arg(Arg::with_name("a").short("a").long("all").help("show all"))
        .arg(
            Arg::with_name("t")
                .short("t")
                .long("title")
                .help("show titles"),
        )
        .get_matches();

    let text = reqwest::get("https://schedule.hololive.tv/")
        .await?
        .text()
        .await?;
    let mut schedule = get_schedule(&text, matches.occurrences_of("t") != 0).await;
    if matches.occurrences_of("a") == 0 {
        schedule.retain(|s| s.is_streaming);
    }
    for x in schedule {
        println!("{}", x);
    }

    Ok(())
}

async fn get_schedule(text: &str, with_title: bool) -> Vec<Stream> {
    let re = Regex::new(
        r#"(?x)
    thumbnail"[\s\S]+?
    event_category':'(?P<member>.+?)'[\s\S]+?'
    event_label':'(?P<url>.+?)'[\s\S]+?
    height:17px;">\s+(?P<start>\S+)\s+</
    "#,
    )
    .unwrap();
    futures::stream::iter(re.captures_iter(text).map(get_match))
        .map(|s| {
            let mut stream = s;
            async move {
                if with_title {
                    match get_url_title(&stream.url).await {
                        Ok(title) => stream.title = title,
                        _ => {}
                    }
                };
                stream
            }
        })
        .buffer_unordered(10)
        .collect::<Vec<Stream>>()
        .await
}

fn get_match<'a>(cap: Captures<'a>) -> Stream {
    Stream {
        member: match_or_empty(cap.name("member")),
        url: match_or_empty(cap.name("url")),
        start: match_or_empty(cap.name("start")),
        is_streaming: match_or_empty(cap.get(0)).contains("red solid"),
        title: "".to_string(),
    }
}

fn match_or_empty(maybe_match: Option<Match>) -> String {
    match maybe_match {
        Some(matched) => matched.as_str().to_string(),
        _ => "".to_string(),
    }
}

async fn get_url_title(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let text = reqwest::get(url).await?.text().await?;
    Ok(get_title(&text))
}

fn get_title(text: &str) -> String {
    let re = Regex::new(r#"<title>([\s\S]+?)</title>"#).unwrap();
    match re.captures(text) {
        Some(matched) => match_or_empty(matched.get(1)),
        _ => "".to_string(),
    }
}

#[cfg(test)]
mod tests {
    const TEST_HTML: &str = include_str!("./test.html");
    #[tokio::test]
    async fn test_get_schedule() -> Result<(), Box<dyn std::error::Error>> {
        let mut schedule = crate::get_schedule(TEST_HTML, false).await;
        assert_eq!(schedule.len(), 69);
        schedule.retain(|s| s.is_streaming);
        assert_eq!(schedule.len(), 6);
        Ok(())
    }
    #[test]
    fn test_get_title() {
        let title = crate::get_title(TEST_HTML);
        assert_eq!(
            title,
            "ホロライブプロダクション配信予定スケジュール『ホロジュール』".to_string()
        );
    }
}
