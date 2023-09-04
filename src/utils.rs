


#[cfg(target_family = "windows")]
pub mod windows_utils {
    





    fn admin_save() {

        windows::initialize_sta().unwrap();
        let r = unsafe { ShellExecuteW(HWND::NULL, "runas",HELPER_PATH, PWSTR::NULL, PWSTR::NULL, 1) };
        if r.0 < 32 {
            eprintln!("error: {:?}", r);
        }

    }



}
