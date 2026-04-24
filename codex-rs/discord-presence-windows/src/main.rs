#[cfg(target_os = "windows")]
use std::ffi::c_void;
#[cfg(target_os = "windows")]
use std::io::BufRead;
#[cfg(target_os = "windows")]
use std::io::Write;
#[cfg(target_os = "windows")]
use std::ptr;
#[cfg(target_os = "windows")]
use std::sync::mpsc;
#[cfg(target_os = "windows")]
use std::time::Duration;

#[cfg(target_os = "windows")]
use anyhow::Context;
use anyhow::Result;
use clap::Parser;
#[cfg(target_os = "windows")]
use codex_discord_presence::HelperCommand;
#[cfg(target_os = "windows")]
use codex_discord_presence::HelperEvent;

#[derive(Debug, Parser)]
struct Cli {
    #[arg(long = "application-id")]
    application_id: u64,

    #[arg(long = "large-image")]
    large_image: Option<String>,

    #[arg(long = "large-text")]
    large_text: Option<String>,
}

#[cfg(not(target_os = "windows"))]
fn main() -> Result<()> {
    anyhow::bail!("codex-discord-presence-windows must be built for Windows");
}

#[cfg(target_os = "windows")]
fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut client = DiscordPresenceBridge::new(cli.application_id)
        .context("failed to initialize Discord Social SDK client")?;

    write_event(&HelperEvent::Ready)?;

    let (tx, rx) = mpsc::channel();
    let stdin = std::io::stdin();
    std::thread::spawn(move || {
        let mut lines = stdin.lock().lines();
        while let Some(line) = lines.next() {
            match line {
                Ok(line) => {
                    if tx.send(line).is_err() {
                        return;
                    }
                }
                Err(err) => {
                    let _ = write_event(&HelperEvent::Error {
                        message: format!("failed to read stdin for discord presence helper: {err}"),
                    });
                    return;
                }
            }
        }
    });

    loop {
        match rx.recv_timeout(Duration::from_millis(250)) {
            Ok(line) => {
                let command = serde_json::from_str::<HelperCommand>(&line)
                    .with_context(|| format!("failed to parse helper command: {line}"))?;
                match command {
                    HelperCommand::SetPresence {
                        details,
                        state,
                        small_image,
                        small_text,
                        start_timestamp_seconds,
                    } => {
                        if let Err(err) = client.set_presence(
                            &details,
                            state.as_deref(),
                            small_image.as_deref(),
                            small_text.as_deref(),
                            start_timestamp_seconds,
                            cli.large_image.as_deref(),
                        ) {
                            write_event(&HelperEvent::Error {
                                message: err.to_string(),
                            })?;
                        }
                    }
                    HelperCommand::ClearPresence => client.clear_presence(),
                    HelperCommand::Shutdown => break,
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => client.run_callbacks(),
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    client.clear_presence();
    Ok(())
}

#[cfg(target_os = "windows")]
fn write_event(event: &HelperEvent) -> Result<()> {
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    serde_json::to_writer(&mut handle, event)?;
    handle.write_all(b"\n")?;
    handle.flush()?;
    Ok(())
}

#[cfg(target_os = "windows")]
struct DiscordPresenceBridge {
    client: DiscordClient,
    current_activity: Option<DiscordPresenceActivity>,
}

#[cfg(target_os = "windows")]
impl DiscordPresenceBridge {
    fn new(application_id: u64) -> Result<Self> {
        let mut client = DiscordClient::new();
        client.set_application_id(application_id);
        Ok(Self {
            client,
            current_activity: None,
        })
    }

    fn set_presence(
        &mut self,
        details: &str,
        state: Option<&str>,
        small_image: Option<&str>,
        small_text: Option<&str>,
        start_timestamp_seconds: u64,
        large_image: Option<&str>,
    ) -> Result<()> {
        let activity = DiscordPresenceActivity::new(
            details,
            state,
            small_image,
            small_text,
            start_timestamp_seconds,
            large_image,
        );
        self.client.update_presence(&activity.activity);
        self.current_activity = Some(activity);
        self.client.run_callbacks();
        Ok(())
    }

    fn clear_presence(&mut self) {
        self.current_activity = None;
        self.client.clear_presence();
        self.client.run_callbacks();
    }

    fn run_callbacks(&mut self) {
        self.client.run_callbacks();
    }
}

#[cfg(target_os = "windows")]
struct DiscordClient {
    inner: ffi::Discord_Client,
}

#[cfg(target_os = "windows")]
impl DiscordClient {
    fn new() -> Self {
        let mut inner = ffi::Discord_Client {
            opaque: ptr::null_mut(),
        };
        unsafe {
            ffi::Discord_Client_Init(&mut inner);
        }
        Self { inner }
    }

    fn set_application_id(&mut self, application_id: u64) {
        unsafe {
            ffi::Discord_Client_SetApplicationId(&mut self.inner, application_id);
        }
    }

    fn update_presence(&mut self, activity: &DiscordActivity) {
        unsafe {
            ffi::Discord_Client_UpdateRichPresence(
                &mut self.inner,
                &activity.inner as *const _ as *mut _,
                Some(discord_client_result_callback),
                None,
                ptr::null_mut(),
            );
        }
    }

    fn clear_presence(&mut self) {
        unsafe {
            ffi::Discord_Client_ClearRichPresence(&mut self.inner);
        }
    }

    fn run_callbacks(&mut self) {
        unsafe {
            ffi::Discord_RunCallbacks();
        }
    }
}

#[cfg(target_os = "windows")]
impl Drop for DiscordClient {
    fn drop(&mut self) {
        unsafe {
            ffi::Discord_Client_Disconnect(&mut self.inner);
            ffi::Discord_Client_Drop(&mut self.inner);
        }
    }
}

#[cfg(target_os = "windows")]
struct DiscordActivity {
    inner: ffi::Discord_Activity,
}

#[cfg(target_os = "windows")]
impl DiscordActivity {
    fn new() -> Self {
        let mut inner = ffi::Discord_Activity {
            opaque: ptr::null_mut(),
        };
        unsafe {
            ffi::Discord_Activity_Init(&mut inner);
        }
        Self { inner }
    }

    fn set_type(&mut self, activity_type: DiscordActivityTypes) {
        unsafe {
            ffi::Discord_Activity_SetType(&mut self.inner, activity_type);
        }
    }

    fn set_status_display_type(&mut self, display_type: DiscordStatusDisplayTypes) {
        unsafe {
            ffi::Discord_Activity_SetStatusDisplayType(&mut self.inner, &display_type);
        }
    }
}

#[cfg(target_os = "windows")]
impl Drop for DiscordActivity {
    fn drop(&mut self) {
        unsafe {
            ffi::Discord_Activity_Drop(&mut self.inner);
        }
    }
}

#[cfg(target_os = "windows")]
struct DiscordActivityAssets {
    inner: ffi::Discord_ActivityAssets,
}

#[cfg(target_os = "windows")]
impl DiscordActivityAssets {
    fn new() -> Self {
        let mut inner = ffi::Discord_ActivityAssets {
            opaque: ptr::null_mut(),
        };
        unsafe {
            ffi::Discord_ActivityAssets_Init(&mut inner);
        }
        Self { inner }
    }

    fn set_large_image(&mut self, value: &mut DiscordOwnedString) {
        unsafe {
            let mut raw = value.raw();
            ffi::Discord_ActivityAssets_SetLargeImage(&mut self.inner, &mut raw);
        }
    }

    fn set_large_text(&mut self, value: &mut DiscordOwnedString) {
        unsafe {
            let mut raw = value.raw();
            ffi::Discord_ActivityAssets_SetLargeText(&mut self.inner, &mut raw);
        }
    }

    fn set_small_image(&mut self, value: &mut DiscordOwnedString) {
        unsafe {
            let mut raw = value.raw();
            ffi::Discord_ActivityAssets_SetSmallImage(&mut self.inner, &mut raw);
        }
    }

    fn set_small_text(&mut self, value: &mut DiscordOwnedString) {
        unsafe {
            let mut raw = value.raw();
            ffi::Discord_ActivityAssets_SetSmallText(&mut self.inner, &mut raw);
        }
    }
}

#[cfg(target_os = "windows")]
impl Drop for DiscordActivityAssets {
    fn drop(&mut self) {
        unsafe {
            ffi::Discord_ActivityAssets_Drop(&mut self.inner);
        }
    }
}

#[cfg(target_os = "windows")]
struct DiscordActivityTimestamps {
    inner: ffi::Discord_ActivityTimestamps,
}

#[cfg(target_os = "windows")]
impl DiscordActivityTimestamps {
    fn new(start_timestamp_seconds: u64) -> Self {
        let mut inner = ffi::Discord_ActivityTimestamps {
            opaque: ptr::null_mut(),
        };
        unsafe {
            ffi::Discord_ActivityTimestamps_Init(&mut inner);
            ffi::Discord_ActivityTimestamps_SetStart(&mut inner, start_timestamp_seconds);
        }
        Self { inner }
    }
}

#[cfg(target_os = "windows")]
impl Drop for DiscordActivityTimestamps {
    fn drop(&mut self) {
        unsafe {
            ffi::Discord_ActivityTimestamps_Drop(&mut self.inner);
        }
    }
}

#[cfg(target_os = "windows")]
struct DiscordOwnedString {
    bytes: Vec<u8>,
}

#[cfg(target_os = "windows")]
impl DiscordOwnedString {
    fn new(value: &str) -> Self {
        Self {
            bytes: value.as_bytes().to_vec(),
        }
    }

    fn raw(&mut self) -> ffi::Discord_String {
        ffi::Discord_String::from_mut_bytes(&mut self.bytes)
    }
}

#[cfg(target_os = "windows")]
struct DiscordPresenceActivity {
    activity: DiscordActivity,
    _details: DiscordOwnedString,
    _state: Option<DiscordOwnedString>,
    _timestamps: DiscordActivityTimestamps,
    _assets: Option<DiscordPresenceAssets>,
}

#[cfg(target_os = "windows")]
impl DiscordPresenceActivity {
    fn new(
        details: &str,
        state: Option<&str>,
        small_image: Option<&str>,
        small_text: Option<&str>,
        start_timestamp_seconds: u64,
        large_image: Option<&str>,
    ) -> Self {
        let mut activity = DiscordActivity::new();
        activity.set_type(DiscordActivityTypes::Playing);
        activity.set_status_display_type(DiscordStatusDisplayTypes::Details);

        let mut details_owned = DiscordOwnedString::new(details);
        let mut details_raw = details_owned.raw();
        unsafe {
            ffi::Discord_Activity_SetDetails(&mut activity.inner, &mut details_raw);
        }

        let mut state_owned = state.map(DiscordOwnedString::new);
        if let Some(state_owned) = state_owned.as_mut() {
            let mut state_raw = state_owned.raw();
            unsafe {
                ffi::Discord_Activity_SetState(&mut activity.inner, &mut state_raw);
            }
        }

        let timestamps = DiscordActivityTimestamps::new(start_timestamp_seconds);
        unsafe {
            ffi::Discord_Activity_SetTimestamps(
                &mut activity.inner,
                &timestamps.inner as *const _ as *mut _,
            );
        }

        let mut assets = if large_image.is_some() || small_image.is_some() || small_text.is_some() {
            Some(DiscordPresenceAssets::new(
                large_image,
                /*large_text*/ None,
                small_image,
                small_text,
            ))
        } else {
            None
        };

        if let Some(assets) = assets.as_mut() {
            unsafe {
                ffi::Discord_Activity_SetAssets(
                    &mut activity.inner,
                    &assets.assets.inner as *const _ as *mut _,
                );
            }
        }

        Self {
            activity,
            _details: details_owned,
            _state: state_owned,
            _timestamps: timestamps,
            _assets: assets,
        }
    }
}

#[cfg(target_os = "windows")]
struct DiscordPresenceAssets {
    assets: DiscordActivityAssets,
    _large_image: Option<DiscordOwnedString>,
    _large_text: Option<DiscordOwnedString>,
    _small_image: Option<DiscordOwnedString>,
    _small_text: Option<DiscordOwnedString>,
}

#[cfg(target_os = "windows")]
impl DiscordPresenceAssets {
    fn new(
        large_image: Option<&str>,
        large_text: Option<&str>,
        small_image: Option<&str>,
        small_text: Option<&str>,
    ) -> Self {
        let mut assets = DiscordActivityAssets::new();
        let mut large_image_owned = large_image.map(DiscordOwnedString::new);
        let mut large_text_owned = large_text.map(DiscordOwnedString::new);
        let mut small_image_owned = small_image.map(DiscordOwnedString::new);
        let mut small_text_owned = small_text.map(DiscordOwnedString::new);

        if let Some(value) = large_image_owned.as_mut() {
            assets.set_large_image(value);
        }
        if let Some(value) = large_text_owned.as_mut() {
            assets.set_large_text(value);
        }
        if let Some(value) = small_image_owned.as_mut() {
            assets.set_small_image(value);
        }
        if let Some(value) = small_text_owned.as_mut() {
            assets.set_small_text(value);
        }

        Self {
            assets,
            _large_image: large_image_owned,
            _large_text: large_text_owned,
            _small_image: small_image_owned,
            _small_text: small_text_owned,
        }
    }
}

#[cfg(target_os = "windows")]
#[repr(i32)]
#[derive(Clone, Copy)]
enum DiscordActivityTypes {
    Playing = 0,
}

#[cfg(target_os = "windows")]
#[repr(i32)]
#[derive(Clone, Copy)]
enum DiscordStatusDisplayTypes {
    Details = 2,
}

#[cfg(target_os = "windows")]
unsafe extern "C" fn discord_client_result_callback(
    result: *mut ffi::Discord_ClientResult,
    _user_data: *mut c_void,
) {
    if result.is_null() {
        return;
    }

    let successful = unsafe { ffi::Discord_ClientResult_Successful(result) };
    if successful {
        return;
    }

    let mut error = ffi::Discord_String::empty();
    unsafe {
        ffi::Discord_ClientResult_Error(result, &mut error);
    }
    let _ = write_event(&HelperEvent::Error {
        message: error.to_string_lossy(),
    });
}

#[cfg(target_os = "windows")]
mod ffi {
    use super::c_void;

    #[repr(C)]
    pub struct Discord_String {
        ptr: *mut u8,
        size: usize,
    }

    impl Discord_String {
        pub fn empty() -> Self {
            Self {
                ptr: std::ptr::null_mut(),
                size: 0,
            }
        }

        pub fn from_mut_bytes(value: &mut Vec<u8>) -> Self {
            Self {
                ptr: value.as_mut_ptr(),
                size: value.len(),
            }
        }

        pub fn to_string_lossy(&self) -> String {
            if self.ptr.is_null() || self.size == 0 {
                return String::new();
            }

            let bytes = unsafe { std::slice::from_raw_parts(self.ptr.cast_const(), self.size) };
            String::from_utf8_lossy(bytes).into_owned()
        }
    }

    #[repr(C)]
    pub struct Discord_Client {
        pub opaque: *mut c_void,
    }

    #[repr(C)]
    pub struct Discord_Activity {
        pub opaque: *mut c_void,
    }

    #[repr(C)]
    pub struct Discord_ActivityAssets {
        pub opaque: *mut c_void,
    }

    #[repr(C)]
    pub struct Discord_ActivityTimestamps {
        pub opaque: *mut c_void,
    }

    #[repr(C)]
    pub struct Discord_ClientResult {
        pub opaque: *mut c_void,
    }

    #[link(name = "discord_partner_sdk")]
    unsafe extern "C" {
        pub fn Discord_RunCallbacks();
        pub fn Discord_Client_Init(self_: *mut Discord_Client);
        pub fn Discord_Client_Drop(self_: *mut Discord_Client);
        pub fn Discord_Client_Disconnect(self_: *mut Discord_Client);
        pub fn Discord_Client_SetApplicationId(self_: *mut Discord_Client, application_id: u64);
        pub fn Discord_Client_ClearRichPresence(self_: *mut Discord_Client);
        pub fn Discord_ClientResult_Error(
            self_: *mut Discord_ClientResult,
            return_value: *mut Discord_String,
        );
        pub fn Discord_ClientResult_Successful(self_: *mut Discord_ClientResult) -> bool;
        pub fn Discord_Client_UpdateRichPresence(
            self_: *mut Discord_Client,
            activity: *mut Discord_Activity,
            callback: Option<unsafe extern "C" fn(*mut Discord_ClientResult, *mut c_void)>,
            callback_user_data_free: Option<unsafe extern "C" fn(*mut c_void)>,
            callback_user_data: *mut c_void,
        );
        pub fn Discord_Activity_Init(self_: *mut Discord_Activity);
        pub fn Discord_Activity_Drop(self_: *mut Discord_Activity);
        pub fn Discord_Activity_SetType(
            self_: *mut Discord_Activity,
            value: super::DiscordActivityTypes,
        );
        pub fn Discord_Activity_SetStatusDisplayType(
            self_: *mut Discord_Activity,
            value: *const super::DiscordStatusDisplayTypes,
        );
        pub fn Discord_Activity_SetState(self_: *mut Discord_Activity, value: *mut Discord_String);
        pub fn Discord_Activity_SetDetails(
            self_: *mut Discord_Activity,
            value: *mut Discord_String,
        );
        pub fn Discord_Activity_SetAssets(
            self_: *mut Discord_Activity,
            value: *mut Discord_ActivityAssets,
        );
        pub fn Discord_Activity_SetTimestamps(
            self_: *mut Discord_Activity,
            value: *mut Discord_ActivityTimestamps,
        );
        pub fn Discord_ActivityAssets_Init(self_: *mut Discord_ActivityAssets);
        pub fn Discord_ActivityAssets_Drop(self_: *mut Discord_ActivityAssets);
        pub fn Discord_ActivityAssets_SetLargeImage(
            self_: *mut Discord_ActivityAssets,
            value: *mut Discord_String,
        );
        pub fn Discord_ActivityAssets_SetLargeText(
            self_: *mut Discord_ActivityAssets,
            value: *mut Discord_String,
        );
        pub fn Discord_ActivityAssets_SetSmallImage(
            self_: *mut Discord_ActivityAssets,
            value: *mut Discord_String,
        );
        pub fn Discord_ActivityAssets_SetSmallText(
            self_: *mut Discord_ActivityAssets,
            value: *mut Discord_String,
        );
        pub fn Discord_ActivityTimestamps_Init(self_: *mut Discord_ActivityTimestamps);
        pub fn Discord_ActivityTimestamps_Drop(self_: *mut Discord_ActivityTimestamps);
        pub fn Discord_ActivityTimestamps_SetStart(
            self_: *mut Discord_ActivityTimestamps,
            value: u64,
        );
    }
}
