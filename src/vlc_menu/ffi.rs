use super::{c_char, Handle, Track};
#[repr(C)]
pub struct Device {
    pub next: *mut Device,
    pub id: *const c_char,
    pub name: *const c_char,
}
#[repr(C)]
pub struct Title {
    pub duration: i64,
    pub name: *const c_char,
    pub flags: u32,
}
#[repr(C)]
pub struct Chapter {
    pub start: i64,
    pub duration: i64,
    pub name: *const c_char,
}
#[repr(C)]
pub struct Codec {
    pub codec: u32,
    pub original: u32,
    pub id: i32,
    pub kind: i32,
    pub profile: i32,
    pub level: i32,
    pub detail: Handle,
    pub bitrate: u32,
    pub language: *const c_char,
    pub description: *const c_char,
}
#[repr(C)]
pub struct Module {
    pub name: *const c_char,
    pub short: *const c_char,
    pub long: *const c_char,
    pub help: *const c_char,
    pub next: *mut Module,
}
extern "C" {
    pub fn libvlc_free(v: Handle);
    pub fn libvlc_media_player_get_media(p: Handle) -> Handle;
    pub fn libvlc_media_get_mrl(m: Handle) -> *mut c_char;
    pub fn libvlc_media_get_meta(m: Handle, kind: i32) -> *mut c_char;
    pub fn libvlc_media_tracks_get(m: Handle, out: *mut *mut *mut Codec) -> u32;
    pub fn libvlc_media_tracks_release(tracks: *mut *mut Codec, count: u32);
    pub fn libvlc_media_get_codec_description(kind: i32, codec: u32) -> *const c_char;
    pub fn libvlc_media_player_get_full_title_descriptions(
        p: Handle,
        out: *mut *mut *mut Title,
    ) -> i32;
    pub fn libvlc_title_descriptions_release(t: *mut *mut Title, count: u32);
    pub fn libvlc_media_player_get_full_chapter_descriptions(
        p: Handle,
        title: i32,
        out: *mut *mut *mut Chapter,
    ) -> i32;
    pub fn libvlc_chapter_descriptions_release(t: *mut *mut Chapter, count: u32);
    pub fn libvlc_media_player_get_title(p: Handle) -> i32;
    pub fn libvlc_media_player_set_title(p: Handle, n: i32);
    pub fn libvlc_media_player_set_chapter(p: Handle, n: i32);
    pub fn libvlc_media_player_set_rate(p: Handle, rate: f32) -> i32;
    pub fn libvlc_video_get_track_description(p: Handle) -> *mut Track;
    pub fn libvlc_video_set_track(p: Handle, id: i32) -> i32;
    pub fn libvlc_audio_output_device_enum(p: Handle) -> *mut Device;
    pub fn libvlc_audio_output_device_list_release(d: *mut Device);
    pub fn libvlc_audio_output_device_set(p: Handle, module: *const c_char, id: *const c_char);
    pub fn libvlc_audio_set_channel(p: Handle, id: i32) -> i32;
    pub fn libvlc_audio_toggle_mute(p: Handle);
    pub fn libvlc_video_set_scale(p: Handle, x: f32);
    pub fn libvlc_video_set_aspect_ratio(p: Handle, x: *const c_char);
    pub fn libvlc_video_set_crop_geometry(p: Handle, x: *const c_char);
    pub fn libvlc_video_set_deinterlace(p: Handle, x: *const c_char);
    pub fn libvlc_video_take_snapshot(
        p: Handle,
        num: u32,
        path: *const c_char,
        width: u32,
        height: u32,
    ) -> i32;
    pub fn libvlc_video_set_subtitle_file(p: Handle, path: *const c_char) -> i32;
    pub fn libvlc_audio_equalizer_get_preset_count() -> u32;
    pub fn libvlc_audio_equalizer_get_preset_name(i: u32) -> *const c_char;
    pub fn libvlc_audio_equalizer_new_from_preset(i: u32) -> Handle;
    pub fn libvlc_audio_equalizer_release(eq: Handle);
    pub fn libvlc_media_player_set_equalizer(p: Handle, eq: Handle) -> i32;
    pub fn libvlc_video_set_adjust_int(p: Handle, option: u32, v: i32);
    pub fn libvlc_video_set_adjust_float(p: Handle, option: u32, v: f32);
    pub fn libvlc_audio_set_delay(p: Handle, v: i64) -> i32;
    pub fn libvlc_video_set_spu_delay(p: Handle, v: i64) -> i32;
    pub fn libvlc_audio_filter_list_get(instance: Handle) -> *mut Module;
    pub fn libvlc_video_filter_list_get(instance: Handle) -> *mut Module;
    pub fn libvlc_module_description_list_release(m: *mut Module);
    pub fn libvlc_media_add_option(m: Handle, option: *const c_char);
    pub fn libvlc_vlm_add_broadcast(
        e: Handle,
        name: *const c_char,
        input: *const c_char,
        output: *const c_char,
        count: i32,
        opts: *const *const c_char,
        enabled: i32,
        looped: i32,
    ) -> i32;
    pub fn libvlc_vlm_play_media(e: Handle, name: *const c_char) -> i32;
    pub fn libvlc_vlm_stop_media(e: Handle, name: *const c_char) -> i32;
    pub fn libvlc_vlm_del_media(e: Handle, name: *const c_char) -> i32;
    pub fn libvlc_vlm_show_media(e: Handle, name: *const c_char) -> *mut c_char;
    pub fn libvlc_vlm_get_event_manager(e: Handle) -> Handle;
    pub fn libvlc_vlm_release(e: Handle);
}
