use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use std::sync::Arc;

use serenity::client::bridge::voice::ClientVoiceManager;
use serenity::client::Client;
use serenity::framework::standard::{
    help_commands,
    macros::{command, group, help},
    Args, CommandGroup, CommandResult, HelpOptions,
};
use serenity::model::id::{GuildId, UserId};
use serenity::model::voice::VoiceState;
use serenity::model::{channel::Message, gateway::Ready};
use serenity::prelude::{Context, EventHandler};
use serenity::{
    client::bridge::gateway::ShardManager, framework::StandardFramework, prelude::*,
    Result as SerenityResult,
};

use log::{error, info};

mod commands;

use commands::{danbooru::*, general::*, owner::*, rule34::*};

type Result<T> = ::std::result::Result<T, failure::Error>;

struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

struct VoiceManager;

impl TypeMapKey for VoiceManager {
    type Value = Arc<Mutex<ClientVoiceManager>>;
}

struct ChannelStore;

impl TypeMapKey for ChannelStore {
    type Value = TempChannelStore;
}

struct Reddit;

impl TypeMapKey for Reddit {
    type Value = RedditSearch;
}

struct Handler;

impl EventHandler for Handler {
    fn ready(&self, _: Context, ready: Ready) {
        if let Some(shard) = ready.shard {
            info!(
                "{} is connected on shard {}/{}!",
                ready.user.name,
                shard[0] + 1,
                shard[1],
            );
        }
    }
    fn voice_state_update(
        &self,
        ctx: Context,
        guild_id: Option<GuildId>,
        _: Option<VoiceState>,
        _: VoiceState,
    ) {
        if let Some(id) = guild_id {
            check_temp_chans(&ctx, &id);
        }
    }
}

#[help]
fn my_help(
    context: &mut Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    help_commands::with_embeds(context, msg, args, help_options, groups, owners)
}

group!({
    name: "general",
    options: {},
    commands: [reddit, temporary_channel, ping, quit]
});

// ytdl/ffmpeg integration bad
//group!({
//name: "voice",
//options: {},
//commands: [join, play, leave]
//});

group!({
    name: "nsfw",
    options: {},
    commands: [rule34, danbooru]
});

fn main() {
    env_logger::init();

    let mut token_file = File::open("bot_token.txt").unwrap();
    let mut token = String::new();
    token_file.read_to_string(&mut token).unwrap();
    token = token.trim().to_owned();

    let mut client = Client::new(&token, Handler).expect("Err creating client");

    {
        let mut data = client.data.write();
        data.insert::<VoiceManager>(Arc::clone(&client.voice_manager));
        data.insert::<ShardManagerContainer>(Arc::clone(&client.shard_manager));
        data.insert::<ChannelStore>(TempChannelStore::new());
        data.insert::<Reddit>(RedditSearch::new());
    }

    let owners = match client.cache_and_http.http.get_current_application_info() {
        Ok(info) => {
            let mut set = HashSet::new();
            set.insert(info.owner.id);

            set
        }
        Err(why) => panic!("Couldn't get application info: {:?}", why),
    };

    client.with_framework(
        StandardFramework::new()
            .configure(|c| c.owners(owners).prefix("!"))
            .group(&GENERAL_GROUP)
            //.group(&VOICE_GROUP)
            .group(&NSFW_GROUP)
            .help(&MY_HELP),
    );

    let _ = client
        .start()
        .map_err(|why| error!("Client ended: {:?}", why));
}

#[command]
#[description("Replies with \"Pong!\".")]
fn ping(ctx: &mut Context, msg: &Message) -> CommandResult {
    check_msg(msg.reply(&ctx, "Pong!"));

    Ok(())
}

fn check_msg(result: SerenityResult<Message>) {
    if let Err(why) = result {
        error!("Error sending message: {:?}", why);
    }
}
