#[cfg(windows)]
pub fn run(
    url: String,
    receiver: std::sync::mpsc::Receiver<()>,
    profile: std::path::PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    use tao::{
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoopBuilder},
        window::WindowBuilder,
    };
    let event_loop = EventLoopBuilder::<()>::with_user_event().build();
    let quit_proxy = event_loop.create_proxy();
    std::thread::spawn(move || {
        if receiver.recv().is_ok() {
            let _ = quit_proxy.send_event(());
        }
    });
    let window = WindowBuilder::new()
        .with_title("Atelier — Roadmap")
        .with_inner_size(tao::dpi::LogicalSize::new(1440., 960.))
        .with_min_inner_size(tao::dpi::LogicalSize::new(850., 600.))
        .build(&event_loop)?;
    let status_path = profile.parent().unwrap().join("desktop-status.json");
    let mut context = wry::WebContext::new(Some(profile));
    let allowed = url.clone();
    let ready = Arc::new(AtomicBool::new(false));
    let ipc_ready = ready.clone();
    let ipc_proxy = event_loop.create_proxy();
    let webview=wry::WebViewBuilder::with_web_context(&mut context)
        .with_url(&url).with_devtools(cfg!(debug_assertions))
        .with_navigation_handler(move|target|target.starts_with(&allowed)||target.starts_with("blob:"))
        .with_new_window_req_handler(|_|false)
        .with_ipc_handler(move|request|match request.body().as_str(){
            "atelier-ready"=>{ipc_ready.store(true,Ordering::Relaxed);let _=std::fs::write(&status_path,serde_json::json!({"ready":true,"pid":std::process::id(),"version":env!("CARGO_PKG_VERSION")}).to_string());},
            "atelier-close-ready"=>{let _=ipc_proxy.send_event(());},
            _=>{}
        }).build(&window)?;
    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::UserEvent(()) => *control_flow = ControlFlow::Exit,
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                if !ready.load(Ordering::Relaxed) {
                    *control_flow = ControlFlow::Exit;
                } else {
                    let _ = webview
                        .evaluate_script("window.__atelierClose().catch(e=>tell(e.message))");
                }
            }
            _ => {}
        }
        if !matches!(*control_flow, ControlFlow::ExitWithCode(_)) {
            *control_flow = ControlFlow::Wait;
        }
    });
}
#[cfg(windows)]
pub fn single_instance() -> bool {
    use std::ptr;
    #[link(name = "kernel32")]
    extern "system" {
        fn CreateMutexW(
            attributes: *const std::ffi::c_void,
            owner: i32,
            name: *const u16,
        ) -> *mut std::ffi::c_void;
        fn GetLastError() -> u32;
    }
    #[link(name = "user32")]
    extern "system" {
        fn FindWindowW(class: *const u16, title: *const u16) -> *mut std::ffi::c_void;
        fn ShowWindow(window: *mut std::ffi::c_void, command: i32) -> i32;
        fn SetForegroundWindow(window: *mut std::ffi::c_void) -> i32;
    }
    let name: Vec<u16> = "Local\\AtelierRoadmap-v1"
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let handle = unsafe { CreateMutexW(ptr::null(), 0, name.as_ptr()) };
    if handle.is_null() {
        return true;
    }
    if unsafe { GetLastError() } == 183 {
        let title: Vec<u16> = "Atelier — Roadmap".encode_utf16().chain(Some(0)).collect();
        let window = unsafe { FindWindowW(ptr::null(), title.as_ptr()) };
        if !window.is_null() {
            unsafe {
                ShowWindow(window, 9);
                SetForegroundWindow(window);
            }
        }
        return false;
    }
    // Keep the mutex handle alive until the process exits.
    true
}
#[cfg(windows)]
pub fn error(message: &str) {
    #[link(name = "user32")]
    extern "system" {
        fn MessageBoxW(
            window: *mut std::ffi::c_void,
            text: *const u16,
            title: *const u16,
            kind: u32,
        ) -> i32;
    }
    let text: Vec<u16> = message.encode_utf16().chain(Some(0)).collect();
    let title: Vec<u16> = "Atelier — erro ao iniciar"
        .encode_utf16()
        .chain(Some(0))
        .collect();
    unsafe {
        MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), 0x10);
    }
}
