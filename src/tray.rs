//! Win32 tray icon, popup menu and deadline balloons. Runs on the Tao UI thread.
use crate::desktop::DesktopEvent;
use std::{
    ffi::c_void,
    sync::{
        atomic::{AtomicIsize, Ordering},
        Mutex, OnceLock,
    },
};
use tao::event_loop::EventLoopProxy;
type Hwnd = *mut c_void;
type WndProc = unsafe extern "system" fn(Hwnd, u32, usize, isize) -> isize;
#[repr(C)]
struct Guid {
    a: u32,
    b: u16,
    c: u16,
    d: [u8; 8],
}
#[repr(C)]
struct NotifyIconData {
    size: u32,
    window: Hwnd,
    id: u32,
    flags: u32,
    callback: u32,
    icon: *mut c_void,
    tip: [u16; 128],
    state: u32,
    state_mask: u32,
    info: [u16; 256],
    timeout: u32,
    title: [u16; 64],
    info_flags: u32,
    guid: Guid,
    balloon_icon: *mut c_void,
}
#[repr(C)]
struct Point {
    x: i32,
    y: i32,
}
#[link(name = "shell32")]
extern "system" {
    fn Shell_NotifyIconW(message: u32, data: *const NotifyIconData) -> i32;
}
#[link(name = "user32")]
extern "system" {
    fn LoadIconW(instance: *mut c_void, name: *const u16) -> *mut c_void;
    fn SetWindowLongPtrW(window: Hwnd, index: i32, value: isize) -> isize;
    fn CallWindowProcW(
        previous: WndProc,
        window: Hwnd,
        message: u32,
        wparam: usize,
        lparam: isize,
    ) -> isize;
    fn CreatePopupMenu() -> *mut c_void;
    fn AppendMenuW(menu: *mut c_void, flags: u32, id: usize, text: *const u16) -> i32;
    fn TrackPopupMenu(
        menu: *mut c_void,
        flags: u32,
        x: i32,
        y: i32,
        reserved: i32,
        window: Hwnd,
        rect: *const c_void,
    ) -> i32;
    fn DestroyMenu(menu: *mut c_void) -> i32;
    fn GetCursorPos(point: *mut Point) -> i32;
    fn SetForegroundWindow(window: Hwnd) -> i32;
    fn PostMessageW(window: Hwnd, msg: u32, wparam: usize, lparam: isize) -> i32;
    fn RegisterWindowMessageW(name: *const u16) -> u32;
}
const CALLBACK: u32 = 0x8000 + 42;
static ORIGINAL: AtomicIsize = AtomicIsize::new(0);
static PROXY: OnceLock<Mutex<EventLoopProxy<DesktopEvent>>> = OnceLock::new();
static TASKBAR: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
fn send(event: DesktopEvent) {
    if let Some(proxy) = PROXY.get() {
        let _ = proxy.lock().unwrap().send_event(event);
    }
}
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}
fn text<const N: usize>(to: &mut [u16; N], from: &str) {
    let mut at = 0;
    for ch in from.chars() {
        let mut chars = [0; 2];
        let units = ch.encode_utf16(&mut chars);
        if at + units.len() >= N {
            break;
        }
        to[at..at + units.len()].copy_from_slice(units);
        at += units.len()
    }
}
unsafe extern "system" fn procedure(
    window: Hwnd,
    message: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    if message == 0x0112 && (wparam & 0xfff0) == 0xf020 {
        send(DesktopEvent::WindowRequest("minimize"));
        return 0;
    }
    if message == TASKBAR.load(Ordering::Relaxed) && message != 0 {
        send(DesktopEvent::RebuildTray);
    }
    if message == CALLBACK {
        match lparam as u32 {
            0x0202 | 0x0203 | 0x0405 => send(DesktopEvent::Show),
            0x0205 => {
                let menu = CreatePopupMenu();
                let open = wide("Abrir Atelier");
                let exit = wide("Salvar e sair");
                AppendMenuW(menu, 0, 1, open.as_ptr());
                AppendMenuW(menu, 0, 2, exit.as_ptr());
                let mut point = Point { x: 0, y: 0 };
                GetCursorPos(&mut point);
                SetForegroundWindow(window);
                let choice = TrackPopupMenu(
                    menu,
                    0x0100 | 0x0002,
                    point.x,
                    point.y,
                    0,
                    window,
                    std::ptr::null(),
                );
                DestroyMenu(menu);
                PostMessageW(window, 0, 0, 0);
                if choice == 1 {
                    send(DesktopEvent::Show)
                }
                if choice == 2 {
                    send(DesktopEvent::ExitRequested)
                }
            }
            _ => {}
        }
        return 0;
    }
    let old = ORIGINAL.load(Ordering::Relaxed);
    if old != 0 {
        CallWindowProcW(
            std::mem::transmute::<isize, WndProc>(old),
            window,
            message,
            wparam,
            lparam,
        )
    } else {
        0
    }
}
pub struct Tray {
    data: NotifyIconData,
    pub available: bool,
}
impl Tray {
    pub fn new(window: Hwnd, proxy: EventLoopProxy<DesktopEvent>) -> Result<Self, String> {
        let _ = PROXY.set(Mutex::new(proxy));
        let old = unsafe { SetWindowLongPtrW(window, -4, procedure as *const () as isize) };
        if old == 0 {
            return Err("Não foi possível preparar os comandos de janela.".into());
        }
        ORIGINAL.store(old, Ordering::Relaxed);
        let name = wide("TaskbarCreated");
        TASKBAR.store(
            unsafe { RegisterWindowMessageW(name.as_ptr()) },
            Ordering::Relaxed,
        );
        let mut data: NotifyIconData = unsafe { std::mem::zeroed() };
        data.size = std::mem::size_of::<NotifyIconData>() as u32;
        data.window = window;
        data.id = 1;
        data.flags = 1 | 2 | 4;
        data.callback = CALLBACK;
        data.icon = unsafe { LoadIconW(std::ptr::null_mut(), 32512usize as *const u16) };
        text(&mut data.tip, "Atelier — projetos, prazos e lembretes");
        let mut tray = Self {
            data,
            available: false,
        };
        let _ = tray.add();
        Ok(tray)
    }
    pub fn add(&mut self) -> Result<(), String> {
        self.available = unsafe { Shell_NotifyIconW(0, &self.data) } != 0;
        if !self.available {
            Err("O Windows não disponibilizou a bandeja.".into())
        } else {
            Ok(())
        }
    }
    pub fn notify(&mut self, title: &str, body: &str) {
        self.data.flags = 16;
        self.data.info.fill(0);
        self.data.title.fill(0);
        text(&mut self.data.info, body);
        text(&mut self.data.title, title);
        self.data.info_flags = 1;
        self.data.timeout = 10000;
        unsafe {
            Shell_NotifyIconW(1, &self.data);
        }
        self.data.flags = 1 | 2 | 4;
    }
    pub fn remove(&self) {
        unsafe {
            Shell_NotifyIconW(2, &self.data);
        }
    }
}

impl Drop for Tray {
    fn drop(&mut self) {
        self.remove();
    }
}
