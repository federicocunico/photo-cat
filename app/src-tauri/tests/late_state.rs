//! Executable proof for asynchronous startup with the pinned Tauri version.
use tauri::{test, Manager, State};

struct Ready(&'static str);

#[tauri::command]
fn probe(state: State<'_, Ready>) -> &'static str {
    state.0
}

#[test]
fn missing_state_rejects_and_manage_after_setup_enables_commands() {
    let app = test::mock_builder()
        .setup(|_| Ok(()))
        .invoke_handler(tauri::generate_handler![probe])
        .build(test::mock_context(test::noop_assets()))
        .unwrap();
    let window = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    let invoke = || {
        test::get_ipc_response(
            &window,
            tauri::webview::InvokeRequest {
                cmd: "probe".into(),
                callback: tauri::ipc::CallbackFn(0),
                error: tauri::ipc::CallbackFn(1),
                url: if cfg!(windows) {
                    "http://tauri.localhost"
                } else {
                    "tauri://localhost"
                }
                .parse()
                .unwrap(),
                body: Default::default(),
                headers: Default::default(),
                invoke_key: test::INVOKE_KEY.into(),
            },
        )
    };
    let error = invoke().unwrap_err().to_string();
    assert!(error.contains("state"), "{error}");
    let handle = app.handle().clone();
    std::thread::spawn(move || assert!(handle.manage(Ready("ready"))))
        .join()
        .unwrap();
    assert_eq!(invoke().unwrap().deserialize::<String>().unwrap(), "ready");
}
