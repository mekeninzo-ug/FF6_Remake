use std::ffi::c_void;
use std::mem::size_of;
use std::ptr::{null, null_mut};
use std::time::Instant;

use crate::app::{Game, Input};
use crate::gfx::{Canvas, HEIGHT, WIDTH};

type Hwnd = *mut c_void;
type Hdc = *mut c_void;
type Hinstance = *mut c_void;
type Hicon = *mut c_void;
type Hcursor = *mut c_void;
type Hbrush = *mut c_void;
type WndProc = Option<unsafe extern "system" fn(Hwnd, u32, usize, isize) -> isize>;

const WS_OVERLAPPEDWINDOW: u32 = 0x00CF_0000;
const CW_USEDEFAULT: i32 = 0x8000_0000u32 as i32;
const PM_REMOVE: u32 = 0x0001;
const WM_PAINT: u32 = 0x000F;
const WM_ERASEBKGND: u32 = 0x0014;
const WM_DESTROY: u32 = 0x0002;
const WM_CLOSE: u32 = 0x0010;
const WM_QUIT: u32 = 0x0012;
const DIB_RGB_COLORS: u32 = 0;
const SRCCOPY: u32 = 0x00CC_0020;
const BI_RGB: u32 = 0;

#[repr(C)]
struct Point {
    x: i32,
    y: i32,
}

#[repr(C)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[repr(C)]
struct Msg {
    hwnd: Hwnd,
    message: u32,
    w_param: usize,
    l_param: isize,
    time: u32,
    point: Point,
    private: u32,
}

#[repr(C)]
struct WndClassW {
    style: u32,
    wnd_proc: WndProc,
    cls_extra: i32,
    wnd_extra: i32,
    instance: Hinstance,
    icon: Hicon,
    cursor: Hcursor,
    background: Hbrush,
    menu_name: *const u16,
    class_name: *const u16,
}

#[repr(C)]
struct BitmapInfoHeader {
    size: u32,
    width: i32,
    height: i32,
    planes: u16,
    bit_count: u16,
    compression: u32,
    size_image: u32,
    x_pels_per_meter: i32,
    y_pels_per_meter: i32,
    colors_used: u32,
    colors_important: u32,
}

#[repr(C)]
struct BitmapInfo {
    header: BitmapInfoHeader,
    colors: [u32; 1],
}

#[link(name = "user32")]
extern "system" {
    fn RegisterClassW(class: *const WndClassW) -> u16;
    fn CreateWindowExW(
        ex_style: u32,
        class_name: *const u16,
        window_name: *const u16,
        style: u32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        parent: Hwnd,
        menu: *mut c_void,
        instance: Hinstance,
        parameter: *mut c_void,
    ) -> Hwnd;
    fn ShowWindow(window: Hwnd, command: i32) -> i32;
    fn UpdateWindow(window: Hwnd) -> i32;
    fn DefWindowProcW(window: Hwnd, message: u32, w_param: usize, l_param: isize) -> isize;
    fn PostQuitMessage(exit_code: i32);
    fn PostMessageW(window: Hwnd, message: u32, w_param: usize, l_param: isize) -> i32;
    fn PeekMessageW(message: *mut Msg, window: Hwnd, min: u32, max: u32, remove: u32) -> i32;
    fn TranslateMessage(message: *const Msg) -> i32;
    fn DispatchMessageW(message: *const Msg) -> isize;
    fn ValidateRect(window: Hwnd, rect: *const Rect) -> i32;
    fn AdjustWindowRect(rect: *mut Rect, style: u32, menu: i32) -> i32;
    fn GetClientRect(window: Hwnd, rect: *mut Rect) -> i32;
    fn GetDC(window: Hwnd) -> Hdc;
    fn ReleaseDC(window: Hwnd, dc: Hdc) -> i32;
    fn GetAsyncKeyState(key: i32) -> i16;
}

#[link(name = "gdi32")]
extern "system" {
    fn StretchDIBits(
        dc: Hdc,
        x_dest: i32,
        y_dest: i32,
        width_dest: i32,
        height_dest: i32,
        x_source: i32,
        y_source: i32,
        width_source: i32,
        height_source: i32,
        bits: *const c_void,
        info: *const BitmapInfo,
        usage: u32,
        rop: u32,
    ) -> i32;
    fn SetStretchBltMode(dc: Hdc, mode: i32) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleW(module_name: *const u16) -> Hinstance;
    fn Sleep(milliseconds: u32);
}

unsafe extern "system" fn window_proc(
    window: Hwnd,
    message: u32,
    w_param: usize,
    l_param: isize,
) -> isize {
    match message {
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            ValidateRect(window, null());
            0
        }
        _ => DefWindowProcW(window, message, w_param, l_param),
    }
}

struct Keys {
    old: [bool; 256],
}

impl Keys {
    fn new() -> Self {
        Self { old: [false; 256] }
    }

