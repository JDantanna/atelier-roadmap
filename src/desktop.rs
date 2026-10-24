use crate::notifications::Notice;
#[derive(Clone, Debug)]
pub enum DesktopEvent {
    Quit,
    Show,
    Hide,
    Minimize,
    WindowRequest(&'static str),
    ExitRequested,
    Notify(Notice),
    RebuildTray,
}
pub fn run(
    url: String,
    receiver: std::sync::mpsc::Receiver<()>,
    notifications: std::sync::mpsc::Receiver<Notice>,
    profile: std::path::PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    use tao::{
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoopBuilder},
        platform::windows::WindowExtWindows,
        window::WindowBuilder,
    };
    let event_loop = EventLoopBuilder::<DesktopEvent>::with_user_event().build();
    let quit_proxy = event_loop.create_proxy();
    std::thread::spawn(move || {
        if receiver.recv().is_ok() {
            let _ = quit_proxy.send_event(DesktopEvent::Quit);
        }
    });
    let notification_proxy = event_loop.create_proxy();
    std::thread::spawn(move || {
        while let Ok(n) = notifications.recv() {
            let _ = notification_proxy.send_event(DesktopEvent::Notify(n));
        }
    });
    let window = WindowBuilder::new()
        .with_title("Atelier — Roadmap")
        .with_inner_size(tao::dpi::LogicalSize::new(1440., 960.))
        .with_min_inner_size(tao::dpi::LogicalSize::new(850., 600.))
        .build(&event_loop)?;
    let mut tray = crate::tray::Tray::new(
        window.hwnd() as *mut std::ffi::c_void,
        event_loop.create_proxy(),
    )?;
    let tray_available = tray.available;
    let status_path = profile.parent().unwrap().join("desktop-status.json");
    let mut context = wry::WebContext::new(Some(profile));
    let allowed = url.clone();
    let ready = Arc::new(AtomicBool::new(false));
    let ipc_ready = ready.clone();
    let ipc_proxy = event_loop.create_proxy();
    let webview=wry::WebViewBuilder::with_web_context(&mut context).with_url(&url).with_devtools(cfg!(debug_assertions)).with_navigation_handler(move|target|target.starts_with(&allowed)||target.starts_with("blob:")).with_new_window_req_handler(|_|false).with_ipc_handler(move|request|{
  match request.body().as_str(){
   "atelier-ready"=>{ipc_ready.store(true,Ordering::Relaxed);let _=std::fs::write(&status_path,serde_json::json!({"ready":true,"trayAvailable":tray_available,"pid":std::process::id(),"version":env!("CARGO_PKG_VERSION")}).to_string());},
   "atelier-close-ready"=>{let _=ipc_proxy.send_event(DesktopEvent::Quit);},
   payload=>{if let Ok(v)=serde_json::from_str::<serde_json::Value>(payload){if v["action"]=="window"{let event=match v["target"].as_str(){Some("tray")=>Some(DesktopEvent::Hide),Some("taskbar")=>Some(DesktopEvent::Minimize),_=>None};if let Some(e)=event{let _=ipc_proxy.send_event(e);}}}}
  }
 }).build(&window)?;
    event_loop.run(move|event,_,flow|{
  match event{
   Event::UserEvent(DesktopEvent::Quit)=>{tray.remove();*flow=ControlFlow::Exit;},
   Event::UserEvent(DesktopEvent::Show)=>{window.set_visible(true);window.set_minimized(false);window.set_focus();},
   Event::UserEvent(DesktopEvent::Hide)=>{if !tray.available{let _=tray.add();}if tray.available{window.set_visible(false)}else{window.set_minimized(true);let _=webview.evaluate_script("tell('A bandeja não está disponível neste Windows. O Atelier foi mantido na barra de tarefas.')");}},
   Event::UserEvent(DesktopEvent::Minimize)=>window.set_minimized(true),
   Event::UserEvent(DesktopEvent::RebuildTray)=>{let _=tray.add();},
   Event::UserEvent(DesktopEvent::Notify(n))=>tray.notify(&n.title,&n.body),
   Event::UserEvent(DesktopEvent::ExitRequested)=>{window.set_visible(true);let _=webview.evaluate_script("window.__atelierNativeExit()");},
   Event::UserEvent(DesktopEvent::WindowRequest(reason))=>{window.set_visible(true);window.set_minimized(false);let _=webview.evaluate_script(&format!("window.__atelierWindowRequest('{}')",reason));},
   Event::WindowEvent{event:WindowEvent::CloseRequested,..}=>{if !ready.load(Ordering::Relaxed){tray.remove();*flow=ControlFlow::Exit;}else{window.set_visible(true);let _=webview.evaluate_script("window.__atelierWindowRequest('close')");}},
   _=>{}
  }
  if !matches!(*flow,ControlFlow::ExitWithCode(_)){*flow=ControlFlow::Wait;}
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
