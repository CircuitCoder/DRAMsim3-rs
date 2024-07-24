extern "C" {
  pub fn ds3_create(
    data: *mut libc::c_void,
    config_path: *const i8,
    sim_path: *const i8,
    callback: extern "C" fn(*mut libc::c_void, u64, bool),
  ) -> *mut libc::c_void;
  pub fn ds3_tick(sys: *mut libc::c_void);
  pub fn ds3_can_add(sys: *mut libc::c_void, addr: u64, is_write: bool) -> bool;
  pub fn ds3_add(sys: *mut libc::c_void, addr: u64, is_write: bool) -> bool;
  pub fn ds3_drop(sys: *mut libc::c_void);
}