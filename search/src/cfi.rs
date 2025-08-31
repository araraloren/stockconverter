use color_eyre::eyre::eyre;
use reqwest::Client;

use crate::{Exchange, Stock};

#[derive(Debug)]
pub struct Cfi {
    pub reqwest: Client,
}

impl Cfi {
    pub async fn init(builder: reqwest::ClientBuilder) -> color_eyre::Result<Self> {
        let reqwest = builder.build()?;
        let res = reqwest.get("https://stock.cfi.cn").send().await?;

        if !res.status().is_success() {
            return Err(eyre!("Can not access cfi website: {}", res.status()));
        }

        Ok(Self { reqwest })
    }
}

#[derive(Debug, Clone)]
pub struct Input {
    pub key: String,

    pub his: String,

    pub longtime: String,
}

impl Default for Input {
    fn default() -> Self {
        let local = chrono::Local::now();
        let longtime = format!("{}", local.format("%Y%m%d%H%M%S%3f"));

        Self {
            key: Default::default(),
            his: String::from("pc"),
            longtime,
        }
    }
}

impl crate::QueryInput for Input {
    fn set_keyword(&mut self, keyword: String) {
        self.key = keyword;
    }
}

#[derive(Debug)]
pub struct Output {
    code: String,
    name: String,
}

impl TryFrom<Output> for Stock {
    type Error = color_eyre::Report;

    fn try_from(value: Output) -> Result<Self, Self::Error> {
        let exchange = Exchange::guess_from_stock(&value.code);

        Ok(Stock {
            name: value.name,
            code: value.code,
            exchange: exchange?,
        })
    }
}

impl crate::Search for Cfi {
    type Input = Input;
    type Output = Output;

    async fn search_all(&self, info: &Self::Input) -> color_eyre::Result<Vec<Self::Output>> {
        use neure::prelude::*;

        let url = "https://quote.cfi.cn/backgettext.aspx";
        let res = self
            .reqwest
            .get(url)
            .query(&[
                ("keyword", &info.key),
                ("his", &info.his),
                ("longtime", &info.longtime),
            ])
            .send()
            .await?;

        let text = res.text().await?;

        let stock_code = neu::digit(10).repeat_times::<6>().quote(">", "</td>");
        let stock_name = neu::ascii().not().repeat_one_more().quote(";>", "</td>");
        let mut ctx = CharsCtx::new(&text);
        let mut curr_code: Option<&str> = None;
        let mut curr_name: Option<&str> = None;
        let mut outputs = vec![];

        while !ctx.is_empty() && ctx.offset() < ctx.len() {
            if let Ok(code) = ctx.ctor(&stock_code) {
                if curr_code.is_none() || curr_name.is_none() {
                    curr_code = Some(code);
                    curr_name = None;
                } else if curr_code.is_some()
                    && curr_name.is_some()
                    && let (Some(code), Some(name)) = (curr_code.take(), curr_name.take())
                {
                    outputs.push(Output {
                        code: code.to_string(),
                        name: name.to_string(),
                    });
                }
            } else if let Ok(name) = ctx.ctor(&stock_name) {
                if curr_code.is_some() {
                    curr_name = Some(name);
                }
            } else {
                ctx.inc(1);
            }
        }

        Ok(outputs)
    }
}
