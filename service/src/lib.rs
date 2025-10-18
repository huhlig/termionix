//
// Copyright 2017-2025 Hans W. Uhlig. All Rights Reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//

//!

mod client;
mod connection;
mod result;
mod server;

pub use self::client::{ClientCallbackHandler, ClientEventHandler, TelnetClientHandler, connect};
pub use self::connection::TelnetConnection;
pub use self::result::{TelnetError, TelnetResult};
pub use self::server::{
    ServerCallbackHandler, ServerEventHandler, TelnetServer, TelnetServerHandler,
};
pub use termionix_ansicodes as ansi;
pub use termionix_codec as codec;
pub use termionix_terminal as terminal;

#[cfg(test)]
mod tests {
    use crate::{
        ClientCallbackHandler, ClientEventHandler, ServerCallbackHandler, ServerEventHandler,
        TelnetConnection, TelnetResult, TelnetServer, connect, terminal::TerminalEvent,
    };
    use futures::executor::block_on;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tokio::net::TcpListener;
    use tokio::time::sleep;
    use tracing::{error, trace};

    type Callback = dyn Fn(TelnetServer, TelnetConnection) -> bool;

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_manual_server() {
        // Shared state for clients to track received messages
        let received_messages = Arc::new(Mutex::new(Vec::new()));

        // --- 1. Setup Server ---
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let mut server = TelnetServer::create(listener).unwrap();

        // Server Logic: Broadcast any "LineCompleted" (message) to all other clients
        let server_handler = Arc::new(ServerCallbackHandler {
            on_startup: Some(Box::new(|_server| {
                trace!("Server Started");
            })),
            on_connect: Some(Box::new(|_server, _conn| {
                println!("Server: Client connected");
            })),
            on_event: ServerEventHandler::Single {
                on_event: Box::new(
                    |server: TelnetServer, sender: TelnetConnection, event: TerminalEvent| {
                        if let TerminalEvent::LineCompleted { line, .. } = event {
                            println!(
                                "Server received: '{}' from {}",
                                line.to_string(),
                                sender.id()
                            );
                            // Broadcast to all *other* connections
                            let server_clone = server.clone();
                            let message = format!("Echo: {}", line.to_string());

                            if let Err(err) = block_on(async move {
                                server_clone
                                    .broadcast(message.as_str(), Option::<Box<Callback>>::None)
                                    .await
                            }) {
                                error!("Broadcast error: {err:?}");
                            }
                        }
                    },
                ),
            },
            on_error: Some(Box::new(|_server, _conn, error| {
                trace!("Server: Client Error: {error}");
            })),
            on_timeout: Some(Box::new(|_server, _conn| {
                trace!("Server: Client Timed out");
            })),
            on_disconnect: Some(Box::new(|_server, _conn| {
                trace!("Server: Client Disconnected");
            })),
            on_shutdown: Some(Box::new(|_server| {
                trace!("Server: Server Shutdown");
            })),
        });

        server.run(server_handler, None).await.unwrap();

        // Give the server a moment to start
        sleep(Duration::from_millis(100)).await;

        // --- 2. Setup 3 Clients ---
        let mut clients = Vec::new();
        for _ in 0..3 {
            let received = received_messages.clone();

            let client_handler = Arc::new(ClientCallbackHandler {
                on_connect: Some(Box::new(|conn| {
                    trace!("Client {} Connected", conn.id());
                })),
                on_event: ClientEventHandler::Single {
                    on_event: Box::new(move |conn, event| {
                        if let TerminalEvent::LineCompleted { line, .. } = event {
                            let msg = line.to_string();
                            println!("Client {} received: {}", conn.id(), msg);
                            received.lock().unwrap().push(msg);
                        }
                    }),
                },
                on_error: Some(Box::new(|conn, error| {
                    trace!("Client {} Error: {}", conn.id(), error);
                })),
                on_timeout: Some(Box::new(|conn| {
                    trace!("Client {} Timed Out", conn.id());
                })),
                on_disconnect: Some(Box::new(|conn| {
                    trace!("Client {} Disconnected", conn.id());
                })),
            });

            clients.push(connect(addr, client_handler, None).await.unwrap());
        }

        // --- 3. Chat interaction ---
        // Send messages
        for (i, client) in clients.iter_mut().enumerate() {
            let msg = format!("Hello from client {}", i);
            let msg = msg.as_str();
            client.send(msg).await.unwrap();
            // Small delay to ensure processing order for the test assertions
            sleep(Duration::from_millis(50)).await;
        }

        // Wait for processing
        sleep(Duration::from_millis(500)).await;

        // --- 4. Validation & Shutdown ---
        let messages = received_messages.lock().unwrap();
        assert_eq!(messages.len(), 3, "Should have received 3 echo messages");
        assert!(messages.contains(&"Echo: Hello from client 0".to_string()));
        assert!(messages.contains(&"Echo: Hello from client 1".to_string()));
        assert!(messages.contains(&"Echo: Hello from client 2".to_string()));

        // The server loop runs indefinitely, so we abort the task to "shut down"
        server.shutdown().await.expect("TODO: Shutdown Panic");
    }
}
