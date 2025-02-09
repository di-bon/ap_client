extern crate core;

use std::collections::HashMap;
use std::{panic, thread};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use ap_listener::{Command as ListenerCommand, Listener};
use ap_sc_notifier::SimulationControllerNotifier;
use ap_transmitter::{Command as TransmitterCommand, Transmitter};
use crossbeam_channel::{unbounded, Receiver, Sender};
use messages::Message;
use messages::node_event::NodeEvent;
use wg_2024::controller::DroneCommand;
use wg_2024::network::NodeId;
use wg_2024::packet::{NodeType, Packet};
use crate::client::Client;
use crate::logic::{ClientCommand, ClientLogic, Getter};

mod logic;
mod client;

pub enum Command {
    Quit,
}

pub struct DibClient {
    node_id: NodeId,
    listener: Arc<Mutex<Listener>>,
    listener_command_tx: Sender<ListenerCommand>,
    logic: Arc<Mutex<Client>>,
    logic_command_tx: Sender<ClientCommand>,
    transmitter: Arc<Mutex<Transmitter>>,
    transmitter_command_tx: Sender<TransmitterCommand>,
    command_rx: Receiver<Command>,
}

impl DibClient {
    #[must_use]
    pub fn new_dib_client(
        node_id: NodeId,
        listener_rx: Receiver<Packet>,
        drones_tx: HashMap<NodeId, Sender<Packet>>,
        simulation_controller_tx: Sender<NodeEvent>,
        drone_command_rx: Receiver<DroneCommand>,
        actions: Vec<Message>,
    ) -> (Self, Sender<Command>) {
        let (listener_to_transmitter_tx, listener_to_transmitter_rx) = unbounded();
        let (listener_to_server_logic_tx, listener_to_server_logic_rx) = unbounded();
        let (logic_to_transmitter_tx, logic_to_transmitter_rx) = unbounded();
        let (listener_command_tx, listener_command_rx) = unbounded();

        let simulation_controller_notifier =
            SimulationControllerNotifier::new(simulation_controller_tx);
        let simulation_controller_notifier = Arc::new(simulation_controller_notifier);

        let (transmitter_command_tx, transmitter_command_rx) = unbounded();

        let transmitter = Transmitter::new(
            node_id,
            NodeType::Server,
            listener_to_transmitter_rx,
            logic_to_transmitter_rx,
            drones_tx,
            simulation_controller_notifier.clone(),
            transmitter_command_rx,
            Duration::from_secs(60),
            drone_command_rx,
        );

        let listener = Listener::new(
            node_id,
            listener_to_transmitter_tx,
            listener_to_server_logic_tx,
            listener_rx,
            listener_command_rx,
            simulation_controller_notifier.clone(),
        );

        let (logic_command_tx, logic_command_rx) = unbounded();

        let logic = Client::new(
            node_id,
            logic_to_transmitter_tx,
            listener_to_server_logic_rx,
            logic_command_rx,
            actions,
        );

        assert_eq!(transmitter.get_node_id(), listener.get_node_id());
        assert_eq!(transmitter.get_node_id(), logic.get_node_id());

        let transmitter = Arc::new(Mutex::new(transmitter));
        let listener = Arc::new(Mutex::new(listener));
        let logic = Arc::new(Mutex::new(logic));

        let (command_tx, command_rx) = unbounded();

        let result = Self {
            node_id,
            listener,
            listener_command_tx,
            logic,
            logic_command_tx,
            transmitter,
            transmitter_command_tx,
            command_rx,
        };

        (result, command_tx)
    }

    pub fn run(&mut self) {
        panic::set_hook(Box::new(|info| {
            let panic_msg = format!("Panic occurred: {info}");
            log::error!("{panic_msg}");
            eprintln!("{panic_msg}");
        }));
    }
}

pub trait DibGetter {
    fn get_node_id(&self) -> NodeId;

    fn get_listener(&self) -> Arc<Mutex<Listener>>;

    fn get_listener_tx(&self) -> &Sender<ListenerCommand>;

    fn get_logic(&self) -> Arc<Mutex<Client>>;

    fn get_logic_tx(&self) -> &Sender<ClientCommand>;

    fn get_transmitter(&self) -> Arc<Mutex<Transmitter>>;

    fn get_transmitter_tx(&self) -> &Sender<TransmitterCommand>;

