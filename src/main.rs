use std::fs::File;
use std::io::Read;
use std::sync::Arc;

use commands::general::RedditSearch;
use poise::{command, say_reply, ReplyHandle};
use serenity::client::Client;
use serenity::{gateway::ShardManager, prelude::*, Result as SerenityResult};

use log::error;

mod commands;
use commands::*;

type CommandError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = ::std::result::Result<T, CommandError>;

struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

struct Data {
    reddit: Arc<Mutex<RedditSearch>>,
    //chan_store: Arc<Mutex<TempChannelStore>>,
}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

//struct Handler;

//#[async_trait]
//impl EventHandler for Handler {
//async fn voice_state_update(
//&self,
//ctx: serenity::client::Context,
//_: Option<VoiceState>,
//state: VoiceState,
//) {
//if let Some(id) = state.guild_id {
//check_temp_chans(&ctx, &id).await;
//}
//}
//}

/// Show this menu
#[command(track_edits, prefix_command)]
pub async fn help(
    ctx: Context<'_>,
    #[description = "Specific command to show help about"] command: Option<String>,
) -> Result<()> {
    let config = poise::builtins::HelpConfiguration {
        extra_text_at_bottom: "\
Type !help command for more info on a command.
You can edit your message to the bot and the bot will edit its response.",
        ..Default::default()
    };
    poise::builtins::help(ctx, command.as_deref(), config).await?;
    Ok(())
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let mut token_file = File::open("bot_token.txt").unwrap();
    let mut token = String::new();
    token_file.read_to_string(&mut token).unwrap();
    token = token.trim().to_owned();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some("!".into()),
                edit_tracker: Some(Arc::new(poise::EditTracker::for_timespan(
                    std::time::Duration::from_secs(3600),
                ))),
                case_insensitive_commands: true,
                ..Default::default()
            },
            commands: vec![
                help(),
                ping(),
                general::reddit(),
                //general::tempchan(),
                rule34::rule34(),
            ],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {
                    reddit: Arc::new(Mutex::new(RedditSearch::new())),
                    //chan_store: Arc::new(Mutex::new(TempChannelStore::new())),
                })
            })
        })
        .build();

    let intents = GatewayIntents::non_privileged().union(GatewayIntents::MESSAGE_CONTENT);
    let mut client = Client::builder(&token, intents)
        //.event_handler(Handler)
        .framework(framework)
        .await
        .expect("Err creating client");

    //{
    //let mut data = client.data.write().await;
    //data.insert::<ShardManagerContainer>(Arc::clone(&client.shard_manager));
    //}

    if let Err(error) = client.start().await {
        error!("Client ended: {:?}", error)
    }
}

/// Replies with "Pong!".
#[command(prefix_command)]
async fn ping(ctx: Context<'_>) -> Result<()> {
    check_msg(say_reply(ctx, "Pong!").await);

    Ok(())
}

fn check_msg(result: SerenityResult<ReplyHandle<'_>>) {
    if let Err(why) = result {
        error!("Error sending message: {:?}", why);
    }
}
