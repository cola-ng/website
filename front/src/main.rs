fn main() {
    #[cfg(target_arch = "wasm32")]
    if let Err(error) = colang_front::browser::start() {
        web_sys::console::error_1(&error);
        if let Some(root) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("app"))
        {
            root.set_text_content(Some("页面加载失败，请刷新重试。"));
        }
    }
}
