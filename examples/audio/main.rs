// Transformed directly from https://github.com/avivbeeri/nok-synth/blob/1485de8219017391cbd807843fd6fe6b1e5a2c68/synth.c

#![allow(dead_code)]

use dome_cloomnik::{register_modules, CallbackChannel, Channel, ChannelState, Context, WrenVM};

#[no_mangle]
#[allow(non_snake_case)]
extern "C" fn PLUGIN_onInit(get_api: *mut libc::c_void, ctx: *mut libc::c_void) -> libc::c_int {
    unsafe {
        dome_cloomnik::init_plugin(
            get_api,
            ctx,
            dome_cloomnik::Hooks {
                on_init: Some(on_init),
                pre_update: None,
                post_update: None,
                pre_draw: None,
                post_draw: None,
                on_shutdown: None,
            },
        )
    }
}

static mut GLOBAL_TIME: f64 = 0.0;

#[derive(Debug, Clone, Copy)]
enum OscType {
    Sine,
    Square,
    Saw,
    Triangle,
}

impl Default for OscType {
    fn default() -> Self {
        OscType::Sine
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct Note {
    duration: f64,
    pitch: i8,
    octave: i8,
}

#[derive(Debug, Default, Clone, Copy)]
struct Envelop {
    attack: f64,
    decay: f64,
    release: f64,

    start_amp: f64,
    sustain_amp: f64,
    trigger_on_time: f64,
    trigger_off_time: f64,
    playing: bool,
}

#[derive(Debug, Clone, Copy)]
enum SynthMode {
    Tone,
    Pattern,
    Note,
}

impl Default for SynthMode {
    fn default() -> Self {
        SynthMode::Tone
    }
}

#[derive(Debug, Default)]
struct Synth {
    r#type: OscType,
    volume: f32,
    note: Note,
    frequency: f32,
    length: f32,
    active: bool,
    r#loop: bool,
    env: Envelop,
    position: usize,
    start_time: f64,

    swap_pattern: bool,
    pattern: Option<Box<[Note]>>,
    pending_pattern: Option<Box<[Note]>>,

    mode: SynthMode,
}

impl Synth {
    fn advance_pattern(&mut self) {
        if let Some(ref pattern) = self.pattern {
            let note = pattern[self.position];
            if (unsafe { GLOBAL_TIME } - self.start_time) >= note.duration {
                self.position += 1;
                if self.position >= pattern.len() {
                    if self.r#loop {
                        self.position = 0;
                    } else {
                        self.pattern = None;
                        self.active = false;
                        self.env.playing = false;
                    }
                }
                self.start_time = unsafe { GLOBAL_TIME };
            }
        }
    }

    fn activate(&mut self) {
        self.active = true;
        if !self.env.playing {
            self.env.trigger_on_time = unsafe { GLOBAL_TIME };
            self.env.playing = true;
        }
    }
}

fn get_note_frequency(octave: f32, note_index: f32) -> f32 {
    const C4: f32 = 261.68;

    C4 * 2.0f32.powf((((octave - 4.0) * 12.0) + note_index) / 12.0)
}

fn is_note_letter(c: u8) -> bool {
    (b'a'..=b'g').contains(&c.to_ascii_lowercase())
}

fn w(frequency: f32) -> f32 {
    2.0 * std::f32::consts::PI * frequency
}

fn phase(frequency: f32, time: f32) -> f32 {
    (w(frequency) * time).sin()
}

fn envelop(env: &Envelop, time: f64) -> f32 {
    let mut amp;

    if env.playing {
        let life_time = time - env.trigger_on_time;
        if life_time < env.attack {
            amp = (life_time / env.attack) * env.start_amp;
        } else if env.attack < life_time && life_time < (env.decay + env.attack) {
            amp = (((life_time - env.attack) / env.decay) * (env.sustain_amp - env.start_amp))
                + env.start_amp;
        } else {
            amp = env.sustain_amp;
        }
    } else {
        let life_time = time - env.trigger_off_time;
        amp = (1.0 - (life_time / env.release)) * env.sustain_amp;
    }

    if amp < 0.0001 {
        amp = 0.0;
    }

    amp as f32
}

fn synth_mix(channel: &CallbackChannel<Synth>, buffer: &mut [[f32; 2]]) {
    const STEP: f64 = 1.0 / 44100.0;

    let mut synth = channel.data_mut();

    let mut freq = 0.0;
    if synth.pattern.is_none() {
        freq = synth.frequency;
    }

    if synth.frequency < 20.0 || !synth.active {
        unsafe { GLOBAL_TIME += STEP * (buffer.len() as f64) };
        synth.advance_pattern();
    } else {
        for sample in buffer {
            if synth.length > 0.0
                && (unsafe { GLOBAL_TIME } - synth.start_time) >= synth.length as f64
            {
                synth.length = 0.0;
                synth.frequency = 0.0;
            }

            if let Some(ref pattern) = synth.pattern {
                let note = pattern[synth.position];
                freq = get_note_frequency(note.octave as f32, note.pitch as f32);
            }

            if freq > 20.0 {
                let mut s = match synth.r#type {
                    OscType::Sine => phase(freq, unsafe { GLOBAL_TIME } as f32),
                    OscType::Square => {
                        if phase(freq, unsafe { GLOBAL_TIME } as f32) > 0.0 {
                            1.0
                        } else {
                            -1.0
                        }
                    }
                    OscType::Triangle => {
                        phase(freq, unsafe { GLOBAL_TIME } as f32).asin()
                            * (2.0 * std::f32::consts::PI)
                    }
                    OscType::Saw => {
                        (2.0 / std::f32::consts::PI)
                            * freq
                            * std::f32::consts::PI
                            * (unsafe { GLOBAL_TIME as f32 } % (1.0 / freq))
                            - (std::f32::consts::PI / 2.0)
                    }
                };

                s = s * envelop(&synth.env, unsafe { GLOBAL_TIME }) * synth.volume;

                sample[0] = s;
                sample[1] = s;
            }

            unsafe { GLOBAL_TIME += STEP };
            synth.advance_pattern();
        }
    }
}

