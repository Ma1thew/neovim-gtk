use base64::encode;

pub fn get_katex_font_css() -> String {
format!(r#"
@font-face {{
  font-family: 'KaTeX_AMS';
  src: url("data:font/woff2;charset=utf-8;base64,{}") format('woff2');
  font-weight: normal;
  font-style: normal;
}}
@font-face {{
  font-family: 'KaTeX_Caligraphic';
  src: url("data:font/woff2;charset=utf-8;base64,{}") format('woff2');
  font-weight: bold;
  font-style: normal;
}}
@font-face {{
  font-family: 'KaTeX_Caligraphic';
  src: url("data:font/woff2;charset=utf-8;base64,{}") format('woff2');
  font-weight: normal;
  font-style: normal;
}}
@font-face {{
  font-family: 'KaTeX_Fraktur';
  src: url("data:font/woff2;charset=utf-8;base64,{}") format('woff2');
  font-weight: bold;
  font-style: normal;
}}
@font-face {{
  font-family: 'KaTeX_Fraktur';
  src: url("data:font/woff2;charset=utf-8;base64,{}") format('woff2');
  font-weight: normal;
  font-style: normal;
}}
@font-face {{
  font-family: 'KaTeX_Main';
  src: url("data:font/woff2;charset=utf-8;base64,{}") format('woff2');
  font-weight: bold;
  font-style: normal;
}}
@font-face {{
  font-family: 'KaTeX_Main';
  src: url("data:font/woff2;charset=utf-8;base64,{}") format('woff2');
  font-weight: bold;
  font-style: italic;
}}
@font-face {{
  font-family: 'KaTeX_Main';
  src: url("data:font/woff2;charset=utf-8;base64,{}") format('woff2');
  font-weight: normal;
  font-style: italic;
}}
@font-face {{
  font-family: 'KaTeX_Main';
  src: url("data:font/woff2;charset=utf-8;base64,{}") format('woff2');
  font-weight: normal;
  font-style: normal;
}}
@font-face {{
  font-family: 'KaTeX_Math';
  src: url("data:font/woff2;charset=utf-8;base64,{}") format('woff2');
  font-weight: bold;
  font-style: italic;
}}
@font-face {{
  font-family: 'KaTeX_Math';
  src: url("data:font/woff2;charset=utf-8;base64,{}") format('woff2');
  font-weight: normal;
  font-style: italic;
}}
@font-face {{
  font-family: 'KaTeX_SansSerif';
  src: url("data:font/woff2;charset=utf-8;base64,{}") format('woff2');
  font-weight: bold;
  font-style: normal;
}}
@font-face {{
  font-family: 'KaTeX_SansSerif';
  src: url("data:font/woff2;charset=utf-8;base64,{}") format('woff2');
  font-weight: normal;
  font-style: italic;
}}
@font-face {{
  font-family: 'KaTeX_SansSerif';
  src: url("data:font/woff2;charset=utf-8;base64,{}") format('woff2');
  font-weight: normal;
  font-style: normal;
}}
@font-face {{
  font-family: 'KaTeX_Script';
  src: url("data:font/woff2;charset=utf-8;base64,{}") format('woff2');
  font-weight: normal;
  font-style: normal;
}}
@font-face {{
  font-family: 'KaTeX_Size1';
  src: url("data:font/woff2;charset=utf-8;base64,{}") format('woff2');
  font-weight: normal;
  font-style: normal;
}}
@font-face {{
  font-family: 'KaTeX_Size2';
  src: url("data:font/woff2;charset=utf-8;base64,{}") format('woff2');
  font-weight: normal;
  font-style: normal;
}}
@font-face {{
  font-family: 'KaTeX_Size3';
  src: url("data:font/woff2;charset=utf-8;base64,{}") format('woff2');
  font-weight: normal;
  font-style: normal;
}}
@font-face {{
  font-family: 'KaTeX_Size4';
  src: url("data:font/woff2;charset=utf-8;base64,{}") format('woff2');
  font-weight: normal;
  font-style: normal;
}}
@font-face {{
  font-family: 'KaTeX_Typewriter';
  src: url("data:font/woff2;charset=utf-8;base64,{}") format('woff2');
  font-weight: normal;
  font-style: normal;
}}
.katex {{
  font: normal 1.21em KaTeX_Main, Times New Roman, serif;
  line-height: 1.2;
  text-indent: 0;
  text-rendering: auto;
  border-color: currentColor;
}}
"#,
encode(include_bytes!("../resources/preview/katex/fonts/KaTeX_AMS-Regular.woff2")),
encode(include_bytes!("../resources/preview/katex/fonts/KaTeX_Caligraphic-Bold.woff2")),
encode(include_bytes!("../resources/preview/katex/fonts/KaTeX_Caligraphic-Regular.woff2")),
encode(include_bytes!("../resources/preview/katex/fonts/KaTeX_Fraktur-Bold.woff2")),
encode(include_bytes!("../resources/preview/katex/fonts/KaTeX_Fraktur-Regular.woff2")),
encode(include_bytes!("../resources/preview/katex/fonts/KaTeX_Main-Bold.woff2")),
encode(include_bytes!("../resources/preview/katex/fonts/KaTeX_Main-BoldItalic.woff2")),
encode(include_bytes!("../resources/preview/katex/fonts/KaTeX_Main-Italic.woff2")),
encode(include_bytes!("../resources/preview/katex/fonts/KaTeX_Main-Regular.woff2")),
encode(include_bytes!("../resources/preview/katex/fonts/KaTeX_Math-BoldItalic.woff2")),
encode(include_bytes!("../resources/preview/katex/fonts/KaTeX_Math-Italic.woff2")),
encode(include_bytes!("../resources/preview/katex/fonts/KaTeX_SansSerif-Bold.woff2")),
encode(include_bytes!("../resources/preview/katex/fonts/KaTeX_SansSerif-Italic.woff2")),
encode(include_bytes!("../resources/preview/katex/fonts/KaTeX_SansSerif-Regular.woff2")),
encode(include_bytes!("../resources/preview/katex/fonts/KaTeX_Script-Regular.woff2")),
encode(include_bytes!("../resources/preview/katex/fonts/KaTeX_Size1-Regular.woff2")),
encode(include_bytes!("../resources/preview/katex/fonts/KaTeX_Size2-Regular.woff2")),
encode(include_bytes!("../resources/preview/katex/fonts/KaTeX_Size3-Regular.woff2")),
encode(include_bytes!("../resources/preview/katex/fonts/KaTeX_Size4-Regular.woff2")),
encode(include_bytes!("../resources/preview/katex/fonts/KaTeX_Typewriter-Regular.woff2")),
)
}
