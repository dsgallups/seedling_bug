use std::{sync::Arc, time::Duration};

use bevy::{prelude::*, time::common_conditions::on_timer};
use bevy_seedling::{
    SeedlingPlugin,
    node::{FirewheelNode, RegisterNode},
    prelude::AudioEvents,
};
use firewheel::{
    channel_config::{ChannelConfig, ChannelCount},
    diff::EventQueue,
    event::{NodeEventType, ProcEvents},
    node::{
        AudioNode, AudioNodeInfo, AudioNodeProcessor, ConstructProcessorContext, EmptyConfig,
        ProcBuffers, ProcExtra, ProcInfo, ProcessStatus,
    },
};
use midix::prelude::*;
use midix_soundfont_synth::prelude::*;

use crate::{
    player::{SynthCommands, SynthPlayer},
    soundfont::SoundFontAsset,
};

mod player;
mod soundfont;

fn main() -> AppExit {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, SeedlingPlugin::default(), soundfont::plugin));
    //intentionally registering a simple node
    app.register_simple_node::<MidiSynthNode>();

    app.add_systems(Startup, spawn_player).add_systems(
        Update,
        (
            ready_midi_player,
            process_midi_commands,
            play_tone.run_if(on_timer(Duration::from_millis(800))),
        ),
    );

    app.run()
}

fn spawn_player(mut commands: Commands, assets: Res<AssetServer>) {
    commands.spawn((
        SynthPlayer(assets.load("8bitsf.sf2")),
        SynthCommands::default(),
    ));
}

fn play_tone(nodes: Query<&mut SynthCommands>, mut enable: Local<bool>) {
    let msg = if *enable {
        ChannelVoiceMessage::new(
            Channel::One,
            VoiceEvent::note_on(key!(C, 3), Velocity::new_unchecked(60)),
        )
    } else {
        ChannelVoiceMessage::new(
            Channel::One,
            VoiceEvent::note_off(key!(C, 3), Velocity::new_unchecked(0)),
        )
    };

    for mut node in nodes {
        info!("MESSAGE SENT TO NODE!");
        node.send(msg);
    }
    *enable = !*enable;
}

/// System that spawns MIDI synthesizer nodes for entities with soundfonts
///
/// once the soundfont has loaded.
fn ready_midi_player(
    mut commands: Commands,
    soundfont_assets: Res<Assets<SoundFontAsset>>,
    query: Query<(Entity, &SynthPlayer), (Without<FirewheelNode>, With<SynthCommands>)>,
) {
    for (entity, soundfont) in &query {
        // Check if soundfont is loaded
        let Some(soundfont_asset) = soundfont_assets.get(&soundfont.0) else {
            continue;
        };

        // Get config or use defaults

        let node = MidiSynthNode::new(Arc::clone(&soundfont_asset.file), true);

        // Add the node and its configuration to the entity
        // bevy_seedling will automatically handle node creation and connection
        commands.entity(entity).insert(node);
    }
}

/// System that processes MIDI commands and sends them to the audio nodes
fn process_midi_commands(mut query: Query<(&FirewheelNode, &mut SynthCommands, &mut AudioEvents)>) {
    for (_, mut commands, mut events) in &mut query {
        if commands.queue.is_empty() {
            continue;
        }

        // Take all pending commands
        let pending = commands.take();

        // Send commands to the audio node as custom events
        for command in pending {
            events.push(NodeEventType::custom(command));
        }
    }
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
