use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use std::sync::Arc;

use serenity::async_trait;
use serenity::client::Client;
use serenity::framework::standard::{
    help_commands,
    macros::{command, group, help},
    Args, CommandGroup, CommandResult, HelpOptions,
};
use serenity::http::Http;
use serenity::model::id::UserId;
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

struct ChannelStore;

impl TypeMapKey for ChannelStore {
    type Value = TempChannelStore;
}

struct Reddit;

impl TypeMapKey for Reddit {
    type Value = RedditSearch;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        if let Some(shard) = ready.shard {
            info!(
                "{} is connected on shard {}/{}!",
                ready.user.name,
                shard[0] + 1,
                shard[1],
            );
        }
    }
    async fn voice_state_update(&self, ctx: Context, _: Option<VoiceState>, state: VoiceState) {
        if let Some(id) = state.guild_id {
            check_temp_chans(&ctx, &id).await;
        }
    }
}

#[help]
async fn my_help(
    context: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    let _ = help_commands::with_embeds(context, msg, args, help_options, groups, owners).await;
    Ok(())
}

#[group]
#[summary = "The normal commands"]
#[commands(reddit, temporary_channel, ping, quit)]
struct General;

#[group]
#[summary = "The other commands"]
#[commands(rule34, danbooru)]
struct Nsfw;

#[tokio::main]
async fn main() {
    env_logger::init();

    let mut token_file = File::open("bot_token.txt").unwrap();
    let mut token = String::new();
    token_file.read_to_string(&mut token).unwrap();
    token = token.trim().to_owned();

    let http = Http::new(&token);

    let owners = match http.get_current_application_info().await {
        Ok(info) => {
            let mut set = HashSet::new();
            set.insert(info.owner.id);

            set
        }
        Err(why) => panic!("Couldn't get application info: {:?}", why),
    };

    let framework = StandardFramework::new()
        .configure(|c| c.owners(owners).prefix("!"))
        .group(&GENERAL_GROUP)
        .group(&NSFW_GROUP)
        .help(&MY_HELP);

    let intents = GatewayIntents::all();
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .framework(framework)
        .type_map_insert::<ChannelStore>(TempChannelStore::new())
        .type_map_insert::<Reddit>(RedditSearch::new())
        .await
        .expect("Err creating client");

    {
        let mut data = client.data.write().await;
        data.insert::<ShardManagerContainer>(Arc::clone(&client.shard_manager));
    }

    let _ = client
        .start()
        .await
        .map_err(|why| error!("Client ended: {:?}", why));
}

#[command]
#[description("Replies with \"Pong!\".")]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    check_msg(msg.reply(&ctx, "Pong!").await);

    Ok(())
}

fn check_msg(result: SerenityResult<Message>) {
    if let Err(why) = result {
        error!("Error sending message: {:?}", why);
    }
}
