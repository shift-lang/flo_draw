/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::entity_channel::*;
use crate::entity_id::*;
use crate::error::*;

use futures::future::BoxFuture;
use futures::prelude::*;

use std::marker::PhantomData;

///
/// Converts an entity channel from one type to another
///
pub struct ConvertEntityChannel<TSourceChannel, TNewMessage> {
    source_channel: TSourceChannel,
    new_message: PhantomData<TNewMessage>,
}

impl<TSourceChannel, TNewMessage> ConvertEntityChannel<TSourceChannel, TNewMessage>
where
    TSourceChannel: EntityChannel,
    TSourceChannel::Message: From<TNewMessage>,
    TNewMessage: Send,
{
    ///
    /// Creates a new convertion entity channel
    ///
    pub fn new(
        source_channel: TSourceChannel,
    ) -> ConvertEntityChannel<TSourceChannel, TNewMessage> {
        ConvertEntityChannel {
            source_channel: source_channel,
            new_message: PhantomData,
        }
    }
}

impl<TSourceChannel, TNewMessage> EntityChannel
    for ConvertEntityChannel<TSourceChannel, TNewMessage>
where
    TSourceChannel: EntityChannel,
    TSourceChannel::Message: From<TNewMessage>,
    TNewMessage: Send,
{
    type Message = TNewMessage;

    fn entity_id(&self) -> EntityId {
        self.source_channel.entity_id()
    }

    fn is_closed(&self) -> bool {
        self.source_channel.is_closed()
    }

    fn send(
        &mut self,
        message: Self::Message,
    ) -> BoxFuture<'static, Result<(), EntityChannelError>> {
        let message = TSourceChannel::Message::from(message);
        let future = self.source_channel.send(message);

        async move {
            future.await?;

            Ok(())
        }
        .boxed()
    }
}
