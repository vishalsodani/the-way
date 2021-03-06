//! Fuzzy search capabilities
use std::borrow::Cow;
use std::sync::Arc;

use skim::prelude::{unbounded, SkimOptionsBuilder};
use skim::{AnsiString, ItemPreview, Skim, SkimItem, SkimItemReceiver, SkimItemSender};

use crate::errors::LostTheWay;
use crate::language::Language;
use crate::the_way::{snippet::Snippet, TheWay};

/// searchable snippet information
#[derive(Debug)]
struct SearchSnippet {
    snippet: Snippet,
    /// Highlighted title
    text_highlight: String,
    /// Highlighted code
    code_highlight: String,
}

impl<'a> SkimItem for SearchSnippet {
    fn display(&self) -> Cow<AnsiString> {
        Cow::Owned(AnsiString::parse(&self.text_highlight))
    }

    fn text(&self) -> Cow<str> {
        Cow::Owned(self.snippet.get_header())
    }

    fn preview(&self) -> ItemPreview {
        ItemPreview::AnsiText(self.code_highlight.to_owned())
    }

    fn output(&self) -> Cow<str> {
        self.snippet.copy().expect("Clipboard Error");
        Cow::Owned(String::new())
    }
}

impl TheWay {
    /// Converts a list of snippets into searchable objects and opens the search window
    pub(crate) fn make_search(
        &self,
        snippets: Vec<Snippet>,
        highlight_color: &str,
    ) -> color_eyre::Result<()> {
        let default_language = Language::default();
        let search_snippets: Vec<_> = snippets
            .into_iter()
            .map(|snippet| SearchSnippet {
                code_highlight: snippet
                    .pretty_print_code(&self.highlighter)
                    .unwrap_or_default()
                    .join(""),
                text_highlight: snippet
                    .pretty_print_header(
                        &self.highlighter,
                        self.languages
                            .get(&snippet.language)
                            .unwrap_or(&default_language),
                    )
                    .unwrap_or_default()
                    .join(""),
                snippet,
            })
            .collect();
        search(search_snippets, highlight_color)?;
        Ok(())
    }
}

/// Makes a fuzzy search window with the bottom panel listing each snippet's index, description,
/// language and tags (all searchable) and the top panel showing the code for the selected snippet.
fn search(input: Vec<SearchSnippet>, highlight_color: &str) -> color_eyre::Result<()> {
    let color = format!("bg+:{}", highlight_color);
    let options = SkimOptionsBuilder::default()
        .height(Some("100%"))
        .preview(Some(""))
        .preview_window(Some("up:70%"))
        .multi(true)
        .reverse(true)
        .color(Some(&color))
        .build()
        .map_err(|_| LostTheWay::SearchError)?;

    let (tx_item, rx_item): (SkimItemSender, SkimItemReceiver) = unbounded();
    for item in input {
        let _ = tx_item.send(Arc::new(item));
    }
    drop(tx_item); // so that skim could know when to stop waiting for more items.

    let selected_items =
        Skim::run_with(&options, Some(rx_item)).map_or_else(Vec::new, |out| out.selected_items);
    for item in &selected_items {
        println!("{}", item.output());
    }
    Ok(())
}
