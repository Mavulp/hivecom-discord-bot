use failure::bail;
use reqwest::{header, Client};
use url::form_urlencoded;
use xml::reader::{EventReader, XmlEvent};

use crate::check_msg;
use crate::Result;
use serenity::framework::standard::{macros::command, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;

#[command]
#[aliases(danb)]
#[description("Searches danbooru.donmai.us for images and returns one at random.")]
#[usage("TAGS")]
#[only_in(guilds)]
fn danbooru(ctx: &mut Context, msg: &Message) -> CommandResult {
    let channel = if let Channel::Guild(channel) = msg.channel_id.to_channel(&ctx).unwrap() {
        channel
    } else {
        check_msg(
            msg.channel_id
                .say(&ctx.http, "Groups and DMs not supported"),
        );

        return Ok(());
    };
    let channel = channel.read();

    let content = &msg.content;
    let tags = content
        .split(' ')
        .skip(1)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    // TODO Terrible
    let mut tags = tags
        .join(" ")
        .replace(", ", ",")
        .replace(' ', "_")
        .replace(',', " ");

    if tags.is_empty() {
        check_msg(msg.channel_id.say(&ctx.http, "Specify at least one tag"));
        return Ok(());
    }

    if !channel.nsfw {
        if tags.find("rating:").is_some() {
            check_msg(msg.channel_id.say(
                &ctx.http,
                "You can only specify the rating in NSFW channels",
            ));
            return Ok(());
        } else {
            tags.push_str(" rating:s");
        }
    }

    let response = match get_url(&tags) {
        Ok(v) => v,
        Err(e) => e.to_string(),
    };

    check_msg(msg.channel_id.say(&ctx.http, &response));

    Ok(())
}

fn get_url(tags: &str) -> Result<String> {
    let url: String = form_urlencoded::Serializer::new(String::from(
        "http://danbooru.donmai.us/posts.xml?random=true",
    ))
    .append_pair("limit", "1")
    .append_pair("tags", tags)
    .finish();

    get_attribute(&url, "file-url")
}

fn get_attribute(url: &str, xml_name: &str) -> Result<String> {
    let response = Client::new()
        .get(url)
        .header(header::CONNECTION, "close")
        .send()?;

    let parser = EventReader::new(response);

    let mut iter = parser.into_iter();
    while let Some(ev) = iter.next() {
        match ev {
            Ok(XmlEvent::StartElement { name, .. }) => {
                if name.local_name != xml_name {
                    continue;
                }
                if let Some(Ok(XmlEvent::Characters(found))) = iter.next() {
                    return Ok(found);
                }
            }
            _ => (),
        }
    }

    bail!("No results")
}