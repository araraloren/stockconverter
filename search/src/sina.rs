use std::time::{SystemTime, UNIX_EPOCH};

use color_eyre::eyre::eyre;
use neure::{neu::whitespace, prelude::*};
use reqwest::Client;

use crate::{Exchange, Stock};

#[derive(Debug)]
pub struct Sina {
    pub reqwest: Client,
}

impl Sina {
    pub async fn init(builder: reqwest::ClientBuilder) -> color_eyre::Result<Self> {
        let reqwest = builder.build()?;
        let res = reqwest.get("https://finance.sina.com.cn").send().await?;

        if !res.status().is_success() {
            return Err(eyre!("Can not access sina website: {}", res.status()));
        }

        Ok(Self { reqwest })
    }
}

#[derive(Debug, Clone)]
pub struct Input {
    pub key: String,

    pub count: usize,
}

impl Default for Input {
    fn default() -> Self {
        let time = SystemTime::now();
        let elpased = time.duration_since(UNIX_EPOCH).expect("Ooop!");

        Self {
            key: Default::default(),
            count: elpased.as_millis() as usize,
        }
    }
}

impl crate::QueryInput for Input {
    fn set_keyword(&mut self, keyword: String) {
        self.key = keyword;
    }

    fn reset(&mut self) {
        self.count += 1;
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

impl crate::Search for Sina {
    type Input = Input;
    type Output = Output;

    async fn search_all(&self, info: &Self::Input) -> color_eyre::Result<Vec<Self::Output>> {
        let url = format!(
            "https://suggest3.sinajs.cn/suggest/type=&key={}&name=suggestdata_{}",
            info.key, info.count
        );

        let res = self.reqwest.get(url).send().await?;
        let text = res.text().await?;

        let name = '='.not().repeat_full();
        let fields = re::array([';', ',', '"']).not().repeat_full().sep(",");
        let parser = "var"
            .sep_once(
                whitespace().repeat_full(),
                name.sep_once("=", fields.sep(";").quote("\"", "\"")),
            )
            .map(|(_, (_, fields))| Ok(fields));
        let suggests = CharsCtx::new(&text)
            .ignore(whitespace().repeat_full())
            .ctor(&parser)?;

        Ok(suggests
            .into_iter()
            .filter(|v| v.len() >= 3)
            .map(|v| Output {
                code: v[2].to_string(),
                name: v[0].to_string(),
            })
            .collect())
    }
}
