// Based on https://github.com/jplatte/ruma-client/blob/master/examples/hello_world.rs
#![feature(conservative_impl_trait)]
#![feature(try_from)]

extern crate futures;
extern crate ruma_client;
extern crate ruma_events;
extern crate ruma_identifiers;
extern crate tokio_core;
extern crate url;

use std::convert::TryFrom;

use futures::Future;
use ruma_client::Client;
use ruma_client::api::r0;
use ruma_client::api::unstable;
use ruma_events::EventType;
use ruma_events::room::message::{MessageEventContent, MessageType, TextMessageEventContent};
use ruma_identifiers::RoomAliasId;
use tokio_core::reactor::{Core as TokioCore, Handle as TokioHandle};
use url::Url;

use std::collections::HashMap;

// from https://stackoverflow.com/a/43992218/1592377
#[macro_export]
macro_rules! clone {
    (@param _) => ( _ );
    (@param $x:ident) => ( $x );
    ($($n:ident),+ => move || $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move || $body
        }
    );
    ($($n:ident),+ => move |$($p:tt),+| $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move |$(clone!(@param $p),)+| $body
        }
    );
}

fn hello_world(
    tokio_handle: &TokioHandle,
    homeserver_url: Url,
) -> impl Future<Item = (), Error = ruma_client::Error> + 'static {
    let client = Client::https(tokio_handle, homeserver_url, None).unwrap();

    let mut one_time_keys = HashMap::new();
    one_time_keys.insert("curve25519:AAAAAQ".to_owned(), "/qyvZvwjiTxGdGU0RCguDCLeR+nmsb3FfNG3/Ve4vU8".to_owned());

    client.register_guest().and_then(clone!(client => move |_| {
        // No keys yet, so this should return an empty list
        unstable::keys::upload::call(client, unstable::keys::upload::Request {
            device_keys: None,
            one_time_keys: None,
        }).map(|response| {
             println!("Before uploading one-time keys: {:?}", response);
             ()
        })
    })).and_then(clone!(client => move |_| {
        // Upload some keys, should now get a non-zero response...
        unstable::keys::upload::call(client, unstable::keys::upload::Request {
            device_keys: None,
            one_time_keys: Some(one_time_keys),
        }).map(|response| {
             println!("After uploading one-time keys: {:?}", response);
             ()
        })
    })).and_then(clone!(client => move |_| {
        r0::alias::get_alias::call(client.clone(), r0::alias::get_alias::Request {
            room_alias: RoomAliasId::try_from("#test-e2e:matrix.org").unwrap(),
        }).and_then(clone!(client => move |response| {
            let room_id = response.room_id;

            // Join room for testing e2e
            r0::membership::join_room_by_id::call(
                client.clone(),
                r0::membership::join_room_by_id::Request {
                    room_id: room_id.clone(),
                    third_party_signed: None,
            }).and_then(move |_| {
            // Send message to room
                r0::send::send_message_event::call(client, r0::send::send_message_event::Request {
                    room_id: room_id,
                    event_type: EventType::RoomMessage,
                    txn_id: "1".to_owned(),
                    data: MessageEventContent::Text(TextMessageEventContent {
                        body: "Hello World!".to_owned(),
                        msgtype: MessageType::Text,
                    }),
                })
            })

        }))
    })).map(|_| ())
}

fn main() {
    let mut core = TokioCore::new().unwrap();
    let handle = core.handle();
    let server = Url::parse("https://matrix.org/").unwrap();

    core.run(hello_world(&handle, server)).unwrap();
}