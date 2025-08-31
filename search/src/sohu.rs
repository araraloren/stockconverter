use std::{
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

use color_eyre::eyre::eyre;
use reqwest::Client;

use crate::{Exchange, Stock};

#[derive(Debug)]
pub struct SoHu {
    pub reqwest: Client,
}

impl SoHu {
    pub async fn init(builder: reqwest::ClientBuilder) -> color_eyre::Result<Self> {
        let reqwest = builder.build()?;
        let res = reqwest.get("https://q.stock.sohu.com").send().await?;

        if !res.status().is_success() {
            return Err(eyre!("Can not access sohu website: {}", res.status()));
        }

        Ok(Self { reqwest })
    }
}

#[derive(Debug, Clone)]
pub struct Input {
    pub key: String,

    pub time: usize,
}

impl Default for Input {
    fn default() -> Self {
        let time = SystemTime::now();
        let elpased = time.duration_since(UNIX_EPOCH).expect("Ooop!");

        Self {
            key: Default::default(),
            time: elpased.as_millis() as usize,
        }
    }
}

impl crate::QueryInput for Input {
    fn set_keyword(&mut self, keyword: String) {
        self.key = keyword;
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
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

impl crate::Search for SoHu {
    type Input = Input;
    type Output = Output;

    async fn search_all(&self, info: &Self::Input) -> color_eyre::Result<Vec<Self::Output>> {
        use neure::prelude::*;

        let (key, _, _) = encoding_rs::GBK.encode(&info.key);
        let key = urlencoding::encode_binary(&key);
        let method = "search";
        let callback = "searchBox1.output";
        let ty = "all";
        let url = format!(
            "https://q.stock.sohu.com/app1/stockSearch?method={}&callback={}&type={}&keyword={}&_={}",
            method, callback, ty, key, info.time
        );
        let res = self.reqwest.get(url).send().await?;
        let text = res.text().await?;

        let item: neu::Not<[char; 2], char> = re::array(['(', ')']).not();
        let parser = item.repeat_full().then(item.repeat_full().quote("(", ")"));
        let json = CharsCtx::new(&text).ctor(&parser).map(|(_, a)| a)?;
        let json = serde_json::Value::from_str(json)?;
        let array = json
            .get("result")
            .and_then(|v| v.as_array())
            .ok_or_else(|| eyre!("Invalid json format"))?;

        let mut outputs = vec![];

        for item in array {
            if let Some(array) = item.as_array().filter(|v| v.len() >= 3)
                && let (Some(code), Some(name)) = (
                    array.get(1).and_then(|v| v.as_str()),
                    array.get(2).and_then(|v| v.as_str()),
                )
            {
                let code = code.to_string();
                let name = name.chars().filter(|v| !v.is_ascii()).collect::<String>();

                outputs.push(Output { code, name });
            }
        }

        Ok(outputs)
    }
}
