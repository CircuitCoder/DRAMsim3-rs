use std::{
    cell::UnsafeCell,
    ffi::{c_void, CStr},
    ptr::null_mut,
};

mod ffi;

// equavilent to `*const dyn FnMut(u64, bool)`,
// but raw fat pointer is unstable
#[repr(C)]
#[derive(Clone, Copy)]
struct RawCallbackFnMut {
    fun: unsafe extern "C" fn(data: *mut c_void, addr: u64, is_write: bool),
    data: *mut c_void,
}

impl RawCallbackFnMut {
    pub const fn invalid() -> Self {
        RawCallbackFnMut {
            fun: Self::_invalid_fn,
            data: null_mut(),
        }
    }

    pub fn from_fn_mut<F: FnMut(u64, bool)>(f: &mut F) -> Self {
        RawCallbackFnMut {
            fun: Self::_invoke_fn_mut::<F>,
            data: f as *mut F as *mut c_void,
        }
    }

    unsafe extern "C" fn _invalid_fn(_data: *mut c_void, _addr: u64, _is_write: bool) {
        unreachable!("callback is invoked outside tick");
    }

    unsafe extern "C" fn _invoke_fn_mut<F: FnMut(u64, bool)>(
        f: *mut c_void,
        addr: u64,
        is_write: bool,
    ) {
        (*(f as *mut F))(addr, is_write)
    }

    extern "C" fn _invoke_cb(this: *mut c_void, addr: u64, is_write: bool) {
        unsafe {
            let this = *(this as *mut RawCallbackFnMut);
            (this.fun)(this.data, addr, is_write);
        }
    }
}

pub struct MemorySystem {
    ffi_ptr: *mut c_void,

    // _cb is used by both C++ and Rust side,
    // use UnsafeCell to avoid potential aliasing UB
    _cb: Box<UnsafeCell<RawCallbackFnMut>>,
}

// safety: `Send` generally means safe under mutex
unsafe impl Send for MemorySystem {}

impl MemorySystem {
    pub fn new(config: &CStr, dir: &CStr) -> MemorySystem {
        let cb_box: Box<UnsafeCell<RawCallbackFnMut>> =
            Box::new(UnsafeCell::new(RawCallbackFnMut::invalid()));
        let cb_ptr: *mut RawCallbackFnMut = cb_box.get();

        let handle = unsafe {
            ffi::ds3_create(
                cb_ptr as *mut c_void,
                config.as_ptr(),
                dir.as_ptr(),
                RawCallbackFnMut::_invoke_cb,
            )
        };

        MemorySystem {
            ffi_ptr: handle,
            _cb: cb_box,
        }
    }

    pub fn tick(&mut self, mut f: impl FnMut(u64, bool)) {
        let cb_ptr: *mut RawCallbackFnMut = self._cb.get();
        // safety: we hold `&mut self` here, no data races when changing cb_ptr
        unsafe {
            *cb_ptr = RawCallbackFnMut::from_fn_mut(&mut f);
            ffi::ds3_tick(self.ffi_ptr);

            // could be omitted, add as additional safe guard
            *cb_ptr = RawCallbackFnMut::invalid();
        }
    }

    pub fn can_add(&self, addr: u64, is_write: bool) -> bool {
        unsafe { ffi::ds3_can_add(self.ffi_ptr, addr, is_write) }
    }

    pub fn add(&mut self, addr: u64, is_write: bool) -> bool {
        unsafe { ffi::ds3_add(self.ffi_ptr, addr, is_write) }
    }

    pub fn tck(&self) -> f64 {
        unsafe { ffi::ds3_get_tck(self.ffi_ptr) }
    }

    pub fn bus_bits(&self) -> usize {
        (unsafe { ffi::ds3_get_bus_bits(self.ffi_ptr) }) as usize
    }

    pub fn burst_length(&self) -> usize {
        (unsafe { ffi::ds3_get_burst_length(self.ffi_ptr) }) as usize
    }

    pub fn queue_size(&self) -> usize {
        (unsafe { ffi::ds3_get_queue_size(self.ffi_ptr) }) as usize
    }
}

impl Drop for MemorySystem {
    fn drop(&mut self) {
        unsafe {
            ffi::ds3_drop(self.ffi_ptr);
        }
    }
}

#[test]
fn test() -> anyhow::Result<()> {
    use std::cell::RefCell;
    use std::ffi::CString;
    use std::path::PathBuf;
    use std::rc::Rc;

    let mut config = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    config.push("DRAMsim3/configs/HBM2_4Gb_x128.ini");

    let dir = tempfile::tempdir()?;

    let config_c = CString::new(config.as_os_str().as_encoded_bytes())?;
    let dir_c = CString::new(dir.path().as_os_str().as_encoded_bytes())?;

    let mut pushed = false;
    let resolved: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));

    let sys_resolved = resolved.clone();

    let mut sys = MemorySystem::new(&config_c, &dir_c);

    loop {
        sys.tick(|addr, is_write| {
            assert_eq!(addr, 0xdeadbeef);
            assert_eq!(is_write, false);
            *sys_resolved.borrow_mut() = true;
        });
        if !pushed && sys.can_add(0xdeadbeef, false) {
            assert!(sys.add(0xdeadbeef, false));
            pushed = true;
        }

        {
            if *resolved.borrow() {
                break;
            }
        }
    }

    Ok(())
}
