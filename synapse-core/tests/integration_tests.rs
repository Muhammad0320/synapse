use synapse_core::cortex::{CortexDataPlane, AgentThought};
use std::time::Instant;
use std::thread;
use std::time::Duration;

#[test]
fn test_zero_copy_swarm_speed() {
    let topic_name = "hospital_triage_swarm";
    let cortex = CortexDataPlane::new(topic_name);

    // 1. Create the Publisher (Agent A)
    let publisher = cortex.create_publisher().expect("Failed to create publisher");
    
    // 2. Create the Subscriber (Agent B)
    let subscriber = cortex.create_subscriber().expect("Failed to create subscriber");

    // Give the background iceoryx2 daemon 50ms to wire up the shared memory segments
    thread::sleep(Duration::from_millis(50));
    
    println!("Warming up the OS and Memory Allocator...");

    // ==============================
    // The warmup block

        let warmup_thought = AgentThought {
            agent_id: [0; 16],
            thought_id: 0, 
            payload_size: 0 
        };

        let init_sample = publisher.loan_uninit().expect("Failed to loan init memory");
        init_sample.write_payload(warmup_thought).send().expect("Failed to send init");

        // Spin until the subscriber recieves the warmup
        let mut warmed_up = false;
        while !warmed_up {
            if let Some(_) = subscriber.receive().expect("Failed to receive init") {
                warmed_up = true 
            }
      }

    // ============================

    println!("Bismillah. Starting Zero-Copy Swarm Benchmark...");

    let test_thought = AgentThought {
        agent_id: [1; 16], // Mock Agent UUID
        thought_id: 999,
        payload_size: 1024,
    };


    // Start the timer
    let start_time = Instant::now();

    // Agent A writes to physical RAM
    let sample = publisher.loan_uninit().expect("Failed to loan memory");
    let sample = sample.write_payload(test_thought);
    sample.send().expect("Failed to send thought");
     
    // Agent B reads from physical RAM
    let mut received = false;
    while !received {
        if let Some(sample) = subscriber.receive().expect("Failed to receive") {
            assert_eq!(sample.thought_id, 999);
            received = true;
        }
    }

    // Stop the timer
    let duration = start_time.elapsed();

    println!("========================================");
    println!("Agent Thought Transmitted & Verified!");
    println!("Latency: {:?}", duration);
    println!("========================================");
    
    // A standard socket takes ~100-200 microseconds. 
    // We expect this to be under 10 microseconds, often under 1 microsecond.
    assert!(duration.as_micros() < 50, "Latency too high for high-frequency agents!");
}