    fn get_command_rx(&self) -> &Receiver<Command>;
}

impl DibGetter for DibClient {
    fn get_node_id(&self) -> NodeId {
        self.node_id
    }

    fn get_listener(&self) -> Arc<Mutex<Listener>> {
        self.listener.clone()
    }

    fn get_listener_tx(&self) -> &Sender<ListenerCommand> {
        &self.listener_command_tx
    }

    fn get_logic(&self) -> Arc<Mutex<Client>> {
        self.logic.clone()
    }

    fn get_logic_tx(&self) -> &Sender<ClientCommand> {
        &self.logic_command_tx
    }

    fn get_transmitter(&self) -> Arc<Mutex<Transmitter>> {
        self.transmitter.clone()
    }

    fn get_transmitter_tx(&self) -> &Sender<TransmitterCommand> {
        &self.transmitter_command_tx
    }

    fn get_command_rx(&self) -> &Receiver<Command> {
        &self.command_rx
    }
}

pub trait DibServerTrait: DibGetter {
    /// Starts the client
    /// # Panics
    /// - Panics if the transmitter thread cannot acquire the lock on the transmitter
    /// - Panics if the listener thread cannot acquire the lock on the listener
    /// - Panics if the server logic thread cannot acquire the lock on the server logic
    fn run(&mut self) {
        panic::set_hook(Box::new(|info| {
            let panic_msg = format!("Panic occurred: {info}");
            log::error!("{panic_msg}");
            eprintln!("{panic_msg}");
        }));

        let listener = self.get_listener().clone();

        let listener_handle = thread::Builder::new()
            .name(format!("client_{}_listener", self.get_node_id()))
            .spawn(move || {
                let mut listener = match listener.lock() {
                    Ok(listener) => listener,
                    Err(err) => {
                        panic!("Error while starting listener: {err:?}");
                    }
                };
                listener.run();
            })
            .unwrap_or_else(|_| {
                panic!(
                    "Cannot spawn a new thread 'client_{}_listener'",
                    self.get_node_id()
                )
            });

        let transmitter = self.get_transmitter().clone();
        let transmitter_handle = thread::Builder::new()
            .name(format!("client_{}_transmitter", self.get_node_id()))
            .spawn(move || {
                let mut transmitter = match transmitter.lock() {
                    Ok(transmitter) => transmitter,
                    Err(err) => {
                        panic!("Error while starting transmitter: {err:?}");
                    }
                };
                transmitter.run();
            })
            .unwrap_or_else(|_| {
                panic!(
                    "Cannot spawn a new thread 'client_{}_transmitter'",
                    self.get_node_id()
                )
            });

        let logic = self.get_logic().clone();
        let client_logic_handle = thread::Builder::new()
            .name(format!("client_{}_logic", self.get_node_id()))
            .spawn(move || {
                let mut logic = match logic.lock() {
                    Ok(logic) => logic,
                    Err(err) => {
                        panic!("Error while starting logic: {err:?}");
                    }
                };
                logic.run();
            })
            .unwrap_or_else(|_| {
                panic!(
                    "Cannot spawn a new thread 'client_{}_logic'",
                    self.get_node_id()
                )
            });

        #[allow(clippy::never_loop)]
        'command_loop: loop {
            let command = self.get_command_rx().recv();
            match command {
                Ok(command) => match command {
                    Command::Quit => {
                        let command = ListenerCommand::Quit;
                        self.get_listener_tx()
                            .send(command)
                            .unwrap_or_else(|_| panic!("Cannot communicate with listener thread"));

                        let command = ClientCommand::Quit;
                        self.get_logic_tx()
                            .send(command)
                            .unwrap_or_else(|_| panic!("Cannot communicate with logic thread"));

                        let command = TransmitterCommand::Quit;
                        self.get_transmitter_tx().send(command).unwrap_or_else(|_| {
                            panic!("Cannot communicate with transmitter thread")
                        });

                        break 'command_loop;
                    }
                },
                Err(error) => {
                    let error = format!("Error while receiving Command's. Error: {error:?}");
                    log::error!("{error}");
                    panic!("{error}");
                }
            }
        }

        let _ = listener_handle.join();
        let _ = client_logic_handle.join();
        let _ = transmitter_handle.join();
    }
}

impl DibServerTrait for DibClient {}