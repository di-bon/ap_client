use std::thread;
use std::time::Duration;
use crossbeam_channel::{Receiver, Sender};
use messages::{ChatResponse, ErrorType, MediaRequest, MediaResponse, Message, MessageType, RequestType, ResponseType, ServerType, TextRequest, TextResponse};
use rand::{Rng, RngCore};
use wg_2024::network::NodeId;
use regex::Regex;
use crate::logic::{ClientCommand, ClientLogic, Getter};

pub struct Client {
    node_id: NodeId,
    client_logic_to_transmitter_tx: Sender<Message>,
    listener_to_client_logic_rx: Receiver<Message>,
    command_rx: Receiver<ClientCommand>,
    actions: Vec<Message>,
}

impl Getter for Client {
    fn get_node_id(&self) -> NodeId {
        self.node_id
    }

    fn get_server_command_rx(&self) -> &Receiver<ClientCommand> {
        &self.command_rx
    }

    fn get_listener_to_server_logic_rx(&self) -> &Receiver<Message> {
        &self.listener_to_client_logic_rx
    }

    fn get_server_logic_to_transmitter_tx(&self) -> &Sender<Message> {
        &self.client_logic_to_transmitter_tx
    }
}

impl ClientLogic for Client {
    fn process_response(&mut self, session_id: u64, source_id: NodeId, response_type: &ResponseType) {
        match response_type {
            ResponseType::TextResponse(text_response) => {
                self.process_text_response(source_id, text_response);
            }
            ResponseType::MediaResponse(media_response) => {
                self.process_media_response(source_id, media_response);
            }
            ResponseType::ChatResponse(chat_response) => {
                self.process_chat_response(source_id, chat_response);
            }
            ResponseType::DiscoveryResponse(server_type) => {
                self.process_discovery_response(source_id, server_type);
            }
        }
    }
}

impl Client {
    pub fn new(
        node_id: NodeId,
        client_logic_to_transmitter_tx: Sender<Message>,
        listener_to_client_logic_rx: Receiver<Message>,
        command_rx: Receiver<ClientCommand>,
        actions: Vec<Message>,
    ) -> Self {
        Self {
            node_id,
            client_logic_to_transmitter_tx,
            listener_to_client_logic_rx,
            command_rx,
            actions
        }
    }

    fn process_text_response(&mut self, source: NodeId, text_response: &TextResponse) {
        match text_response {
            TextResponse::TextList(list) => {
                log::info!("Received TextList: {list:?}");

                /*
                let mut rng = rand::rng();
                let text_resource = rng.random_range(0..list.len());
                let text_resource = list
                    .get(text_resource)
                    .unwrap_or_else(|_| panic!("Resource at index {text_resource} not found"));

                let session_id = rng.next_u64();
                let request = Message {
                    source: self.node_id,
                    destination: source,
                    session_id,
                    content: MessageType::Request(
                        RequestType::TextRequest(
                            TextRequest::Text(text_resource.clone())
                        )
                    ),
                };
                self
                    .client_logic_to_transmitter_tx
                    .send(request)
                    .unwrap_or_else(|_| panic!("Cannot communicate with transmitter"));
                 */
            }
            TextResponse::Text(text) => {
                let re = Regex::new(r"\{\{\s*([^{}\s]+\.png)\s*}}").unwrap();                let medias: Vec<String> = re
                    .find_iter(text)
                    .map(|mat| mat.as_str().to_string())
                    .collect();
                let mut rng = rand::rng();

                for media in medias {
                    let session_id = rng.next_u64();
                    let request = Message {
                        source: self.node_id,
                        destination: source,
                        session_id,
                        content: MessageType::Request(
                            RequestType::MediaRequest(
                                MediaRequest::Media(media)
                            )
                        ),
                    };
                    self
                        .client_logic_to_transmitter_tx
                        .send(request)
                        .unwrap_or_else(|_| panic!("Cannot communicate with transmitter"));
                }
            }
            TextResponse::NotFound(filename) => {
                log::warn!("File {filename} not found. Full response: {text_response:?}");
            }
        }
    }

    fn process_media_response(&mut self, source: NodeId, media_response: &MediaResponse) {
        match media_response {
            MediaResponse::MediaList(list) => {

            }
            MediaResponse::Media(media) => {
                Self::open_png_from_bytes(media.clone());
            }
            MediaResponse::NotFound(media_name) => {

            }
        }
    }

    fn open_png_from_bytes(png_data: Vec<u8>) -> std::io::Result<()> {
        use image::io::Reader as ImageReader;
        use std::process::Command;
        use std::env::temp_dir;

        // Decode PNG image
        let img = ImageReader::new(std::io::Cursor::new(png_data))
            .with_guessed_format()
            .unwrap()
            .decode()
            .unwrap();

        // Create a temporary file path
        let mut temp_path = temp_dir();
        temp_path.push("received_image.png");

        // Save the image to a file
        img.save(&temp_path).unwrap();

        // Open the image using the default system viewer
        #[cfg(target_os = "windows")]
        Command::new("cmd").args(&["/C", temp_path.to_str().unwrap()]).spawn()?;

        #[cfg(target_os = "macos")]
        Command::new("open").arg(temp_path.to_str().unwrap()).spawn()?;

        #[cfg(target_os = "linux")]
        Command::new("xdg-open").arg(temp_path.to_str().unwrap()).spawn()?;

        Ok(())
    }

    fn process_chat_response(&mut self, source: NodeId, chat_response: &ChatResponse) {
        match chat_response {
            ChatResponse::ClientList(list) => {

            }
            ChatResponse::MessageFrom { from, message } => {

            }
            ChatResponse::MessageSent => {

            }
        }
    }

    fn process_discovery_response(&mut self, source: NodeId, server_type: &ServerType) {

    }
}