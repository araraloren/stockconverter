pub mod cfi;
pub mod cninfo;
pub mod hexun;
pub mod sina;
pub mod sohu;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Deserialize, serde::Serialize,
)]
pub enum Exchange {
    ShangHai,
    ShenZhen,
    BeiJing,
    HongKong,
}

#[derive(
    Debug, Clone, Copy, Default, cote::prelude::CoteOpt, cote::prelude::CoteVal, PartialEq, Eq,
)]
#[coteval(igcase)]
pub enum Tool {
    Sina,
    #[default]
    CnInfo,
    Cfi,
    HeXun,
    SoHu,
}

impl Exchange {
    pub fn guess_from_stock(val: &str) -> color_eyre::Result<Exchange> {
        if HongKong.valid(val).is_some() {
            Ok(Self::HongKong)
        } else if ShangHai.valid(val).is_some() {
            Ok(Self::ShangHai)
        } else if ShenZhen.valid(val).is_some() {
            Ok(Self::ShenZhen)
        } else if BeiJing.valid(val).is_some() {
            Ok(Self::BeiJing)
        } else {
            Err(color_eyre::eyre::eyre!("Not a valid stock number: {val}"))
        }
    }
}

pub trait Search {
    type Input: QueryInput;
    type Output: TryInto<Stock>;

    fn search_all(
        &self,
        input: &Self::Input,
    ) -> impl Future<Output = color_eyre::Result<Vec<Self::Output>>>;

    fn search(&self, input: &Self::Input) -> impl Future<Output = color_eyre::Result<Stock>> {
        async {
            let outputs = self.search_all(input).await?;
            let mut hongkong = None;

            for output in outputs {
                let stock: Result<Stock, _> = output.try_into();

                if let Ok(stock) = stock {
                    if matches!(stock.exchange, Exchange::HongKong) {
                        if hongkong.is_none() {
                            hongkong = Some(stock);
                        }
                    } else {
                        return Ok(stock);
                    }
                }
            }

            hongkong.ok_or_else(|| {
                color_eyre::eyre::eyre!("Can not find valid stock number in results")
            })
        }
    }
}

pub trait QueryInput {
    fn set_keyword(&mut self, keyword: String);

    fn reset(&mut self) {}
}

#[derive(Debug, Clone)]
pub struct Stock {
    pub name: String,
    pub code: String,
    pub exchange: Exchange,
}

impl Stock {
    pub fn new(name: String, code: String, exchange: Exchange) -> Self {
        Self {
            name,
            code,
            exchange,
        }
    }

    pub fn normalize(&self) -> String {
        let val = self.code.as_str();

        match self.exchange {
            Exchange::ShangHai => ShangHai.format(val),
            Exchange::ShenZhen => ShenZhen.format(val),
            Exchange::BeiJing => BeiJing.format(val),
            Exchange::HongKong => HongKong.format(val),
        }
    }
}

pub fn normalize_stock_number(val: &str) -> Option<String> {
    if HongKong.valid(val).is_some() {
        Some(HongKong.format(val))
    } else if ShangHai.valid(val).is_some() {
        Some(ShangHai.format(val))
    } else if ShenZhen.valid(val).is_some() {
        Some(ShenZhen.format(val))
    } else if BeiJing.valid(val).is_some() {
        Some(BeiJing.format(val))
    } else {
        None
    }
}

pub trait Format {
    fn format(&self, val: &str) -> String;
}

pub trait Valid {
    fn valid(&self, val: &str) -> Option<()>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ShangHai;

impl Format for ShangHai {
    fn format(&self, val: &str) -> String {
        format!("{}{}", 1, val)
    }
}

impl Valid for ShangHai {
    fn valid(&self, val: &str) -> Option<()> {
        if let Some(v) = val.get(0..2)
            && matches!(v, "68" | "60")
        {
            return Some(());
        }

        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ShenZhen;

impl Format for ShenZhen {
    fn format(&self, val: &str) -> String {
        format!("{}{}", 0, val)
    }
}

impl Valid for ShenZhen {
    fn valid(&self, val: &str) -> Option<()> {
        if let Some(v) = val.get(0..2)
            && matches!(v, "00" | "30")
        {
            return Some(());
        }

        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BeiJing;

impl Format for BeiJing {
    fn format(&self, val: &str) -> String {
        format!("{}{}", 8, val)
    }
}

impl Valid for BeiJing {
    fn valid(&self, val: &str) -> Option<()> {
        if let Some(v) = val.get(0..2)
            && matches!(v, "88" | "87" | "83" | "43")
        {
            return Some(());
        }

        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct HongKong;

impl Format for HongKong {
    fn format(&self, val: &str) -> String {
        format!("{}{}", 5, val)
    }
}

impl Valid for HongKong {
    fn valid(&self, val: &str) -> Option<()> {
        (val.len() == 5).then_some(())
    }
}
