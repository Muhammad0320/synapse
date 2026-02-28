pub mod cortex;

use rkyv::{Archive, Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

use rkyv::to_bytes;

// The fundamental unit of our Flight Recorder.
#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
pub struct AgentState {
    pub transaction_id: u64,
    pub timestamp: i64,
    pub status: AgentStatus, 

    // We will map this to Loro's CRDTs later to track var
    pub memory_offset: u32  
}

// The current execution state of the agent in the swarm.
#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
pub enum AgentStatus {
    Idle, 
    Reasoning, // Waiting on LLM token generation
    ToolExecution, // Executing an external API or tool
    Halted,     // Interdicted by the Aegis security layer   
}

// Every thought and action is an Op.
#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
pub struct OpLog {

    pub agent_id: [u8; 16],
    pub state: AgentState,

    // This payload will hold the actual memory delta (the thought or action)
    pub payload_size:  u32, 

}

// ---------------------- WAL
pub struct WalMessage {

    pub log: OpLog,

    pub callback: oneshot::Sender<()>,

}

pub struct WalEngine {
    sender: mpsc::Sender<WalMessage>
}

impl WalEngine {

    // Bootsraps the WAL backgound thread
    pub async fn start(file_path: &str) -> Self {

        let (tx, mut rx) = mpsc::channel::<WalMessage>(100_000);

        let mut file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(file_path)
                    .await
                    .expect("Failed to open WAL file");

        // Spawns the dedicated background thread for Group Commiting 
        tokio::spawn( async move {

            let mut batch = Vec::new();
            let mut callbacks = Vec::new(); 

            loop {

                // Wait for the first though to arrive
                if let Some(msg) = rx.recv().await {

                    batch.push(msg.log);
                    callbacks.push(msg.callback);

                    // Drain everything else currently in the queue (The Batching)
                    while let Ok(msg) = rx.try_recv() {

                        batch.push(msg.log);
                        callbacks.push(msg.callback);

                    }

                    // Zero-copy Serialize the entire batch using rkyv to_bytes converts the data to raw bytes instantly
                    let bytes = to_bytes::<rkyv::rancor::Error>(&batch).expect("Failed to serialize batch");

                    // 1. Write the batch to the OS buffer
                    if let Err(e) = file.write_all(&bytes).await {

                        eprintln!("WAL Write Error: {}", e);

                        continue;
                    }

                    // 2. Force to OS to write to the physical metal (The durability guarantee)
                    if let Err(e) = file.sync_data().await {

                        eprintln!("WAL Sync Error: {}", e); 
                        continue;

                    }

                    // 3. Wake up all the agents!
                    for cb in callbacks.drain(..) {
                        let _ = cb.send(());
                    }
                    batch.clear();
                }
            }
        } );

        WalEngine {sender: tx}
    }

    /// The function the Agent calls to save its thought
    pub async fn append(&self, log: OpLog) {

        let (tx, rx) = oneshot::channel();

        let msg = WalMessage {log, callback: tx};

        // Send to the WAL thread
        let _ = self.sender.send(msg).await;

        // The agent pauses here *only* until the disk write is confirmed
        let _ = rx.await;
    }

}