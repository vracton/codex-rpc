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

#[cfg(target_os = "windows")]
const MODEL_BADGE_IMAGE: &str = "gpt-54";

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
                        small_text,
                        start_timestamp_seconds,
                    } => {
                        if let Err(err) = client.set_presence(
                            &details,
                            state.as_deref(),
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
}

#[cfg(target_os = "windows")]
impl DiscordPresenceBridge {
    fn new(application_id: u64) -> Result<Self> {
        let mut client = DiscordClient::new();
        client.set_application_id(application_id);
        Ok(Self { client })
    }

    fn set_presence(
        &mut self,
        details: &str,
        state: Option<&str>,
        small_text: Option<&str>,
        start_timestamp_seconds: u64,
        large_image: Option<&str>,
    ) -> Result<()> {
        let mut activity = DiscordActivity::new();
        activity.set_type(DiscordActivityTypes::Playing);
        activity.set_details(details);
        activity.set_status_display_type(DiscordStatusDisplayTypes::Details);
        if let Some(state) = state {
            activity.set_state(state);
        }
        activity.set_start_timestamp(start_timestamp_seconds);
        if large_image.is_some() || small_text.is_some() {
            activity.set_assets(
                large_image,
                /*large_text*/ None,
                Some(MODEL_BADGE_IMAGE),
                small_text,
            );
        }
        self.client.update_presence(&activity);
        self.client.run_callbacks();
        Ok(())
    }

    fn clear_presence(&mut self) {
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

    fn set_details(&mut self, details: &str) {
        let mut details = ffi::Discord_String::from_str(details);
        unsafe {
            ffi::Discord_Activity_SetDetails(&mut self.inner, &mut details);
        }
    }

    fn set_state(&mut self, state: &str) {
        let mut state = ffi::Discord_String::from_str(state);
        unsafe {
            ffi::Discord_Activity_SetState(&mut self.inner, &mut state);
        }
    }

    fn set_status_display_type(&mut self, display_type: DiscordStatusDisplayTypes) {
        unsafe {
            ffi::Discord_Activity_SetStatusDisplayType(&mut self.inner, &display_type);
        }
    }

    fn set_start_timestamp(&mut self, start_timestamp_seconds: u64) {
        let mut timestamps = ffi::Discord_ActivityTimestamps {
            opaque: ptr::null_mut(),
        };
        unsafe {
            ffi::Discord_ActivityTimestamps_Init(&mut timestamps);
            ffi::Discord_ActivityTimestamps_SetStart(&mut timestamps, start_timestamp_seconds);
            ffi::Discord_Activity_SetTimestamps(&mut self.inner, &mut timestamps);
            ffi::Discord_ActivityTimestamps_Drop(&mut timestamps);
        }
    }

    fn set_assets(
        &mut self,
        large_image: Option<&str>,
        large_text: Option<&str>,
        small_image: Option<&str>,
        small_text: Option<&str>,
    ) {
        let mut assets = ffi::Discord_ActivityAssets {
            opaque: ptr::null_mut(),
        };
        unsafe {
            ffi::Discord_ActivityAssets_Init(&mut assets);
            if let Some(large_image) = large_image {
                let mut large_image = ffi::Discord_String::from_str(large_image);
                ffi::Discord_ActivityAssets_SetLargeImage(&mut assets, &mut large_image);
            }
            if let Some(large_text) = large_text {
                let mut large_text = ffi::Discord_String::from_str(large_text);
                ffi::Discord_ActivityAssets_SetLargeText(&mut assets, &mut large_text);
            }
            if let Some(small_image) = small_image {
                let mut small_image = ffi::Discord_String::from_str(small_image);
                ffi::Discord_ActivityAssets_SetSmallImage(&mut assets, &mut small_image);
            }
            if let Some(small_text) = small_text {
                let mut small_text = ffi::Discord_String::from_str(small_text);
                ffi::Discord_ActivityAssets_SetSmallText(&mut assets, &mut small_text);
            }
            ffi::Discord_Activity_SetAssets(&mut self.inner, &mut assets);
            ffi::Discord_ActivityAssets_Drop(&mut assets);
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

    let error = unsafe { ffi::Discord_ClientResult_Error(result) };
    if error == ffi::Discord_Client_Error::DISCORD_CLIENT_ERROR_NONE {
        return;
    }

    let mut message = ffi::Discord_String::empty();
    unsafe {
        ffi::Discord_ClientResult_Message(result, &mut message);
    }
    let _ = write_event(&HelperEvent::Error {
        message: message.to_string_lossy(),
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

        pub fn from_str(value: &str) -> Self {
            Self {
                ptr: value.as_ptr().cast_mut(),
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

    #[repr(i32)]
    #[allow(non_camel_case_types)]
    #[derive(Clone, Copy, PartialEq, Eq)]
    pub enum Discord_Client_Error {
        DISCORD_CLIENT_ERROR_NONE = 0,
    }

    #[link(name = "discord_partner_sdk")]
    unsafe extern "C" {
        pub fn Discord_RunCallbacks();
        pub fn Discord_Client_Init(self_: *mut Discord_Client);
        pub fn Discord_Client_Drop(self_: *mut Discord_Client);
        pub fn Discord_Client_Disconnect(self_: *mut Discord_Client);
        pub fn Discord_Client_SetApplicationId(self_: *mut Discord_Client, application_id: u64);
        pub fn Discord_Client_ClearRichPresence(self_: *mut Discord_Client);
        pub fn Discord_ClientResult_Error(self_: *mut Discord_ClientResult)
        -> Discord_Client_Error;
        pub fn Discord_ClientResult_Message(
            self_: *mut Discord_ClientResult,
            return_value: *mut Discord_String,
        );
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
