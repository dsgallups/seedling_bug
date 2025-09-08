use std::sync::Arc;

use bevy::prelude::*;
use firewheel::{
    channel_config::{ChannelConfig, ChannelCount},
    event::ProcEvents,
    node::{
        AudioNode, AudioNodeInfo, AudioNodeProcessor, ConstructProcessorContext, EmptyConfig,
        ProcBuffers, ProcExtra, ProcInfo, ProcessStatus,
    },
};
use midix::prelude::*;
use midix_soundfont_synth::prelude::*;

fn main() {
    let mut app = App::new();
}

/// Configuration for the MIDI synthesizer node
#[derive(Debug, Clone, Component, TypePath)]
pub struct MidiSynthNode {
    /// The soundfont data
    pub soundfont: Arc<SoundFont>,
    /// Enable reverb and chorus
    pub enable_reverb_and_chorus: bool,
}

impl MidiSynthNode {
    /// Create a new node with a loaded soundfont and reverb/chorus param
    pub fn new(soundfont: Arc<SoundFont>, enable_reverb_and_chorus: bool) -> Self {
        Self {
            soundfont,
            enable_reverb_and_chorus,
        }
    }
}

impl AudioNode for MidiSynthNode {
    type Configuration = EmptyConfig;

    fn info(&self, _config: &Self::Configuration) -> AudioNodeInfo {
        AudioNodeInfo::new()
            .debug_name("MIDI Synthesizer")
            .channel_config(ChannelConfig {
                num_inputs: ChannelCount::ZERO,
                num_outputs: ChannelCount::STEREO,
            })
    }

    fn construct_processor(
        &self,
        _config: &Self::Configuration,
        cx: ConstructProcessorContext,
    ) -> impl AudioNodeProcessor {
        MidiSynthProcessor::new(self, cx)
    }
}

/// MIDI synthesizer audio node processor
pub struct MidiSynthProcessor {
    synthesizer: Synthesizer,
}

impl MidiSynthProcessor {
    /// Create a new MIDI synthesizer processor
    pub fn new(config: &MidiSynthNode, cx: ConstructProcessorContext) -> Self {
        let mut settings = SynthesizerSettings::new(cx.stream_info.sample_rate.get() as i32);
        settings.enable_reverb_and_chorus = config.enable_reverb_and_chorus;

        let synthesizer = Synthesizer::new(config.soundfont.clone(), &settings)
            .expect("Failed to create synthesizer");

        Self { synthesizer }
    }

    /// Process a MIDI command
    fn process_message(&mut self, command: ChannelVoiceMessage) {
        self.synthesizer.process_midi_message(command);
    }
}

impl AudioNodeProcessor for MidiSynthProcessor {
    fn process(
        &mut self,
        info: &ProcInfo,
        ProcBuffers { outputs, .. }: ProcBuffers,
        events: &mut ProcEvents,
        _extra: &mut ProcExtra,
    ) -> ProcessStatus {
        let mut message_received = false;
        // Process incoming MIDI events
        for event in events.drain() {
            if let Some(message) = event.downcast_ref::<ChannelVoiceMessage>() {
                message_received = true;
                self.process_message(*message);
            }
        }
        info!("called process, {message_received}");

        let frames = info.frames;

        // guaranteed to be 2 due to our node's STEREO value.
        let (left, right) = outputs.split_at_mut(1);
        // Render audio from the synthesizer
        self.synthesizer
            .render(&mut left[0][..frames], &mut right[0][..frames]);
        ProcessStatus::outputs_not_silent()
    }
}
