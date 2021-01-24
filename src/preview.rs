use gtk;
use webkit2gtk::*;
use gtk::BoxExt;
use gtk::WidgetExt;
use pulldown_cmark::{html, Options, Parser};
use horrorshow::helper::doctype;
use horrorshow::{Raw, html};

use std::rc::Rc;
use std::ops::Deref;
use std::cell::RefCell;

use neovim_lib::NeovimApi;
use crate::nvim;
use crate::color::Color;
use crate::preview_fonts::get_katex_font_css;

pub enum PreviewType {
    Markdown,
    HTML,
    Plain,
}

struct Theme {
    bg: Color,
    fg: Color,
    bg_faded: Color,
    fg_faded: Color,
}

struct State {
    nvim: Option<Rc<nvim::NeovimClient>>,
    prev_type: PreviewType,
    should_refresh: bool,
    body_font: String,
    mono_font: String,
    katex_font_css: String,
    theme: Theme,
}

impl State {
    pub fn new() -> Self {
        State {
            nvim: None,
            prev_type: PreviewType::Plain,
            should_refresh: false,
            body_font: String::from("sans-serif"),
            mono_font: String::from("monospace"),
            katex_font_css: get_katex_font_css(),
            theme: Theme {
                bg: Color(1.0, 1.0, 1.0),
                fg: Color(0.0, 0.0, 0.0),
                bg_faded: Color(0.9725, 0.9725, 0.9725),
                fg_faded: Color(0.4666, 0.4666, 0.4666),
            },
        }
    }
}

pub struct Preview {
    container: gtk::Box,
    webview: WebView,
    state: Rc<RefCell<State>>,
}

impl Preview {
    pub fn new() -> Self {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let context = WebContext::get_default().unwrap();
        let webview = WebView::new_with_context(&context);
        let state = Rc::new(RefCell::new(State::new()));

        let settings = webkit2gtk::Settings::new();
        settings.set_enable_developer_extras(true);
        webview.set_settings(&settings);
        webview.get_inspector().unwrap().show();
        webview.set_can_focus(false);

        container.pack_start(&webview, true, true, 0);

        Preview {
            container,
            webview,
            state,
        }
    }

    pub fn activate(&self, nvim: &Rc<nvim::NeovimClient>) {
        let mut state = self.state.borrow_mut();

        if state.nvim.is_none() {
            state.nvim = Some(nvim.clone());
        }
    }

    pub fn set_type(&self, prev_type: PreviewType) {
        {
            let mut state = self.state.borrow_mut();
            state.prev_type = prev_type;
        }
        self.refresh(0, 1); // might be able to skip bufenter refresh
    }

    pub fn set_fonts(&self, body: &str, mono: &str) {
        println!("{}, {}", body, mono);
        {
            let mut state = self.state.borrow_mut();
            state.body_font = body.to_string();
            state.mono_font = mono.to_string();
        }
        self.refresh(0, 1);
    }

    pub fn is_visible(&self, should_ref: bool) {
        self.container.set_visible(should_ref);
        {
            let mut state = self.state.borrow_mut();
            state.should_refresh = should_ref;
        }
        self.refresh(0, 1);
    }

    pub fn set_theme(&self, bg: Color, fg: Color) {
        {
            let mut state = self.state.borrow_mut();
            let bg_faded = bg.clone().fade(0.05);
            let fg_faded = fg.clone().fade(0.3);
            state.theme = Theme {
                bg,
                fg,
                bg_faded,
                fg_faded,
            }
        }
        self.refresh(0, 1);
    }

    pub fn refresh(&self, line_number: i64, max_lines: i64) {
        if ! self.state.borrow().should_refresh {
            return
        }
        let state = self.state.borrow();
        let mut nvim = state.nvim.as_ref().unwrap().nvim().unwrap();
        let buffer = nvim.get_current_buf().unwrap();
        let lines = buffer.get_lines(&mut nvim, 0, -1, true).unwrap();
        let file_name = format!("file://{}", match buffer.get_name(&mut nvim).unwrap().as_str() {
            "" => {
                format!("{}/temp", nvim.eval("getcwd()").unwrap().as_str().unwrap())
            },
            path => path.to_string()
        });
        match &state.prev_type {
            PreviewType::HTML => self.webview.load_html(&lines.join("\n"), Some(&file_name)),
            PreviewType::Markdown => self.webview.load_html(&self.render(&lines.join("\n"), line_number as f64 / max_lines as f64), Some(&file_name)),
            PreviewType::Plain => self.webview.load_html(&self.render(format!("```\n{}\n```", &lines.join("\n")).as_str(), line_number as f64 / max_lines as f64), None),
        }
    }

