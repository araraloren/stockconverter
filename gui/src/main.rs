#![cfg_attr(
    all(target_os = "windows", not(debug_assertions),),
    windows_subsystem = "windows"
)]

use std::{fmt::Debug, time::Duration};

use iced::{
    alignment::{Horizontal, Vertical},
    futures::{SinkExt, Stream, channel::mpsc::Sender},
    task::Handle,
    widget::{
        button, column, container, horizontal_rule, radio, row, slider, text, text::LineHeight,
        text_editor, text_input,
    },
    window::{Settings, icon},
    *,
};

use reqwest::{Client, cookie::Jar};
use search::cninfo;
use search::hexun;
use search::sina;
use search::{QueryInput, Stock};
use search::{Search, cfi};
use search::{Tool, sohu};

const APP_PNG: &[u8] = include_bytes!("../app.png");

pub fn main() -> iced::Result {
    iced::application(Gui::new, Gui::update, Gui::view)
        .title("股票简称转代码")
        .window(Settings {
            icon: icon::from_file_data(APP_PNG, None).ok(),
            ..Default::default()
        })
        .window_size(Size {
            width: 800.,
            height: 500.,
        })
        .default_font(Font::with_name("黑体"))
        .run()
}

#[derive(Debug, Default)]
pub struct Gui {
    delay: f64,
    path: String,
    input: text_editor::Content,
    tool_sel: Option<Tool>,
    infobar: String,
    output: text_editor::Content,
    content: String,
    stocks: Vec<Stock>,
    task_handle: Option<Handle>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Nothing,
    InputAct(text_editor::Action),
    OutputAct(text_editor::Action),
    ToolSel(Tool),
    SetDelay(f64),
    SetPath(String),
    SetInfobar(String),
    StartTask,
    StopTask,
    CleanOutput,
    ReportFailed((String, String)),
    AppendStock(Stock),
    TaskFinished(bool),
    ExportResult,
}

impl Gui {
    pub fn new() -> Self {
        Self {
            delay: 1.0,
            path: String::default(),
            tool_sel: Some(Tool::CnInfo),
            input: text_editor::Content::default(),
            output: text_editor::Content::default(),
            infobar: String::default(),
            content: String::default(),
            task_handle: None,
            stocks: vec![],
        }
    }

    pub fn task_delay(&self) -> u64 {
        (self.delay * 50.) as _
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Nothing => {}
            Message::InputAct(action) => {
                self.input.perform(action);
            }
            Message::OutputAct(action) => {
                self.output.perform(action);
            }
            Message::ToolSel(tool) => {
                self.tool_sel = Some(tool);
            }
            Message::SetDelay(value) => {
                self.delay = value;
            }
            Message::ExportResult => {
                let path = if self.path.is_empty() {
                    "output.ebk"
                } else {
                    self.path.as_ref()
                };
                let path = path.to_string();
                let mut content = String::default();

                if !self.stocks.is_empty() {
                    for stock in &self.stocks {
                        content.push_str(&stock.normalize());
                        content.push('\n');
                    }
                    return Task::future(async move {
                        if let Err(e) = tokio::fs::write(path, content).await {
                            Message::SetInfobar(format!("写入文件错误: {e:?}"))
                        } else {
                            Message::Nothing
                        }
                    });
                }
            }
            Message::SetPath(path) => {
                self.path = path;
            }
            Message::TaskFinished(_) => {
                self.task_handle = None;
            }
            Message::CleanOutput => {
                self.stocks.clear();
                self.content.clear();
                self.output = text_editor::Content::with_text(&self.content);
            }
            Message::ReportFailed((keyword, msg)) => {
                self.infobar = format!("搜索关键字 `{keyword}` 失败: {msg}");
                self.content.push_str(&format!("{keyword}: 无可用的结果\n"));
                self.output = text_editor::Content::with_text(&self.content);
            }
            Message::AppendStock(stock) => {
                self.content
                    .push_str(&format!("{} ==> {}\n", stock.name, stock.code));
                self.output = text_editor::Content::with_text(&self.content);
                self.stocks.push(stock);
            }
            Message::SetInfobar(value) => {
                self.infobar = value;
            }
            Message::StartTask => {
                if self.task_handle.is_none() {
                    let tool = self.tool_sel.unwrap_or_default();
                    let delay = self.task_delay();
                    let keywords: Vec<String> = self
                        .input
                        .lines()
                        .map(|v| v.text.trim().to_string())
                        .filter(|v| !v.is_empty())
                        .collect();

                    self.content.clear();
                    self.stocks.clear();

                    let (task, handle) =
                        Task::stream(start_task(tool, keywords, delay)).abortable();

                    self.task_handle = Some(handle.abort_on_drop());

                    return task;
                }
            }
            Message::StopTask => {
                self.task_handle.take();
            }
        }
        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let input = text_editor(&self.input)
            .on_action(Message::InputAct)
            .placeholder("按行分隔的关键字，从全民智投复制粘到这里")
            .height(Length::Fill);
        let output = text_editor(&self.output)
            .on_action(Message::OutputAct)
            .placeholder("搜索的结果，按行分隔")
            .height(Length::Fill);

        let cninfo = radio("巨潮信息网", Tool::CnInfo, self.tool_sel, Message::ToolSel);

