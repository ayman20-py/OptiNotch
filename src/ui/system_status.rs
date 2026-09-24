use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BatteryStatus {
    pub percentage: u8,
    pub is_charging: bool,
    pub has_battery: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VolumeStatus {
    pub percentage: u8,
    pub is_muted: bool,
}

pub struct SystemStatusTracker {
    battery: Arc<std::sync::Mutex<BatteryStatus>>,
    volume: Arc<std::sync::Mutex<VolumeStatus>>,
    #[allow(dead_code)]
    running: Arc<AtomicBool>,
}

impl SystemStatusTracker {
    pub fn new() -> Self {
        let battery = Arc::new(std::sync::Mutex::new(get_battery_status()));
        let volume = Arc::new(std::sync::Mutex::new(get_volume_status()));
        let running = Arc::new(AtomicBool::new(true));

        let b_clone = Arc::clone(&battery);
        let v_clone = Arc::clone(&volume);
        let r_clone = Arc::clone(&running);

        // Fast background polling thread (100ms interval for ultra-responsive live volume changes)
        thread::spawn(move || {
            let mut tick = 0u32;
            while r_clone.load(Ordering::Relaxed) {
                // Poll volume every 50ms for instant live updates
                let cur_vol = get_volume_status();
                {
                    let mut v = v_clone.lock().unwrap();
                    *v = cur_vol;
                }

                // Poll battery every 1s (20 ticks of 50ms)
                tick = (tick + 1) % 20;
                if tick == 0 {
                    let cur_bat = get_battery_status();
                    let mut b = b_clone.lock().unwrap();
                    *b = cur_bat;
                }

                thread::sleep(Duration::from_millis(50));
            }
        });

        Self {
            battery,
            volume,
            running,
        }
    }

    pub fn get_battery(&self) -> BatteryStatus {
        *self.battery.lock().unwrap()
    }

    pub fn get_volume(&self) -> VolumeStatus {
        *self.volume.lock().unwrap()
    }
}

pub fn get_battery_status() -> BatteryStatus {
    use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    let mut status: SYSTEM_POWER_STATUS = unsafe { std::mem::zeroed() };
    if unsafe { GetSystemPowerStatus(&mut status) } != 0 {
        // BatteryFlag:
        // 128 = No system battery (Desktop PC)
        // 255 = Unknown
        let has_battery = status.BatteryFlag != 128
            && (status.BatteryFlag & 128) == 0
            && status.BatteryLifePercent <= 100;
        let is_charging = (status.BatteryFlag & 8) != 0 || status.ACLineStatus == 1;
        let percentage = if status.BatteryLifePercent <= 100 {
            status.BatteryLifePercent
        } else {
            0
        };
        BatteryStatus {
            percentage,
            is_charging,
            has_battery,
        }
    } else {
        BatteryStatus {
            percentage: 0,
            is_charging: false,
            has_battery: false,
        }
    }
}

pub fn get_volume_status() -> VolumeStatus {
    get_volume_core_audio().unwrap_or(VolumeStatus {
        percentage: 50,
        is_muted: false,
    })
}

// Low-level Core Audio volume query using Windows COM
fn get_volume_core_audio() -> Option<VolumeStatus> {
    use windows_sys::core::GUID;
    use windows_sys::Win32::System::Com::{
        CLSCTX_ALL, CoCreateInstance, CoInitializeEx, COINIT_MULTITHREADED,
    };

    unsafe {
        let _ = CoInitializeEx(std::ptr::null_mut(), COINIT_MULTITHREADED as _);

        // CLSID_MMDeviceEnumerator
        let clsid_mmdevice_enumerator = GUID {
            data1: 0xBCDE0395,
            data2: 0xE52F,
            data3: 0x467C,
            data4: [0x8E, 0x3D, 0xC4, 0x57, 0x92, 0x91, 0x69, 0x2E],
        };

        // IID_IMMDeviceEnumerator
        let iid_immdevice_enumerator = GUID {
            data1: 0xA95664D2,
            data2: 0x9614,
            data3: 0x4F35,
            data4: [0xA7, 0x46, 0xDE, 0x8D, 0xB6, 0x36, 0x17, 0xE6],
        };

        let mut enumerator: *mut std::ffi::c_void = std::ptr::null_mut();
        let hr = CoCreateInstance(
            &clsid_mmdevice_enumerator,
            std::ptr::null_mut(),
            CLSCTX_ALL,
            &iid_immdevice_enumerator,
            &mut enumerator,
        );

        if hr != 0 || enumerator.is_null() {
            return None;
        }

        // IMMDeviceEnumerator vtable:
        // [0] QueryInterface, [1] AddRef, [2] Release
        // [3] EnumAudioEndpoints
        // [4] GetDefaultAudioEndpoint(EDataFlow dataFlow, ERole role, IMMDevice **ppEndpoint)
        type GetDefaultAudioEndpointFn = unsafe extern "system" fn(
            this: *mut std::ffi::c_void,
            data_flow: i32, // eRender = 0
            role: i32,      // eMultimedia = 1
            pp_endpoint: *mut *mut std::ffi::c_void,
        ) -> i32;

        type ReleaseFn = unsafe extern "system" fn(this: *mut std::ffi::c_void) -> u32;

        let vtable = *(enumerator as *mut *mut *mut std::ffi::c_void);
        let get_default_endpoint: GetDefaultAudioEndpointFn =
            std::mem::transmute(*vtable.offset(4));
        let release_enumerator: ReleaseFn = std::mem::transmute(*vtable.offset(2));

        let mut device: *mut std::ffi::c_void = std::ptr::null_mut();
        let hr = get_default_endpoint(enumerator, 0, 1, &mut device);
        release_enumerator(enumerator);

        if hr != 0 || device.is_null() {
            return None;
        }

        // IMMDevice vtable:
        // [0] QueryInterface, [1] AddRef, [2] Release
        // [3] Activate(REFIID iid, DWORD dwClsCtx, PROPVARIANT *pActivationParams, void **ppInterface)
        type ActivateFn = unsafe extern "system" fn(
            this: *mut std::ffi::c_void,
            iid: *const GUID,
            dw_cls_ctx: u32,
            p_activation_params: *mut std::ffi::c_void,
            pp_interface: *mut *mut std::ffi::c_void,
        ) -> i32;

        let dev_vtable = *(device as *mut *mut *mut std::ffi::c_void);
        let activate: ActivateFn = std::mem::transmute(*dev_vtable.offset(3));
        let release_device: ReleaseFn = std::mem::transmute(*dev_vtable.offset(2));

        // IID_IAudioEndpointVolume = {5CDF2C82-841E-4546-9722-0CF74078229A}
        let iid_iaudio_endpoint_volume = GUID {
            data1: 0x5CDF2C82,
            data2: 0x841E,
            data3: 0x4546,
            data4: [0x97, 0x22, 0x0C, 0xF7, 0x40, 0x78, 0x22, 0x9A],
        };

        let mut endpoint_volume: *mut std::ffi::c_void = std::ptr::null_mut();
        let hr = activate(
            device,
            &iid_iaudio_endpoint_volume,
            CLSCTX_ALL,
            std::ptr::null_mut(),
            &mut endpoint_volume,
        );
        release_device(device);

        if hr != 0 || endpoint_volume.is_null() {
            return None;
        }

        // IAudioEndpointVolume vtable:
        // [0] QueryInterface, [1] AddRef, [2] Release
        // [3] RegisterControlChangeNotify
        // [4] UnregisterControlChangeNotify
        // [5] GetChannelCount
        // [6] SetMasterVolumeLevel
        // [7] SetMasterVolumeLevelScalar
        // [8] GetMasterVolumeLevel
        // [9] GetMasterVolumeLevelScalar(float *pfLevel)
        // [10] SetChannelVolumeLevel
        // [11] SetChannelVolumeLevelScalar
        // [12] GetChannelVolumeLevel
        // [13] GetChannelVolumeLevelScalar
        // [14] SetMute
        // [15] GetMute(BOOL *pbMute)
        type GetMasterVolumeLevelScalarFn = unsafe extern "system" fn(
            this: *mut std::ffi::c_void,
            pf_level: *mut f32,
        ) -> i32;

        type GetMuteFn = unsafe extern "system" fn(
            this: *mut std::ffi::c_void,
            pb_mute: *mut i32,
        ) -> i32;

        let vol_vtable = *(endpoint_volume as *mut *mut *mut std::ffi::c_void);
        let get_vol_scalar: GetMasterVolumeLevelScalarFn =
            std::mem::transmute(*vol_vtable.offset(9));
        let get_mute: GetMuteFn = std::mem::transmute(*vol_vtable.offset(15));
        let release_vol: ReleaseFn = std::mem::transmute(*vol_vtable.offset(2));

        let mut level: f32 = 0.0;
        let mut mute: i32 = 0;
        let _ = get_vol_scalar(endpoint_volume, &mut level);
        let _ = get_mute(endpoint_volume, &mut mute);
        release_vol(endpoint_volume);

        let percentage = (level.clamp(0.0, 1.0) * 100.0).round() as u8;
        let is_muted = mute != 0;

        Some(VolumeStatus {
            percentage,
            is_muted,
        })
    }
}
