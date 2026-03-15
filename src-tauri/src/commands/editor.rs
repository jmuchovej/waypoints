use domain::mark::mark_data::MathEngine;
use tauri::State;
use tokio::sync::Mutex;

#[tauri::command]
#[specta::specta]
pub async fn editor_render_math(
    expression: String,
    engine: MathEngine,
    display_mode: bool,
) -> Result<String, String> {
    match engine {
        MathEngine::Typst => tokio::task::spawn_blocking(move || {
            editor::typst::render_math_to_svg(&expression, display_mode)
        })
        .await
        .map_err(|e| format!("task join error: {e}"))?,
        MathEngine::LaTeX => Err("LaTeX rendering is client-side via KaTeX".into()),
    }
}
