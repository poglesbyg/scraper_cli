use clap::{ArgGroup, Parser};
use reqwest;
use scraper::{Html, Selector};
use serde_json::Value;
use std::collections::HashMap;
use thiserror::Error;
use vader_sentiment::SentimentIntensityAnalyzer;

#[derive(Parser)]
#[command(name = "Scraper CLI")]
#[command(about = "A simple web scraper for extracting headlines and performing sentiment analysis", long_about = None)]
#[command(group(ArgGroup::new("mode").required(true).args(&["url", "all"])))]
struct Args {
    /// URL to scrape
    #[arg(short, long, group = "mode")]
    url: Option<String>,

    /// Analyze all sources
    #[arg(short, long, group = "mode")]
    all: bool,
}

#[derive(Debug, Error)]
enum ScraperError {
    #[error("Network request error: {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("Failed to parse response")]
    ParseError,
}

const SOURCES: &[&str] = &[
    "https://www.nytimes.com",
    "https://www.theguardian.com",
    "https://www.bbc.com",
    "https://www.nature.com",
    "https://www.economist.com",
];

#[tokio::main]
async fn main() -> Result<(), ScraperError> {
    let args = Args::parse();

    if args.all {
        let mut all_headlines = Vec::new();
        for source in SOURCES {
            let headlines = fetch_website_data(source).await?;
            all_headlines.extend(headlines.get("headlines").unwrap().clone());
        }
        let sentiment_results = perform_sentiment_analysis(&all_headlines)?;
        print_sentiment_results(&sentiment_results);
    } else if let Some(url) = args.url {
        let headlines = fetch_website_data(&url).await?;
        let headlines_list = headlines.get("headlines").unwrap();
        let sentiment_results = perform_sentiment_analysis(headlines_list)?;
        print_sentiment_results(&sentiment_results);
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
    } else if url.contains("bbc.com") {
        (
            Selector::parse("h2[data-testid='card-headline']")
                .map_err(|_| ScraperError::ParseError)?,
            None,
        )
    } else if url.contains("nature.com") {
        (
            Selector::parse("a.c-card__link").map_err(|_| ScraperError::ParseError)?,
            None,
        )
    } else if url.contains("economist.com") {
        (
            Selector::parse("a[data-analytics]").map_err(|_| ScraperError::ParseError)?,
            None,
        )
    } else {
        return Err(ScraperError::ParseError);
    };

    // Define a list of specific unwanted headlines
    let unwanted_headlines = vec![
        "Connections Companion",
        "Spelling Bee",
        "The Crossword",
        "Read full edition",
    ];

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

fn perform_sentiment_analysis(
    headlines: &Vec<String>,
) -> Result<Vec<HashMap<String, Value>>, ScraperError> {
    let analyzer = SentimentIntensityAnalyzer::new();
    let mut results = Vec::new();

    for headline in headlines {
        let sentiment = analyzer.polarity_scores(headline);
        let sentiment_value = sentiment.get("compound").unwrap_or(&0.0);

        let mut result = HashMap::new();
        result.insert("headline".to_string(), Value::String(headline.clone()));
        result.insert(
            "sentiment".to_string(),
            Value::Number(serde_json::Number::from_f64(*sentiment_value).unwrap()),
        );

        results.push(result);
    }

    Ok(results)
}

fn print_sentiment_results(results: &Vec<HashMap<String, Value>>) {
    for result in results {
        println!(
            "Headline: {}\nSentiment: {}\n",
            result["headline"], result["sentiment"]
        );
    }

    let average_sentiment: f64 = results
        .iter()
        .map(|result| result["sentiment"].as_f64().unwrap())
        .sum::<f64>()
        / results.len() as f64;

    println!("Overall Sentiment: {}\n", average_sentiment);
}
