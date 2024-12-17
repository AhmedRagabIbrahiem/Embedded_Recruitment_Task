use crate::message::EchoMessage;
use crate::message::{ClientMessage, client_message};
use crate::message::{ServerMessage, server_message, AddResponse};
use log::{error, info, warn};
use prost::Message;
use std::{
    io::{self, ErrorKind, Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

struct Client {
    stream: TcpStream,
}

impl Client {
    pub fn new(stream: TcpStream) -> Self {
        Client { stream }
    }

    pub fn handle(&mut self) -> io::Result<()> {
        let mut buffer = vec![0u8; 1024];
        // Read data from the client
        let bytes_read = self.stream.read(&mut buffer)?;
        println!("The buffer is: {:?}", &buffer[..bytes_read]);
        if bytes_read == 0 {
            println!("Client disconnected.");
            return Ok(());
        }
        let decoded_message = match ClientMessage::decode(&buffer[..bytes_read]) {
            Ok(msg) => msg, // Successfully decoded
            Err(e) => {
                eprintln!("Failed to decode ClientMessage: {}", e);
                return Ok(()); // Handle or exit on decoding failure
            }
        };
        
        match decoded_message.message {
            Some(client_message::Message::EchoMessage(echo)) => {
                println!("Received EchoMessage with content: {}", echo.content);
                // Echo back the same message in a ServerMessage
            let server_message = ServerMessage {
                message: Some(server_message::Message::EchoMessage(echo)),
            };

            let mut buffer = Vec::new();
            server_message.encode(&mut buffer).expect("Failed to encode ServerMessage");

            self.stream.write_all(&buffer)?;
            self.stream.flush()?;
            println!("Echoed back the message");
            }
            Some(client_message::Message::AddRequest(add_request)) => {
                println!(
                    "Received AddRequest with values a: {}, b: {}",
                    add_request.a, add_request.b
                );
                // Create the AddResponse
            let add_response = AddResponse {
                result: add_request.a + add_request.b,
            };

            // Wrap the response in a ServerMessage
            let server_message = ServerMessage {
                message: Some(server_message::Message::AddResponse(add_response)),
            };

            // Encode the ServerMessage
            let mut buffer = Vec::new();
            server_message.encode(&mut buffer).expect("Failed to encode ServerMessage");

            // Send the buffer back to the client
            self.stream.write_all(&buffer)?;
            self.stream.flush()?;
            println!("Sent AddResponse with result: {}", add_response.result);
            }
            None => {
                println!("No message received in ClientMessage.");
            }
        }

        Ok(())
    }
}

pub struct Server {
    listener: TcpListener,
    is_running: Arc<AtomicBool>,
}

impl Server {
    /// Creates a new server instance
    pub fn new(addr: &str) -> io::Result<Self> {
        let listener = TcpListener::bind(addr)?;
        let is_running = Arc::new(AtomicBool::new(false));
        Ok(Server {
            listener,
            is_running,
        })
    }

    /// Runs the server, listening for incoming connections and handling them
    pub fn run(&self) -> io::Result<()> {
        self.is_running.store(true, Ordering::SeqCst); // Set the server as running
        println!("Server is running on {}", self.listener.local_addr()?);

        // Set the listener to non-blocking mode
        self.listener.set_nonblocking(true)?;

        while self.is_running.load(Ordering::SeqCst) {
            match self.listener.accept() {
                Ok((stream, addr)) => {
                    info!("New client connected: {}", addr);

                    // Handle the client request
                    let mut client = Client::new(stream);
                    while self.is_running.load(Ordering::SeqCst) {
                        if let Err(e) = client.handle() {
                            error!("Error handling client: {}", e);
                            break;
                        }
                    }
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    // No incoming connections, sleep briefly to reduce CPU usage
                    thread::sleep(Duration::from_millis(100));
                }
                Err(e) => {
                    error!("Error accepting connection: {}", e);
                }
            }
        }

        info!("Server stopped.");
        Ok(())
    }

    /// Stops the server by setting the `is_running` flag to `false`
    pub fn stop(&self) {
        if self.is_running.load(Ordering::SeqCst) {
            self.is_running.store(false, Ordering::SeqCst);
            info!("Shutdown signal sent.");
        } else {
            warn!("Server was already stopped or not running.");
        }
    }
}