    fn mark_to_html(markdown: &str) -> String {
        let mut options = Options::empty();
        options.insert(Options::ENABLE_STRIKETHROUGH);
        options.insert(Options::ENABLE_TABLES);
        options.insert(Options::ENABLE_FOOTNOTES);
        options.insert(Options::ENABLE_TASKLISTS);
        let parser = Parser::new_ext(&markdown, options);
        let mut buffer = String::new();
        html::push_html(&mut buffer, parser);
        buffer
    }

    pub fn render(&self, markdown: &str, scroll: f64) -> String {
        let state = self.state.borrow();
        let scroll = format!(
            r#"
            let target = document.documentElement.scrollHeight * {};
            function scrollDown() {{ window.scroll(0, target); }};
            window.onload = scrollDown;
            "#,
            scroll
        );
        let fonts = format!(
            r#"
            body {{
                font-family: {}, sans-serif;
            }}
            main code {{
                font-family: {}, monospace;
            }}
            "#,
            state.body_font,
            state.mono_font,
        );
        let theme = format!(
            r#"
            body, main table tr {{
                background-color: {}
            }}
            body, main h1, main h2, main p, main code {{
                color: {}
            }}
            main code {{
                border: {}
            }}
            main blockquote {{
                border-left: 4px solid {}
            }}
            main code, main tt, main pre {{
                background-color: {}
            }}
            main h1, main h2 {{
                border-bottom: 1px solid {}
            }}
            main h6, main blockquote {{
                color: {}
            }}
            main table th, main table td, main .highlight pre, main pre {{
                border: 1px solid {}
            }}
            main table tr {{
                border-top: 1px solid {}
            }}
            "#,
            state.theme.bg.to_hex(),
            state.theme.fg.to_hex(),
            state.theme.fg.to_hex(),
            state.theme.bg_faded.to_hex(),
            state.theme.bg_faded.to_hex(),
            state.theme.fg_faded.to_hex(),
            state.theme.fg_faded.to_hex(),
            state.theme.fg_faded.to_hex(),
            state.theme.fg_faded.to_hex(),
        );
        let katex_load = r#"
            renderMathInElement(document.body, {
                "delimiters": [
                    {left: "$$", right: "$$", display: true},
                    {left: "$", right: "$", display: false},
                    {left: "\\(", right: "\\)", display: false},
                    {left: "\\[", right: "\\]", display: true}
                ]
            });
            "#;

        format!(
            "{}",
            html!(
                : doctype::HTML;
                html {
                    head {
                        style {
                            : "body { width: 95%; margin: 0 auto }";
                            : "img { max-width: 80% }";
                            : (fonts.clone());
                            : (theme.clone());
//                            : Raw(HLJS_CSS.as_str());
                            : Raw(include_str!("../resources/preview/katex/katex.css"));
                            : Raw(state.katex_font_css.clone());
                        }
                        script {
//                            : Raw(JS.as_str());
                            : Raw(include_str!("../resources/preview/katex/katex.js"));
                            : Raw(include_str!("../resources/preview/katex/auto-render.js"));
                    }
                        script {
                            : (scroll.clone());
//                            : Raw("hljs.initHighlightingOnLoad();")
                        }
                    }
                    body {
                        : Raw("<main>");
                            : Raw(&Preview::mark_to_html(markdown));
                        : Raw("</main>");
                        script {
                            : Raw(katex_load.clone());
                        }
                    }
                }
            )
        )
    }
}

impl Deref for Preview {
    type Target = gtk::Box;

    fn deref(&self) -> &gtk::Box {
        &self.container
    }
}