    fn poll(&mut self) -> Input {
        let mut current = [false; 256];
        for key in [
            0x25, 0x26, 0x27, 0x28, 0x0D, 0x1B, 0x58, 0x5A, 0x49, 0x10, 0xA0, 0xA1, 0x74, 0x78,
        ] {
            current[key] = unsafe { GetAsyncKeyState(key as i32) as u16 & 0x8000 != 0 };
        }
        let pressed = |key: usize| current[key] && !self.old[key];
        let input = Input {
            left: current[0x25],
            up: current[0x26],
            right: current[0x27],
            down: current[0x28],
            confirm: pressed(0x0D) || pressed(0x5A),
            cancel: pressed(0x1B) || pressed(0x58),
            menu: pressed(0x1B) || pressed(0x49),
            dash: current[0x58] || current[0x10] || current[0xA0] || current[0xA1],
            save: pressed(0x74),
            load: pressed(0x78),
        };
        self.old = current;
        input
    }
}

pub fn run() -> Result<(), String> {
    run_for_frames(None)
}

pub fn smoke_test() -> Result<(), String> {
    run_for_frames(Some(3))
}

fn run_for_frames(frame_limit: Option<usize>) -> Result<(), String> {
    let instance = unsafe { GetModuleHandleW(null()) };
    if instance.is_null() {
        return Err(String::from("GetModuleHandleW failed"));
    }
    let class_name: Vec<u16> = "FF6NativeWindow\0".encode_utf16().collect();
    let window_name: Vec<u16> = "Final Fantasy VI - Native Rust Prototype\0".encode_utf16().collect();
    let class = WndClassW {
        style: 0,
        wnd_proc: Some(window_proc),
        cls_extra: 0,
        wnd_extra: 0,
        instance,
        icon: null_mut(),
        cursor: null_mut(),
        background: null_mut(),
        menu_name: null(),
        class_name: class_name.as_ptr(),
    };
    if unsafe { RegisterClassW(&class) } == 0 {
        return Err(String::from("RegisterClassW failed"));
    }

    let style = WS_OVERLAPPEDWINDOW;
    let mut rect = Rect {
        left: 0,
        top: 0,
        right: WIDTH as i32 * 3,
        bottom: HEIGHT as i32 * 3,
    };
    if unsafe { AdjustWindowRect(&mut rect, style, 0) } == 0 {
        return Err(String::from("AdjustWindowRect failed"));
    }
    let window = unsafe {
        CreateWindowExW(
            0,
            class_name.as_ptr(),
            window_name.as_ptr(),
            style,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            rect.right - rect.left,
            rect.bottom - rect.top,
            null_mut(),
            null_mut(),
            instance,
            null_mut(),
        )
    };
    if window.is_null() {
        return Err(String::from("CreateWindowExW failed"));
    }
    unsafe {
        ShowWindow(window, 5);
        UpdateWindow(window);
    }
    let dc = unsafe { GetDC(window) };
    if dc.is_null() {
        return Err(String::from("GetDC failed"));
    }
    unsafe { SetStretchBltMode(dc, 3) };

    let info = BitmapInfo {
        header: BitmapInfoHeader {
            size: size_of::<BitmapInfoHeader>() as u32,
            width: WIDTH as i32,
            height: -(HEIGHT as i32),
            planes: 1,
            bit_count: 32,
            compression: BI_RGB,
            size_image: (WIDTH * HEIGHT * 4) as u32,
            x_pels_per_meter: 0,
            y_pels_per_meter: 0,
            colors_used: 0,
            colors_important: 0,
        },
        colors: [0],
    };

    let mut canvas = Canvas::new();
    let mut game = Game::new();
    let mut keys = Keys::new();
    let mut message = unsafe { std::mem::zeroed::<Msg>() };
    let mut previous_frame = Instant::now();
    let mut frames = 0usize;

    loop {
        while unsafe { PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) } != 0 {
            if message.message == WM_QUIT {
                unsafe { ReleaseDC(window, dc) };
                return Ok(());
            }
            unsafe {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }

        let now = Instant::now();
        let delta = now.duration_since(previous_frame);
        previous_frame = now;
        game.update(keys.poll(), delta);
        game.draw(&mut canvas);

        let mut client = Rect { left: 0, top: 0, right: 0, bottom: 0 };
        if unsafe { GetClientRect(window, &mut client) } != 0 {
            let width = client.right - client.left;
            let height = client.bottom - client.top;
            if width > 0 && height > 0 {
                let copied = unsafe {
                    StretchDIBits(
                        dc,
                        0,
                        0,
                        width,
                        height,
                        0,
                        0,
                        WIDTH as i32,
                        HEIGHT as i32,
                        canvas.pixels.as_ptr().cast(),
                        &info,
                        DIB_RGB_COLORS,
                        SRCCOPY,
                    )
                };
                if copied == 0 || copied == -1 {
                    unsafe { ReleaseDC(window, dc) };
                    return Err(String::from("StretchDIBits failed"));
                }
            }
        }
        frames += 1;
        if frame_limit.is_some_and(|limit| frames >= limit) {
            if unsafe { PostMessageW(window, WM_CLOSE, 0, 0) } == 0 {
                unsafe { ReleaseDC(window, dc) };
                return Err(String::from("PostMessageW(WM_CLOSE) failed"));
            }
        }
        unsafe { Sleep(16) };
    }
}
