use std::time::Duration;

use bevy::{
    prelude::*,
    time::common_conditions::{once_after_delay, repeating_after_delay},
};
use bevy_seedling::{
    pool::{Sampler, SamplerOf},
    prelude::*,
};

fn main() -> AppExit {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, SeedlingPlugin::default()));

    app.add_systems(Startup, spawn_player);

    app.add_systems(
        Update,
        (
            play.run_if(once_after_delay(Duration::from_secs(1))),
            restart.run_if(once_after_delay(Duration::from_secs(2))),
            toggle_play.run_if(repeating_after_delay(Duration::from_secs(3))),
        ),
    );

    app.run()
}

fn spawn_player(mut commands: Commands, assets: Res<AssetServer>) {
    commands.spawn((
        SamplePlayer::new(assets.load("selfless_courage.ogg")),
        PlaybackSettings::default()
            .with_playback(false)
            .with_on_complete(OnComplete::Preserve),
    ));
}

fn play(mut settings: Single<&mut PlaybackSettings>) {
    info!("Initially playing!");
    settings.play_from = PlayFrom::Frames(0);
    settings.play();
}
fn restart(mut settings: Single<&mut PlaybackSettings>) {
    info!("Restarting!");
    settings.play_from = PlayFrom::Frames(0);
    settings.pause();
}

fn toggle_play(mut player: Query<(&Sampler, &mut PlaybackSettings)>) {
    info!("Toggling playback!");
    let (sampler, mut settings) = player.single_mut().unwrap();

    if sampler.is_playing() {
        settings.pause();
    } else {
        settings.play();
    }
}
