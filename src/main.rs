use clap::Parser;
use reqwest;
use scraper::{Html, Selector};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Parser)]
#[command(name = "Scraper CLI")]
#[command(about = "A simple web scraper for extracting headlines", long_about = None)]
struct Args {
    /// URL to scrape
    #[arg(short, long)]
    url: String,
}

#[derive(Debug, Error)]
enum ScraperError {
    #[error("Network request error: {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("Failed to parse response")]
    ParseError,
}

#[tokio::main]
async fn main() -> Result<(), ScraperError> {
    let args = Args::parse();

    let url = args.url;
    let headlines = fetch_website_data(&url).await?;

    for headline in headlines.get("headlines").unwrap() {
        println!("{}", headline);
    }

    Ok(())
}

async fn fetch_website_data(url: &str) -> Result<HashMap<String, Vec<String>>, ScraperError> {
    let response = reqwest::get(url).await?;
    let text = response.text().await?;

    // Parse the HTML
    let document = Html::parse_document(&text);

    // Determine which website to scrape from based on the URL
    let (headline_selector, attribute) = if url.contains("nytimes.com") {
        (
            Selector::parse("p.indicate-hover").map_err(|_| ScraperError::ParseError)?,
            None,
        )
    } else if url.contains("theguardian.com") {
        (
            Selector::parse("a.dcr-lv2v9o").map_err(|_| ScraperError::ParseError)?,
            Some("aria-label"),
        )
    } else {
        return Err(ScraperError::ParseError);
    };

    // Define a list of specific unwanted headlines
    let unwanted_headlines = vec!["Connections Companion", "Spelling Bee", "The Crossword"];

    // Extract the headlines
    let headlines: Vec<String> = document
        .select(&headline_selector)
        .filter_map(|element| {
            let text = match attribute {
                Some(attr) => element.value().attr(attr).unwrap_or("").trim().to_string(),
                None => element
                    .text()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .trim()
                    .to_string(),
            };
            if text.split_whitespace().count() > 1 && !unwanted_headlines.contains(&text.as_str()) {
                // Filter out one-word and unwanted headlines
                Some(text)
            } else {
                None
            }
        })
        .collect();

    let mut data = HashMap::new();
    data.insert("headlines".to_string(), headlines);

    Ok(data)
}
