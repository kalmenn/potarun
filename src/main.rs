mod spoofer;
mod minecraft_server_runner;
mod mc_protocol;

use minecraft_server_runner::McServer;
use mc_protocol::{
    Codec,
    packets::{
        serverbound::{self, ServerboundPacket},
        clientbound::{self, ClientboundPacket},
    },
};

use std::net::SocketAddr;
use tokio::{
    net::{TcpListener},
    io,
    task,
};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let socket: SocketAddr = "127.0.0.1:6969".parse().expect("this should be a valid socket");
    loop {
        {
            let listener = TcpListener::bind(socket)
                .await
                .expect("Couldn't bind to TCP socket");
            println!("bound to tcp socket");

            let (sender, mut reciever) = tokio::sync::mpsc::channel::<()>(1);

            // We handle connections and loop until we recieve a Login request
            loop{if tokio::select!(
                Ok((stream, address)) = listener.accept() => {
                    let address = format!("\x1b[38;5;14m{}\x1b[0m", address);
                    println!("Connection from {}", address);

                    let sender = sender.clone();

                    task::spawn(async move {
                        match async {
                            let status = |message: &str| {
                                println!("{} → {}", address, message);
                            };

                            let mut codec = Codec::new(stream)?;

                            loop {match codec.read_packet().await? {
                                ServerboundPacket::Handshake(packet) => {
                                    status(&format!("Switching state to: {}", packet.next_state));
                                },
                                ServerboundPacket::Status(packet) => {match packet {
                                    serverbound::StatusPacket::StatusRequest{} => {
                                        status("Requested status");
                                        let json_response = serde_json::json!({
                                            "description": [
                                                {
                                                    "text": "Hors Ligne ...\n",
                                                    "color": "gold"
                                                },
                                                {
                                                    "text": "Connectez vous pour démarrer le serveur",
                                                    "color": "dark_green"
                                                }
                                            ],
                                            "players": {
                                                "max": 0,
                                                "online": 1,
                                                "sample": [
                                                    {
                                                        "name": "J'ai pas hacké je jure",
                                                        "id": "4566e69f-c907-48ee-8d71-d7ba5aa00d20"
                                                    }
                                                ]
                                            },
                                            "version": {
                                                "name": "1.19.2",
                                                "protocol": 760
                                            }
                                        }).to_string();

                                        println!("{json_response}");

                                        codec.send_packet(clientbound::StatusPacket::StatusResponse { json_response }).await?;
                                        status("Sent status");
                                    },
                                    serverbound::StatusPacket::PingRequest{ payload } => {
                                        status("Requested ping");
                                        codec.send_packet(clientbound::StatusPacket::PingResponse{payload}).await?;
                                        status("Sent pong");
                                    },
                                }},
                                // TODO: Login Requests
                            };}
                            io::Result::Ok(true)
                        }.await {
                            Ok(should_we_start) => {
                                println!("Closed connection to {address}");
                                if should_we_start {
                                    sender.send(()).await.expect("channel shouldn't close");
                                }
                            },
                            Err(err) => {
                                println!("Killed connection to {address} on error: {err}");
                            }
                        }
                    });
                    false
                },
                _ = reciever.recv() => {
                    // There should always be at least one sender alive.
                    // But just in case, we return anyway if we recieve None
                    true // We should start the server
                }
            ){break}}
        }
        {
            let mut server = McServer::with_args(
                "/bin/bash", 
                &[
                    "start.sh"
                ]
            ).unwrap();

            let exit_status = server.wait_for_exit().await.unwrap();
            println!("Server exited on status: {}", exit_status);
        }
    }
}