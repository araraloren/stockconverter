use color_eyre::eyre::eyre;
use reqwest::Client;

use crate::{Exchange, Stock};

#[derive(Debug)]
pub struct CnInfo {
    pub reqwest: Client,
}

impl CnInfo {
    pub async fn init(builder: reqwest::ClientBuilder) -> color_eyre::Result<Self> {
        let reqwest = builder.build()?;
        let res = reqwest.get("https://www.cninfo.com.cn").send().await?;

        if !res.status().is_success() {
            return Err(eyre!("Can not access cninfo website: {}", res.status()));
        }

        Ok(Self { reqwest })
    }
}

#[derive(Debug, Clone)]
pub struct Input {
    pub key: String,

    pub max: usize,
}

impl Default for Input {
    fn default() -> Self {
        Self {
            key: Default::default(),
            max: 10,
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
    zwjc: String,
    #[serde(rename = "type")]
    exchange: String,
}

pub const TYPE_SHJ: &str = "shj";
pub const TYPE_HKE: &str = "hke";

impl TryFrom<Output> for Stock {
    type Error = color_eyre::Report;

    fn try_from(value: Output) -> Result<Self, Self::Error> {
        let exchange = match value.exchange.as_str() {
            TYPE_HKE => Ok(Exchange::HongKong),
            _ => Exchange::guess_from_stock(&value.code),
        };

        Ok(Stock {
            name: value.zwjc,
            code: value.code,
            exchange: exchange?,
        })
    }
}

impl crate::Search for CnInfo {
    type Input = Input;
    type Output = Output;

    async fn search_all(&self, info: &Self::Input) -> color_eyre::Result<Vec<Self::Output>> {
        let url = "https://www.cninfo.com.cn/new/information/topSearch/query";
        let builder = self
            .reqwest
            .post(url)
            .query(&[("keyWord", &info.key), ("maxNum", &info.max.to_string())]);

        let res = builder.send().await?;
        let text = res.text().await?;

        Ok(serde_json::from_str(&text)?)
    }
}
