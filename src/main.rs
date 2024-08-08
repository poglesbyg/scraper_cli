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

    // Define a selector to extract the headlines
    let headline_selector =
        Selector::parse("p.indicate-hover").map_err(|_| ScraperError::ParseError)?;

    // Define a list of specific unwanted headlines
    let unwanted_headlines = vec!["Connections Companion", "Spelling Bee", "The Crossword"];

    // Extract the headlines
    let headlines: Vec<String> = document
        .select(&headline_selector)
        .filter_map(|element| {
            let class_name = element.value().attr("class").unwrap_or("");
            if class_name.contains("css") {
                let text = element
                    .text()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .trim()
                    .to_string();
                if text.split_whitespace().count() > 1
                    && !unwanted_headlines.contains(&text.as_str())
                {
                    // Filter out one-word and unwanted headlines
                    return Some(text);
                }
            }
            None
        })
        .collect();

    let mut data = HashMap::new();
    data.insert("headlines".to_string(), headlines);

    Ok(data)
}
