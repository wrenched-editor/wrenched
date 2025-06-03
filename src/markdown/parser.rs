use pulldown_cmark::{
    BlockQuoteKind, BrokenLinkCallback, Event, HeadingLevel, Options, Parser, Tag,
    TagEnd,
};
use tracing::{error, warn};

use super::{
    elements::MarkdownContent,
    text::{
        styles::{MarkerKind, TextMarker},
        Link,
    },
};
use crate::{
    layout_flow::LayoutFlow,
    markdown::{
        elements::{
            CodeBlock, Header, HorizontalLine, IndentationDecoration, Indented,
            ListMarker, MarkdownList, Paragraph,
        },
        text::{InlinedImage, MarkdownText},
    },
};

pub struct MarkerState {
    bold_start: usize,
    italic_start: usize,
    strikethrough_start: usize,
    markers: Vec<TextMarker>,
    links: Vec<Link>,
    link_url: String,
    link_start: usize,
}

impl MarkerState {
    fn new() -> Self {
        Self {
            bold_start: 0,
            italic_start: 0,
            strikethrough_start: 0,
            markers: Vec::new(),
            links: Vec::new(),
            link_url: "".into(),
            link_start: 0,
        }
    }

    fn clear(&mut self) {
        self.markers.clear();
        self.links.clear();
    }

    fn process_marker(&mut self, event: &Event, text_end: usize) -> bool {
        match event {
            Event::Start(Tag::Strong) => {
                self.bold_start = text_end;
                true
            }
            Event::Start(Tag::Emphasis) => {
                self.italic_start = text_end;
                true
            }
            Event::Start(Tag::Strikethrough) => {
                self.strikethrough_start = text_end;
                true
            }
            Event::Start(Tag::Link {
                link_type: _,
                dest_url,
                title: _,
                id: _,
            }) => {
                self.link_url = dest_url.to_string();
                self.link_start = text_end;
                true
            }
            Event::End(TagEnd::Strong) => {
                self.markers.push(TextMarker {
                    start_pos: self.bold_start,
                    end_pos: text_end,
                    kind: MarkerKind::Bold,
                });
                true
            }
            Event::End(TagEnd::Emphasis) => {
                self.markers.push(TextMarker {
                    start_pos: self.strikethrough_start,
                    end_pos: text_end,
                    kind: MarkerKind::Italic,
                });
                true
            }
            Event::End(TagEnd::Strikethrough) => {
                self.markers.push(TextMarker {
                    start_pos: self.strikethrough_start,
                    end_pos: text_end,
                    kind: MarkerKind::Strikethrough,
                });
                true
            }
            Event::End(TagEnd::Link) => {
                self.links.push(Link {
                    url: self.link_url.clone(),
                    index_range: self.link_start..text_end,
                });
                true
            }
            _ => false,
        }
    }
}

impl Default for MarkerState {
    fn default() -> Self {
        Self::new()
    }
}

fn process_header_events<'a, T: BrokenLinkCallback<'a>>(
    events: &mut Parser<'a, T>,
    header_level: &HeadingLevel,
) -> MarkdownContent {
    let mut text = String::new();
    let mut marker_state = MarkerState::new();
    for event in events {
        if marker_state.process_marker(&event, text.len()) {
            continue;
        }
        match event {
            Event::Text(cow_str) => text.push_str(&cow_str),
            Event::End(TagEnd::Heading(_)) => {
                let text = MarkdownText::new(
                    text,
                    marker_state.markers,
                    Vec::new(),
                    Vec::new(),
                );
                return MarkdownContent::Header(Header::new(text, *header_level));
            }
            e => {
                error!("Header tag parsing expects only some event but {e:?} was received")
            }
        }
    }
    panic!("Header tag parsing expects Heading end tag and none was received");
}

fn process_code_block_events<'a, T: BrokenLinkCallback<'a>>(
    events: &mut Parser<'a, T>,
    language: Option<String>,
) -> MarkdownContent {
    let mut text = String::new();
    for event in events {
        match event {
            Event::Text(cow_str) => text.push_str(&cow_str),
            Event::End(TagEnd::CodeBlock) => {
                return MarkdownContent::CodeBlock(CodeBlock::new(text, language));
            }
            e => {
                error!("Header tag parsing expects only some event but {e:?} was received")
            }
        }
    }
    panic!("Header tag parsing expects Heading end tag and none was received");
}

