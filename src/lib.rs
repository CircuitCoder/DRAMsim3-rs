use std::{ffi::CStr, os::unix::ffi::OsStrExt};

mod ffi;

pub struct MemorySystem {
    ffi_ptr: *mut libc::c_void,
    _cb: Box<Box<dyn FnMut(u64, bool)>>,
}

extern "C" fn inflate_cb(deflated: *mut libc::c_void, addr: u64, is_write: bool) {
    let inflated = unsafe {
        &mut *(deflated as *mut Box<dyn FnMut(u64, bool)>)
    };
    inflated(addr, is_write);
}

impl MemorySystem {
    pub fn new<F: FnMut(u64, bool) + 'static>(
        config: &CStr,
        dir: &CStr,
        cb: F,
    ) -> MemorySystem {
        let mut cb_box: Box<Box<dyn FnMut(u64, bool)>> = Box::new(Box::new(cb));

        let handle = unsafe {
            ffi::ds3_create(
                cb_box.as_mut() as *mut _ as *mut libc::c_void,
                config.as_ptr(),
                dir.as_ptr(),
                inflate_cb,
            )
        };

        MemorySystem {
            ffi_ptr: handle,
            _cb: cb_box,
        }
    }

    pub fn tick(&mut self) {
        unsafe {
            ffi::ds3_tick(self.ffi_ptr);
        }
    }

    pub fn can_add(&self, addr: u64, is_write: bool) -> bool {
        unsafe {
            ffi::ds3_can_add(self.ffi_ptr, addr, is_write)
        }
    }

    pub fn add(&mut self, addr: u64, is_write: bool) -> bool {
        unsafe {
            ffi::ds3_add(self.ffi_ptr, addr, is_write)
        }
    }

    pub fn tck(&self) -> f64 {
        unsafe {
            ffi::ds3_get_tck(self.ffi_ptr)
        }
    }

    pub fn bus_bits(&self) -> usize {
        (unsafe {
            ffi::ds3_get_bus_bits(self.ffi_ptr)
        }) as usize
    }

    pub fn burst_length(&self) -> usize {
        (unsafe {
            ffi::ds3_get_burst_length(self.ffi_ptr)
        }) as usize
    }

    pub fn queue_size(&self) -> usize {
        (unsafe {
            ffi::ds3_get_queue_size(self.ffi_ptr)
        }) as usize
    }
}

impl Drop for MemorySystem {
    fn drop(&mut self) {
        unsafe { ffi::ds3_drop(self.ffi_ptr); }
    }
}

#[test]
fn test() -> anyhow::Result<()> {
    use std::path::PathBuf;
    use std::ffi::CString;
    use std::cell::RefCell;
    use std::rc::Rc;

    let mut config = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    config.push("DRAMsim3/configs/HBM2_4Gb_x128.ini");
    
    let dir = tempfile::tempdir()?;

    let config_c = CString::new(config.as_os_str().as_bytes())?;
    let dir_c = CString::new(dir.path().as_os_str().as_bytes())?;

    let mut pushed = false;
    let resolved: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));

    let sys_resolved = resolved.clone();

    let mut sys = MemorySystem::new(&config_c, &dir_c, move |addr, is_write| {
        assert_eq!(addr, 0xdeadbeef);
        assert_eq!(is_write, false);
        *sys_resolved.borrow_mut() = true;
    });

    loop {
        sys.tick();
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