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
        Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
    mem::drop
};

/// Client struct for handling individual connections
struct Client {
    stream: TcpStream,
}

impl Client {
    pub fn new(stream: TcpStream) -> Self {
        Client { stream }
    }

    pub fn handle(&mut self) -> io::Result<()> {
        let mut buffer = vec![0u8; 1024];
        loop {
        // Read data from the client
        let bytes_read = self.stream.read(&mut buffer)?;
        println!("The buffer is: {:?}", &buffer[..bytes_read]);
        if bytes_read == 0 {
            println!("Client disconnected.");
            break;
        }

        println!("Received buffer: {:?}", &buffer[..bytes_read]);

        // Decode and handle the message (as per your previous logic)
        let decoded_message = match ClientMessage::decode(&buffer[..bytes_read]) {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("Failed to decode ClientMessage: {}", e);
                return Ok(());
            }
        };

        // Match on the decoded message and send a response
        match decoded_message.message {
            Some(client_message::Message::EchoMessage(echo)) => {
                println!("Received EchoMessage: {}", echo.content);

                let response = ServerMessage {
                    message: Some(server_message::Message::EchoMessage(echo)),
                };
                self.send_response(response)?;
            }
            Some(client_message::Message::AddRequest(add_request)) => {
                println!(
                    "Received AddRequest with values a: {}, b: {}",
                    add_request.a, add_request.b
                );

                let response = ServerMessage {
                    message: Some(server_message::Message::AddResponse(AddResponse {
                        result: add_request.a + add_request.b,
                    })),
                };
                self.send_response(response)?;
            }
            None => println!("No message in ClientMessage"),
        }
    }
        Ok(())
    }

    fn send_response(&mut self, message: ServerMessage) -> io::Result<()> {
        let mut buffer = Vec::new();
        message.encode(&mut buffer).expect("Failed to encode ServerMessage");
        self.stream.write_all(&buffer)?;
        self.stream.flush()?;
        Ok(())
    }
}

pub struct Server {
    listener: TcpListener,
    is_running: Arc<AtomicBool>,
    thread_handles: Mutex<Vec<JoinHandle<()>>>,
}

impl Server {
    /// Creates a new server instance
    pub fn new(addr: &str) -> io::Result<Self> {
        let listener = TcpListener::bind(addr)?;
        let is_running = Arc::new(AtomicBool::new(false));
        let thread_handles = Mutex::new(Vec::new());
        Ok(Server {
            listener,
            is_running,
            thread_handles,
        })
    }

    /// Runs the server, listening for incoming connections and handling them in separate threads
    pub fn run(&self) -> io::Result<()> {
        self.is_running.store(true, Ordering::SeqCst); // Set the server as running
        println!("Server is running on {}", self.listener.local_addr()?);
    
        // Set the listener to non-blocking mode
        //self.listener.set_nonblocking(true)?;

        while self.is_running.load(Ordering::SeqCst) {
            match self.listener.accept() {
                Ok((stream, addr)) => {
                    println!("New client connected: {}", addr);
                    // Handle the client in a new thread
                    let thread_handle = std::thread::spawn(move || {
                        let mut client = Client::new(stream);
                        if let Err(e) = client.handle() {
                            eprintln!("Error handling client: {}", e);
                        }
                    });
                    self.thread_handles.lock().unwrap().push(thread_handle);
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No incoming connections, sleep briefly
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                Err(e) => {
                    eprintln!("Error accepting connection: {}", e);
                }
                
            }
    }
    println!("Stopping server... Waiting for threads to finish.");

    // After stopping the server, join all thread handles
    let mut handles = self.thread_handles.lock().unwrap();
    while let Some(thread_handle) = handles.pop() {
        if let Err(e) = thread_handle.join() {
            eprintln!("Error joining thread: {:?}", e);
        }
    }
            
        println!("Server stopped.");
        Ok(())
    }

    /// Stops the server by setting the `is_running` flag to `false`
    pub fn stop(&self) {
        if self.is_running.load(Ordering::SeqCst) {
            self.is_running.store(false, Ordering::SeqCst);
            println!("Shutdown signal sent.");
        } else {
            println!("Server was already stopped or not running.");
        }
    }
        
}