fn synth_update(channel: &CallbackChannel<Synth>, _vm: &WrenVM) {
    let mut synth = channel.data_mut();
    if synth.swap_pattern && synth.pending_pattern.is_some() {
        synth.pattern = synth.pending_pattern.take();
        synth.position = 0;
        synth.swap_pattern = false;
        synth.start_time = unsafe { GLOBAL_TIME };
    }
}

static mut CHANNEL: Option<Channel<Synth>> = None;
fn synth() -> std::sync::RwLockReadGuard<'static, Synth> {
    use std::hint::unreachable_unchecked;
    unsafe {
        match CHANNEL {
            Some(ref channel) => match channel.data() {
                Some(data) => data,
                None => unreachable_unchecked(),
            },
            None => unreachable_unchecked(),
        }
    }
}
fn synth_mut() -> std::sync::RwLockWriteGuard<'static, Synth> {
    use std::hint::unreachable_unchecked;
    unsafe {
        match CHANNEL {
            Some(ref channel) => match channel.data_mut() {
                Some(data) => data,
                None => unreachable_unchecked(),
            },
            None => unreachable_unchecked(),
        }
    }
}

struct SynthClass;
impl SynthClass {
    fn set_volume(vm: &WrenVM) {
        synth_mut().volume = 0.0f32.max(vm.get_slot_double(1) as f32);
    }

    fn get_volume(vm: &WrenVM) {
        vm.set_slot_double(0, synth().volume as f64);
    }

    fn play_tone(vm: &WrenVM) {
        let mut synth = synth_mut();
        synth.frequency = vm.get_slot_double(1) as f32;
        synth.length = vm.get_slot_double(2) as f32 / 1_000.0;
        synth.start_time = unsafe { GLOBAL_TIME };

        synth.activate();

        let ctx = vm.get_context();
        ctx.log("Begin");
        ctx.log(&format!("Frequency: {}\n", synth.frequency));
    }

    fn play_note(vm: &WrenVM) {
        let octave = vm.get_slot_double(1) as f32;
        let pitch = vm.get_slot_double(2) as f32;
        let mut synth = synth_mut();
        synth.frequency = get_note_frequency(octave, pitch);
        synth.length = vm.get_slot_double(3) as f32;
        synth.start_time = unsafe { GLOBAL_TIME };

        synth.activate();

        let ctx = vm.get_context();
        ctx.log("Begin");
        ctx.log(&format!(
            "Octave: {} - Note: {} - Frequency: {}\n",
            octave, pitch, synth.frequency
        ));
    }

