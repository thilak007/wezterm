use crate::mux::{Mux, MuxNotification, MuxSubscriber};
use crate::server::codec::*;
use crate::server::listener::sessionhandler::SessionHandler;
use crate::server::pollable::*;
use anyhow::{bail, Context, Error};
use crossbeam::channel::TryRecvError;
use log::error;
use std::collections::HashSet;

pub struct ClientSession<S: ReadAndWrite> {
    stream: S,
    to_write_rx: PollableReceiver<DecodedPdu>,
    mux_rx: MuxSubscriber,
    handler: SessionHandler,
}

impl<S: ReadAndWrite> ClientSession<S> {
    pub fn new(stream: S) -> Self {
        let (to_write_tx, to_write_rx) =
            pollable_channel().expect("failed to create pollable_channel");
        let mux = Mux::get().expect("to be running on gui thread");
        let mux_rx = mux.subscribe().expect("Mux::subscribe to succeed");
        let handler = SessionHandler::new(to_write_tx);
        Self {
            stream,
            to_write_rx,
            mux_rx,
            handler,
        }
    }

    pub fn run(&mut self) {
        if let Err(e) = self.process() {
            error!("While processing session loop: {}", e);
        }
    }

    fn process(&mut self) -> Result<(), Error> {
        let mut read_buffer = Vec::with_capacity(1024);
        let mut tabs_to_output = HashSet::new();

        loop {
            loop {
                match self.to_write_rx.try_recv() {
                    Ok(decoded) => {
                        log::trace!("writing pdu with serial {}", decoded.serial);
                        decoded.pdu.encode(&mut self.stream, decoded.serial)?;
                        self.stream.flush().context("while flushing stream")?;
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => bail!("ClientSession was destroyed"),
                };
            }
            loop {
                match self.mux_rx.try_recv() {
                    Ok(notif) => match notif {
                        // Coalesce multiple TabOutputs for the same tab
                        MuxNotification::TabOutput(tab_id) => tabs_to_output.insert(tab_id),
                    },
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => bail!("mux_rx is Disconnected"),
                };
            }

            for tab_id in tabs_to_output.drain() {
                self.handler.schedule_tab_push(tab_id);
            }

            let mut poll_array = [
                self.to_write_rx.as_poll_fd(),
                self.stream.as_poll_fd(),
                self.mux_rx.as_poll_fd(),
            ];
            poll_for_read(&mut poll_array);

            if poll_array[1].revents != 0 || self.stream.has_read_buffered() {
                loop {
                    self.stream.set_non_blocking(true)?;
                    let res = Pdu::try_read_and_decode(&mut self.stream, &mut read_buffer);
                    self.stream.set_non_blocking(false)?;
                    match res {
                        Ok(Some(decoded)) => self.handler.process_one(decoded),
                        Ok(None) => break,
                        Err(err) => {
                            log::error!("Error decoding: {}", err);
                            return Err(err);
                        }
                    }
                }
            }
        }
    }
}
