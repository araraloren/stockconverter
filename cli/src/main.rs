use std::time::Duration;

use cote::prelude::Cote;
use reqwest::{Client, cookie::Jar};
use search::QueryInput;
use search::Search;
use search::Stock;
use search::Tool;
use search::cfi;
use search::cninfo;
use search::hexun;
use search::sina;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    inner_main().await
}

#[derive(Debug, Cote)]
#[cote(shellcomp, aborthelp)]
struct Cli {
    /// Set the task delay
    #[arg(value = 50usize)]
    delay: Option<usize>,

    /// Select search tools
    #[arg(alias = "-t", scvalues = ["cninfo", "sina", "cfi", "hexun"], value = Tool::CnInfo)]
    tool: Option<Tool>,

    /// Set the search keyword
    #[pos(index = 1..)]
    keywords: Option<Vec<String>>,
}

async fn inner_main() -> color_eyre::Result<()> {
    let Cli {
        delay,
        tool,
        keywords,
    } = Cli::parse_env()?;
    let mut keywords = keywords.unwrap_or_default();
    let tool = tool.unwrap();
    let delay = delay.unwrap();

    if !atty::is(atty::Stream::Stdin) {
        let mut buff = String::default();

        while let Ok(count) = std::io::stdin().read_line(&mut buff) {
            if count > 0 {
                keywords.push(buff.trim().to_string());
                buff.clear();
            } else {
                break;
            }
        }
    }

    Searcher {
        tool,
        delay,
        keywords,
    }
    .invoke()
    .await
}

#[derive(Debug)]
pub struct Searcher {
    tool: Tool,
    delay: usize,
    keywords: Vec<String>,
}

impl Searcher {
    pub async fn invoke(self) -> color_eyre::Result<()> {
        let builder = Client::builder()
            .user_agent(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:140.0) Gecko/20100101 Firefox/140.0",
            )
            .cookie_store(true)
            .cookie_provider(Jar::default().into());

        println!("got keywords count: {}", self.keywords.len());

        let stocks = match self.tool {
            Tool::CnInfo => {
                let tool = cninfo::CnInfo::init(builder).await?;

                self.search(&tool).await?
            }
            Tool::Sina => {
                let tool = sina::Sina::init(builder).await?;

                self.search(&tool).await?
            }
            Tool::Cfi => {
                let tool = cfi::Cfi::init(builder).await?;

                self.search(&tool).await?
            }
            Tool::HeXun => {
                let tool = hexun::Hexun::init(builder).await?;

                self.search(&tool).await?
            }
        };

        for stock in stocks {
            println!("{}", stock.normalize());
        }
        Ok(())
    }

    pub async fn search<T>(self, tool: &T) -> color_eyre::Result<Vec<Stock>>
    where
        T: Search,
        T::Input: Clone + Default,
    {
        self.search_with(tool, <T::Input>::default()).await
    }

    pub async fn search_with<T>(
        self,
        tool: &T,
        mut input: T::Input,
    ) -> color_eyre::Result<Vec<Stock>>
    where
        T: Search,
        T::Input: Clone,
    {
        let mut stocks = vec![];

        for keyword in self.keywords {
            println!("try to search {keyword}",);

            stocks.push(
                tool.search({
                    input.reset();
                    input.set_keyword(keyword);
                    &input
                })
                .await?,
            );

            sleep(Duration::from_millis(self.delay as u64)).await;
        }

        Ok(stocks)
    }
}
