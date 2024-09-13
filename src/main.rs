#![cfg_attr(not(test), windows_subsystem = "windows")]

mod fonts;
mod readed_page;

use eframe::{
    egui::{self, Response, RichText, Ui},
    epaint::{FontFamily, FontId, Vec2},
};
use readed_page::ReadedPage;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

const TEXT_SIZE: f32 = 20.0;

pub fn main() -> eframe::Result<()> {
    let mut native_options = eframe::NativeOptions::default();
    native_options.centered = true;
    native_options.viewport.decorations = Some(true);
    native_options.vsync = false;
    eframe::run_native(
        "Large Text Reader",
        native_options,
        Box::new(|cc| Ok(Box::new(ReaderApp::new(cc)))),
    )
}

#[derive(Default)]
struct ReaderApp {
    /* 입력 관련 */
    /// 입력받은 텍스트 파일 경로 (실시간 수정됨)
    input_box_file_path: String,
    /// 텍스트 입력받은 상태에서 버튼을 눌렀을 경우, 파일 경로가 정상적일시 등록됨
    file_path: Option<PathBuf>,
    /// 버튼을 눌러 파일을 읽은 후, 읽은 내용이 저장됨
    readed_file_text: Arc<Mutex<Option<String>>>,
    /// 입력 텍스트박스 Response
    input_box_file_path_response: Option<Response>,

    last_size: (f32, f32),
    reformed_text: Option<Vec<String>>,
    page_per_path: HashMap<PathBuf, ReadedPage>,

    /// 텍스트 파일 입력 버튼 오른쪽에 표시될 메세지, 디버그용도로 사용
    debug_info: String,

    /// 현재 읽고있는 페이지
    page: usize,
}

impl ReaderApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // 폰트 설정
        cc.egui_ctx.set_fonts(fonts::get_fonts());

        let mut result = Self::default();

        // 이전 데이터 복구
        if let Ok(data) = std::fs::read_to_string("reader.json") {
            let map: HashMap<String, String> = serde_json::from_str(&data).unwrap();

            // 파일별 페이지 복구
            let page = map.get("page");
            if page.is_some() {
                result.page_per_path = serde_json::from_str(page.unwrap()).unwrap();
            }

            // 읽던 파일 복구
            let file_path = map.get("file_path");
            // 파일이 존재하는지도 체크함
            if file_path.is_some() {
                result.file_path = serde_json::from_str(file_path.unwrap()).unwrap();
                if result.file_path.is_some()
                    && std::fs::metadata(&result.file_path.clone().unwrap()).is_ok()
                {
                    // 메세지박스 채우기
                    result.input_box_file_path = result
                        .file_path
                        .clone()
                        .unwrap()
                        .to_string_lossy()
                        .to_string();

                    // 파일 읽기
                    let path = result.file_path.clone().unwrap();
                    let out = result.readed_file_text.clone();
                    Self::read(path, out);

                    // 페이지 설정
                    result.page = result
                        .page_per_path
                        .entry(result.file_path.clone().unwrap())
                        .or_default()
                        .page;
                }
            }
        }

        result
    }
}

impl Drop for ReaderApp {
    fn drop(&mut self) {
        if self.file_path.is_some() {
            // 페이지 저장
            self.page_per_path
                .insert(self.file_path.clone().unwrap(), ReadedPage::new(self.page));
        }
        let mut map = HashMap::new();
        map.insert("page", serde_json::to_string(&self.page_per_path).unwrap());
        map.insert("file_path", serde_json::to_string(&self.file_path).unwrap());

        std::fs::write("reader.json", serde_json::to_string(&map).unwrap()).unwrap();
    }
}

impl eframe::App for ReaderApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 최근 연 목록
        egui::Window::new("Recent")
            .default_open(false)
            .vscroll(true)
            .show(ctx, |ui| self.show_history(ui));

        // 입력창
        self.display_file_path_input_box(ctx);

        // 입력창 표시 후, 텍스트 출력 가능 높이
        let text_displayable_size = ctx.available_rect().size();
        /* 창 크기가 바뀌었으면 변경된 텍스트 삭제 */
        if self.last_size.0 != text_displayable_size.x
            || self.last_size.1 != text_displayable_size.y
        {
            self.last_size = (text_displayable_size.x, text_displayable_size.y);
            self.reformed_text = None;
        }

        /* 이미 읽은 텍스트가 있을때 */
        if self.readed_file_text.lock().unwrap().as_ref().is_some() {
            egui::CentralPanel::default().show(ctx, |ui| {
                /* 수정된 텍스트가 없으면 이미 읽은 텍스트를 수정한다. */
                if self.reformed_text.is_none() {
                    self.reform_text(ctx, ui, text_displayable_size);
                }

                // 텍스트가 있고 페이지가 정상적일떄
                // 바로 위에서 reformed_text를 설정해주기때문에 체크로직 삭제함
                if self.reformed_text.as_ref().unwrap().len() > self.page {
                    // 페이지 표시
                    ui.label(
                        RichText::new(&self.reformed_text.as_ref().unwrap()[self.page])
                            .size(TEXT_SIZE),
                    );
                }
            });
        }

        self.get_keyboard_input_and_change_now_page(ctx);

        self.debug_info = format!("현재 페이지 : {}", self.page);
    }
}

