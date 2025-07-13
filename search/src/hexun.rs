use color_eyre::eyre::eyre;
use reqwest::Client;

use crate::{Exchange, Stock};

#[derive(Debug)]
pub struct Hexun {
    pub reqwest: Client,
}

impl Hexun {
    pub async fn init(builder: reqwest::ClientBuilder) -> color_eyre::Result<Self> {
        let reqwest = builder.build()?;
        let res = reqwest.get("https://stock.hexun.com/").send().await?;

        if !res.status().is_success() {
            return Err(eyre!("Can not access hexun website: {}", res.status()));
        }

        Ok(Self { reqwest })
    }
}

#[derive(Debug, Clone)]
pub struct Input {
    pub key: String,

    pub ty: String,
}

impl Default for Input {
    fn default() -> Self {
        // 6871526549834742
        let local = chrono::Local::now();
        let mut ty = format!("stock?math=0.{}", local.format("%Y%m%d%H%M%S%3f"));

        ty.pop();
        Self {
            key: Default::default(),
            ty,
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
    orgcode: String,
    marketcode: String,
}

impl TryFrom<Output> for Stock {
    type Error = color_eyre::Report;

    fn try_from(value: Output) -> Result<Self, Self::Error> {
        let exchange = if value.marketcode == "a" {
            match value.orgcode.as_str() {
                "SSE" => Ok(Exchange::ShangHai),
                "SZSE" => Ok(Exchange::ShenZhen),
                "BJSE" => Ok(Exchange::BeiJing),
                e => Err(color_eyre::eyre::eyre!("Not support exchange `{}`", e)),
            }
        } else {
            Err(color_eyre::eyre::eyre!(
                "Not support market `{}`",
                value.marketcode
            ))
        };

        Ok(Stock {
            name: value.name,
            code: value.code,
            exchange: exchange?,
        })
    }
}

impl crate::Search for Hexun {
    type Input = Input;
    type Output = Output;

    async fn search_all(&self, info: &Self::Input) -> color_eyre::Result<Vec<Self::Output>> {
        let (key, _, _) = encoding_rs::GBK.encode(&info.key);
        let key = urlencoding::encode_binary(&key);
        let url = format!("https://so.hexun.com/ajax.do?key={}&type={}", key, info.ty);
        let res = self.reqwest.get(url).send().await?;

        let text = res.text().await?;

        let (_, json) = text
            .split_once("=")
            .ok_or_else(|| color_eyre::eyre::eyre!("Not a valid return from hexun: {text}"))?;

        let outputs: Vec<Output> = serde_json::from_str(json)?;

        Ok(outputs)
    }
}
