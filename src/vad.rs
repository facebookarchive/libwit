use libc::{c_int, c_double, c_void};

extern {
    pub fn wvs_still_talking(state: *const c_void, samples: *const i16, nb_samples: c_int) -> c_int;
    pub fn wvs_init(threshold: c_double, sample_rate: c_int) -> *const c_void;
    pub fn wvs_clean(state: *const c_void);
}