impl ReaderApp {
    fn display_file_path_input_box(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("file_path_input_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("File Path:");
                self.input_box_file_path_response =
                    Some(ui.text_edit_singleline(&mut self.input_box_file_path));
                if ui.button("Open").clicked() {
                    // " 제거
                    self.input_box_file_path = self
                        .input_box_file_path
                        .trim_matches(|c| c == '"' || c == '\'' || c == ' ')
                        .to_owned();

                    // 파일이 있으면 등록
                    if std::fs::metadata(&self.input_box_file_path).is_ok() {
                        if self.file_path.is_some() {
                            // 페이지 저장
                            self.page_per_path.insert(
                                self.file_path.clone().unwrap(),
                                ReadedPage::new(self.page),
                            );
                        }
                        self.file_path = Some(PathBuf::from(&self.input_box_file_path));
                        // 페이지 복구
                        self.page = self
                            .page_per_path
                            .entry(self.file_path.clone().unwrap())
                            .or_default()
                            .page;
                        self.reformed_text = None;
                        *self.readed_file_text.lock().unwrap() = None;

                        let path = self.file_path.clone().unwrap();
                        let out = self.readed_file_text.clone();
                        std::thread::spawn(|| Self::read(path, out));
                    }
                }
                ui.label(&self.debug_info);
            });
        });
    }

    fn read(path: PathBuf, out: Arc<Mutex<Option<String>>>) {
        use std::io::Read;

        let mut file = std::fs::File::open(path).unwrap();
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();

        if let Ok(data) = String::from_utf8(data.clone()) {
            let mut out = out.lock().unwrap();
            *out = Some(data.into());
        } else {
            let data = encoding_rs::EUC_KR.decode(&data).0.to_string();
            let mut out = out.lock().unwrap();
            *out = Some(data.into());
        }
    }

    fn get_keyboard_input_and_change_now_page(&mut self, ctx: &egui::Context) {
        if self
            .input_box_file_path_response
            .take()
            .unwrap()
            .has_focus()
        {
            return;
        }

        if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
            self.page += 1;
        } else if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
            if self.page > 0 {
                self.page -= 1;
            }
        } else if ctx.input(|i| i.key_pressed(egui::Key::PageDown)) {
            self.page += 10;
        } else if ctx.input(|i| i.key_pressed(egui::Key::PageUp)) {
            if self.page > 10 {
                self.page -= 10;
            } else {
                self.page = 0;
            }
        } else if ctx.input(|i| i.key_pressed(egui::Key::Home)) {
            self.page = 0;
        } else if ctx.input(|i| i.key_pressed(egui::Key::End)) {
            self.page = usize::MAX;
        }
        if self.reformed_text.is_some() && self.page >= self.reformed_text.as_ref().unwrap().len() {
            self.page = self.reformed_text.as_ref().unwrap().len() - 1;
        }
    }

    /// 이미 읽은 텍스트를 잘라서 reform_text에 저장한다.
    fn reform_text(&mut self, _ctx: &egui::Context, ui: &mut Ui, frame_size: Vec2) {
        let readed_text = {
            let readed_text = self.readed_file_text.lock().unwrap();
            readed_text.as_ref().unwrap().to_string()
        };
        let mut result = Vec::new();
        let mut page = String::new();
        let wraped_text = ui.fonts(|fonts| {
            let font_id = FontId::new(TEXT_SIZE, FontFamily::Monospace);
            fonts
                .layout_delayed_color(readed_text, font_id.clone(), frame_size.x - 16.0)
                .rows
                .clone()
        });

        /* 높이별로 자름 */
        let mut filled_height = 0.0;
        for row in wraped_text {
            if filled_height + row.height() > (frame_size.y - 16.0) {
                result.push(page);
                page = String::new();
                filled_height = 0.0;
            }

            let line = row.glyphs.iter().map(|c| c.chr).collect::<String>();
            page.push_str(&line);
            page.push('\n');
            filled_height += row.height();
        }
        if !page.is_empty() {
            result.push(page);
        }
        // std::fs::write("temp.txt", result.join("\n")).unwrap();
        self.reformed_text = Some(result);
    }

    fn show_history(&mut self, ui: &mut Ui) {
        let mut history = self
            .page_per_path
            .iter()
            .map(|(path, page)| {
                let path = path.to_string_lossy().to_string();
                let page = page.page;
                (path, page)
            })
            .collect::<Vec<_>>();
        history.sort_by(|a, b| a.1.cmp(&b.1));

        for (path, page) in history {
            ui.horizontal(|ui| {
                ui.label(
                    Path::new(&path)
                        .file_name()
                        .unwrap_or_default()
                        .to_str()
                        .unwrap(),
                );
                ui.label(&format!("page : {}", page));
                if ui.button("Open").clicked() {
                    if self.file_path.is_some() {
                        // 페이지 저장
                        self.page_per_path
                            .insert(self.file_path.clone().unwrap(), ReadedPage::new(self.page));
                    }

                    self.input_box_file_path = path;
                    self.file_path = Some(PathBuf::from(&self.input_box_file_path));
                    // 페이지 복구
                    self.page = self
                        .page_per_path
                        .entry(self.file_path.clone().unwrap())
                        .or_default()
                        .page;
                    self.reformed_text = None;
                    *self.readed_file_text.lock().unwrap() = None;

                    let path = self.file_path.clone().unwrap();
                    let out = self.readed_file_text.clone();
                    std::thread::spawn(|| Self::read(path, out));
                }
            });
        }
    }
}
