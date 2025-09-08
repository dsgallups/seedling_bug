use bevy::prelude::*;
use midix::prelude::ChannelVoiceMessage;

use crate::soundfont::SoundFontAsset;

/// Component that specifies which soundfont to use for a MIDI synth
#[derive(Component)]
#[require(SynthCommands)]
pub struct SynthPlayer(pub Handle<SoundFontAsset>);

/// Component for sending MIDI commands to a synthesizer node
#[derive(Component, Default)]
pub struct SynthCommands {
    /// Queue of MIDI commands to send
    pub queue: Vec<ChannelVoiceMessage>,
}

impl SynthCommands {
    /// Add a MIDI command to the queue
    pub fn send(&mut self, command: ChannelVoiceMessage) {
        self.queue.push(command);
    }

    /// Take all commands, leaving the queue empty
    pub fn take(&mut self) -> Vec<ChannelVoiceMessage> {
        std::mem::take(&mut self.queue)
    }
}
