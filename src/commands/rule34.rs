use poise::{command, say_reply};
use rand::{thread_rng, Rng};
use reqwest::{header, Client};
use url::form_urlencoded;

use crate::check_msg;
use crate::Result;

use crate::Context;

/// Searches rule34.xxx for images and returns one at random.
#[command(
    prefix_command,
    guild_only,
    nsfw_only,
    aliases("r34"),
    track_edits,
    category = "nsfw"
)]
pub async fn rule34(
    ctx: Context<'_>,
    #[description = "Comma separated tags"]
    #[rest]
    tags: String,
) -> Result<()> {
    let tags = tags
        .split(' ')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    // TODO Terrible
    let tags = tags
        .join(" ")
        .replace(", ", ",")
        .replace(' ', "_")
        .replace(',', " ");

    let response = match get_amount(&tags).await {
        Ok(amount) => {
            if amount < 1 {
                check_msg(say_reply(ctx, "No Results").await);
                return Ok(());
            }

            let rand: u64 = {
                let mut rng = thread_rng();
                rng.gen_range(0..amount)
            };

            match get_url(&tags, rand).await {
                Ok(v) => v,
                Err(e) => e.to_string(),
            }
        }
        Err(e) => e.to_string(),
    };

    check_msg(say_reply(ctx, &response).await);

    Ok(())
}

async fn get_amount(tags: &str) -> Result<u64> {
    use std::str::FromStr;

    let url: String = form_urlencoded::Serializer::new(String::from(
        "https://rule34.xxx/index.php?page=dapi&s=post&q=index",
    ))
    .append_pair("limit", "0")
    .append_pair("pid", "0")
    .append_pair("tags", tags)
    .finish();

    Ok(u64::from_str(&get_attribute(&url, "count").await?)?)
}

async fn get_url(tags: &str, pid: u64) -> Result<String> {
    let url: String = form_urlencoded::Serializer::new(String::from(
        "https://rule34.xxx/index.php?page=dapi&s=post&q=index",
    ))
    .append_pair("limit", "1")
    .append_pair("pid", &pid.to_string())
    .append_pair("tags", tags)
    .finish();

    get_attribute(&url, "file_url").await
}

async fn get_attribute(url: &str, name: &str) -> Result<String> {
    use xml::reader::{EventReader, XmlEvent};

    let response = Client::new()
        .post(url)
        .header(header::CONNECTION, "close")
        .send()
        .await?
        .bytes()
        .await?;

    let content = String::from_utf8_lossy(&response);
    let parser = EventReader::from_str(&content);

    for ev in parser {
        match ev {
            Ok(XmlEvent::StartElement { attributes, .. }) => {
                for attr in attributes {
                    if attr.name.local_name == name {
                        return Ok(attr.value.clone());
                    } else if attr.name.local_name == "reason" {
                        return Err(format!("No results: {}", attr.value).into());
                    }
                }
            }
            _ => (),
        }
    }
    return Err(String::from("API deserialization failed").into());
}
