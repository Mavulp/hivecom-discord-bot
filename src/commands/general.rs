use std::collections::HashMap;
//use std::fs::OpenOptions;
//use std::io::ErrorKind::NotFound as FileNotFound;

//use poise::serenity_prelude::CreateChannel;
use poise::{command, say_reply};
use serenity::model::id::ChannelId;
use serenity::model::prelude::*;

use rawr::auth::AnonymousAuthenticator;
use rawr::client::RedditClient;
use rawr::options::ListingOptions;
use rawr::traits::{Content, Stickable};

use circular_queue::CircularQueue;

use crate::check_msg;
use crate::Context;
use crate::Result;

/// Gets an unseen URL from the last 100 on a subreddit of choice.
#[command(prefix_command, guild_only, track_edits)]
pub async fn reddit(
    ctx: Context<'_>,
    subreddit: String,
    count: Option<usize>,
    #[flag] comments: bool,
) -> Result<()> {
    let channel = if let Channel::Guild(channel) = ctx.channel_id().to_channel(&ctx).await.unwrap()
    {
        channel
    } else {
        unreachable!("Guild only command");
    };

    let result = {
        let mut reddit = ctx.data().reddit.lock().await;

        if comments {
            match reddit.get_comments(subreddit.clone(), ctx.channel_id(), count.unwrap_or(1) - 1) {
                Some((sub, id)) => Ok(format!("https://reddit.com/r/{}/comments/{}/", sub, id)),
                None => Err(String::from("No post for that subreddit was sent previously").into()),
            }
        } else {
            reddit.get_post(subreddit, ctx.channel_id(), channel.nsfw)
        }
    };

    match result {
        Ok(url) => check_msg(say_reply(ctx, &url).await),
        //TODO Err msg
        Err(e) => check_msg(say_reply(ctx, &e.to_string()).await),
    }

    Ok(())
}

pub struct RedditSearch {
    client: RedditClient,
    known: HashMap<(ChannelId, String), CircularQueue<(String, String)>>,
}

impl RedditSearch {
    pub fn new() -> Self {
        RedditSearch {
            client: RedditClient::new("discord-bot", AnonymousAuthenticator::new()),
            known: HashMap::new(),
        }
    }

    pub fn get_comments(
        &mut self,
        sub: String,
        channel: ChannelId,
        count: usize,
    ) -> Option<(String, String)> {
        let known = self
            .known
            .entry((channel, sub))
            .or_insert(CircularQueue::with_capacity(100));

        known.iter().nth(count).map(|s| s.clone())
    }

    pub fn get_post(&mut self, sub: String, channel: ChannelId, nsfw: bool) -> Result<String> {
        let subreddit = self.client.subreddit(&sub);

        let hot = subreddit.hot(ListingOptions::default())?;

        let known = self
            .known
            .entry((channel, sub))
            .or_insert(CircularQueue::with_capacity(100));

        for post in hot.take(100) {
            if post.stickied() || (!nsfw && post.nsfw()) {
                continue;
            }

            if let Some(url) = post.link_url() {
                let id = post.name()[3..].to_string();
                if let Some(_) = known.iter().find(|&(_, k)| k == &id) {
                    continue;
                }
                known.push((post.subreddit().name, id));

                return Ok(url);
            }
        }
        if nsfw {
            Err(String::from("Found no URLs in most recent 100 posts").into())
        } else {
            Err(String::from("Found no SFW URLs in the most recent 100 posts").into())
        }
    }
}

///// Creates a temporary channel that will get removed once empty.
//#[command(prefix_command, guild_only, track_edits)]
//pub async fn tempchan(ctx: Context<'_>, name: String) -> Result<()> {
//let guild_id = ctx.guild_id().expect("guild only command");

//let author_in_voice = ctx
//.guild()
//.expect("guild only command")
//.voice_states
//.get(&ctx.author().id)
//.is_some();

//if !author_in_voice {
//check_msg(
//say_reply(
//ctx,
//"You must be in a voice channel to create a temporary channel",
//)
//.await,
//);

//return Ok(());
//};

//let channel = match guild_id
//.create_channel(&ctx, CreateChannel::new(name).kind(ChannelType::Voice))
//.await
//{
//Ok(c) => c,
//Err(e) => {
//check_msg(say_reply(ctx, &format!("Failed to create channel: {}", e)).await);
//return Ok(());
//}
//};

//let result = {
//let mut chan_store = ctx.data().chan_store.lock().await;
//chan_store.add_chan(&channel.id)
//};

//if let Err(e) = result {
//error!("Failed to save temp channel: {}", e);
//check_msg(say_reply(ctx, "Failed to store temporary channel").await);
//ctx.channel_id().delete(ctx.http()).await?;
//return Ok(());
//}

//if let Err(e) = guild_id
//.move_member(ctx.http(), ctx.author().id, channel.id)
//.await
//{
//error!("Failed to move user: {}", e);
//check_msg(say_reply(ctx, "Failed to move user").await);
//channel.delete(ctx.http()).await?;
//}

//Ok(())
//}

//pub async fn check_temp_chans(ctx: &serenity::client::Context, guild_id: &GuildId) {
//let guild = ctx.guild().expect("guild only command");

//let mut chan_store = ctx.data().chan_store.lock().await;

//if let Ok(channels) = guild.channels(ctx.http).await {
//for (channel_id, _) in channels {
//if chan_store.contains(&channel_id) {
//if guild
//.voice_states
//.iter()
//.filter(|(_, vs)| vs.channel_id.map(|id| channel_id == id) == Some(true))
//.count()
//< 1
//{
//if let Err(e) = channel_id.delete(ctx.http).await {
//error!("Failed to delete temporary channel: {}", e);
//} else {
//if let Err(e) = chan_store.remove_chan(&channel_id) {
//error!("{}", e);
//}
//}
//}
//}
//}
//}
//}

//pub struct TempChannelStore {
//temp_chans: Vec<u64>,
//}

//impl TempChannelStore {
//pub fn new() -> Self {
//let result = OpenOptions::new().read(true).open("db/channels.bin");

//match result {
//Ok(f) => {
//let chans: Vec<u64> = match bincode::deserialize_from(f) {
//Ok(cs) => cs,
//Err(e) => {
//error!("Failed to deserialize channels: {}", e);
//Vec::new()
//}
//};

//TempChannelStore { temp_chans: chans }
//}
//Err(e) => {
//// FIXME What
//if FileNotFound != e.kind() {
//error!("Failed to open channel file: {}", e);
//}

//TempChannelStore {
//temp_chans: Vec::new(),
//}
//}
//}
//}

//pub fn add_chan(&mut self, id: &ChannelId) -> Result<()> {
//self.temp_chans.push(id.get());
//let file = OpenOptions::new()
//.write(true)
//.create(true)
//.open("db/channels.bin")?;

//bincode::serialize_into(file, &self.temp_chans)?;

//Ok(())
//}
//pub fn remove_chan(&mut self, id: &ChannelId) -> Result<()> {
//let idx = self
//.temp_chans
//.iter()
//.enumerate()
//.find(|&(_, c)| c == &id.get())
//.map(|(i, _)| i)
//.ok_or(String::from("ChannelId not found"))?;
//self.temp_chans.remove(idx);

//let file = OpenOptions::new()
//.write(true)
//.create(true)
//.open("db/channels.bin")?;

//bincode::serialize_into(file, &self.temp_chans)?;
//Ok(())
//}

//pub fn contains(&self, id: &ChannelId) -> bool {
//self.temp_chans.contains(&id.get())
//}
//}
