use futures::channel::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time;
use thread_id;
use ws::{CloseCode, Handler, Message};

use bo::*;
use db_ops::*;
use process_request::*;
use security::*;

const TO_CLOSE: &'static str = "##CLOSE##";

// Server WebSocket handler
struct Server {
    out: ws::Sender,
    sender: Sender<String>,
    dbs: Arc<Databases>,
    db: Arc<SelectedDatabase>,
    client: Client,
}

impl Handler for Server {
    fn on_open(&mut self, _: ws::Handshake) -> ws::Result<()> {
        let ws_sender = self.out.clone();
        let (sender, mut receiver): (Sender<String>, Receiver<String>) = channel(100);
        self.sender = sender;
        let _read_thread = thread::spawn(move || loop {
            match receiver.try_next() {
                Ok(message) => match message {
                    Some(message) => match message.as_ref() {
                        TO_CLOSE => {
                            println!("Closing server connection");
                            break;
                        }
                        message => {
                            match ws_sender.send(message) {
                                Ok(_) => {}
                                Err(e) => println!("ws_ops::_read_thread::send::Error {}", e),
                            };
                        }
                    },
                    None => {
                        println!("ws_ops::_read_thread::error::None");
                    }
                },
                _ => thread::sleep(time::Duration::from_millis(2)),
            }
        });
        Ok(())
    }

    fn on_message(&mut self, msg: Message) -> ws::Result<()> {
        let message = msg.as_text().unwrap();
        println!(
            "[{}] Server got message '{}'. ",
            thread_id::get(),
            clean_string_to_log(&message, &self.dbs)
        );
        match process_request(
            &message,
            &mut self.sender,
            &self.db,
            &self.dbs,
            &mut self.client,
        ) {
            Response::Error { msg } => {
                println!("Error: {}", msg);
                match self.sender.try_send(format!("error {} \n", msg)) {
                    Ok(_) => {}
                    Err(e) => println!(
                        "ws_ops::_read_thread::process_request::try_send::Error {}",
                        e
                    ),
                }
            }
            _ => {
                match self.sender.try_send(format!("ok \n")) {
                    Ok(_) => {}
                    Err(e) => println!(
                        "ws_ops::_read_thread::process_request::_::try_send::Error {}",
                        e
                    ),
                }
                println!("ws::Success processed");
            }
        }

        Ok(())
    }

    fn on_close(&mut self, code: CloseCode, reason: &str) {
        println!("WebSocket closing for ({:?}) {}", code, reason);
        match self.sender.try_send(TO_CLOSE.to_string()) {
            //To close the read thread
            Ok(_) => {}
            Err(e) => println!("on_close::Error {}", e),
        }
        process_request(
            "unwatch-all",
            &mut self.sender,
            &self.db,
            &self.dbs,
            &mut self.client,
        );
    }
}

pub fn start_web_socket_client(dbs: Arc<Databases>, ws_address: Arc<String>) {
    let ws_address = ws_address.to_string();
    println!("Starting the web socket client with addr: {}", ws_address);
    let server = thread::spawn(move || {
        let (sender, _): (Sender<String>, Receiver<String>) = channel(100);
        ws::Builder::new()
            .with_settings(ws::Settings {
                max_connections: 100000,
                ..ws::Settings::default()
            })
            .build(move |out| Server {
                out,
                db: create_temp_selected_db("init".to_string()),
                dbs: dbs.clone(),
                sender: sender.clone(),
                client: Client::new_empty(),
            })
            .unwrap()
            .listen(ws_address)
            .unwrap()
    });

    println!("WebSocket started ");
    let _ = server.join();
}