        let sina = radio("新浪财经", Tool::Sina, self.tool_sel, Message::ToolSel);

        let cfi = radio("中财网", Tool::Cfi, self.tool_sel, Message::ToolSel);

        let hexun = radio("和讯网", Tool::HeXun, self.tool_sel, Message::ToolSel);

        let sohu = radio("搜狐网", Tool::SoHu, self.tool_sel, Message::ToolSel);

        let choices = container(
            row![cninfo, sina, hexun, sohu, cfi]
                .padding(10)
                .spacing(5)
                .height(Length::Fill)
                .width(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::FillPortion(1))
        .style(container::bordered_box);

        let delay = row![
            slider(1.0..=50.0, self.delay, Message::SetDelay),
            text(format!("延迟: {}毫秒", self.task_delay())),
        ]
        .spacing(5)
        .align_y(Vertical::Center);

        let start = button("搜索").on_press_maybe(if self.task_handle.is_some() {
            None
        } else {
            Some(Message::StartTask)
        });

        let stop =
            button("停止").on_press_maybe(self.task_handle.as_ref().map(|_| Message::StopTask));

        let path = text_input("output.ebk", &self.path).on_input(Message::SetPath);

        let export = button("导出").on_press_maybe(if self.content.is_empty() {
            None
        } else {
            Some(Message::ExportResult)
        });

        let operators = row![delay, start, stop, path, export]
            .spacing(5)
            .padding(5)
            .height(Length::FillPortion(1)); //.height(Length::Fixed(80.));

        let infobar = text_input("状态栏", &self.infobar)
            .line_height(LineHeight::Absolute(Pixels(12.0)))
            .size(Pixels::from(12.0))
            .align_x(Horizontal::Left)
            .width(Length::Fill);

        let rule = horizontal_rule(2);

        let main_container = container(
            column![
                row![input, output]
                    .padding(10)
                    .spacing(5)
                    .height(Length::FillPortion(8))
                    .width(Length::Fill),
                choices,
                operators,
                rule,
                infobar,
            ]
            .spacing(5)
            .height(Length::Fill)
            .width(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(container::bordered_box);

        main_container.into()
    }
}

pub async fn try_unwrap<T, E: Debug>(
    t: std::result::Result<T, E>,
    send: &mut Sender<Message>,
) -> Option<T> {
    match t {
        Ok(val) => Some(val),
        Err(e) => {
            send.send(Message::SetInfobar(format!("捕获到错误: {e:?}")))
                .await
                .unwrap();
            None
        }
    }
}

pub fn start_task(
    tool: Tool,
    keywords: Vec<String>,
    delay: u64,
) -> impl Stream<Item = Message> + 'static {
    iced::stream::channel(1024, async move |mut send| {
        let builder = Client::builder()
            .user_agent(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:140.0) Gecko/20100101 Firefox/140.0",
            )
            .cookie_store(true)
            .cookie_provider(Jar::default().into());

        let mut success = false;

        match tool {
            Tool::CnInfo => {
                let tool = cninfo::CnInfo::init(builder).await;

                if let Some(tool) = try_unwrap(tool, &mut send).await {
                    process(tool, keywords, &mut send, delay).await;
                    success = true;
                }
            }
            Tool::Sina => {
                let tool = sina::Sina::init(builder).await;

                if let Some(tool) = try_unwrap(tool, &mut send).await {
                    process(tool, keywords, &mut send, delay).await;
                    success = true;
                }
            }
            Tool::Cfi => {
                let tool = cfi::Cfi::init(builder).await;

                if let Some(tool) = try_unwrap(tool, &mut send).await {
                    process(tool, keywords, &mut send, delay).await;
                    success = true;
                }
            }
            Tool::HeXun => {
                let tool = hexun::Hexun::init(builder).await;

                if let Some(tool) = try_unwrap(tool, &mut send).await {
                    process(tool, keywords, &mut send, delay).await;
                    success = true;
                }
            }
            Tool::SoHu => {
                let tool = sohu::SoHu::init(builder).await;

                if let Some(tool) = try_unwrap(tool, &mut send).await {
                    process(tool, keywords, &mut send, delay).await;
                    success = true;
                }
            }
        }

        send.send(Message::TaskFinished(success)).await.unwrap();
    })
}

pub async fn process<T>(tool: T, keywords: Vec<String>, send: &mut Sender<Message>, delay: u64)
where
    T: Search,
    T::Input: Default,
{
    let mut input = <T::Input>::default();

    for keyword in keywords {
        send.send(Message::SetInfobar(format!("搜索关键字 `{keyword}`...")))
            .await
            .unwrap();

        let stock = tool
            .search({
                input.reset();
                input.set_keyword(keyword.clone());
                &input
            })
            .await;

        match stock {
            Ok(stock) => {
                let report =
                    Message::SetInfobar(format!("搜索关键字 `{keyword}` ====> {}", stock.code));

                send.send(report).await.unwrap();
                send.send(Message::AppendStock(stock)).await.unwrap();
            }
            Err(e) => {
                send.send(Message::ReportFailed((keyword, e.to_string())))
                    .await
                    .unwrap();
            }
        }

        tokio::time::sleep(Duration::from_millis(delay)).await;
    }
}
