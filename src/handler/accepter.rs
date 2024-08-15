//! Accepter implementation for Kapacitor UDF server.
//!
//! This module defines an `Accepter` struct that implements the `Accepter` trait.
//! It's responsible for accepting new Unix socket connections and setting up agents to handle them.

use async_std::{
    os::unix::net::UnixStream,
    sync::{Arc, Mutex},
    task::{self},
};
use async_trait::async_trait;
use kapacitor_udf::{agent::Agent, traits::AccepterTrait};
use std::sync::atomic::{AtomicI64, Ordering};
use tracing::{debug, error, info};

use crate::handler::{config::IndicatorOptions, indicator_handler::IndicatorHandler};

/// An accepter for new UDF connections.
///
/// This struct keeps track of the number of connections it has accepted
/// and creates a new agent for each connection.
#[derive(Debug)]
pub struct Accepter {
    /// Counter for the number of connections accepted.
    count: Arc<AtomicI64>,
}

impl Accepter {
    /// Creates a new `Accepter` instance.
    ///
    /// # Returns
    ///
    /// A new `Accepter` with the connection count initialized to 0.
    pub fn new() -> Self {
        Accepter {
            count: Arc::new(AtomicI64::new(0)),
        }
    }
}

impl Default for Accepter {
    /// Provides a default instance of `Accepter`.
    ///
    /// This is equivalent to calling `Accepter::new()`.
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AccepterTrait for Accepter {
    /// Accepts a new connection and sets up an agent to handle it.
    ///
    /// This method is called each time a new Unix socket connection is established.
    /// It creates a new `Agent` with an `IndicatorHandler` and spawns a task to run it.
    ///
    /// # Arguments
    ///
    /// * `stream` - The Unix stream for the new connection.
    fn accept(&self, stream: UnixStream) {
        // Increment and get the current connection count
        let count = self.count.fetch_add(1, Ordering::SeqCst);
        debug!("Accept() called, connection count: {}", count);

        // Create a new agent for this connection
        let mut agent = Agent::new(Box::new(stream.clone()), Box::new(stream));
        let responses = agent.responses().clone();

        // Create and set the handler for this agent
        let options = IndicatorOptions::default(); // You can customize this if needed
        let handler = Box::new(IndicatorHandler::new(responses, options));
        agent.set_handler(Some(handler));

        // Wrap the agent in Arc<Mutex<>> for safe sharing across tasks
        let agent = Arc::new(Mutex::new(agent));

        info!("Starting agent for connection {}", count);

        // Spawn a new task to run this agent
        task::spawn(async move {
            if let Err(e) = agent.lock().await.start() {
                error!("Agent for connection {} finished with error: {}", count, e);
                std::process::exit(1);
            }
            info!("Agent for connection {} finished", count);
        });
    }
}
