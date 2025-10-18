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

mod client;
mod connection;
mod result;
mod server;
mod service;
mod svc;
mod svc2;

pub use self::client::TelnetClient;
pub use self::connection::Connection;
pub use self::result::{TelnetError, TelnetResult};
pub use self::server::TelnetServer;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::sync::Mutex;
    use tracing::error;
    use termionix_terminal::TerminalEvent;
    use crate::service::TelnetService;

    /// Echo service that broadcasts messages to all connected clients
    struct ChatEchoService {
        clients: Arc<Mutex<HashMap<u64, tokio::sync::mpsc::UnboundedSender<String>>>>,
        next_id: Arc<Mutex<u64>>,
    }

    impl ChatEchoService {
        fn new() -> Self {
            Self {
                clients: Arc::new(Mutex::new(HashMap::new())),
                next_id: Arc::new(Mutex::new(0)),
            }
        }

        async fn handle_connection(&self, mut terminal: Connection) {


            // Create channel for this client
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();

            // Register client
            {
                let mut clients = self.clients.lock().await;
                clients.insert(client_id, tx);
            }

            // Send welcome message
            let welcome = format!(
                "Welcome! You are client #{}. Type messages to echo to all clients.\n",
                client_id
            );
            if terminal.send(welcome.as_str()).await.is_err() {
                return;
            }

            let clients_clone = self.clients.clone();

            // Spawn task to handle incoming messages from other clients
            let mut terminal_clone = terminal.clone();
            tokio::spawn(async move {
                while let Some(msg) = rx.recv().await {
                    if terminal_clone.send(msg.as_str()).await.is_err() {
                        break;
                    }
                }
            });

            // Handle incoming messages from this client
            loop {
                match terminal.next().await {
                    Some(Ok(event)) => match event {
                        TerminalEvent::CharacterData { .. } => {
                        }
                        TerminalEvent::LineCompleted { line, .. } => {
                            let broadcast = format!("[Client #{}]: {}", client_id, line.stripped());

                            // Broadcast to all clients
                            let clients = clients_clone.lock().await;
                            for (id, tx) in clients.iter() {
                                if *id != client_id {
                                    let _ = tx.send(broadcast.clone());
                                }
                            }
                        }
                        TerminalEvent::Bell => {}
                        TerminalEvent::Clear { .. } => {}
                        TerminalEvent::EraseLine { .. } => {}
                        TerminalEvent::EraseCharacter { .. } => {}
                        TerminalEvent::NoOperation => {}
                        TerminalEvent::Break => {}
                        TerminalEvent::InterruptProcess => {}
                        TerminalEvent::CursorPosition { .. } => {}
                        TerminalEvent::ResizeWindow { .. } => {}
                        TerminalEvent::TelnetOptionStatus(_) => {}
                        TerminalEvent::MudServerData(_) => {}
                        TerminalEvent::MudServerStatus(_) => {}
                    }
                    Some(Err(err)) => {
                        error!("{}", err)
                    }
                    None => {
                        // Nothing
                    }
                }
            }

            // Unregister client
            let mut clients = self.clients.lock().await;
            clients.remove(&client_id);
        }
    }

    impl TelnetService for ChatEchoService {
        fn on_connect(&self, connection: &Connection) {
            // Assign client ID
            let client_id = {
                let mut id = self.next_id.lock().await;
                let current_id = *id;
                *id += 1;
                current_id
            };
            connection.
        }
    }

    #[tokio::test]
    async fn test_chat_echo_service() {
        // Setup server on a random available port
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let service = Arc::new(ChatEchoService::new());

        // Spawn server task
        let server_handle = tokio::spawn(async move {
            let server = TelnetServer::create(listener, service, None).unwrap();

            server.listen().await

        });

        // Give server time to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Connect first client
        let client1 = TelnetClient::connect(addr).await.unwrap();

        // Connect second client
        let client2 = TelnetClient::connect(addr).await.unwrap();
        let mut client2_terminal = client2.into_terminal();

        // Give clients time to receive welcome messages
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Read welcome message from client1
        let mut buffer1 = vec![0u8; 1024];
        let n1 = client1_terminal.read(&mut buffer1).await.unwrap();
        let welcome1 = String::from_utf8_lossy(&buffer1[..n1]);
        assert!(welcome1.contains("Welcome"));
        assert!(welcome1.contains("client #0"));

        // Read welcome message from client2
        let mut buffer2 = vec![0u8; 1024];
        let n2 = client2_terminal.read(&mut buffer2).await.unwrap();
        let welcome2 = String::from_utf8_lossy(&buffer2[..n2]);
        assert!(welcome2.contains("Welcome"));
        assert!(welcome2.contains("client #1"));

        // Client1 sends a message
        client1_terminal
            .write_all(b"Hello from client 1\n")
            .await
            .unwrap();

        // Give time for message to be processed
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Client2 should receive the message
        let n2 = client2_terminal.read(&mut buffer2).await.unwrap();
        let msg2 = String::from_utf8_lossy(&buffer2[..n2]);
        assert!(msg2.contains("[Client #0]"));
        assert!(msg2.contains("Hello from client 1"));

        // Client2 sends a message
        client2_terminal
            .write_all(b"Hello from client 2\n")
            .await
            .unwrap();

        // Give time for message to be processed
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Client1 should receive the message
        let n1 = client1_terminal.read(&mut buffer1).await.unwrap();
        let msg1 = String::from_utf8_lossy(&buffer1[..n1]);
        assert!(msg1.contains("[Client #1]"));
        assert!(msg1.contains("Hello from client 2"));

        // Cleanup
        drop(client1_terminal);
        drop(client2_terminal);
        server_handle.abort();
    }
}