    fn note_on(vm: &WrenVM) {
        let octave = vm.get_slot_double(1) as f32;
        let pitch = vm.get_slot_double(2) as f32;
        let mut synth = synth_mut();
        synth.frequency = get_note_frequency(octave, pitch);

        synth.activate();
        vm.get_context().log(&format!(
            "Octave: {} - Note: {} - Frequency: {}\n",
            octave, pitch, synth.frequency
        ));
    }

    fn note_off(_vm: &WrenVM) {
        let mut synth = synth_mut();
        synth.env.trigger_off_time = unsafe { GLOBAL_TIME };
        synth.env.playing = false;
    }

    fn store_pattern(vm: &WrenVM) {
        const BPM: f64 = 144.0;
        const DEFAULT_DURATION: i8 = 4;
        const DEFAULT_OCTAVE: i8 = 4;

        let ctx = vm.get_context();
        let pattern_str = vm.get_slot_bytes(1);

        let pattern = pattern_str
            .split(|&c| c == b' ')
            .map(|mut token| {
                let (mut d, i) = atoi::FromRadix10::from_radix_10(token);
                if i == 0 {
                    d = DEFAULT_DURATION;
                }

                token = &token[i..];

                let mut note = Note {
                    duration: 240.0 / BPM / (d as f64),
                    ..Default::default()
                };

                if let Some(b'.') = token.get(0) {
                    note.duration *= 1.5;
                    token = &token[1..];
                }

                let sharp = matches!(token.get(0), Some(b'#'));
                if sharp {
                    token = &token[1..];
                }

                if let Some(&letter) = token.get(0) {
                    if is_note_letter(letter) {
                        let k = letter.to_ascii_lowercase() & 7;
                        note.pitch = ((((k as f64 * 1.6) as i32) + 8 + (sharp as i32)) % 12) as i8;
                        token = &token[1..];
                    }
                }

                note.octave = DEFAULT_OCTAVE;
                if !token.is_empty() {
                    if (b'1'..=b'8').contains(&token[0]) {
                        note.octave = DEFAULT_OCTAVE + ((token[0] - b'1') as i8);
                    } else if matches!(token[0], b'_' | b'-' | b'p' | b'P') {
                        note.octave = 0;
                    }
                }

                note
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();

        for note in pattern.iter() {
            ctx.log(&format!(
                "Duration: {} - Pitch: {} - Octave: {}\n",
                note.duration, note.pitch, note.octave,
            ));
        }

        synth_mut().pending_pattern = Some(pattern);
    }

    fn play_pattern(_vm: &WrenVM) {
        let mut synth = synth_mut();
        synth.swap_pattern = true;
        synth.active = true;
        synth.env.playing = true;
        synth.env.trigger_on_time = unsafe { GLOBAL_TIME };
        synth.start_time = unsafe { GLOBAL_TIME };
    }
}

fn on_init(ctx: Context) -> Result<(), ()> {
    ctx.log("init hook triggered\n");

    register_modules! {
        ctx,
        module "synth" {
            class Synth = SynthClass {
                foreign static volume=(v) = set_volume
                foreign static volume = get_volume
                foreign static playTone(frequency, time) = play_tone
                foreign static playNote(octave, note, time) = play_note
                foreign static noteOn(octave, note) = note_on
                foreign static noteOff() = note_off
                foreign static storePattern(pattern) = store_pattern
                foreign static playPattern() = play_pattern
            }
        }
    };

    let mut channel = ctx.create_channel(
        synth_mix,
        synth_update,
        Synth {
            env: Envelop {
                attack: 0.02,
                decay: 0.01,
                release: 0.02,
                start_amp: 1.0,
                sustain_amp: 1.0,
                trigger_on_time: 0.0,
                trigger_off_time: 0.0,
                playing: false,
            },
            volume: 0.5,
            r#type: OscType::Saw,
            frequency: get_note_frequency(4.0, 0.0),
            length: 0.0,
            r#loop: false,
            pattern: None,
            start_time: 0.0,
            pending_pattern: None,
            ..Default::default()
        },
    );
    channel.set_state(ChannelState::Playing);
    unsafe { CHANNEL = Some(channel) };

    Ok(())
}
