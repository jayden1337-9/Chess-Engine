#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
use std::time::Duration;

#[derive(Clone, Copy)]
pub enum Sound {
    Select,
    Move,
    Capture,
    Check,
    Checkmate,
    Castle,
    Promotion,
    Draw,
    Undo,
    Restart,
    Error,
    Click,
}

fn notes(sound: Sound) -> &'static [(f32, u64)] {
    match sound {
        Sound::Select => &[(620.0, 38)],
        Sound::Move => &[(430.0, 48), (560.0, 55)],
        Sound::Capture => &[(260.0, 65), (190.0, 85)],
        Sound::Check => &[(740.0, 75), (980.0, 95)],
        Sound::Checkmate => &[(660.0, 110), (520.0, 110), (390.0, 160), (260.0, 260)],
        Sound::Castle => &[(420.0, 65), (560.0, 65), (720.0, 95)],
        Sound::Promotion => &[(520.0, 65), (660.0, 65), (790.0, 100), (1040.0, 140)],
        Sound::Draw => &[(520.0, 100), (440.0, 130)],
        Sound::Undo => &[(520.0, 55), (390.0, 75)],
        Sound::Restart => &[(390.0, 65), (520.0, 65), (650.0, 90)],
        Sound::Error => &[(180.0, 100)],
        Sound::Click => &[(540.0, 24)],
    }
}

#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
pub struct Audio {
    _stream: Option<rodio::OutputStream>,
    handle: Option<rodio::OutputStreamHandle>,
    enabled: bool,
}

#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
impl Audio {
    pub fn new() -> Self {
        match rodio::OutputStream::try_default() {
            Ok((stream, handle)) => Self { _stream: Some(stream), handle: Some(handle), enabled: true },
            Err(_) => Self { _stream: None, handle: None, enabled: true },
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn play(&self, sound: Sound) {
        use rodio::{source::{SineWave, Source}, Sink};
        if !self.enabled { return; }
        let Some(handle) = self.handle.as_ref() else { return; };
        let Ok(sink) = Sink::try_new(handle) else { return; };
        for (frequency, millis) in notes(sound) {
            sink.append(
                SineWave::new(*frequency)
                    .take_duration(Duration::from_millis(*millis))
                    .amplify(0.11),
            );
        }
        sink.detach();
    }
}

#[cfg(target_arch = "wasm32")]
pub struct Audio {
    context: Option<web_sys::AudioContext>,
    enabled: bool,
}

#[cfg(target_arch = "wasm32")]
impl Audio {
    pub fn new() -> Self {
        Self { context: web_sys::AudioContext::new().ok(), enabled: true }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn play(&self, sound: Sound) {
        if !self.enabled { return; }
        let Some(context) = self.context.as_ref() else { return; };
        let _ = context.resume();
        let mut offset = 0.0;
        for (frequency, millis) in notes(sound) {
            let Ok(oscillator) = context.create_oscillator() else { continue; };
            let Ok(gain) = context.create_gain() else { continue; };
            let start = context.current_time() + offset;
            let end = start + *millis as f64 / 1000.0;
            oscillator.set_type(web_sys::OscillatorType::Sine);
            let _ = oscillator.frequency().set_value_at_time(*frequency, start);
            let _ = gain.gain().set_value_at_time(0.075, start);
            let _ = gain.gain().set_value_at_time(0.0001, end);
            let _ = oscillator.connect_with_audio_node(&gain);
            let _ = gain.connect_with_audio_node(&context.destination());
            let _ = oscillator.start_with_when(start);
            let _ = oscillator.stop_with_when(end);
            offset += *millis as f64 / 1000.0 + 0.018;
        }
    }
}

#[cfg(target_os = "android")]
pub struct Audio {
    enabled: bool,
}

#[cfg(target_os = "android")]
impl Audio {
    pub fn new() -> Self {
        Self { enabled: true }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn play(&self, _sound: Sound) {
        // Android audio is intentionally disabled until a platform-native backend is added.
    }
}
