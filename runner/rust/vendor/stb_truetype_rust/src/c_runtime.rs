use core;

#[cfg(not(any(unix, target_os = "windows")))]
compile_error!("stb_truetype_rust allocator currently supports Unix and Windows targets");

#[cfg(unix)]
#[link(name = "c")]
extern "C" {
    #[link_name = "malloc"]
    fn c_malloc(size: usize) -> *mut u8;
    #[link_name = "realloc"]
    fn c_realloc(ptr: *mut u8, size: usize) -> *mut u8;
    #[link_name = "free"]
    fn c_free(ptr: *mut u8);
}

#[cfg(target_os = "windows")]
#[link(name = "msvcrt")]
extern "C" {
    #[link_name = "malloc"]
    fn c_malloc(size: usize) -> *mut u8;
    #[link_name = "realloc"]
    fn c_realloc(ptr: *mut u8, size: usize) -> *mut u8;
    #[link_name = "free"]
    fn c_free(ptr: *mut u8);
}

pub trait One {
    fn one() -> Self;
}

impl One for i32 {
    fn one() -> Self {
        1
    }
}

impl One for u32 {
    fn one() -> Self {
        1
    }
}

impl One for u8 {
    fn one() -> Self {
        1
    }
}

impl One for u16 {
    fn one() -> Self {
        1
    }
}

pub unsafe fn postInc<T: core::ops::AddAssign + One + Copy>(a: *mut T) -> T {
    let result: T = *a;
    *a += One::one();
    return result;
}

pub unsafe fn preInc<T: core::ops::AddAssign + One + Copy>(a: *mut T) -> T {
    *a += One::one();
    return *a;
}

pub unsafe fn postDec<T: core::ops::SubAssign + One + Copy>(a: *mut T) -> T {
    let result: T = *a;
    *a -= One::one();
    return result;
}

pub unsafe fn preDec<T: core::ops::SubAssign + One + Copy>(a: *mut T) -> T {
    *a -= One::one();
    return *a;
}

pub unsafe fn preIncPtr<T>(a: *mut *mut T) -> *mut T {
    *a = (*a).offset(1);
    return *a;
}

pub unsafe fn preDecPtr<T>(a: *mut *mut T) -> *mut T {
    *a = (*a).offset(-1);
    return *a;
}

pub unsafe fn postIncPtr<T>(a: *mut *mut T) -> *mut T {
    let result: *mut T = *a;
    *a = (*a).offset(1);
    return result;
}

pub unsafe fn postDecPtr<T>(a: *mut *mut T) -> *mut T {
    let result: *mut T = *a;
    *a = (*a).offset(-1);
    return result;
}

pub unsafe fn preIncConstPtr<T>(a: *mut *const T) -> *const T {
    *a = (*a).offset(1);
    return *a;
}

pub unsafe fn preDecConstPtr<T>(a: *mut *const T) -> *const T {
    *a = (*a).offset(-1);
    return *a;
}

pub unsafe fn postIncConstPtr<T>(a: *mut *const T) -> *const T {
    let result: *const T = *a;
    *a = (*a).offset(1);
    return result;
}

pub unsafe fn postDecConstPtr<T>(a: *mut *const T) -> *const T {
    let result: *const T = *a;
    *a = (*a).offset(-1);
    return result;
}

pub unsafe fn memcpy(src: *mut u8, dest: *mut u8, count: u64) {
    core::ptr::copy_nonoverlapping(dest, src, count as usize);
}

pub unsafe fn memset(src: *mut u8, value: i32, count: u64) {
    core::ptr::write_bytes(src, value as u8, count as usize);
}

pub unsafe fn malloc(count: u64) -> *mut u8 { c_malloc(count as usize) }

pub unsafe fn realloc<T>(data: *mut T, count: u64) -> *mut u8 { c_realloc(data as *mut u8, count as usize) }

pub unsafe fn free<T>(data: *mut T) { c_free(data as *mut u8); }

pub fn _lrotl(x: u32, y: i32) -> u32 {
    return (x << y) | (x >> (32 - y));
}

pub fn abs(x: i32) -> i32 {
    return i32::abs(x);
}

pub fn pow(x: f32, p: f32) -> f32 {
    return libm::powf(x, p);
}

pub fn fabs(x: f32) -> f32 {
    return libm::fabsf(x);
}

pub fn fmod(x: f32, y: f32) -> f32 {
    return libm::fmodf(x, y);
}

pub unsafe fn strlen(str: *mut u8) -> i32 {
    let mut ptr = str;
    let mut result = 0;

    while *ptr != 0 {
        ptr = ptr.offset(1);
        result += 1;
    }

    return result;
}

pub fn sqrt(x: f32) -> f32 {
    return libm::sqrtf(x);
}

pub fn acos(x: f32) -> f32 {
    return libm::acosf(x);
}

pub fn cos(x: f32) -> f32 {
    return libm::cosf(x);
}

pub fn floor(x: f32) -> f32 {
    return libm::floorf(x);
}

pub fn ceil(x: f32) -> f32 {
    return libm::ceilf(x);
}
