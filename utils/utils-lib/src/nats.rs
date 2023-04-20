//use async_nats::jetstream;
//use async_nats::jetstream::consumer::push::Messages;
//use async_nats::jetstream::consumer::DeliverPolicy;
use async_nats::{
    jetstream::{stream::Config, Context},
    Client,
};
//use async_nats::{Client, Message, Subscriber};

use anyhow::{anyhow, Result};
use bytes::Bytes;
use dashmap::DashSet;
use serde::Serialize;
use std::sync::Arc;

pub trait JetStreamable {
    /// Returns the name of the JetStream associated with this message type.
    fn subject(&self) -> String;
}

trait NatsResultExt<T> {
    fn to_anyhow(self) -> Result<T>;

    fn with_message(self, message: &'static str) -> Result<T>;
}

impl<T> NatsResultExt<T> for std::result::Result<T, async_nats::Error> {
    fn with_message(self, message: &'static str) -> Result<T> {
        match self {
            Ok(v) => Ok(v),
            Err(err) => Err(anyhow!("NATS Error: {:?} ({})", err, message)),
        }
    }

    fn to_anyhow(self) -> Result<T> {
        match self {
            Ok(v) => Ok(v),
            Err(err) => Err(anyhow!("NATS Error: {:?}", err)),
        }
    }
}

#[derive(Clone)]
pub struct TypedNats {
    jetstream: Context,
    jetstream_created_streams: Arc<DashSet<String>>,
    config: Config,
}

impl TypedNats {
    #[must_use]
    pub fn new(nc: Client) -> Self {
        let jetstream = async_nats::jetstream::new(nc);
        let stream_name = "stats".to_string();
        let subjects = vec![
            "stats.events.volume".to_string(),
            "stats.events.nexus".to_string(),
            "stats.events.pool".to_string(),
            "stats.events.replica".to_string(),
        ];
        let config = async_nats::jetstream::stream::Config {
            name: stream_name,
            subjects,
            max_messages: 100,
            ..async_nats::jetstream::stream::Config::default()
        };
        TypedNats {
            jetstream,
            jetstream_created_streams: Arc::default(),
            config,
        }
    }

    pub async fn ensure_jetstream_exists(&self) -> Result<()> {
        if !self.jetstream_created_streams.contains("stats") {
            self.add_jetstream_stream().await?;

            self.jetstream_created_streams.insert("stats".to_string());
        }

        Ok(())
    }

    async fn add_jetstream_stream(&self) -> Result<()> {
        //tracing::debug!(name = config.name, "Creating jetstream stream.");
        self.jetstream
            .get_or_create_stream(self.config.clone())
            .await
            .to_anyhow()?;

        Ok(())
    }

    pub async fn publish_jetstream<T>(&self, value: &T) -> Result<u64>
    where
        T: JetStreamable + Serialize,
    {
        self.ensure_jetstream_exists().await?;
        let subject = value.subject();
        let payload = Bytes::from(serde_json::to_vec(value)?);
        let x = self
            .jetstream
            .publish(subject, payload)
            .await
            .to_anyhow()?
            .await
            .to_anyhow()?
            .sequence;
        //let y = x.await.map_err(|e| anyhow::anyhow!(e))?;
        //let seq = y.sequence;

        // let sequence = self
        //     .jetstream
        //     .publish(
        //         value.subject().clone(),
        //         Bytes::from(serde_json::to_vec(value)?),
        //     )
        //     .await
        //     .to_anyhow()?
        //     .await
        //     .to_anyhow()?
        //     .sequence;

        Ok(x)
    }
}