fn discar_html_block_events<'a, T: BrokenLinkCallback<'a>>(
    events: &mut Parser<'a, T>,
) {
    for event in events.by_ref() {
        println!("Event: {event:?}");
        if let Event::End(TagEnd::HtmlBlock) = event {
            break;
        }
    }
}

fn process_list_events<'a, T: BrokenLinkCallback<'a>>(
    events: &mut Parser<'a, T>,
) -> Vec<LayoutFlow<MarkdownContent>> {
    let mut list_elements = Vec::new();

    while let Some(event) = events.next() {
        println!("Event: {event:?}");
        if let Event::Start(Tag::Item) = event {
            list_elements
                .push(process_events(events, Some(Event::End(TagEnd::Item))));
        } else if let Event::End(TagEnd::List(_)) = event {
            break;
        } else {
            panic!("List tag parsing expects List end tag; received {event:?}");
        }
    }
    list_elements
}

fn process_events<'a, T: BrokenLinkCallback<'a>>(
    events: &mut Parser<'a, T>,
    untill: Option<Event>,
) -> LayoutFlow<MarkdownContent> {
    let mut res = LayoutFlow::new();

    let mut text = String::new();
    let mut marker_state = MarkerState::new();
    let mut inline_images = Vec::new();

    while let Some(event) = events.next() {
        println!("Event: {event:?}");
        if let Some(event_) = &untill {
            if &event == event_ {
                break;
            }
        }
        if marker_state.process_marker(&event, text.len()) {
            continue;
        }
        match event {
            Event::Start(tag) => match &tag {
                Tag::Image {
                    link_type: _,
                    dest_url,
                    title: _,
                    id: _,
                } => {
                    // TODO: Use title and alt text.
                    let _some_text = process_image_events(events);
                    inline_images
                        .push(InlinedImage::new(dest_url.to_string(), text.len()));
                }
                Tag::CodeBlock(kind) => {
                    let lanauge = match kind {
                        pulldown_cmark::CodeBlockKind::Indented => None,
                        pulldown_cmark::CodeBlockKind::Fenced(language) => {
                            if language.is_empty() {
                                None
                            } else {
                                Some(language.to_string())
                            }
                        }
                    };
                    res.push(process_code_block_events(events, lanauge));
                }
                Tag::Table(_alignments) => {
                    warn!("Markdown tables not supported")
                }
                Tag::Paragraph => {}
                Tag::Heading {
                    level,
                    id: _,
                    classes: _,
                    attrs: _,
                } => res.push(process_header_events(events, level)),
                Tag::BlockQuote(block_quote_kind) => {
                    let flow = process_events(
                        events,
                        Some(Event::End(TagEnd::BlockQuote(*block_quote_kind))),
                    );
                    let decoration = match block_quote_kind {
                        Some(BlockQuoteKind::Note) => IndentationDecoration::Note,
                        Some(BlockQuoteKind::Tip) => IndentationDecoration::Tip,
                        Some(BlockQuoteKind::Important) => {
                            IndentationDecoration::Important
                        }
                        Some(BlockQuoteKind::Warning) => {
                            IndentationDecoration::Warning
                        }
                        Some(BlockQuoteKind::Caution) => {
                            IndentationDecoration::Caution
                        }
                        None => IndentationDecoration::Indentation,
                    };
                    res.push(MarkdownContent::Indented(Indented::new(
                        decoration, flow,
                    )));
                }
                Tag::HtmlBlock => {
                    warn!("HtmlBlock is ignored");
                    discar_html_block_events(events);
                }
                Tag::List(list_marker) => {
                    if !text.is_empty() {
                        // Some elements don't insert text into paragraphs. So we need to do
                        // last check for unprocessed text.
                        res.push(MarkdownContent::Paragraph(Paragraph::new(
                            MarkdownText::new(
                                text.clone(),
                                marker_state.markers.clone(),
                                inline_images.clone(),
                                marker_state.links.clone(),
                            ),
                        )));
                        text.clear();
                        marker_state.markers.clear();
                        inline_images.clear();
                        marker_state.links.clear();
                    }
                    let list = process_list_events(events);
                    // TODO: Think about the markers. There should be a better way to set them up
                    let marker = if let Some(list_marker) = list_marker {
                        ListMarker::Numbers {
                            start_number: *list_marker as u32,
                            layouted: Vec::new(),
                        }
                    } else {
                        ListMarker::Symbol {
                            symbol: Box::new("•".to_string().into()),
                        }
                    };
                    res.push(MarkdownContent::List(MarkdownList::new(list, marker)));
                }
                Tag::FootnoteDefinition(_cow_str) => todo!(),
                Tag::DefinitionList => {
                    warn!("DefinitionList in markdown is not supported!")
                }
                Tag::DefinitionListTitle => {
                    warn!("DefinitionList in markdown is not supported!")
                }
                Tag::DefinitionListDefinition => {
                    warn!("DefinitionList in markdown is not supported!")
                }
                Tag::TableHead => todo!(),
                Tag::TableRow => todo!(),
                Tag::TableCell => todo!(),
                Tag::MetadataBlock(_metadata_block_kind) => {
                    warn!("MetadataBlock in markdown are not supported")
                }
                _ => {}
            },
            Event::End(end_tag) => {
                match end_tag {
                    TagEnd::Paragraph => {
                        // TODO: Work on the links and inlined_images
                        if !text.trim().is_empty() || !inline_images.is_empty() {
                            res.push(MarkdownContent::Paragraph(Paragraph::new(
                                MarkdownText::new(
                                    text.clone(),
                                    marker_state.markers.clone(),
                                    inline_images.clone(),
                                    marker_state.links.clone(),
                                ),
                            )));
                            text.clear();
                            marker_state.clear();
                            inline_images.clear();
                        }
                    }
                    TagEnd::FootnoteDefinition => todo!(),
                    TagEnd::Table => todo!(),
                    TagEnd::TableHead => todo!(),
                    TagEnd::TableRow => todo!(),
                    TagEnd::TableCell => todo!(),
                    e => {
                        warn!("Markdown parsing unprocessed end tag: {e:?}");
                    }
                }
            }
            Event::Text(text_bit) => {
                text.push_str(&text_bit);
            }
            Event::Code(text_bit) => {
                // TODO: Maybe it should be a text_manager with both text and markers.
                marker_state.markers.push(TextMarker {
                    start_pos: text.len(),
                    end_pos: text.len() + text_bit.len(),
                    kind: MarkerKind::InlineCode,
                });
                text.push_str(&text_bit);
            }
            Event::Html(text_bit) => {
                // TODO: This looks a bit fishy
                marker_state.markers.push(TextMarker {
                    start_pos: text.len(),
                    end_pos: text.len() + text_bit.len(),
                    kind: MarkerKind::InlineCode,
                });
                text.push_str(&text_bit);
            }
            Event::HardBreak => {
                text.push('\n');
            }
            Event::SoftBreak => {
                text.push(' ');
            }
            Event::Rule => {
                res.push(MarkdownContent::HorizontalLine(HorizontalLine::new()));
            }
            Event::FootnoteReference(_text) => {
                warn!("FootnoteReference in markdown is not supported!")
            }
            Event::TaskListMarker(_marker) => {
                warn!("TaskListMarker in markdown is not supported!")
            }
            Event::InlineHtml(_) => {
                warn!("InlineHtml in markdown is not supported!")
            }
            Event::InlineMath(_) => {
                warn!("InlineMath in markdown is not supported!")
            }
            Event::DisplayMath(_) => {
                warn!("DisplayMath in markdown is not supported!")
            }
        }
    }

    if !text.is_empty() {
        // Some elements don't insert text into paragraphs. So we need to do
        // last check for unprocessed text.
        res.push(MarkdownContent::Paragraph(Paragraph::new(
            MarkdownText::new(
                text.clone(),
                marker_state.markers.clone(),
                inline_images.clone(),
                marker_state.links.clone(),
            ),
        )));
    }

    res
}

pub fn parse_markdown(text: &str) -> LayoutFlow<MarkdownContent> {
    let mut parser = Parser::new_ext(
        text,
        //Options::ENABLE_TABLES
        //| Options::ENABLE_FOOTNOTES
        //| Options::ENABLE_STRIKETHROUGH
        Options::ENABLE_STRIKETHROUGH //| Options::ENABLE_TASKLISTS
        | Options::ENABLE_GFM, //| Options::ENABLE_HEADING_ATTRIBUTES,
    );

    process_events(&mut parser, None)
}

fn process_image_events<'a, T: BrokenLinkCallback<'a>>(
    events: &mut Parser<'a, T>,
) -> String {
    let mut text = String::new();
    for event in events {
        match event {
            Event::Text(cow_str) => text = cow_str.to_string(),
            Event::End(TagEnd::Image) => return text,
            e => {
                error!("Image tag parsing expects only Text event but {e:?} was received")
            }
        }
    }
    error!("Image tag parsing expects Image End tag and none was received");
    String::new()
}
